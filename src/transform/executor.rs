use algorithmia::Algorithmia;
//use common::ffmpeg::FFMpeg;
//use common::ffmpeg;
use std::path::*;
use common::file_mgmt;
use rayon::prelude::*;
use rayon;
use serde_json::Value;
use super::functions::{advanced_batch, advanced_single};
use super::utilities::advanced_input_search;
use common::video_error::VideoError;
use common::rayon_stuff::{try_algorithm_default, try_algorithm_advanced};
use common::misc;
use common::structs::prelude::*;
use common::json_utils::AdvancedInput;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::ops::*;
use std::io::{self, Write};
use std_semaphore::Semaphore;
use std::ascii::AsciiExt;

static FPSMAX: f64 = 60f64;
//used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
               batch_size: usize,
               starting_threads: isize,
               function: &(Fn(&Alter, Vec<usize>, Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError> + Sync)) -> Result<Altered, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(misc::frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let mut semaphore_global: Arc<Semaphore> = Arc::new(Semaphore::new(starting_threads));
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Alter::new(client.clone(),
                                             data.regex().to_owned(),
                                             output_regex.to_owned(),
                                             local_out_dir.to_owned(),
                                             data.frames_dir().to_owned(),
                                             remote_dir.to_owned()));
    frame_batches.par_iter().map(move |batch| {
        let error_lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let time = time_global.clone();
        try_algorithm_default(function, &formatted_data, &batch, semaphore, error_lock, time)
    }).weight_max().collect_into(&mut result);
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
                input: &Value) -> Result<Altered, VideoError> {
    let search: Arc<AdvancedInput> = Arc::new(advanced_input_search(input)?);
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let semaphore_global: Arc<Semaphore> = Arc::new(Semaphore::new(starting_threads));
    let early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let frame_batches = if search.option() == "batch" { misc::frame_batches(batch_size, data.num_frames()) } else { misc::frame_batches(1, data.num_frames()) };
    let time_global: Arc<Mutex<SystemTime>> = Arc::new(Mutex::new(SystemTime::now()));
    let formatted_data = Arc::new(Alter::new(client.clone(),
                                             data.regex().to_owned(),
                                             output_regex.to_owned(),
                                             local_out_dir.to_owned(),
                                             data.frames_dir().to_owned(),
                                             remote_dir.to_owned()));

    io::stderr().write(b"starting parallel map.\n")?;
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        let semaphore = semaphore_global.clone();
        let time = time_global.clone();
        if search.option() == "batch" {
            io::stderr().write(b"found batch mode.\n")?;
            try_algorithm_advanced(&advanced_batch, &formatted_data, &batch,
                                   algorithm, &search, semaphore, lock, time)
        } else {
            io::stderr().write(b"found single mode.\n")?;
            try_algorithm_advanced(&advanced_single, &formatted_data, &batch,
                                   algorithm, &search, semaphore, lock, time)
        }
    }).weight_max().collect_into(&mut result);
    try!(io::stderr().write(b"exited parallel map.\n"));
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}
