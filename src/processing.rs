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

//split video limits the fps to FPSMAX, if its higher we only sample at FPSMAX
pub fn scatter(ffmpeg: &FFMpeg,
               video_file: &Path,
               frame_dir: &Path,
               regex: &str,
               fps: Option<f64>,
               quality: bool) -> Result<Scattered, VideoError> {
    try!(file_mgmt::create_directory(frame_dir));
    println!("scattering video into frames and audio");
    let origin_fps = try!(ffmpeg.get_video_fps(video_file));
    let output_fps = match fps {
        Some(fps) => {fps},
        None => {
            if origin_fps <= FPSMAX { origin_fps }
    else {FPSMAX}
        }};
    let result = try!(ffmpeg.split_video(video_file, frame_dir, &regex, output_fps, quality));
    Ok(Scattered::new(PathBuf::from(frame_dir), result.len(), PathBuf::from(video_file), output_fps, regex.to_string()))
}

//combines video frames in directory frames_dir with audio_file to create a video file.
pub fn gather(ffmpeg: &FFMpeg,
              output_file: &Path,
              data: alter::Altered,
              original_file: &Path) -> Result<Gathered, VideoError> {
    println!("gathering frames and audio into video.");
    let catted_video_no_audio = PathBuf::from(format!("/tmp/{}-{}", "temp", output_file.file_name().unwrap().to_str().unwrap()));
    try!(ffmpeg.cat_video(&catted_video_no_audio, data.frames_dir(), data.regex(), data.fps()));
    let video_with_streams = try!(ffmpeg.attach_streams(&catted_video_no_audio, output_file, original_file));
    Ok(Gathered::new(video_with_streams, data.fps()))
}

//processes the images generated from the scatter op in an asynchronous algo call loop.
pub fn process(client: &Algorithmia,
               algorithm: &str,
               algo_input: Option<&Json>,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
               num_threads: usize,
               batch_size: usize) -> Result<alter::Altered, VideoError> {
    let config = rayon::Configuration::new().set_num_threads(num_threads);
    try!(rayon::initialize(config));
    //batch size is only used if the algorithm accepts batching and/or the user defined advanced input has a $BATCH_FILE_INPUT & $BATCH_FILE_OUTPUT designated.
    match algo_input {
        Some(advanced_input) => {
            println!("advanced input found");
            advanced_alter(client, data, remote_dir, local_out_dir, output_regex, algorithm, batch_size, advanced_input)
        }
        //no custom json input, so we use defaults.
        None => {
            if algorithm.to_ascii_lowercase().as_str().contains("deepfilter") {
                default_template_alter(client, data, remote_dir, local_out_dir, output_regex, batch_size, &alter_handling::deep_filter)
            } else if algorithm.to_ascii_lowercase().as_str().contains("salnet") {
                default_template_alter(client, data, remote_dir, local_out_dir, output_regex, batch_size, &alter_handling::salnet)
            } else if algorithm.to_ascii_lowercase().as_str().contains("colorfulimagecolorization") {
                default_template_alter(client, data, remote_dir, local_out_dir, output_regex, batch_size, &alter_handling::colorful_colorization)
            } else {
                println!("failed to pattern match anything.");
                Err(String::from("not implemented.").into())
            }
        }
    }
}

pub fn extract(client: &Algorithmia,
               algorithm: &str,
               algo_input: Option<&Json>,
               data: &Scattered,
               remote_dir: &str,
               local_out_dir: &Path,
               output_regex: &str,
               num_threads: usize,
               duration: f64,
               batch_size: usize) -> Result<Json, VideoError> {
    match algo_input {
        Some(advanced_input) => {
            println!("advanced input found");
            advanced_extract(client, data, remote_dir, algorithm, batch_size, duration, advanced_input)
        }
        //no custom json input, so we use defaults.
        None => {
//            if algorithm.to_ascii_lowercase().as_str().contains("deepfilter") {
//                default_template_extract(client, data, remote_dir, batch_size, duration, &alter_handling::deep_filter)
//            } else if algorithm.to_ascii_lowercase().as_str().contains("salnet") {
//                default_template_extract(client, data, remote_dir, batch_size, duration, &alter_handling::salnet)
//            } else if algorithm.to_ascii_lowercase().as_str().contains("colorfulimagecolorization") {
//                default_template_extract(client, data, remote_dir, batch_size, duration, &alter_handling::colorful_colorization)
//            } else {
//                println!("failed to pattern match anything.");
//                Err(String::from("not implemented.").into())
//            }
            unimplemented!()
        }
    }
}

//used to template all of the default image proc algorithms, uses rayon for multi-threading and uses Arc<Mutex> locking to fail early if an exception is found.
fn default_template_alter(client: &Algorithmia,
                          data: &Scattered,
                          remote_dir: &str,
                          local_out_dir: &Path,
                          output_regex: &str,
                          batch_size: usize,
                          function: &(Fn(&alter::Alter, Vec<usize>) -> Result<Vec<PathBuf>, VideoError> + Sync)) -> Result<Altered, VideoError>
{
    //generate batches of frames by number, based on the batch size.
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(frame_batches(batch_size, data.num_frames()));
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

fn default_template_extract(client: &Algorithmia,
                            data: &Scattered,
                            remote_dir: &str,
                            batch_size: usize,
                            duration: f64,
                            function: &(Fn(&extract::Extract, Vec<usize>) -> Result<Vec<Json>, VideoError> + Sync)) -> Result<Json, VideoError>
{
    //generate batches of frames by number, based on the batch size.
    let frame_stamp: f64 = duration / data.num_frames() as f64;
    let frame_batches: Box<Vec<Vec<usize>>> = Box::new(frame_batches(batch_size, data.num_frames()));
    let mut result: Vec<Result<Vec<Json>, VideoError>> = Vec::new();
    //mutex lock that allows us to end early.
    let input = Arc::new(extract::Extract::new(client.clone(),
                                           data.regex().to_owned(),
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
    let processed_frames: Vec<Json> = match result.into_iter().collect::<Result<Vec<Vec<_>>, _>>() {
        Ok(frames) => frames.concat(),
        Err(err) => return Err(format!("error, video processing failed: {}", err).into())
    };
    let processed: Json = try!(utilities::combine_extracted_data(&processed_frames, frame_stamp));

    Ok(processed)
}
fn advanced_alter(client: &Algorithmia,
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
    let frame_batches = if search.option() == "batch" {frame_batches(batch_size, data.num_frames())}
        else {frame_batches(1, data.num_frames())};

    let input = Arc::new(alter::Alter::new(client.clone(),
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
            match alter_handling::advanced_batch(&input, batch.to_owned(), algorithm.to_string(), &search) {
                Ok(data) => Ok(data),
                Err(err) => {
                    let mut terminate = lock.lock().unwrap();
                    let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                    *terminate = Some(terminate_msg.clone());
                    Err(terminate_msg.into())
                }
            }
        } else {
            match alter_handling::advanced_single(&input, batch.to_owned(), algorithm.to_string(), &search) {
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

fn advanced_extract(client: &Algorithmia,
                    data: &Scattered,
                    remote_dir: &str,
                    algorithm: &str,
                    batch_size: usize,
                    duration: f64,
                    input: &Json) -> Result<Json, VideoError>
{
   unimplemented!()
}

fn frame_batches(batch_size: usize, number_of_frames: usize) -> Vec<Vec<usize>> {
    let array: Vec<usize> = (1..number_of_frames).collect::<Vec<usize>>();
    array.chunks(batch_size).map(|chunk| { chunk.iter().cloned().collect() }).collect::<Vec<Vec<usize>>>()
}