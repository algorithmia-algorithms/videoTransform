use algorithmia::{client, Algorithmia};
use algorithmia::algo::*;
use algorithmia::error::Error::ApiError;
use std::path::*;
use rustc_serialize::json::{self, Json, ToJson};
use std::collections::BTreeMap;
use file_mgmt;
use std::error::Error;
use video_error::VideoError;
use std::ffi::OsStr;
use structs::extract;
use utilities::*;
use std::ops::Index;
use either::{Left, Right};

pub fn nudity_detection(input: &extract::Extract, batch: Vec<usize>) -> Result<Vec<Json>, VideoError> {
    let algorithm = "algo://sfw/NudityDetectioni2v/0.2.4";
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));
    let mut obj = BTreeMap::new();
    obj.insert("image".to_string(), Json::Array(remote_pre_frames.iter()
        .map(|frame| {Json::String(frame.clone())}).collect::<Vec<Json>>()));
    let input_json: Json= obj.to_json();
    let response: AlgoResponse = try!(try_algorithm(input.client(), algorithm, &input_json));
    let output_json: Json = try!(response.as_json()
        .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json.")));
    let output: Vec<Json> = output_json.as_array().unwrap().iter().map(|dat| {dat.clone()}).collect::<Vec<_>>();
    Ok(output)
}

pub fn illustration_tagger(input: &extract::Extract, batch: Vec<usize>) -> Result<Vec<Json>, VideoError> {
    let algorithm = "algo://deeplearning/IllustrationTagger/0.2.3";
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));
    let mut output: Vec<Json> = Vec::new();
    for i in 0..remote_pre_frames.len() {
        let mut obj = BTreeMap::new();
        obj.insert("image".to_string(), Json::Array(remote_pre_frames.iter()
            .map(|frame| { Json::String(frame.clone()) }).collect::<Vec<Json>>()));
        let input_json: Json = obj.to_json();
        let response: AlgoResponse = try!(try_algorithm(input.client(), algorithm, &input_json));
        let output_json: Json = try!(response.as_json()
            .ok_or(format!("algorithm failed, ending early:\n algorithm response did not parse as valid json.")));
        output.push(output_json);
    }
    Ok(output)
}

pub fn advanced_single(input: &extract::Extract, batch: Vec<usize>, algorithm: String, search: &SearchResult) -> Result< Vec<Json>, VideoError> {
    unimplemented!()
}

pub fn advanced_batch(input: &extract::Extract, batch: Vec<usize>, algorithm: String, search: &SearchResult) -> Result< Vec<Json>, VideoError> {
    unimplemented!()
}
