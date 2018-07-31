use algorithmia::Algorithmia;
use common::video_error::VideoError;
use common::structs::prelude::*;
use super::functions;
use common::json_utils::combine_data_extract;
use rayon::prelude::*;
use common::threading::*;
use common::watchdog::Watchdog;
use serde_json::Value;
use std::sync::{Arc};
use common::misc;

static FPSMAX: f64 = 60f64;


pub fn default(data: Extract,
               num_of_frames: usize,
               batch_size: usize,
               duration: f64,
               starting_threads: isize,
                max_threads: isize,
               function: &Default<Extract, Value>) -> Result<Value, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_stamp: f64 = duration / num_of_frames as f64;
    let frame_batches: Box<Vec<Vec<usize>>> = misc::frame_batches_simple(batch_size, num_of_frames);
    let mut result: Vec<Result<Vec<Value>, ()>> = Vec::new();

    let global_threadable = Threadable::create(starting_threads, max_threads, data);
    let inner_threadable = global_threadable.clone();
    let wd = Watchdog::create(global_threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();
    frame_batches.par_iter().map(move |batch| {
//        let thread_t = inner_threadable.clone();
        let res = try_algorithm_default(function, &batch, &inner_threadable);
        if res.is_ok() {
            wd_t.send_success_signal();
        }
        res
    }).weight_max().collect_into(&mut result);
    wd.terminate();
    match global_threadable.extract_term_signal() {
        None => {
            let processed_frames: Vec<Value> = result.into_iter().collect::<Result<Vec<Vec<_>>, _>>().unwrap().concat();
            let processed: Value = combine_data_extract(&processed_frames, frame_stamp)?;
            Ok(processed)
        }
        Some(err) => Err(format!("error, video processing failed: {}", err).into())
    }
}

pub fn advanced(data: Extract,
                num_of_frames: usize,
                algorithm: &str,
                batch_size: usize,
                duration: f64,
                starting_threads: isize,
                max_threads: isize,
                input: AdvancedInput) -> Result<Value, VideoError> {
    let frame_stamp: f64 = duration / num_of_frames as f64;
    let search: Arc<AdvancedInput> = Arc::new(input);
    let frame_batches = misc::frame_batches_advanced(batch_size, num_of_frames, search.option());

    let mut result: Vec<Result<Vec<Value>, ()>> = Vec::new();

    let global_threadable = Threadable::create(starting_threads, max_threads, data);
    let sharable_threadable = global_threadable.clone();
    let wd = Watchdog::create(global_threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();

    frame_batches.par_iter().map(move |batch| {
//        let thread_t = sharable_threadable.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&functions::advanced_batch,  &batch,
                                   algorithm, &search, &sharable_threadable)
        } else {
            try_algorithm_advanced(&functions::advanced_single, &batch,
                                   algorithm, &search, &sharable_threadable)
        };
        if res.is_ok() {
            wd_t.send_success_signal();
        }
        res
    }).weight_max().collect_into(&mut result);
    wd.terminate();
    match global_threadable.extract_term_signal() {
        None => {
            let processed_frames: Vec<Value> = result.into_iter().collect::<Result<Vec<Vec<_>>, _>>().unwrap().concat();
            let processed: Value = combine_data_extract(&processed_frames, frame_stamp)?;
            Ok(processed)
        }
        Some(err) => Err(format!("error, video processing failed: {}", err).into())
    }
}

