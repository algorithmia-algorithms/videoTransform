#[macro_use] extern crate algorithmia;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate hyper;
extern crate regex;
extern crate rayon;
extern crate env_logger;
extern crate uuid;
extern crate either;
extern crate std_semaphore;
use algorithmia::prelude::*;
use serde_json::Value;
use serde_json::Number;
use std::path::*;
use uuid::Uuid;
mod common;
use common::ffmpeg::FFMpeg;
use common::utilities;
use common::file_mgmt;
use common::ffmpeg;
use common::video_error::VideoError;
use common::processing;
use common::alter;
use common::extract;
use common::structs::gathered::Gathered;
use common::structs::scattered::Scattered;

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


struct Algo;
// this version doesn't auto-create Algo, so you can create it yourself
algo_entrypoint!(Entry => Algo::helper);

impl Algo {
    fn helper(&self, entry: Entry) -> Result<AlgoOutput, Box<std::error::Error>> {
        let batch_size = 10;
        let starting_threads = 20;
        let parameters: PreDefines = prep(RunFormat::TestLocal, batch_size, starting_threads, &entry.output_file, &entry.input_file, entry.image_compression.clone().is_some())?;
        let fps: Option<f64> = entry.fps.map(|num: Number| { num.as_f64() }).and_then(|x| x);
        let image_compression: Option<u64> = entry.image_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
        let video_compression: Option<u64> = entry.video_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
        utilities::early_exit(&parameters.client, &entry.output_file)?;
        let video = file_mgmt::get_file(&entry.input_file, &parameters.local_input_file, &parameters.client)?;
        let scatter_data: Scattered = processing::scatter(&parameters.ffmpeg, &video, &parameters.scattered_working_directory,
                                                          &parameters.scatter_regex, fps, image_compression)?;
        let processed_data = processing::alter(&parameters.client, &entry.algorithm, entry.advanced_input.as_ref(),
                                               &scatter_data, &parameters.data_api_work_directory, &parameters.processed_working_directory,
                                               &parameters.process_regex, parameters.starting_threads, parameters.batch_size)?;
        let gathered: Gathered = processing::gather(&parameters.ffmpeg, &parameters.video_working_directory, &parameters.local_output_file, processed_data,
                                                    scatter_data.original_video(), video_compression)?;
        let uploaded = file_mgmt::upload_file(&entry.output_file, gathered.video_file(), &parameters.client)?;
        let result = Exit { output_file: uploaded };
        Ok(AlgoOutput::from(&result))
    }
}

impl Default for Algo {
    fn default() -> Algo {
        env_logger::init();
        Algo
    }
}


enum RunFormat{
    ProdAlgo,
    ProdLocal,
    TestAlgo,
    TestLocal
}

struct PreDefines{
    client: Algorithmia,
    scattered_working_directory: PathBuf,
    processed_working_directory: PathBuf,
    video_working_directory: PathBuf,
    data_api_work_directory: String,
    local_input_file: PathBuf,
    local_output_file: PathBuf,
    ffmpeg: FFMpeg,
    scatter_regex: String,
    process_regex: String,
    batch_size: usize,
    starting_threads: usize
}

