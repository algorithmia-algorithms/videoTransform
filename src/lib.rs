#[macro_use] extern crate algorithmia;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate regex;
extern crate rayon;
extern crate uuid;
extern crate either;
extern crate std_semaphore;
use algorithmia::prelude::*;
use serde_json::Value;
use serde_json::Number;
use std::path::*;
mod common;
mod extract;
mod transform;
mod processing;
use common::algo::{early_exit, get_file, upload_file};
use common::misc::json_to_file;
use common::structs::prelude::{Gathered, Scattered};
use common::preprocess::{PreDefines, ExecutionStyle};

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


pub struct Algo;
// this version doesn't auto-create Algo, so you can create it yourself
algo_entrypoint!(Entry => Algo::helper);

enum Objective {
    Transform,
    Extract
}


impl Algo {
    fn helper(&self, entry: Entry) -> Result<AlgoOutput, Box<std::error::Error>> {
        let batch_size = 5;
        let starting_threads = 5;
        let max_threads = 35;
        let format = ExecutionStyle::Algo;
        let objective = Objective::Transform;
        let parameters: PreDefines = PreDefines::create(format, batch_size, starting_threads, max_threads,
                                          &entry.output_file, &entry.input_file,
                                          entry.image_compression.clone().is_some())?;

        let fps: Option<f64> = entry.fps.map(|num: Number| { num.as_f64() }).and_then(|x| x);
        let image_compression: Option<u64> = entry.image_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
        let video_compression: Option<u64> = entry.video_compression.map(|num: Number| { num.as_u64() }).and_then(|x| x);
        early_exit(&parameters.client, &entry.output_file)?;
        let video = get_file(&entry.input_file, &parameters.local_input_file, &parameters.data_api_work_directory, &parameters.client)?;
        let scatter_data: Scattered = processing::scatter(&parameters.ffmpeg, &video, &parameters.scattered_working_directory,
                                                          &parameters.scatter_regex, fps, image_compression)?;

        let video_file: PathBuf = match objective {
            Objective::Transform => {
                let processed_data = processing::transform(&parameters.client, &entry.algorithm, entry.advanced_input.as_ref(),
                                                           &scatter_data, &parameters.data_api_work_directory, &parameters.processed_working_directory,
                                                           &parameters.process_regex, parameters.max_threads, parameters.starting_threads, parameters.batch_size)?;
                let gathered: Gathered = processing::gather(&parameters.ffmpeg, &parameters.video_working_directory, &parameters.local_output_file, processed_data,
                                                            scatter_data.original_video(), video_compression)?;
                gathered.video_file().clone()
            }
            Objective::Extract => {
                let duration: f64 = parameters.ffmpeg.get_video_duration(&parameters.local_input_file)?;
                let processed_data: Value = processing::extract(&parameters.client, &entry.algorithm,
                                                                entry.advanced_input.as_ref(), &scatter_data,
                                                                &parameters.data_api_work_directory,
                                                                parameters.starting_threads, parameters.max_threads,
                                                                duration, batch_size)?;
                let saved_file: PathBuf = json_to_file(&processed_data, &parameters.local_output_file)?;
                saved_file
            }
        };
        let uploaded = upload_file(&entry.output_file, &video_file, &parameters.client)?;
        let result = Exit { output_file: uploaded };
        Ok(AlgoOutput::from(&result))
    }
}

impl Default for Algo {
    fn default() -> Algo {
        Algo
    }
}

#[cfg(test)]
mod test {
    use super::Algo;
    use super::algorithmia::prelude::*;
    use std::borrow::Cow;

    #[test]
    fn basic_test() {
        let raw = json!({
    "input_file" : "data://jpeck/deleteme/PARTISAN540.mp4",
    "output_file" : "data://quality/Videos/PARTISAN540color2fps.mp4",
    "algorithm":"algo://deeplearning/ColorfulImageColorization",
    "advanced_input": {"image": "$SINGLE_INPUT", "location": "$SINGLE_OUTPUT"},
    "fps":2,
//    "video_compression" : 30,
//    "image_compression" : 20
    });
        println!("data: {:?}", &raw);
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
        match result {
            Ok(_)=> {println!("completed success");}
            Err(ref err) => {println!("errored: {}", err);}
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
    "input_file" : "data://zeryx/Video/shorter_lounge.mp4",
    "output_file" : "data://quality/Videos/silicon_valley_censored.mp4",
    "algorithm" : "algo://cv/CensorFace",
    "fps" : 60,
//    "video_compression" : 25,
//    "image_compression" : 35,
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
    "location": "$SINGLE_OUTPUT",
    });
        let raw = json!({
    "input_file" : "data://quality/videos/kenny_test.mp4",
    "output_file" : "data://quality/Videos/kenny_filtered.mp4",
    "algorithm" : "algo://deeplearning/SalNet",
    "fps" : 30,
//    "video_compression" : 25,
    "advanced_input" : advanced_input
    });
        let json = AlgoInput::Json(Cow::Owned(raw));
        let result = Algo::default().apply(json);
        match result {
            Ok(_)=> {println!("completed success");}
            Err(ref err) => {println!("errored: {}", err);}
        }
    }
}
