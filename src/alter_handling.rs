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
use structs::alter;
use structs::extract;
use utilities;
use std::ops::Index;
use either::{Left, Right};
static MAX_ATTEMPTS: usize = 3usize;

fn batch_file_path(batch: &Vec<usize>, regex: &str, directory: &str) -> Result<Vec<String>, VideoError>
{
    let regexed = try!(batch.iter().map(|iter| {
        file_mgmt::from_regex(regex, iter.clone())
    }).collect::<Result<Vec<String>, VideoError>>());
    Ok(regexed.iter().map(|filename| {
        format!("{}/{}", directory, filename)
    }).collect::<Vec<String>>())
}

//retry 3 times, if it fails 3 times we exit hard.
fn batch_upload_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>, client: &Algorithmia) -> Result<(), VideoError>
{
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        let mut attempts = 0;
        loop {
            let result = file_mgmt::upload_file(&remote_file, &local_file, client);
            if result.is_ok(){
                break;
            }
            else if attempts > MAX_ATTEMPTS {
                let err = result.err().unwrap();
                return Err(format!("failed {} times to upload file {} : \n{}", attempts, local_file.display(), err).into())
            }
                attempts += 1;
        }
    }
    Ok(())
}

fn batch_get_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>, client: &Algorithmia) -> Result<Vec<PathBuf>, VideoError>
{
    let mut output: Vec<PathBuf> = Vec::new();
    let mut attempts = 0;
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        loop {
            let result = file_mgmt::get_file(&remote_file, &local_file, client);
            if result.is_ok() {
                break;
            }
                else if attempts > MAX_ATTEMPTS {
                    let err = result.err().unwrap();
                    return Err(format!("failed {} times to download file {} : \n{}", attempts, remote_file, err).into())
                }
            attempts += 1;
        }
    }
    Ok(output)
}


fn try_algorithm(client: &Algorithmia, algorithm: &str, input: &Json) -> Result<AlgoResponse, VideoError> {
    let mut attempts = 0;
    let mut final_result;
    loop {
        match client.algo(algorithm).timeout(500).pipe(input.clone()) {
            Ok(result) => {
                final_result = result;
                break;
            },
            Err(ApiError(ref err)) if attempts < MAX_ATTEMPTS => {
                println!("failed.");
                attempts += 1;
            },
            Err(err) => {
                println!("failed hard.");
                return Err(format!("algorithm {} failed: \n{}", &algorithm, err).into())
            }
        }
    }
    Ok(final_result)
}
///Everything needs to be owned when passed into these processing templates as rust multi-threading can't accept references.
pub fn deep_filter(input: &alter::Alter, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/DeepFilter/0.3.2";
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));
    let remote_post_frames: Vec<String> = try!(batch_file_path(&batch, input.output_regex(), input.remote_working()));
    let local_post_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.clone())}).collect::<Vec<PathBuf>>();

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));

    let mut obj = BTreeMap::new();
    obj.insert("images".to_string(), Json::Array(remote_pre_frames.iter().map(|frame| {Json::String(frame.clone())}).collect::<Vec<Json>>()));
    obj.insert("savePaths".to_string(), Json::Array(remote_post_frames.iter().map(|frame| {Json::String(frame.clone())}).collect::<Vec<Json>>()));
    obj.insert("filterName".to_string(), Json::String("gan_vogh".to_string()));
    let json = obj.to_json();

    try!(try_algorithm(input.client(), &algorithm, &json));

    let downloaded = try!(batch_get_file( &local_post_frames, &remote_post_frames, input.client()));
    Ok(downloaded)
}

