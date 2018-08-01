use std::path::*;
use serde_json::Value;
use std::string::String;
use common::video_error::VideoError;
use common::structs::prelude::*;
use common::algo::{batch_file_path, try_algorithm, batch_upload_file, batch_get_file};
use common::threading::{Terminator, Threadable};
use std_semaphore::Semaphore;
use std::sync::Arc;
use std::ops::Index;
use either::{Left, Right};

///Everything needs to be owned when passed into these processing templates as rust multi-threading can't accept references.
pub fn deep_filter(input: &Threadable<Alter>, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/DeepFilter/0.6.0";
    let data = input.arc_data().clone();

    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, data.output_regex(), data.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, data.output_regex(), data.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;
    let json = json!({
    "images": remote_pre_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    "savePaths": remote_post_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
    "filterName" : "gan_vogh"
    });
    try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal(), input.arc_semaphore())?;
    let downloaded = batch_get_file(&local_post_frames,
                                    &remote_post_frames, data.client(), input.arc_term_signal())?;
    Ok(downloaded)
}

//TODO: salnet right now has no batch mode, might change later.
pub fn salnet(input: &Threadable<Alter>, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/SalNet/0.2.0";
    let data = input.arc_data().clone();

    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, data.output_regex(), data.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, data.output_regex(), data.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;

    for i in 0..remote_pre_frames.len() {
        let json = json!({
                             "image": remote_pre_frames.index(i).clone(),
                             "location": remote_post_frames.index(i).clone()
                         });
        try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal(), input.arc_semaphore())?;
    }
    let downloaded = batch_get_file(&local_post_frames,
                                    &remote_post_frames, data.client(), input.arc_term_signal())?;
    Ok(downloaded)
}

//TODO: colorful_colorization has no batch mode, might change later
pub fn colorful_colorization(input: &Threadable<Alter>, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/ColorfulImageColorization/1.1.6";
    let data = input.arc_data().clone();

    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, data.output_regex(), data.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, data.output_regex(), data.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;

    let json = json!({
        "image": remote_pre_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>(),
        "location": remote_post_frames.iter().map(|frame| {Value::String(frame.clone())}).collect::<Vec<Value>>()
    });
    try_algorithm(data.client(), &algorithm, &json,input.arc_term_signal(), input.arc_semaphore())?;
    let downloaded = batch_get_file(&local_post_frames, &remote_post_frames,
                                    data.client(), input.arc_term_signal())?;
    Ok(downloaded)
}

pub fn advanced_batch(input: &Threadable<Alter>, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput) -> Result<Vec<PathBuf>, VideoError>
{

    let data = input.arc_data().clone();
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, data.output_regex(), data.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, data.output_regex(), data.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;

    let json: Value = algo_input.replace_variables_transform(Left(&remote_pre_frames),
                                                             Left(&remote_post_frames))?;
    try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal(), input.arc_semaphore())?;

    let downloaded = batch_get_file(&local_post_frames,
                                    &remote_post_frames, data.client(), input.arc_term_signal())?;
    Ok(downloaded)
}

//to keep things as interoperative as possible with batch mode, we keep batch file_path logic until its time to prepare_json, since it's always just a batch size of 1 it's an array with 1 element.
pub fn advanced_single(input: &Threadable<Alter>, batch: Vec<usize>, algorithm: String, algo_input: &AdvancedInput) -> Result<Vec<PathBuf>, VideoError>
{

    let data = input.arc_data();
    let local_pre_frames: Vec<PathBuf> = batch_file_path(&batch, data.input_regex(), data.local_input().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = batch_file_path(&batch, data.input_regex(), data.remote_working())?;
    let remote_post_frames: Vec<String> = batch_file_path(&batch, data.output_regex(), data.remote_working())?;
    let local_post_frames: Vec<PathBuf> = batch_file_path(&batch, data.output_regex(), data.local_output().to_str().unwrap())?
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();


    batch_upload_file(&local_pre_frames, &remote_pre_frames, data.client(), input.arc_term_signal())?;
    for _ in 0..remote_pre_frames.len() {
        let json: Value = algo_input.replace_variables_transform(Right(remote_pre_frames.iter().next().unwrap()),
                                                                 Right(remote_post_frames.iter().next().unwrap()))?;
        try_algorithm(data.client(), &algorithm, &json, input.arc_term_signal(), input.arc_semaphore())?;
    }
    let downloaded = batch_get_file(&local_post_frames,
                                    &remote_post_frames, data.client(), input.arc_term_signal())?;
    Ok(downloaded)
}