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
pub fn default(client: &Algorithmia,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
               batch_size: usize,
               starting_threads: isize,
                max_threads: isize,
               function: &Default<Alter, PathBuf>) -> Result<Altered, VideoError> {
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(misc::frame_batches_simple(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();

    let alter = Alter::new(client.clone(),
                          data.regex().clone(),
                          output_regex.clone(),
                          local_out_dir.clone(),
                          data.frames_dir().clone(),
                          remote_dir.clone());

    let threadable = Threadable::create(starting_threads, max_threads, alter);

    let wd = Watchdog::create(threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();
    frame_batches.par_iter().map(move |batch| {
        let thread_t = threadable.clone();
        let res = try_algorithm_default(function, &batch, thread_t);
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

    let frame_batches = Box::new(misc::frame_batches_advanced(batch_size, data.num_frames(), search.option()));


    let alter = Alter::new(client.clone(),
                          data.regex().clone(),
                          output_regex.clone(),
                          local_out_dir.clone(),
                          data.frames_dir().clone(),
                          remote_dir.clone());

    let threadable = Threadable::create(starting_threads, max_threads, alter);
    let wd = Watchdog::create(threadable.arc_term_signal(), frame_batches.len());
    let wd_t = wd.get_comms();
    io::stderr().write(b"starting parallel map.\n")?;
    frame_batches.par_iter().map(move |batch| {
        let thread_t = threadable.clone();
        let res = if search.option() == "batch" {
            try_algorithm_advanced(&advanced_batch, &batch,
                                   algorithm, &search, thread_t)
        } else {
            try_algorithm_advanced(&advanced_single, &batch,
                                   algorithm, &search, thread_t)
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
