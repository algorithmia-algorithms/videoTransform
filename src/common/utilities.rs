use algorithmia::prelude::*;
use algorithmia::error::ApiError;
use algorithmia::algo::*;
use serde_json;
use serde_json::Value;
use regex::Regex;
use common::file_mgmt::*;
use std::path::*;
use common::video_error::VideoError;
use std::collections::BTreeMap;
use std::time;
use either::{Either, Left, Right};
use std::time::Duration;
use std::ops::Index;
use std::thread;
use std::ops::IndexMut;
use std::collections::VecDeque;
static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";
static BATCH_OUTPUT: &'static str = "$BATCH_OUTPUT";
static SINGLE_OUTPUT: &'static str = "$SINGLE_OUTPUT";
static MAX_ATTEMPTS_ALGO: usize = 3usize;

#[derive(Debug, Clone)]
pub struct SearchResult{
    batch_single: String,
    in_path: VecDeque<String>,
    in_array_iter: Option<usize>,
    out_path: VecDeque<String>,
    out_array_iter: Option<usize>,
    source: Value,
}

impl SearchResult {
    pub fn new (batch_single: String, in_path: VecDeque<String>, in_array_iter: Option<usize>,
                out_path: VecDeque<String>, out_array_iter: Option<usize>, source: Value) -> SearchResult {
        SearchResult{batch_single: batch_single, in_path: in_path,
            in_array_iter: in_array_iter, out_array_iter: out_array_iter,
            out_path: out_path, source: source}
    }
    pub fn option(&self) -> &str {&self.batch_single}
    pub fn in_path(&self) -> &VecDeque<String> {&self.in_path}
    pub fn in_array_iter(&self) -> Option<usize> {self.in_array_iter}
    pub fn out_path(&self) -> &VecDeque<String> {&self.out_path}
    pub fn out_array_iter(&self) -> Option<usize> {self.out_array_iter}
    pub fn source(&self) -> &Value {&self.source}
}

