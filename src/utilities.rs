use rustc_serialize::json::*;
use regex::Regex;
use video_error::VideoError;
use std::collections::BTreeMap;
use either::{Either, Left, Right};
static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";
static BATCH_OUTPUT: &'static str = "$BATCH_OUTPUT";
static SINGLE_OUTPUT: &'static str = "$SINGLE_OUTPUT";

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

//TODO: implement this
pub fn combine_extracted_data(data: &Vec<Json>, frame_stamp: f64) -> Result<Json, VideoError> {
    unimplemented!()
}