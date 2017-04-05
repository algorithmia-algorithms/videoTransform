#[macro_use] extern crate algorithmia;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate hyper;
extern crate regex;
extern crate rayon;
extern crate uuid;
extern crate either;
mod common;
use algorithmia::prelude::*;
use serde_json::Value;
use serde_json::Number;
use common::video_error::VideoError;
use common::ffmpeg::FFMpeg;
use common::utilities;
use common::file_mgmt;
use common::ffmpeg;
use common::processing;
use std::path::*;
use uuid::Uuid;
use common::structs::gathered::Gathered;
use std::env;
use common::structs::scattered::Scattered;
use std::time::*;

#[derive(Debug, Deserialize)]
pub struct Entry{
    input_file: String,
    output_file: String,
    algorithm: String,
    advanced_input: Option<Value>,
    fps: Option<Number>,
    image_compression: Option<Number>,
    video_compression: Option<Number>,
}

#[derive(Debug, Serialize)]
struct Exit{
    output_file: String
}


// Algo should implement EntryPoint or DecodedEntryPoint
// and override at least one of the apply method variants
algo_entrypoint!(Entry);

fn apply(entry: Entry) -> Result<AlgoOutput, Box<std::error::Error>> {
        let data_api_work_directory = "data://.session";
    // let data_api_work_directory = "data://.my/ProcessVideo";
        let client = Algorithmia::default();
    // let client = Algorithmia::client("simA8y8WJtWGW+4h1hB0sLKnvb11");
    let ffmpeg_remote_url = "data://media/bin/ffmpeg-static.tar.gz";
    let batch_size = 10;
    let threads = 5;
    let ffmpeg_working_directory = PathBuf::from("/tmp/ffmpeg");
    let scattered_working_directory = PathBuf::from("/tmp/scattered_frames");
    let processed_working_directory = PathBuf::from("/tmp/processed_frames");
    let video_working_directory = PathBuf::from("/tmp/video");
    let local_output_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), entry.output_file.split("/").last().unwrap().clone()));
    let local_input_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), entry.input_file.split("/").last().unwrap().clone()));
    let input_uuid = Uuid::new_v4();
    let output_uuid = Uuid::new_v4();
    let scatter_regex = if entry.image_compression.is_some() { format!("{}-%07d.jpg", input_uuid) } else { format!("{}-%07d.png", input_uuid) };
    let process_regex = if entry.image_compression.is_some() { format!("{}-%07d.jpg", output_uuid) } else { format!("{}-%07d.png", output_uuid) };
    let fps: Option<f64> = entry.fps.map(|num: Number| { num.as_f64() }).and_then(|x| x);
    let image_compression: Option<u64> = entry.image_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
    let video_compression: Option<u64> = entry.video_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
    utilities::early_exit(&client, &entry.output_file)?;
    //we don't care about the result of clean_up, if it deletes stuff good, if it doesn't thats fine too.
    file_mgmt::clean_up(Some(&scattered_working_directory), Some(&processed_working_directory));
    let ffmpeg: FFMpeg = ffmpeg::new(ffmpeg_remote_url, &ffmpeg_working_directory, &client)?;
    let video = file_mgmt::get_file(&entry.input_file, &local_input_file, &client)?;
    let scatter_data: Scattered = processing::scatter(&ffmpeg, &video, &scattered_working_directory,
                                                      &scatter_regex, fps, image_compression)?;
    let processed_data = processing::alter(&client, &entry.algorithm, entry.advanced_input.as_ref(),
                                           &scatter_data, data_api_work_directory, &processed_working_directory,
                                           &process_regex, threads, batch_size)?;
    let gathered: Gathered = processing::gather(&ffmpeg, &video_working_directory, &local_output_file, processed_data,
                                                scatter_data.original_video(), video_compression)?;
    let uploaded = file_mgmt::upload_file(&entry.output_file, gathered.video_file(), &client)?;
    let result = Exit { output_file: uploaded };
    Ok(AlgoOutput::from(&result))
}

#[cfg(test)]
mod test {
    use super::Algo;
    use super::algorithmia::prelude::*;
    use std::borrow::Cow;
    #[test]
    fn basic_test() {
        let raw = json!({
    "input_file" : "data://media/videos/kenny_test.mp4",
    "output_file" : "data://quality/Videos/kenny_filtered.mp4",
    "algorithm":"algo://deeplearning/DeepFilter",
    "fps":5,
    "video_compression" : 40,
    "image_compression" : 20
    });
        println!("data: {:?}", &raw);
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
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
        let advanced_input = json!({
    "images": "$BATCH_INPUT",
    "savePaths": "$BATCH_OUTPUT",
    "filterName" : "far_away"
    });
        let raw = json!({
    "input_file" : "data://media/videos/kenny_test.mp4",
    "output_file" : "data://quality/Videos/kenny_filtered.mp4",
    "algorithm" : "algo://deeplearning/DeepFilter",
    "fps" : 10,
    "video_compression" : 25,
    "advanced_input" : advanced_input
    });
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
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
}

//#[test]
//fn array_advanced_test() {
//    let array: Vec<Value> = vec![
//        Value::String("$SINGLE_INPUT".to_string()),
//        Value::String("$SINGLE_OUTPUT".to_string()),
//        Value::Number(200i64.into()),
//        Value::Number(200i64.into())
//    ];
//    let json = json!({
//    "images": "$BATCH_INPUT",
//    "savePaths": "$BATCH_OUTPUT",
//    "filterName" : "far_away",
//    "advanced_input" : array
//    });
//    println!("data: {:?}", &json);
//    let result = Algo.apply_json(&json);
//    let test: bool = match result {
//        Ok(output) => {
//            match output {
//                AlgoOutput::Text(text) => {
//                    println!("text: {}", text);
//                    true
//                },
//                AlgoOutput::Json(json) => {
//                    println!("json: {}", json);
//                    true
//                }
//                _ => {
//                    println!("failed");
//                    false
//                }
//            }
//        },
//        Err(failure) => {
//            println!("{}", failure);
//            false
//        }
//    };
//    assert!(test);
//}