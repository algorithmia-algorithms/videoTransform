use algorithmia::Algorithmia;
use common::ffmpeg::FFMpeg;
use common::ffmpeg;
use std::path::*;
use common::file_mgmt;
use rayon::prelude::*;
use rayon;
use serde_json::Value;
use common::video_error::VideoError;
use common::structs::extract;
use common::structs::alter;
use common::structs::alter::Altered;
use common::structs::scattered::Scattered;
use common::structs::gathered::Gathered;
use std::sync::{Arc, Mutex};
use std::ops::*;
use common::alter_handling;
use common::extract_handling;
use common::alter_executor::{advanced_alter, default_template_alter};
use common::extract_executor::{advanced_extract, default_template_extract};
use std::io::{self, Write};
use std::ascii::AsciiExt;
use common::utilities;
use uuid::Uuid;

static MAX_FPS: f64 = 60f64;
static MAX_FRAMES: u64 = 10000;

//split video limits the fps to FPSMAX, if its higher we only sample at FPSMAX
pub fn scatter(ffmpeg: &FFMpeg,
               video_file: &Path,
               frame_dir: &Path,
               regex: &str,
               fps: Option<f64>,
               compression_factor: Option<u64>) -> Result<Scattered, VideoError> {
    file_mgmt::create_directory(frame_dir);
    println!("scattering video into frames and audio");
    let origin_fps = try!(ffmpeg.get_video_fps(video_file));
    let output_fps = match fps {
        Some(fps) => {fps},
        None => {
            if origin_fps <= MAX_FPS {
                origin_fps
            }
                else {
                    MAX_FPS
                }
        }
    };
    let duration:f64 = try!(ffmpeg.get_video_duration(video_file));
    let num_frames: u64 = (duration*output_fps).ceil() as u64;
    if num_frames <= MAX_FRAMES {
        let result = try!(ffmpeg.split_video(video_file, frame_dir, &regex, output_fps, &compression_factor));
        Ok(Scattered::new(PathBuf::from(frame_dir), result.len(), PathBuf::from(video_file), output_fps, regex.to_string()))
    }
        else {
            Err(format!("early exit:\nInput videos total number of frames greater than {}, please reduce fps or reduce the total size of the video file.", MAX_FRAMES).into())
        }
}

//combines video frames in directory frames_dir with audio_file to create a video file.
pub fn gather(ffmpeg: &FFMpeg,
                video_working_directory: &Path,
              output_file: &Path,
              data: alter::Altered,
              original_file: &Path,
              crf: Option<u64>) -> Result<Gathered, VideoError> {
    println!("gathering frames and audio into video.");
    let filename = Uuid::new_v4();
    let extension = try!(output_file.extension().ok_or(format!("failed to find a file extension for output file."))).to_str().unwrap();
    let catted_video_no_audio = PathBuf::from(format!("{}/{}-{}.{}", video_working_directory.display(), "streamless", filename, extension));
    let catted_video_with_audio = PathBuf::from(format!("{}/{}-{}.{}", video_working_directory.display(), "with_streams", filename, extension));
    ffmpeg.cat_video(&catted_video_no_audio, data.frames_dir(), data.regex(), data.fps(), crf)?;
    let video_with_streams = ffmpeg.attach_streams(&catted_video_no_audio, &catted_video_with_audio, &original_file)?;
    Ok(Gathered::new(video_with_streams, data.fps()))
}

// alter branch, used by VideoTransform
pub fn alter(client: &Algorithmia,
             algorithm: &str,
             algo_input: Option<&Value>,
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
                Err(String::from("No default algorithm definition, advanced_input required.").into())
            }
        }
    }
}

//extract branch, used by VideoMetadataExtraction
pub fn extract(client: &Algorithmia,
               algorithm: &str,
               algo_input: Option<&Value>,
               data: &Scattered,
               remote_dir: &str,
               num_threads: usize,
               duration: f64,
               batch_size: usize) -> Result<Value, VideoError> {
    let config = rayon::Configuration::new().set_num_threads(num_threads);
    try!(rayon::initialize(config));
    match algo_input {
        Some(advanced_input) => {
            println!("advanced input found");
            advanced_extract(client, data, remote_dir, algorithm, batch_size, duration, advanced_input)
        }
        //no custom json input, so we use defaults.
        None => {
            if algorithm.to_ascii_lowercase().as_str().contains("nuditydetection") {
                default_template_extract(client, data, remote_dir, batch_size, duration, &extract_handling::nudity_detection)
            } else if algorithm.to_ascii_lowercase().as_str().contains("illustrationtagger") {
                default_template_extract(client, data, remote_dir, batch_size, duration, &extract_handling::illustration_tagger)
            } else {
                println!("failed to pattern match anything.");
                Err(String::from("not implemented.").into())
            }
        }
    }
}

