use algorithmia::{client, Algorithmia, NoAuth};
use ffmpeg::FFMpeg;
use ffmpeg;
use std::path::*;
use file_mgmt;
use alter_handling;
use rustc_serialize::json::Json;
use rayon::prelude::*;
use video_error::VideoError;
use structs::extract;
use structs::alter;
use structs::alter::Altered;
use structs::scattered::Scattered;
use structs::gathered::Gathered;
use extract_handling;
use std::sync::{Arc, Mutex};
use std::ops::*;
use std::io::{self, Write};
use std::ascii::AsciiExt;
static FPSMAX: f64 = 60f64;
use utilities;

pub fn default_template_extract(client: &Algorithmia,
                            data: &Scattered,
                            remote_dir: &str,
                            batch_size: usize,
                            duration: f64,
                            function: &(Fn(&extract::Extract, Vec<usize>) -> Result<Vec<Json>, VideoError> + Sync)) -> Result<Json, VideoError>
{
    //generate batches of frames by number, based on the batch size.
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(utilities::frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<Json>, VideoError>> = Vec::new();
    //mutex lock that allows us to end early.
    let formatted_data = Arc::new(extract::Extract::new(client.clone(),
                                               data.regex().to_owned(),
                                               data.frames_dir().to_owned(),
                                               remote_dir.to_owned()));
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        if let Some(ref err) = *(lock.lock().unwrap()) {
            return Err(err.to_string().into())
        }
        match function(&formatted_data, batch.clone()) {
            Ok(data) => Ok(data),
            Err(err) => {
                let mut terminate = lock.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }).weight_max().collect_into(&mut result);
    let processed_frames: Vec<Json> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    let processed: Json = try!(utilities::combine_extracted_data(&processed_frames, frame_stamp));

    Ok(processed)
}

pub fn advanced_extract(client: &Algorithmia,
                    data: &Scattered,
                    remote_dir: &str,
                    algorithm: &str,
                    batch_size: usize,
                    duration: f64,
                    input: &Json) -> Result<Json, VideoError>
{
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let search: Arc<utilities::SearchResult> = Arc::new(try!(utilities::format_search(input)));
    let mut result: Vec<Result<Vec<Json>, VideoError>> = Vec::new();
    //mutex lock that allows us to end early.
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let frame_batches = if search.option() == "batch" {utilities::frame_batches(batch_size, data.num_frames())}
        else {utilities::frame_batches(1, data.num_frames())};

    let formatted_data = Arc::new(extract::Extract::new(client.clone(),
                                           data.regex().to_owned(),
                                           data.frames_dir().to_owned(),
                                           remote_dir.to_owned()));

    try!(io::stderr().write(b"starting parallel map.\n"));
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        if let Some(ref err) = *(lock.lock().unwrap()) {
            return Err(err.to_string().into())
        }
        if search.option() == "batch" {
            match extract_handling::advanced_batch(&formatted_data, batch.to_owned(), algorithm.to_string(), &search) {
                Ok(data) => Ok(data),
                Err(err) => {
                    let mut terminate = lock.lock().unwrap();
                    let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                    *terminate = Some(terminate_msg.clone());
                    Err(terminate_msg.into())
                }
            }
        } else {
            match extract_handling::advanced_single(&formatted_data, batch.to_owned(), algorithm.to_string(), &search) {
                Ok(data) => Ok(data),
                Err(err) => {
                    let mut terminate = lock.lock().unwrap();
                    let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                    *terminate = Some(terminate_msg.clone());
                    Err(terminate_msg.into())
                }
            }
        }
    }).weight_max().collect_into(&mut result);
    try!(io::stderr().write(b"exited parallel map.\n"));
    let processed_frames: Vec<Json> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    let processed:Json = try!(utilities::combine_extracted_data(&processed_frames, frame_stamp));
    Ok(processed)
}