use std::collections::VecDeque;
//use serde_json;
use serde_json::Value;
use common::video_error::VideoError;
use either::{Either, Left, Right};
use std::ops::{Index, IndexMut};


//depth first search of json blob, returns true if the keyword was found, false if it wasn't.
pub fn search_json(json: &Value, path: &mut VecDeque<String>, keyword: &str) -> Result<(bool, Option<usize>), VideoError> {
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
        return Ok((false, None))
    }
                else if json.is_string() {
                if json.as_str().unwrap() == keyword {
                    return Ok((true, None))
                }
                else {
                    return Ok((false, None))
                }
            }
        else {
            Err(format!("advanced input was neither a json object or an array.").into())
        }
}
//gets the cursor of the json tree into the requested scope
fn get_cursor<'a>(input: &'a mut Value, path_vec: &mut VecDeque<String>) -> Result< &'a mut Value, VideoError> {
    if input.is_object() {
        let path = path_vec.pop_front().ok_or(format!("did not exit properly, path contained a json object, not a final node of array or string."))?;
        let b_tree_obj = input.as_object_mut().unwrap();
        let json = b_tree_obj.get_mut(&path).ok_or(format!("path traversal invalid, path not found."))?;
        Ok(get_cursor(json, path_vec)?)
    } else {
        Ok(input)
    }
}

//using the path variable, this traverses the mutable base Json tree to find the keyword containing key/value pair, once found it replaces the key/value pair with either an Array or String, depending on the data.
pub fn replace_json(base: &mut Value, paths: &mut VecDeque<String>, array_iter: Option<usize>, data: Either<&Vec<String>, &str>) -> Result< (), VideoError> {
    //since for our usecase we'll never see obj -> array -> obj -> result, we can safely exit when it finds the key points to an array.
    let cursor: &mut Value = get_cursor(base, paths)?;
    //we if check instead of pattern match because we don't want to borrow cursor, for the string branch, since we need to replace the json, not the string.
    if cursor.is_array() {
        let arr: &mut Vec<Value> = cursor.as_array_mut().unwrap();
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
