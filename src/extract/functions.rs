
use algorithmia::Algorithmia;
use algorithmia::algo::*;
use algorithmia::error::ApiError;
use std::path::*;
use serde_json::Value;
use common::utilities::replace_variables_extract;
use common::json_utils::AdvancedInput;
use serde_json::Value::*;
use std::error::Error;
use std::string::String;
use common::video_error::VideoError;
use std::ffi::OsStr;
use common::structs::prelude::*;
use common::misc::*;
use std_semaphore::Semaphore;
use std::sync::Arc;
use std::ops::Index;
use either::{Left, Right};
pub fn nudity_detection(input: &Extract, batch: Vec<usize>, semaphore: Arc<Semaphore>) -> Result<Vec<Value>, VideoError> {
    let algorithm = "algo://sfw/NudityDetectioni2v/0.2.4";
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;
    let json = json!({
    "image": remote_pre_frames.iter()
        .map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    });

    //println!("acquiring semaphore");
    semaphore.acquire();
    let response: AlgoResponse = try_algorithm(input.client(), &algorithm, &json)?;
    semaphore.release();
    //println!("releasing semaphore");

    let output_json: Value = response.into_json()
        .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
    let output: Vec<Value> = output_json.as_array().unwrap().iter().map(|dat| {dat.clone()}).collect::<Vec<_>>();
    Ok(output)
}

pub fn illustration_tagger(input: &Extract, batch: Vec<usize>, semaphore: Arc<Semaphore>) -> Result<Vec<Value>, VideoError> {
    let algorithm = "algo://deeplearning/IllustrationTagger/0.2.3";
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;
    let mut output: Vec<Value> = Vec::new();
    for _ in 0..remote_pre_frames.len() {
        let json = json!({
        "image": remote_pre_frames.iter()
            .map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
        });

        //println!("acquiring semaphore");
        semaphore.acquire();
        let response: AlgoResponse = try_algorithm(input.client(), &algorithm, &json)?;
        semaphore.release();
        //println!("releasing semaphore");

        let output_json: Value = response.into_json()
            .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
        output.push(output_json);
    }
    Ok(output)
}

pub fn advanced_single(input: &Extract, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput, semaphore: Arc<Semaphore>) -> Result< Vec<Value>, VideoError> {
    let mut output: Vec<Value> = Vec::new();
    let local_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    batch_upload_file(&local_frames, &remote_frames, input.client())?;
    semaphore.acquire();
    for _ in 0..remote_frames.len() {
        let json: Value = replace_variables_extract(algo_input, Right(remote_frames.iter().next().unwrap()))?;

        //println!("acquiring semaphore");
        let response: AlgoResponse = try_algorithm(input.client(), &algorithm, &json)?;
        //println!("releasing semaphore");

        let output_json: Value = response.into_json()
            .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
        output.push(output_json);
    }
    semaphore.release();
    Ok(output)
}

pub fn advanced_batch(input: &Extract, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput, semaphore: Arc<Semaphore>) -> Result< Vec<Value>, VideoError> {
    let local_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;

    batch_upload_file(&local_frames, &remote_frames, input.client())?;
    let json: Value = replace_variables_extract(algo_input, Left(&remote_frames))?;

    //println!("acquiring semaphore");
    semaphore.acquire();
    let response: AlgoResponse = try_algorithm(input.client(), &algorithm, &json)?;
    semaphore.release();
    //println!("releasing semaphore");

    let output_json: Value = response.into_json()
        .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
    let output: Vec<Value> = output_json.as_array().unwrap().iter().map(|dat| {dat.clone()}).collect::<Vec<_>>();
    Ok(output)
}