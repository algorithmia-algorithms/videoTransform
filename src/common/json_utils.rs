use std::collections::VecDeque;
use serde_json;
use serde_json::Value;
use common::video_error::VideoError;
use either::{Either, Left, Right};
use std::ops::{Index, IndexMut};

static BATCH_INPUT: &'static str = "$BATCH_INPUT";
static SINGLE_INPUT: &'static str = "$SINGLE_INPUT";
static BATCH_OUTPUT: &'static str = "$BATCH_OUTPUT";
static SINGLE_OUTPUT: &'static str = "$SINGLE_OUTPUT";


#[derive(Debug, Clone)]
pub struct AdvancedInput {
    batch_single: String,
    in_path: VecDeque<String>,
    in_array_iter: Option<usize>,
    out_path: VecDeque<String>,
    out_array_iter: Option<usize>,
    source: Value,
}

impl AdvancedInput {
    pub fn new (batch_single: String, in_path: VecDeque<String>, in_array_iter: Option<usize>,
                out_path: VecDeque<String>, out_array_iter: Option<usize>, source: Value) -> AdvancedInput {
        AdvancedInput {batch_single: batch_single, in_path: in_path,
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
//gets the cursor of the json tree into the requested scope, except it leaves the path_vec objects with one value in it, this is used for matching & replacing.
fn get_cursor<'a>(input: &'a mut Value, path_vec: &mut VecDeque<String>) -> Result< &'a mut Value, VideoError> {
    if input.is_object() {
        let path = path_vec.pop_front().ok_or(format!("did not exit properly, path contained a json object, not a final node of array or string."))?;
        let b_tree_obj = input.as_object_mut().unwrap();
        let mut json = b_tree_obj.get_mut(&path).ok_or(format!("path traversal invalid, path not found."))?;
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
