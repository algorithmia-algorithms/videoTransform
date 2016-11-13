use rustc_serialize::json::*;
use algorithmia::{Algorithmia};
use algorithmia::data::file::*;
use algorithmia::error::Error::ApiError;
use algorithmia::algo::*;
use regex::Regex;
use file_mgmt::*;
use std::path::*;
use video_error::VideoError;
use std::collections::BTreeMap;
use time::{Tm, strftime};
use either::{Either, Left, Right};
static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";
static BATCH_OUTPUT: &'static str = "$BATCH_OUTPUT";
static SINGLE_OUTPUT: &'static str = "$SINGLE_OUTPUT";
static MAX_ATTEMPTS: usize = 3usize;

#[derive(Debug, Clone)]
pub struct SearchResult{
    batch_single: String,
    in_path: Vec<String>,
    out_path: Vec<String>,
    source: Json,
}

impl SearchResult {
    pub fn new (batch_single: String, in_path: Vec<String>, out_path: Vec<String>, source: Json) -> SearchResult {
        SearchResult{batch_single: batch_single, in_path: in_path, out_path: out_path, source: source}
    }
    pub fn option(&self) -> &str {&self.batch_single}
    pub fn in_path(&self) -> &Vec<String> {self.in_path.as_ref()}
    pub fn out_path(&self) -> &Vec<String> {self.out_path.as_ref()}
    pub fn source(&self) -> &Json {&self.source}
}

//depth first search of json blob, returns true if the keyword was found, false if it wasn't
fn search_json(tree: &mut BTreeMap<String, Json>, path: &mut Vec<String>, keyword: &str) -> bool {
    for (k, v) in tree {
        path.push(k.to_string());
        match v {
            &mut Json::String(ref text) if text == keyword => {
                return true
            }
            &mut Json::Object(ref mut obj) => {
                if search_json(obj, path, keyword) {return true}
            }
//            Json::Array(mut arr) => {
//                // this won't work with above type sig (need generic sig)
//            }
            _ => {}
        }
        path.pop();
    }
    false
}

//using the path variable, this traverses the mutable base Json tree to find the keyword containing key/value pair, once found it replaces the key/value pair with either an Array or String, depending on the data.
fn replace_json(base: &mut BTreeMap<String, Json>, path: &Vec<String>, data: Either<&Vec<String>, &str>) -> Result< (), VideoError> {
    println!("path: {:?}", path);
    let mut cursor: & mut BTreeMap <String, Json> = base;
    for i in 0..(path.len()-1) {
        let mutable = try!({cursor}.get_mut(&path[i]).ok_or(format!("path {} unavailable for cursor", &path[i])));
        cursor = try!(mutable.as_object_mut().ok_or(format!("cursor is not a json object, invalid path")));
    }
    let direct_path = try!(path.last().ok_or(format!("path was an empty Vec!")));
    cursor.remove(direct_path);
    match data {
        Left(array) => {
            cursor.insert(direct_path.to_string(), Json::Array(array.iter()
                .map( |d | {Json::String(d.to_string())})
                .collect::<Vec<Json>>()));
        }
        Right(single) => {
            cursor.insert(direct_path.to_string(), Json::String(single.to_string()));
        }
    }
    Ok(())
}

//traverses the json blob using the Path variable, finds and replaces a key/value pair.
pub fn prepare_json(obj: &SearchResult, input: Either<&Vec<String>, &str>, output: Either<&Vec<String>, &str>) -> Result<Json, VideoError> {
    let mut mutable = obj.source().as_object().unwrap().clone();
    //for input
    try!(replace_json(&mut mutable, obj.in_path(), input));
    //for output
    try!(replace_json(&mut mutable, obj.out_path(), output));
    Ok(Json::Object(mutable.clone()))
}
pub fn format_search(json: &Json) -> Result<SearchResult, VideoError> {
    let mut mutable_json: Json = json.clone();
    let mut obj = mutable_json.as_object_mut().unwrap();
    let mut batch_in_path = Vec::new();
    let mut batch_out_path = Vec::new();
    let mut single_in_path = Vec::new();
    let mut single_out_path = Vec::new();
    let batch_in = search_json(&mut obj, &mut batch_in_path, BATCH_INPUT);
    let batch_out = search_json(&mut obj, &mut batch_out_path, BATCH_OUTPUT);
    let single_in = search_json(&mut obj, &mut single_in_path, SINGLE_INPUT);
    let single_out = search_json(&mut obj, &mut single_out_path, SINGLE_OUTPUT);
    if batch_in && batch_out {
        println!("json parsed as batch input.");
        Ok(SearchResult::new("batch".to_string(), batch_in_path, batch_out_path, json.clone()))
    }
    else if batch_in || batch_out {
        Err(String::from("json parsing error:\nif batch selected both $BATCH_INPUT and $BATCH_OUTPUT must be defined.").into())
    }
    else if single_in && single_out {
        println!("json parsed as single input.");
        Ok(SearchResult::new("single".to_string(), single_in_path, single_out_path, json.clone()))
    }
    else if single_in || single_out {
        Err(String::from("json parsing error:\nif single selected both $SINGLE_INPUT and $SINGLE)OUTPUT must be defined.").into())
    }
    else {
        Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
    }
}

//takes an array of json blobs & a frame stamp, returns a json object with an array of json objects containing the frame's timestamp & data.
pub fn combine_extracted_data(data: &Vec<Json>, frame_stamp: f64) -> Result<Json, VideoError> {
    let mut finale = BTreeMap::new();
    let mut json: Vec<Json> = Vec::new();
    for iter in 0..data.len() {
        let mut obj = BTreeMap::new();
        let ref value: Json = data[iter];
        let time_s: f64 = iter as f64 * frame_stamp;
        obj.insert("timestamp".to_string(), Json::F64(time_s));
        obj.insert("data".to_string(), value.clone());
        json.push(obj.to_json());
    }
    finale.insert("frame_data".to_string(), Json::Array(json));
    Ok(finale.to_json())
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
        let mut attempts = 0;
        loop {
            let result = upload_file(&remote_file, &local_file, client);
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

pub fn batch_get_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>, client: &Algorithmia) -> Result<Vec<PathBuf>, VideoError>
{
    let mut output: Vec<PathBuf> = Vec::new();
    let mut attempts = 0;
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        loop {
            let result = get_file(&remote_file, &local_file, client);
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


pub fn try_algorithm(client: &Algorithmia, algorithm: &str, input: &Json) -> Result<AlgoResponse, VideoError> {
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