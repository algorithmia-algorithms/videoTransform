use serde_json::Value;
use std::path::*;
use common::video_error::VideoError;
use std::io::{BufWriter, Write};
use std::fs::File;
use serde_json::to_string;

pub fn frame_batches_advanced(batch_size: usize, number_of_frames: usize, option: &str) -> Box<Vec<Vec<usize>>> {
    match option {
        "batch" => {
            frame_batches_simple(batch_size, number_of_frames)
        }
        _ => {
            frame_batches_simple(1, number_of_frames)
        }
    }
}

pub fn frame_batches_simple(batch_size: usize, number_of_frames: usize) -> Box<Vec<Vec<usize>>> {
    let array: Vec<usize> = (1..number_of_frames).collect::<Vec<usize>>();
    Box::new(array.chunks(batch_size).map(|chunk| { chunk.iter().cloned().collect() }).collect::<Vec<Vec<usize>>>())
}

pub fn json_to_file(json: &Value, json_path: &Path) -> Result<PathBuf, VideoError> {
    let local_file = File::create(json_path).map_err(|err| {format!("failed to create local json file {}\n{}", json_path.display(), err)})?;
    let mut writer = BufWriter::new(local_file);
    try!(writer.write_all(to_string(json)?.as_bytes()));
    Ok(PathBuf::from(json_path))
}