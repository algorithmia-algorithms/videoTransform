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

pub fn advanced_single(input: &extract::Extract, batch: Vec<usize>, algorithm: String, search: &utilities::SearchResult) -> Result< Vec<Json>, VideoError> {
    unimplemented!()
}

pub fn advanced_batch(input: &extract::Extract, batch: Vec<usize>, algorithm: String, search: &utilities::SearchResult) -> Result< Vec<Json>, VideoError> {
    unimplemented!()
}
