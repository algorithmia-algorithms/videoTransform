
use std::collections::VecDeque;
use serde_json::Value;
use either::Either;
use common::video_error::VideoError;
use common::json_utils::{replace_json, search_json};

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
    fn new (batch_single: String, in_path: VecDeque<String>, in_array_iter: Option<usize>,
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


    pub fn replace_variables_extract(&self, input: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
        let mut mutable: Value = self.source().clone();
        let mut in_path = self.in_path().clone();
        //for input
        replace_json(&mut mutable, &mut in_path, self.in_array_iter(), input)?;
        Ok(mutable)
    }


    pub fn replace_variables_transform(&self, input: Either<&Vec<String>, &str>, output: Either<&Vec<String>, &str>) -> Result<Value, VideoError> {
        let mut mutable: Value = self.source().clone();
        let mut in_path = self.in_path().clone();
        let mut out_path = self.out_path().clone();
        //for input
        replace_json(&mut mutable, &mut in_path, self.in_array_iter(), input)?;
        //for output
        replace_json(&mut mutable, &mut out_path, self.out_array_iter(), output)?;
        Ok(mutable)
    }


    //only difference between extract & alter format search, extract only cares about input keywords, it doesn't have output keywords.
    pub fn create_extract(json: &Value) -> Result<AdvancedInput, VideoError> {
        let mut batch_in_path = VecDeque::new();
        let mut single_in_path = VecDeque::new();
        let (batch_in, batch_iter) = try!(search_json(json, &mut batch_in_path, BATCH_INPUT));
        let (single_in, single_iter) = try!(search_json(json, &mut single_in_path, SINGLE_INPUT));
        if batch_in {
            println!("json parsed as batch input.");
            Ok(AdvancedInput::new("batch".to_string(), batch_in_path.into(),
                                  batch_iter, VecDeque::new(), None, json.clone()))
        } else if single_in {
            println!("json parsed as single input.");
            Ok(AdvancedInput::new("single".to_string(), single_in_path.into(),
                                  single_iter, VecDeque::new(), None, json.clone()))
        } else {
            Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
        }
    }


    pub fn create_transform(json: &Value) -> Result<AdvancedInput, VideoError> {
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
            Ok(AdvancedInput::new("batch".to_string(), batch_in_path, batch_in_iter,
                                  batch_out_path, batch_out_iter, json.clone()))
        } else if batch_in || batch_out {
            Err(String::from("json parsing error:\nif batch selected both $BATCH_INPUT and $BATCH_OUTPUT must be defined.").into())
        } else if single_in && single_out {
            println!("json parsed as single input.");
            Ok(AdvancedInput::new("single".to_string(), single_in_path, single_in_iter,
                                  single_out_path, single_out_iter, json.clone()))
        } else if single_in || single_out {
            Err(String::from("json parsing error:\nif single selected both $SINGLE_INPUT and $SINGLE_OUTPUT must be defined.").into())
        } else {
            Err(String::from("json parsing error:\nadvanced_input did not contain any keywords!").into())
        }
    }


}