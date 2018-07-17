use algorithmia::Algorithmia;
use algorithmia::algo::*;
use algorithmia::error::ApiError;
use std::path::*;
use serde_json::Value;
use common::utilities::replace_variables_transform;
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

///Everything needs to be owned when passed into these processing templates as rust multi-threading can't accept references.
pub fn deep_filter(input: &Alter, batch: Vec<usize>, semaphore: Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/DeepFilter/0.6.0";
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, input.output_regex(), input.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;
    let json = json!({
    "images": remote_pre_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    "savePaths": remote_post_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    "filterName" : "gan_vogh"
    });
    //    println!("acquiring semaphore");
    semaphore.acquire();
    try_algorithm(input.client(), &algorithm, &json)?;
    semaphore.release();
    //    println!("releasing semaphore");
    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames, input.client())?;
    Ok(downloaded)
}

//TODO: salnet right now has no batch mode, might change later.
pub fn salnet(input: &Alter, batch: Vec<usize>, semaphore: Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/SalNet/0.2.0";
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, input.output_regex(), input.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;

    for i in 0..remote_pre_frames.len() {
        let json = json!({
                             "image": remote_pre_frames.index(i).clone(),
                             "location": remote_post_frames.index(i).clone()
                         });
        semaphore.acquire();
        try_algorithm(input.client(), &algorithm, &json)?;
        semaphore.release();
    }
    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames, input.client())?;
    Ok(downloaded)
}

//TODO: colorful_colorization has no batch mode, might change later
pub fn colorful_colorization(input: &Alter, batch: Vec<usize>, semaphore: Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/ColorfulImageColorization/1.1.6";
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, input.output_regex(), input.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;

    let json = json!({
        "image": remote_pre_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
        "location": remote_post_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>()
    });
    semaphore.acquire();
    try_algorithm(input.client(), &algorithm, &json)?;
    semaphore.release();
    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames, input.client())?;
    Ok(downloaded)
}

pub fn advanced_batch(input: &Alter, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput, semaphore: Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError>
{
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, input.output_regex(), input.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;

    let json: Value = replace_variables_transform(algo_input, Left(&remote_pre_frames), Left(&remote_post_frames))?;
    semaphore.acquire();
    try_algorithm(input.client(), &algorithm, &json)?;
    semaphore.release();

    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames, input.client())?;
    Ok(downloaded)
}

//to keep things as interoperative as possible with batch mode, we keep batch file_path logic until its time to prepare_json, since it's always just a batch size of 1 it's an array with 1 element.
pub fn advanced_single(input: &Alter, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput, semaphore: Arc<Semaphore>) -> Result<Vec<PathBuf>, VideoError>
{
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, input.input_regex(), input.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, input.output_regex(), input.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();
    batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client())?;
    semaphore.acquire();
    for _ in 0..remote_pre_frames.len() {
        let json: Value = replace_variables_transform(algo_input, Right(remote_pre_frames.iter().next().unwrap()), Right(remote_post_frames.iter().next().unwrap()))?;
        try_algorithm(input.client(), &algorithm, &json)?;
    }
    semaphore.release();
    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames, input.client())?;
    Ok(downloaded)
}