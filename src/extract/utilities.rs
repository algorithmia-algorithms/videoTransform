use std::collections::VecDeque;
use serde_json::Value;
use common::json_utils::{AdvancedInput, replace_json, search_json};
use either::Either;
use common::video_error::VideoError;
use std::ops::{Index, IndexMut};

static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";



pub fn process_advanced_input(obj: &AdvancedInput, input: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
    let mut mutable: Value = obj.source().clone();
    let mut in_path = obj.in_path().clone();
    //for input
    replace_json(&mut mutable, &mut in_path, obj.in_array_iter(), input)?;
    Ok(mutable)
}

//only difference between extract & alter format search, extract only cares about input keywords, it doesn't have output keywords.
pub fn advanced_input_search(json: &Value) -> Result<AdvancedInput, VideoError> {
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



//takes an array of json blobs & a frame stamp, returns a json object with an array of json objects containing the frame's timestamp & data.
pub fn combine_data(data: &Vec<Value>, frame_stamp: f64) -> Result<Value, VideoError> {
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