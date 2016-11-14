extern crate algorithmia;
extern crate rustc_serialize;
extern crate hyper;
extern crate regex;
extern crate rayon;
extern crate uuid;
extern crate either;
extern crate time;
#[macro_use] extern crate wrapped_enum;
#[macro_use] extern crate lazy_static;
use algorithmia::{Algorithmia, NoAuth};
use algorithmia::algo::*;
mod video_error;
mod file_mgmt;
mod ffmpeg;
mod processing;
mod structs;
mod utilities;
mod alter_executor;
mod extract_executor;
mod alter_handling;
mod extract_handling;
use video_error::VideoError;
use ffmpeg::FFMpeg;
use std::path::*;
use std::collections::BTreeMap;
use rustc_serialize::json::{Json, ToJson};
use uuid::Uuid;
use structs::gathered::Gathered;
use structs::scattered::Scattered;
#[derive(Default)]
pub struct Algo;

#[derive(Debug)]
struct Entry{
    input_file: String,
    output_file: String,
    algorithm: String,
    advanced_input: Option<Json>,
    fps: Option<f64>,
//    quality: Option<bool>,

}

#[derive(Debug, RustcEncodable)]
struct Exit{
    output_file: String
}

macro_rules! str_field {
($o:expr, $f:expr) => {{
        let field: &Json = try!($o.get($f).ok_or(format!("missing field {}", $f)));
        // try parsing the field as a string
        let result = try!(field.as_string().ok_or(format!("missing field {}", $f)));
        result.to_string()
    }}
    }

// Algo should implement EntryPoint or DecodedEntryPoint
// and override at least one of the apply method variants
impl EntryPoint for Algo {
    fn apply_json(&self, input: &Json) -> Result<AlgoOutput, Box<std::error::Error>> {
        match input.as_object() {
            Some(obj) => {
                let entry = Entry {
                    input_file: str_field!(&obj, "input_file"),
                    output_file: str_field!(&obj, "output_file"),
                    algorithm: str_field!(&obj, "algorithm"),
                    advanced_input: obj.get("advanced_input").cloned(),
//                    quality: obj.get("quality").and_then(|ref quality| {quality.as_boolean()}),
                    fps: obj.get("fps").and_then(|ref fps| {fps.as_f64()})
                };
                match helper(entry) {
                    Ok(output) => Ok(output),
                    Err(err) => Err(format!("error detected: \n{}", err).into())
                }
            }
            None => Err(format!("failed to parse input as json.").into())
        }
    }
}

fn helper(entry: Entry)-> Result<AlgoOutput, VideoError>{
    let data_api_work_directory = "data://.session";
    let client = Algorithmia::client(NoAuth);
    let ffmpeg_remote_url = "data://media/bin/ffmpeg-static.tar.gz";
    let batch_size = 12;
    let threads = 8;
    let ffmpeg_working_directory = PathBuf::from("/tmp/ffmpeg");
    let scattered_working_directory = PathBuf::from("/tmp/scattered_frames");
    let processed_working_directory = PathBuf::from("/tmp/processed_frames");
    let video_working_directory = PathBuf::from("/tmp/input_video");
    let local_output_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), entry.output_file.split("/").last().unwrap().clone()));
    let local_input_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), entry.input_file.split("/").last().unwrap().clone()));
    let input_uuid = Uuid::new_v4();
    let output_uuid = Uuid::new_v4();
//    let quality = match entry.quality {
//        Some(val) => val,
//        None => false
//    };
    //TODO: determine if we want a quality operator to dynamically adjust file compression ratios to improve performance
    let quality = true;
    let scatter_regex = if quality {format!("{}-%07d.jpg", input_uuid)} else {format!("{}-%07d.png", input_uuid)};
    let process_regex = if quality {format!("{}-%07d.jpg", output_uuid)} else {format!("{}-%07d.png", output_uuid)};
    try!(utilities::early_exit(&client, &entry.output_file));
    //we don't care about the result of clean_up, if it deletes stuff good, if it doesn't thats fine too.
    file_mgmt::clean_up(&scattered_working_directory, &processed_working_directory);
    let ffmpeg: FFMpeg = try!(ffmpeg::new(ffmpeg_remote_url, &ffmpeg_working_directory, &client));
    let video = try!(file_mgmt::get_file(&entry.input_file, &local_input_file, &client));
    let scatter_data: Scattered = try!(processing::scatter(&ffmpeg, &video, &scattered_working_directory, &scatter_regex, entry.fps, quality));
    let processed_data = try!(processing::alter(&client, &entry.algorithm, entry.advanced_input.as_ref(), &scatter_data, data_api_work_directory, &processed_working_directory, &process_regex, threads, batch_size));
    let gathered: Gathered = try!(processing::gather(&ffmpeg, &local_output_file, processed_data, scatter_data.original_video()));
    let uploaded = try!(file_mgmt::upload_file(&entry.output_file, gathered.video_file(), &client));
    let result = Exit{output_file: uploaded};

    Ok(AlgoOutput::from(&result))
}

#[test]
fn basic_test() {
    let mut obj = BTreeMap::new();
    obj.insert("input_file".to_string(), Json::String("data://zeryx/Video/shorter_lounge.mp4".to_string()));
    obj.insert("output_file".to_string(), Json::String("data://media/videos/shorter_lounge_filtered.mp4".to_string()));
    obj.insert("algorithm".to_string(), Json::String("algo://deeplearning/DeepFilter".to_string()));
    obj.insert("fps".to_string(), Json::F64(15f64));
    let data = obj.to_json();
    println!("data: {:?}", &data);
    let result = Algo.apply_json(&data);
    let test: bool = match result {
        Ok(output) => {
            match output {
                AlgoOutput::Text(text) => {
                    println!("text: {}", text);
                    true
                },
                AlgoOutput::Json(json) => {
                    println!("json: {}", json);
                    true
                }
                _ => {
                    println!("failed");
                    false
                }
            }
        },
        Err(failure) => {
            println!("{}", failure);
            false
        }
    };
    assert!(test);
}

#[test]
fn advanced_test() {
    let mut obj = BTreeMap::new();
    let mut advanced = BTreeMap::new();
    advanced.insert("images".to_string(), Json::String("$BATCH_INPUT".to_string()));
    advanced.insert("savePaths".to_string(), Json::String("$BATCH_OUTPUT".to_string()));
    advanced.insert("filterName".to_string(), Json::String("gan_vogh".to_string()));
    obj.insert("input_file".to_string(), Json::String("data://zeryx/Video/shorter_lounge.mp4".to_string()));
    obj.insert("output_file".to_string(), Json::String("data://media/videos/shorter_lounge_filtered.mp4".to_string()));
    obj.insert("image_algorithm".to_string(), Json::String("algo://deeplearning/DeepFilter".to_string()));
    obj.insert("fps".to_string(), Json::F64(15f64));
    obj.insert("advanced_input".to_string(), Json::Object(advanced));
    let data = obj.to_json();
    println!("data: {:?}", &data);
    let result = Algo.apply_json(&data);
    let test: bool = match result {
        Ok(output) => {
            match output {
                AlgoOutput::Text(text) => {
                    println!("text: {}", text);
                    true
                },
                AlgoOutput::Json(json) => {
                    println!("json: {}", json);
                    true
                }
                _ => {
                    println!("failed");
                    false
                }
            }
        },
        Err(failure) => {
            println!("{}", failure);
            false
        }
    };
    assert!(test);
}