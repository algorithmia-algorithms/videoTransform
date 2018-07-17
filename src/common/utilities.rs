use std::collections::VecDeque;
use serde_json::Value;
use common::json_utils::{AdvancedInput, replace_json, search_json};
use either::Either;
use common::video_error::VideoError;
use std::ops::{Index, IndexMut};

static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";
static BATCH_OUTPUT: &'static str = "$BATCH_OUTPUT";
static SINGLE_OUTPUT: &'static str = "$SINGLE_OUTPUT";


pub fn replace_variables_extract(obj: &AdvancedInput, input: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
    let mut mutable: Value = obj.source().clone();
    let mut in_path = obj.in_path().clone();
    //for input
    replace_json(&mut mutable, &mut in_path, obj.in_array_iter(), input)?;
    Ok(mutable)
}


pub fn replace_variables_transform(obj: &AdvancedInput, input: Either<&Vec<String>, &str>, output: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
    let mut mutable: Value = obj.source().clone();
    let mut in_path = obj.in_path().clone();
    let mut out_path = obj.out_path().clone();
    //for input
    replace_json(&mut mutable, &mut in_path, obj.in_array_iter(), input)?;
    //for output
    replace_json(&mut mutable, &mut out_path, obj.out_array_iter(), output)?;
    Ok(mutable)
}


//only difference between extract & alter format search, extract only cares about input keywords, it doesn't have output keywords.
pub fn advanced_input_search_extract(json: &Value) -> Result<AdvancedInput, VideoError> {
    let mut batch_in_path = VecDeque::new();
    let mut single_in_path = VecDeque::new();
    let (batch_in, batch_iter) = try!(search_json(json, &mut batch_in_path, BATCH_INPUT));
    let (single_in, single_iter) = try!(search_json(json, &mut single_in_path, SINGLE_INPUT));
    if batch_in {
        println!("json parsed as batch input.");
        Ok(AdvancedInput::new("batch".to_string(), batch_in_path.into(), batch_iter, VecDeque::new(), None, json.clone()))
    } else if single_in {
        println!("json parsed as single input.");
        Ok(AdvancedInput::new("single".to_string(), single_in_path.into(), single_iter, VecDeque::new(), None, json.clone()))
    } else {
        Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
    }
}


pub fn advanced_input_search_transform(json: &Value) -> Result<AdvancedInput, VideoError> {
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
        Ok(AdvancedInput::new("batch".to_string(), batch_in_path, batch_in_iter, batch_out_path, batch_out_iter, json.clone()))
    } else if batch_in || batch_out {
        Err(String::from("json parsing error:\nif batch selected both $BATCH_INPUT and $BATCH_OUTPUT must be defined.").into())
    } else if single_in && single_out {
        println!("json parsed as single input.");
        Ok(AdvancedInput::new("single".to_string(), single_in_path, single_in_iter, single_out_path, single_out_iter, json.clone()))
    } else if single_in || single_out {
        Err(String::from("json parsing error:\nif single selected both $SINGLE_INPUT and $SINGLE_OUTPUT must be defined.").into())
    } else {
        Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
    }
}



//takes an array of json blobs & a frame stamp, returns a json object with an array of json objects containing the frame's timestamp & data.
pub fn combine_data_extract(data: &Vec<Value>, frame_stamp: f64) -> Result<Value, VideoError> {
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