use algorithmia::prelude::*;
use algorithmia::error::ApiError;
use algorithmia::algo::*;
use serde_json::{Value, to_string};
use regex::Regex;
use common::file_mgmt::*;
use std::path::*;
use common::video_error::VideoError;
use std::fs::File;
use std::time;
use std::time::Duration;
use std::ops::Index;
use std::thread;
use std::io::{BufWriter, Write};
use std::ops::IndexMut;
static MAX_ATTEMPTS_ALGO: usize = 3usize;


//exits early if the or if the output path is invalid.
pub fn early_exit(client: &Algorithmia, output_path: &str) -> Result<(), VideoError> {
    //try to upload a 0 size file to the output path, then delete it. if both succeed then the path is valid.
    let r: Result<_, VideoError> = client.file(output_path).put("").map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to upload.\n{}", output_path, err).into());
    let j: Result<_, VideoError> = client.file(output_path).delete().map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to delete.\n{}", output_path, err).into());
    r?;j?;
    Ok(())
}

pub fn frame_batches(batch_size: usize, number_of_frames: usize) -> Vec<Vec<usize>> {
    let array: Vec<usize> = (1..number_of_frames).collect::<Vec<usize>>();
    array.chunks(batch_size).map(|chunk| { chunk.iter().cloned().collect() }).collect::<Vec<Vec<usize>>>()
}

pub fn batch_file_path(batch: &Vec<usize>, regex: &str, directory: &str) -> Result<Vec<String>, VideoError>
{
    let regexed = try!(batch.iter().map(|iter| {
        from_regex(regex, iter.clone())
    }).collect::<Result<Vec<String>, VideoError>>());
    Ok(regexed.iter().map(|filename| {
        format!("{}/{}", directory, filename)
    }).collect::<Vec<String>>())
}

//retry 3 times, if it fails 3 times we exit hard.
pub fn batch_upload_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>, client: &Algorithmia) -> Result<(), VideoError>
{
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        try!(upload_file(&remote_file, &local_file, client));
    }
    Ok(())
}

pub fn batch_get_file(local_file_save_locations: &Vec<PathBuf>, remote_file_get_locations: &Vec<String>, client: &Algorithmia) -> Result<Vec<PathBuf>, VideoError>
{
    let mut output: Vec<PathBuf> = Vec::new();
    for (local_file, remote_file) in local_file_save_locations.iter().zip(remote_file_get_locations.iter()) {
        output.push(get_file_from_algorithmia(&remote_file, &local_file, client)?);
    }
    Ok(output)
}

//fail fast if the exception contains '429'
pub fn try_algorithm(client: &Algorithmia, algorithm: &str, input: &Value) -> Result<AlgoResponse, VideoError> {
    let mut attempts = 0;
    let mut final_result;
    loop {
        match client.algo(algorithm).timeout(500).pipe(input.clone()) {
            Ok(result) => {
                final_result = result;
                break;
            },
            Err(ref err) if attempts < MAX_ATTEMPTS_ALGO && !err.to_string().contains("algorithm hit max number of active calls per session") => {
                println!("failed.");
                thread::sleep(Duration::from_millis((1000*attempts) as u64));
                attempts += 1;
            },
            Err(ref err) => {
                println!("failed hard.");
                return Err(format!("algorithm {} failed: \n{}", &algorithm, err).into())
            }
        }
    }
    Ok(final_result)
}


pub fn json_to_file(json: &Value, json_path: &Path) -> Result<PathBuf, VideoError> {
    let mut local_file = File::create(json_path).map_err(|err| {format!("failed to create local json file {}\n{}", json_path.display(), err)})?;
    let mut writer = BufWriter::new(local_file);
    try!(writer.write_all(to_string(json)?.as_bytes()));
    Ok(PathBuf::from(json_path))
}