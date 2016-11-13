use algorithmia::{client, Algorithmia, NoAuth};
use ffmpeg::FFMpeg;
use ffmpeg;
use std::path::*;
use file_mgmt;
use alter_handling;
use rustc_serialize::json::Json;
use rayon::prelude::*;
use rayon;
use video_error::VideoError;
use structs::extract;
use structs::alter;
use structs::alter::Altered;
use structs::scattered::Scattered;
use structs::gathered::Gathered;
use std::sync::{Arc, Mutex};
use std::ops::*;
use std::io::{self, Write};
use std::ascii::AsciiExt;
static FPSMAX: f64 = 60f64;
use utilities;
 //used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
pub fn default_template_alter(client: &Algorithmia,
                          data: &Scattered,
                          remote_dir: &str,
                          local_out_dir: &Path,
                          output_regex: &str,
                          batch_size: usize,
                          function: &(Fn(&alter::Alter, Vec<usize>) -> Result<Vec<PathBuf>, VideoError> + Sync)) -> Result<Altered, VideoError>
{
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(utilities::frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    //mutex lock that allows us to end early.
    let input = Arc::new(alter::Alter::new(client.clone(),
                                           data.regex().to_owned(),
                                           output_regex.to_owned(),
                                           local_out_dir.to_owned(),
                                           data.frames_dir().to_owned(),
                                           remote_dir.to_owned()));
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        if let Some(ref err) = *(lock.lock().unwrap()) {
            return Err(err.to_string().into())
        }
        match function(&input, batch.clone()) {
            Ok(data) => Ok(data),
            Err(err) => {
                let mut terminate = lock.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }).weight_max().collect_into(&mut result);
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };

    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}


pub fn advanced_alter(client: &Algorithmia,
                      data: &Scattered,
                      remote_dir: &str,
                      local_out_dir: &Path,
                      output_regex: &str,
                      algorithm: &str,
                      batch_size: usize,
                      input: &Json) -> Result<Altered, VideoError>
{
    let search: Arc<utilities::SearchResult> = Arc::new(try!(utilities::format_search(input)));
    let mut result: Vec<Result<Vec<PathBuf>, VideoError>> = Vec::new();
    //mutex lock that allows us to end early.
    let mut early_terminate: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let frame_batches = if search.option() == "batch" {utilities::frame_batches(batch_size, data.num_frames())}
        else {utilities::frame_batches(1, data.num_frames())};

    let formatted_data = Arc::new(alter::Alter::new(client.clone(),
                                           data.regex().to_owned(),
                                           output_regex.to_owned(),
                                           local_out_dir.to_owned(),
                                           data.frames_dir().to_owned(),
                                           remote_dir.to_owned()));

    try!(io::stderr().write(b"starting parallel map.\n"));
    frame_batches.par_iter().map(move |batch| {
        let lock = early_terminate.clone();
        if let Some(ref err) = *(lock.lock().unwrap()) {
            return Err(err.to_string().into())
        }
        if search.option() == "batch" {
            match alter_handling::advanced_batch(&formatted_data, batch.to_owned(), algorithm.to_string(), &search) {
                Ok(data) => Ok(data),
                Err(err) => {
                    let mut terminate = lock.lock().unwrap();
                    let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                    *terminate = Some(terminate_msg.clone());
                    Err(terminate_msg.into())
                }
            }
        } else {
            match alter_handling::advanced_single(&formatted_data, batch.to_owned(), algorithm.to_string(), &search) {
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
    let processed_frames: Vec<PathBuf> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    Ok(Altered::new(PathBuf::from(local_out_dir), processed_frames, data.fps(), output_regex.to_string()))
}
