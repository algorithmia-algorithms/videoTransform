
use algorithmia::algo::*;
use std::path::*;
use serde_json::Value;
use std::string::String;
use common::video_error::VideoError;
use common::structs::prelude::*;
use common::threading::Threadable;
use common::algo::{batch_file_path, try_algorithm, batch_upload_file};
use std_semaphore::Semaphore;
use std::sync::Arc;
use either::{Left, Right};
pub fn nudity_detection(input: &Threadable<Extract>, batch: Vec<usize>) -> Result<Vec<Value>, VideoError> {
    let algorithm = "algo://sfw/NudityDetectioni2v/0.2.4";
    let data = input.arc_data().clone();
    let semaphore = input.arc_semaphore();
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;

    batch_upload_file(&local_pre_frames, &remote_pre_frames,
                      data.client(), input.arc_term_signal())?;
    let json = json!({
    "image": remote_pre_frames.iter()
        .map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    });

    //println!("acquiring semaphore");
    semaphore.acquire();
    let response: AlgoResponse = try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal())?;
    semaphore.release();
    //println!("releasing semaphore");

    let output_json: Value = response.into_json()
        .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
    let output: Vec<Value> = output_json.as_array().unwrap().iter().map(|dat| {dat.clone()}).collect::<Vec<_>>();
    Ok(output)
}

pub fn illustration_tagger(input: &Threadable<Extract>, batch: Vec<usize>) -> Result<Vec<Value>, VideoError> {
    let algorithm = "algo://deeplearning/IllustrationTagger/0.2.3";
    let data = input.arc_data();
    let semaphore = input.arc_semaphore();
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;

    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;
    let mut output: Vec<Value> = Vec::new();
    for _ in 0..remote_pre_frames.len() {
        let json = json!({
        "image": remote_pre_frames.iter()
            .map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
        });

        //println!("acquiring semaphore");
        semaphore.acquire();
        let response: AlgoResponse = try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal())?;
        semaphore.release();
        //println!("releasing semaphore");

        let output_json: Value = response.into_json()
            .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
        output.push(output_json);
    }
    Ok(output)
}

pub fn advanced_single(input: &Threadable<Extract>, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput) -> Result< Vec<Value>, VideoError> {
    let mut output: Vec<Value> = Vec::new();
    let data = input.arc_data().clone();
    let semaphore = input.arc_semaphore();
    let local_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    batch_upload_file(&local_frames, &remote_frames, data.client(), input.arc_term_signal())?;
    semaphore.acquire();
    for _ in 0..remote_frames.len() {
        let json: Value = algo_input.replace_variables_extract(Right(remote_frames.iter().next().unwrap()))?;

        //println!("acquiring semaphore");
        let response: AlgoResponse = try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal())?;
        //println!("releasing semaphore");

        let output_json: Value = response.into_json()
            .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
        output.push(output_json);
    }
    semaphore.release();
    Ok(output)
}

pub fn advanced_batch(input: &Threadable<Extract>, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput) -> Result< Vec<Value>, VideoError> {
    let data = input.arc_data().clone();
    let semaphore = input.arc_semaphore();

    let local_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;

    batch_upload_file(&local_frames, &remote_frames, data.client(), input.arc_term_signal())?;
    let json: Value = algo_input.replace_variables_extract(Left(&remote_frames))?;

    //println!("acquiring semaphore");
    semaphore.acquire();
    let response: AlgoResponse = try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal())?;
    semaphore.release();
    //println!("releasing semaphore");

    let output_json: Value = response.into_json()
        .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json."))?;
    let output: Vec<Value> = output_json.as_array().unwrap().iter().map(|dat| {dat.clone()}).collect::<Vec<_>>();
    Ok(output)
}