//depth first search of json blob, returns true if the keyword was found, false if it wasn't.
fn search_json(json: &Value, path: &mut VecDeque<String>, keyword: &str) -> Result<(bool, Option<usize>), VideoError> {
    if json.is_object() {
        let tree = json.as_object().unwrap();
        for (k, v) in tree {
            path.push_back(k.to_string());
            match v {
                &Value::String(ref text) if text == keyword => {
                    return Ok((true, None))
                }
                &Value::Object(ref cursor) => {
                    let (found, iter) = try!(search_json(&v, path, keyword));
                    if found {
                        return Ok((found, iter))
                    }
                }
                &Value::Array(ref arr) => {
                    for i in 0..arr.len() {
                        let val = arr.index(i);
                        match val {
                            &Value::String(ref str) if str == keyword => {
                                return Ok((true, Some(i)))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            path.pop_back();
        }
            Ok((false, None))
    } else if json.is_array() {
        let arr = json.as_array().unwrap();
        for i in 0..arr.len() {
            let mut val = arr.index(i);
            match val {
                &Value::String(ref str) if str == keyword => {
                    return Ok((true, Some(i)))
                }
                _ => {}
            }
        }
            Ok((false, None))
    } else {
        Err(format!("advanced input was neither a json object or an array.").into())
    }
}
//gets the cursor of the json tree into the requested scope, except it leaves the path_vec objects with one value in it, this is used for matching & replacing.
fn get_cursor<'a>(input: &'a mut Value, path_vec: &mut VecDeque<String>) -> Result< &'a mut Value, VideoError> {
    if input.is_object() {
        let path = path_vec.pop_front().ok_or(format!("did not exit properly, path contained a json object, not a final node of array or string."))?;
        let b_tree_obj = input.as_object_mut().unwrap();
        let mut json = b_tree_obj.get_mut(&path).ok_or(format!("path traversal invalid, path not found."))?;
        Ok(try!(get_cursor(json, path_vec)))
    } else {
        Ok(input)
    }
}

//using the path variable, this traverses the mutable base Json tree to find the keyword containing key/value pair, once found it replaces the key/value pair with either an Array or String, depending on the data.
fn replace_json(base: &mut Value, paths: &mut VecDeque<String>, array_iter: Option<usize>, data: Either<&Vec<String>, &str>) -> Result< (), VideoError> {
    //since for our usecase we'll never see obj -> array -> obj -> result, we can safely exit when it finds the key points to an array.
    let cursor: &mut Value = try!(get_cursor(base, paths));
    //we if check instead of pattern match because we don't want to borrow cursor, for the string branch, since we need to replace the json, not the string.
    if cursor.is_array() {
        let mut arr: &mut Vec<Value> = cursor.as_array_mut().unwrap();
        let index = try!(array_iter.ok_or(format!("array iter must be passed if the final node is an array type.")));
        match data {
            Left(batch) => {
                let mut formatted_array = Value::Array(batch.iter()
                    .map(|d| { Value::String(d.to_string()) }).collect::<Vec<Value>>());
                *arr.index_mut(index) = formatted_array;
            }
            Right(single) => {
                let mut formatted_string = Value::String(single.to_string());
                *arr.index_mut(index) =  formatted_string;
            }
        }
        Ok(())
    } else if cursor.is_string() {
        match data {
            Left(array) => {
                *cursor = Value::Array(array.iter()
                    .map(|d| { Value::String(d.to_string()) })
                    .collect::<Vec<Value>>());
            }
            Right(single) => {
                *cursor = Value::String(single.to_string());
            }
        }
        Ok(())
    } else { Err(format!("something went wrong, you should never get here.").into()) }
}

pub fn prepare_json_extract(obj: &SearchResult, input: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
    let mut mutable: Value = obj.source().clone();
    let mut in_path = obj.in_path().clone();
    //for input
    try!(replace_json(&mut mutable, &mut in_path, obj.in_array_iter(), input));
    Ok(mutable)
}


pub fn prepare_json_alter(obj: &SearchResult, input: Either<&Vec<String>, &str>, output: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
    let mut mutable: Value = obj.source().clone();
    let mut in_path = obj.in_path().clone();
    let mut out_path = obj.out_path().clone();
    //for input
    try!(replace_json(&mut mutable, &mut in_path, obj.in_array_iter(), input));
    //for output
    try!(replace_json(&mut mutable, &mut out_path, obj.out_array_iter(), output));
    Ok(mutable)
}

//only difference between extract & alter format search, extract only cares about input keywords, it doesn't have output keywords.
pub fn extract_format_search(json: &Value) -> Result<SearchResult, VideoError> {
    let mut batch_in_path = VecDeque::new();
    let mut single_in_path = VecDeque::new();
    let (batch_in, batch_iter) = try!(search_json(json, &mut batch_in_path, BATCH_INPUT));
    let (single_in, single_iter) = try!(search_json(json, &mut single_in_path, SINGLE_INPUT));
    if batch_in {
        println!("json parsed as batch input.");
        Ok(SearchResult::new("batch".to_string(), batch_in_path.into(), batch_iter, VecDeque::new(), None, json.clone()))
    } else if single_in {
        println!("json parsed as single input.");
        Ok(SearchResult::new("single".to_string(), single_in_path.into(), single_iter, VecDeque::new(), None, json.clone()))
    } else {
        Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
    }
}
pub fn alter_format_search(json: &Value) -> Result<SearchResult, VideoError> {
    let mut batch_in_path = VecDeque::new();
    let mut batch_out_path = VecDeque::new();
    let mut single_in_path = VecDeque::new();
    let mut single_out_path = VecDeque::new();
    let (batch_in, batch_in_iter) = search_json(json, &mut batch_in_path, BATCH_INPUT)?;
    let (batch_out, batch_out_iter) = search_json(json, &mut batch_out_path, BATCH_OUTPUT)?;
    let (single_in, single_in_iter) = search_json(json, &mut single_in_path, SINGLE_INPUT)?;
    let (single_out, single_out_iter) = search_json(json, &mut single_out_path, SINGLE_OUTPUT)?;
    if batch_in && batch_out {
        println!("json parsed as batch input.");
        Ok(SearchResult::new("batch".to_string(), batch_in_path, batch_in_iter, batch_out_path, batch_out_iter, json.clone()))
    }
    else if batch_in || batch_out {
        Err(String::from("json parsing error:\nif batch selected both $BATCH_INPUT and $BATCH_OUTPUT must be defined.").into())
    }
    else if single_in && single_out {
        println!("json parsed as single input.");
        Ok(SearchResult::new("single".to_string(), single_in_path, single_in_iter, single_out_path, single_out_iter, json.clone()))
    }
    else if single_in || single_out {
        Err(String::from("json parsing error:\nif single selected both $SINGLE_INPUT and $SINGLE)OUTPUT must be defined.").into())
    }
    else {
        Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
    }
}

//takes an array of json blobs & a frame stamp, returns a json object with an array of json objects containing the frame's timestamp & data.
pub fn combine_extracted_data(data: &Vec<Value>, frame_stamp: f64) -> Result<Value, VideoError> {
    let mut combined: Vec<Value> = Vec::new();
    for iter in 0..data.len() {
        let ref value: Value = data[iter];
        let time_s: f64 = iter as f64 * frame_stamp;
        let json = json!({
            "timestamp": time_s,
            "data": value.clone()
        });
        combined.push(json);
    }
    let finale = json!({
    "frame_data" : combined
    });
    Ok(finale)
}

//exits early if the or if the output path is invalid.
pub fn early_exit(client: &Algorithmia, output_path: &str) -> Result<(), VideoError> {
    //try to upload a 0 size file to the output path, then delete it. if both succeed then the path is valid.
    let r: Result<_, VideoError> = client.file(output_path).put("").map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to upload.\n{}", output_path, err).into());
    let j: Result<_, VideoError> = client.file(output_path).delete().map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to delete.\n{}", output_path, err).into());
    try!(r);
    try!(j);
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

pub fn batch_get_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>, client: &Algorithmia) -> Result<Vec<PathBuf>, VideoError>
{
    let mut output: Vec<PathBuf> = Vec::new();
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        output.push(try!(get_file(&remote_file, &local_file, client)));
    }
    Ok(output)
}


pub fn try_algorithm(client: &Algorithmia, algorithm: &str, input: &Value) -> Result<AlgoResponse, VideoError> {
    let mut attempts = 0;
    let mut final_result;
    loop {
        match client.algo(algorithm).timeout(500).pipe(input.clone()) {
            Ok(result) => {
                final_result = result;
                break;
            },
            Err(ref err) if attempts < MAX_ATTEMPTS_ALGO => {
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