fn prep(format: RunFormat,
        batch_size: usize,
        starting_threads: usize,
        output_file: &str,
        input_file: &str,
        has_image_compression: bool
) -> Result<PreDefines, VideoError> {


    let prod_key = "simA8y8WJtWGW+4h1hB0sLKnvb11";
    let test_key = "simA8y8WJtWGW+4h1hB0sLKnvb11";
    let test_api = "https://apitest.algorithmia.com";
    let session = String::from("data://.session");
    let not_session = String::from("data://.my/ProcessVideo");

    let (client, data_work_dir) = match format {
        RunFormat::ProdAlgo => { (Algorithmia::default(), session)}
        RunFormat::ProdLocal => { (Algorithmia::client(prod_key), not_session)}
        RunFormat::TestAlgo => {(Algorithmia::client_with_url(test_api, test_key), session)}
        RunFormat::TestLocal => {(Algorithmia::client_with_url(test_api, test_key), not_session)}
    };
    let ffmpeg_remote_url = "data://media/bin/ffmpeg-static.tar.gz";
    let ffmpeg_working_directory = PathBuf::from("/tmp/ffmpeg");
    let scattered_working_directory = PathBuf::from("/tmp/scattered_frames");
    let processed_working_directory = PathBuf::from("/tmp/processed_frames");
    let video_working_directory = PathBuf::from("/tmp/video");
    let local_output_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), output_file.split("/").last().unwrap().clone()));
    let local_input_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), input_file.split("/").last().unwrap().clone()));
    let input_uuid = Uuid::new_v4();
    let output_uuid = Uuid::new_v4();
    let scatter_regex = if has_image_compression { format!("{}-%07d.jpg", input_uuid) } else { format!("{}-%07d.png", input_uuid) };
    let process_regex = if has_image_compression { format!("{}-%07d.jpg", output_uuid) } else { format!("{}-%07d.png", output_uuid) };
    file_mgmt::clean_up(Some(&scattered_working_directory), Some(&processed_working_directory), &video_working_directory);
    let ffmpeg: FFMpeg = ffmpeg::new(ffmpeg_remote_url, &ffmpeg_working_directory, &client)?;
    Ok(PreDefines{
        client: client,
        scattered_working_directory: scattered_working_directory,
        processed_working_directory: processed_working_directory,
        data_api_work_directory: data_work_dir,
        video_working_directory: video_working_directory,
        local_input_file: local_input_file,
        local_output_file: local_output_file,
        ffmpeg: ffmpeg,
        scatter_regex: scatter_regex,
        process_regex: process_regex,
        batch_size: batch_size,
        starting_threads: starting_threads
    })
}

#[cfg(test)]
mod test {
    use super::Algo;
    use super::algorithmia::prelude::*;
    use std::borrow::Cow;

    #[test]
    fn basic_test() {
        let raw = json!({
    "input_file" : "data://quality/videos/kenny_test.mp4",
    "output_file" : "data://quality/Videos/kenny_filtered.mp4",
    "algorithm":"algo://deeplearning/SalNet",
    "fps":10,
    "video_compression" : 40,
    "image_compression" : 20
    });
        println!("data: {:?}", &raw);
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
        assert!(result.is_ok(), "apply return an error");
        if let AlgoOutput::Binary(_) = result.unwrap() {
            panic!("apply return binary data")
        }
    }

    #[test]
    fn advanced_batch_test() {
        let advanced_input = json!({
    "images": "$BATCH_INPUT",
    "output_loc": "$BATCH_OUTPUT",
    "fill_color": "blur"
    });
        let raw = json!({
    "input_file" : "data://zeryx/Video/K5qACexzwOI.mp4",
    "output_file" : "data://quality/Videos/silicon_valley_censored.mp4",
    "algorithm" : "algo://cv/CensorFace",
    "fps" : 14,
//    "video_compression" : 25,
//    "image_compression" : 25,
    "advanced_input" : advanced_input
    });
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
        match result {
            Ok(_)=> {println!("completed success");}
            Err(ref err) => {println!("errored: {}", err);}
        }
    }

    #[test]
    fn advanced_single_test() {
        let advanced_input = json!({
    "image": "$SINGLE_INPUT",
    "location": "$SINGLE_INPUT",
    });
        let raw = json!({
    "input_file" : "data://quality/videos/kenny_test.mp4",
    "output_file" : "data://quality/Videos/kenny_filtered.mp4",
    "algorithm" : "algo://deeplearning/SalNet",
    "fps" : 20,
    "video_compression" : 25,
    "advanced_input" : advanced_input
    });
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
        assert!(result.is_ok(), "apply return an error");
        if let AlgoOutput::Binary(_) = result.unwrap() {
            panic!("apply return binary data")
        }
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