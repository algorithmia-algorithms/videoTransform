use algorithmia::Algorithmia;
use std::path::*;
use common::video_error::VideoError;
use common::structs::prelude::*;
use super::functions;
use common::json_utils::combine_data_extract;
use rayon::prelude::*;
use common::rayon_stuff::{try_algorithm_default, try_algorithm_advanced, prepare_semaphore};
use common::watchdog::Watchdog;
use std::time::{SystemTime, Duration};
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic};
use std_semaphore::Semaphore;
use std::ops::*;
use std::ascii::AsciiExt;
use common::misc::frame_batches;
use std::io;

static FPSMAX: f64 = 60f64;

pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               batch_size: usize,
               duration: f64,
               starting_threads: isize,
                max_threads: isize,
               function: &(Fn(&Extract, Vec<usize>, Arc<Semaphore>) -> Result<Vec<Value>, VideoError> + Sync)) -> Result<Value, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<Value>, VideoError>> = Vec::new();
    let mut slowdown = atomic::AtomicBool::new(false);
    let mut slowdown_signal_global: Arc<atomic::AtomicBool> = Arc::new(slowdown);
    let semaphore_global: Arc<Semaphore> = prepare_semaphore(starting_threads, max_threads);
    let early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Extract::new(client.clone(),
                                               data.regex().clone(),
                                               data.frames_dir().clone(),
                                               remote_dir.clone()));

    frame_batches.par_iter().map(move |batch| {
        let error_lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let mut slowdown_signal = slowdown_signal_global.clone();
        let time = time_global.clone();
        try_algorithm_default(function, &formatted_data, &batch, semaphore, slowdown_signal, error_lock, time)
    }).weight_max().collect_into(&mut result);
    let processed_frames: Vec<Value> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    let processed: Value = combine_data_extract(&processed_frames, frame_stamp)?;

    Ok(processed)
}

pub fn advanced(client: &Algorithmia,
                data: &Scattered,
                remote_dir: &str,
                algorithm: &str,
                batch_size: usize,
                duration: f64,
                starting_threads: isize,
                max_threads: isize,
                input: AdvancedInput) -> Result<Value, VideoError> {
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let search: Arc<AdvancedInput> = Arc::new(input);
    let frame_batches = if search.option() == "batch" { frame_batches(batch_size, data.num_frames()) }
        else { frame_batches(1, data.num_frames()) };
    let mut slowdown = atomic::AtomicBool::new(false);
    let mut slowdown_signal_global: Arc<atomic::AtomicBool> = Arc::new(slowdown);
    let mut result: Vec<Result<Vec<Value>, VideoError>> = Vec::new();
    let semaphore_global: Arc<Semaphore> = Arc::new(Semaphore::new(starting_threads));
    let early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Extract::new(client.clone(),
                                               data.regex().clone(),
                                               data.frames_dir().clone(),
                                               remote_dir.clone()));

    let wd = Watchdog::create(early_terminate.clone(), frame_batches.len());
    let wd_t = wd.get_comms();

    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let mut slowdown_signal = slowdown_signal_global.clone();
        let time = time_global.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&functions::advanced_batch, &formatted_data, &batch,
                                   algorithm, &search, semaphore, slowdown_signal, lock, time)
        } else {
            try_algorithm_advanced(&functions::advanced_single, &formatted_data, &batch,
                                   algorithm, &search, semaphore, slowdown_signal, lock, time)
        };
        wd_t.send_success_signal();
        res
    }).weight_max().collect_into(&mut result);
    wd.terminate();
    let processed_frames: Vec<Value> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    let processed: Value = combine_data_extract(&processed_frames, frame_stamp)?;
    Ok(processed)
}

