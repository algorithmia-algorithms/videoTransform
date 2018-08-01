use algorithmia::Algorithmia;
use std::path::*;
use rayon::prelude::*;
use super::functions::{advanced_batch, advanced_single};
use common::video_error::VideoError;
use common::watchdog::Watchdog;
use common::threading::*;
use common::misc;
use common::structs::prelude::*;
use std::sync::Arc;
use std::io::{self, Write};

//used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
pub fn default(
               data: Alter,
               number_of_frames: usize,
               fps: f64,
               batch_size: usize,
               starting_threads: isize,
                max_threads: isize,
               function: &Default<Alter, PathBuf>) -> Result<Altered, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = misc::frame_batches_simple(batch_size, number_of_frames);
    let mut result: Vec<Result<Vec<PathBuf>, ()>> = Vec::new();
    let out_dir = PathBuf::from(data.local_output());
    let out_regex = data.output_regex().to_string();
    let global_threadable = Threadable::create(starting_threads, max_threads, data);
    let inner_threadable = global_threadable.clone();

    let wd = Watchdog::create(global_threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();
    frame_batches.par_iter().map(move |batch| {
        let res = try_algorithm_default(function, &batch, &inner_threadable);
        if res.is_ok() {
            wd_t.send_success_signal();
        }
        res
    }).weight(1f64).collect_into(&mut result);
    wd.terminate();
    let signal = global_threadable.extract_term_signal();
    match signal {
        None => {
            println!("failed");
            let processed_frames: Vec<PathBuf> = result.into_iter().collect::<Result<Vec<Vec<_>>, _>>().unwrap().concat();
            let out = Altered::new(PathBuf::from(out_dir), processed_frames, fps, out_regex.to_string());
            Ok(out)
        }
        Some(err) => Err(format!("error, video processing failed: {}", err).into())
    }
}


pub fn advanced(data: Alter,
                number_of_frames: usize,
                fps: f64,
                algorithm: &str,
                batch_size: usize,
                starting_threads: isize,
                max_threads: isize,
                ain: AdvancedInput) -> Result<Altered, VideoError> {
    let mut result: Vec<Result<Vec<PathBuf>, ()>> = Vec::new();
    let search: Arc<AdvancedInput> = Arc::new(ain);
    let frame_batches = misc::frame_batches_advanced(batch_size, number_of_frames, search.option());
    let out_dir = PathBuf::from(data.local_output());
    let out_regex = data.output_regex().to_string();
    let global_threadable = Threadable::create(starting_threads, max_threads, data);
    let inner_threadable = global_threadable.clone();
    let wd = Watchdog::create(global_threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();
    io::stderr().write(b"starting parallel map.\n")?;
    frame_batches.par_iter().map(move |batch| {
//        let thread_t = inner_threadable.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&advanced_batch, &batch,
                                   algorithm, &search, &inner_threadable)
        } else {
            try_algorithm_advanced(&advanced_single, &batch,
                                   algorithm, &search, &inner_threadable)
        };
        if res.is_ok() {
            wd_t.send_success_signal();
        }
        res
    }).collect_into(&mut result);
    wd.terminate();
    println!("exited parallel map.");
    let signal = global_threadable.extract_term_signal();
    match signal {
        None => {
            println!("we detected no failure");
            let processed_frames: Vec<PathBuf> = result.into_iter().collect::<Result<Vec<Vec<_>>, _>>().unwrap().concat();
            let out = Altered::new(PathBuf::from(out_dir), processed_frames, fps, out_regex.to_string());
            Ok(out)
        }
        Some(err) => {
            println!("we detected an error");
            Err(format!("error, video processing failed: {}", err).into())
        }
    }
}
