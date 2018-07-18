use algorithmia::Algorithmia;
use std::path::*;
use common::file_mgmt;
use rayon::prelude::*;
use rayon;
use serde_json::Value;
use super::functions::{advanced_batch, advanced_single};
use common::video_error::VideoError;
use common::watchdog::Watchdog;
use common::rayon_stuff::{try_algorithm_default, try_algorithm_advanced, prepare_semaphore};
use common::misc;
use common::structs::prelude::*;
use std::sync::{Arc, Mutex, atomic};
use std::time::SystemTime;
use std::ops::*;
use std::io::{self, Write};
use std_semaphore::Semaphore;
use std::ascii::AsciiExt;

//used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
               batch_size: usize,
               starting_threads: isize,
                max_threads: isize,
               function: &(Fn(&Alter, Vec<usize>, Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError> + Sync)) -> Result<Altered, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(misc::frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let semaphore_global: Arc<Semaphore> = prepare_semaphore(starting_threads, max_threads);
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let mut slowdown = atomic::AtomicBool::new(false);
    let mut slowdown_signal_global: Arc<atomic::AtomicBool> = Arc::new(slowdown);
    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Alter::new(client.clone(),
                                             data.regex().clone(),
                                             output_regex.clone(),
                                             local_out_dir.clone(),
                                             data.frames_dir().clone(),
                                             remote_dir.clone()));
    let wd = Watchdog::create(early_terminate.clone(), frame_batches.len());
    let wd_t = wd.get_comms();
    frame_batches.par_iter().map(move |batch| {
        let error_lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let mut slowdown_signal = slowdown_signal_global.clone();
        let time = time_global.clone();
        let res = try_algorithm_default(function, &formatted_data, &batch, semaphore, slowdown_signal, error_lock, time);
        wd_t.send_success_signal();
        res
    }).weight_max().collect_into(&mut result);
    wd.terminate();
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}


pub fn advanced(client: &Algorithmia,
                data: &Scattered,
                remote_dir: &str,
                local_out_dir: &Path,
                output_regex: &str,
                algorithm: &str,
                batch_size: usize,
                starting_threads: isize,
                max_threads: isize,
                input: AdvancedInput) -> Result<Altered, VideoError> {
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let search: Arc<AdvancedInput> = Arc::new(input);
    let mut semaphore_global: Arc<Semaphore> = prepare_semaphore(starting_threads, max_threads);
    let semaphore_global: Arc<Semaphore> = Arc::new(Semaphore::new(starting_threads));
    let early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let mut slowdown = atomic::AtomicBool::new(false);
    let mut slowdown_signal_global: Arc<atomic::AtomicBool> = Arc::new(slowdown);

    let frame_batches = if search.option() == "batch" { misc::frame_batches(batch_size, data.num_frames()) }
        else { misc::frame_batches(1, data.num_frames()) };

    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Alter::new(client.clone(),
                                             data.regex().clone(),
                                             output_regex.clone(),
                                             local_out_dir.clone(),
                                             data.frames_dir().clone(),
                                             remote_dir.clone()));
    let wd = Watchdog::create(early_terminate.clone(), frame_batches.len());
    let wd_t = wd.get_comms();
    io::stderr().write(b"starting parallel map.\n")?;
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let time = time_global.clone();
        let mut slowdown_signal = slowdown_signal_global.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&advanced_batch, &formatted_data, &batch,
                                   algorithm, &search, semaphore, slowdown_signal, lock, time)
        } else {
            try_algorithm_advanced(&advanced_single, &formatted_data, &batch,
                                   algorithm, &search, semaphore, slowdown_signal, lock, time)
        };
        wd_t.send_success_signal();
        res
    }).weight_max().collect_into(&mut result);
    wd.terminate();
    io::stderr().write(b"exited parallel map.\n")?;
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}