//TODO: salnet right now has no batch mode, might change later.
pub fn salnet(input: &alter::Alter, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/SalNet/0.1.6";
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));
    let remote_post_frames: Vec<String> = try!(batch_file_path(&batch, input.output_regex(), input.remote_working()));
    let local_post_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.clone())}).collect::<Vec<PathBuf>>();

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));

    let mut obj = BTreeMap::new();
    for i in 0..remote_pre_frames.len(){
        obj.insert("image".to_string(), Json::String(remote_pre_frames.index(i).clone()));
        obj.insert("location".to_string(), Json::String(remote_post_frames.index(i).clone()));
        let json = obj.to_json();
        try!(try_algorithm(input.client(), &algorithm, &json));
    }
    let downloaded = try!(batch_get_file( &local_post_frames, &remote_post_frames, input.client()));
    Ok(downloaded)
}

//TODO: colorful_colorization has no batch mode, might change later
pub fn colorful_colorization(input: &alter::Alter, batch: Vec<usize>) -> Result<Vec<PathBuf>, VideoError>
{
    let algorithm = "algo://deeplearning/ColorfulImageColorization/1.0.1";
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| { PathBuf::from(str.to_owned()) }).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));
    let remote_post_frames: Vec<String> = try!(batch_file_path(&batch, input.output_regex(), input.remote_working()));
    let local_post_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap()))
        .iter().map(|str| { PathBuf::from(str.clone()) }).collect::<Vec<PathBuf>>();

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));

    let mut obj = BTreeMap::new();
    for i in 0..remote_pre_frames.len() {
        obj.insert("image".to_string(), Json::String(remote_pre_frames.index(i).clone()));
        obj.insert("location".to_string(), Json::String(remote_post_frames.index(i).clone()));
        let json = obj.to_json();
        try!(try_algorithm(input.client(), &algorithm, &json));
    }
    let downloaded = try!(batch_get_file(&local_post_frames, &remote_post_frames, input.client()));
    Ok(downloaded)
}

pub fn advanced_batch(input: &alter::Alter, batch: Vec<usize>, algorithm: String, algo_input: &utilities::SearchResult) -> Result<Vec<PathBuf>, VideoError>
{
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));
    let remote_post_frames: Vec<String> = try!(batch_file_path(&batch, input.output_regex(), input.remote_working()));
    let local_post_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.clone())}).collect::<Vec<PathBuf>>();

    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));

    let json: Json = try!(utilities::prepare_json(algo_input, Left(&remote_pre_frames), Left(&remote_post_frames)));
    println!("formatted json: \n {:?}", &json);
    try!(try_algorithm(input.client(), &algorithm, &json));

    let downloaded = try!(batch_get_file( &local_post_frames, &remote_post_frames, input.client()));
    Ok(downloaded)
}

//to keep things as interoperative as possible with batch mode, we keep batch file_path logic until its time to prepare_json, since it's always just a batch size of 1 it's an array with 1 element.
pub fn advanced_single(input: &alter::Alter, batch: Vec<usize>, algorithm: String, algo_input: &utilities::SearchResult) -> Result<Vec<PathBuf>, VideoError>
{
    let local_pre_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.input_regex(), input.local_input().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.to_owned())}).collect::<Vec<PathBuf>>();
    let remote_pre_frames: Vec<String> = try!(batch_file_path(&batch, input.input_regex(), input.remote_working()));
    let remote_post_frames: Vec<String> = try!(batch_file_path(&batch, input.output_regex(), input.remote_working()));
    let local_post_frames: Vec<PathBuf> = try!(batch_file_path(&batch, input.output_regex(), input.local_output().to_str().unwrap()))
        .iter().map(|str| {PathBuf::from(str.clone())}).collect::<Vec<PathBuf>>();
    try!(batch_upload_file(&local_pre_frames, &remote_pre_frames, input.client()));

    let json: Json = try!(utilities::prepare_json(algo_input, Right(remote_pre_frames.iter().next().unwrap()), Right(remote_post_frames.iter().next().unwrap())));
    println!("formatted json: \n {:?}", &json);
    try!(try_algorithm(input.client(), &algorithm, &json));

    let downloaded = try!(batch_get_file( &local_post_frames, &remote_post_frames, input.client()));
    Ok(downloaded)
}