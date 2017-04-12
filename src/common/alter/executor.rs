use algorithmia::Algorithmia;
//use common::ffmpeg::FFMpeg;
//use common::ffmpeg;
use std::path::*;
use common::file_mgmt;
use rayon::prelude::*;
use rayon;
use serde_json::Value;
use common::video_error::VideoError;
use common::structs::alter::{Alter, Altered};
use common::rayon_stuff::{try_algorithm_default, try_algorithm_advanced};
use common::alter::functions;
use common::structs::scattered::Scattered;
use common::structs::gathered::Gathered;
use common::utilities::*;
use common::json_utils::{SearchResult, alter_format_search, combine_extracted_data};
use std::sync::{Arc, Mutex, RwLock};
use std::ops::*;
use std::io::{self, Write};
use std_semaphore;
use std::ascii::AsciiExt;
static FPSMAX: f64 = 60f64;
use common::utilities;
 //used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
                batch_size: usize,
               starting_threads: isize,
               function: &(Fn(&Alter, Vec<usize>) -> Result<Vec<PathBuf>, VideoError> + Sync)) -> Result<Altered, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(utilities::frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let mut semaphore_global: Arc<RwLock<std_semaphore::Semaphore>> = Arc::new(RwLock::new(std_semaphore::Semaphore::new(starting_threads)));
    let mut locked_threads_global: Arc<RwLock<isize>> = Arc::new(RwLock::new(0));
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let formatted_data = Arc::new(Alter::new(client.clone(),
                                           data.regex().to_owned(),
                                           output_regex.to_owned(),
                                           local_out_dir.to_owned(),
                                           data.frames_dir().to_owned(),
                                           remote_dir.to_owned()));
    frame_batches.par_iter().map(move |batch| {
        let error_lock = early_terminate.clone();
        let locked_thread = locked_threads_global.clone();
        let semaphore = semaphore_global.clone();
        try_algorithm_default(function, &formatted_data, &batch, locked_thread, starting_threads, semaphore, error_lock)
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
    let search: Arc<SearchResult> = Arc::new(try!(alter_format_search(input)));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    let mut semaphore_global: Arc<RwLock<std_semaphore::Semaphore>> = Arc::new(RwLock::new(std_semaphore::Semaphore::new(starting_threads)));
    let mut locked_threads_global: Arc<RwLock<isize>> = Arc::new(RwLock::new(starting_threads));
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let frame_batches = if search.option() == "batch" {utilities::frame_batches(batch_size, data.num_frames())}
        else {utilities::frame_batches(1, data.num_frames())};

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
        let locked_threads = locked_threads_global.clone();
        if search.option() == "batch" {
            io::stderr().write(b"found batch mode.\n")?;
            try_algorithm_advanced(&functions::advanced_batch, &formatted_data, &batch,
                                   algorithm, &search, locked_threads, starting_threads, semaphore, lock) }
            else {
                io::stderr().write(b"found single mode.\n")?;
                try_algorithm_advanced(&functions::advanced_single, &formatted_data, &batch,
                                       algorithm, &search, locked_threads, starting_threads, semaphore, lock)
            }
    }).weight_max().collect_into(&mut result);
    try!(io::stderr().write(b"exited parallel map.\n"));
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}
