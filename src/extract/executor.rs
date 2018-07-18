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


pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               batch_size: usize,
               duration: f64,
               starting_threads: isize,
                max_threads: isize,
               function: &Default<Extract, Value>) -> Result<Value, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(misc::frame_batches_simple(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<Value>, VideoError>> = Vec::new();

    let extract = Extract::new(client.clone(),
                            data.regex().clone(),
                            data.frames_dir().clone(),
                            remote_dir.clone());

    let threadable = Threadable::create(starting_threads, max_threads, extract);

    let wd = Watchdog::create(threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();

    frame_batches.par_iter().map(move |batch| {
        let thread_t = threadable.clone();
        let res = try_algorithm_default(function, &batch, thread_t);
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
    let frame_batches = Box::new(misc::frame_batches_advanced(batch_size, data.num_frames(), search.option()));
    let mut result: Vec<Result<Vec<Value>, VideoError>> = Vec::new();

    let extract = Extract::new(client.clone(),
                            data.regex().clone(),
                            data.frames_dir().clone(),
                            remote_dir.clone());

    let threadable = Threadable::create(starting_threads, max_threads, extract);

    let wd = Watchdog::create(threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();

    frame_batches.par_iter().map(move |batch| {
        let thread_t = threadable.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&functions::advanced_batch,  &batch,
                                   algorithm, &search, thread_t)
        } else {
            try_algorithm_advanced(&functions::advanced_single, &batch,
                                   algorithm, &search, thread_t)
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

