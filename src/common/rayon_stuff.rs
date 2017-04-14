use rayon::prelude::*;
use rayon;
use std::{thread};
use std::time::{Duration, SystemTime};
use std::ops::*;
use std::io::{self, Write};
use std_semaphore;
use serde_json::Value;
use std::sync::{Arc, Mutex, RwLock};
use common::video_error::VideoError;
use common::json_utils::SearchResult;
static DURATION:u64 = 5;
use std::thread::sleep;

pub fn try_algorithm_default<T, J>(function: &(Fn(&T, Vec<usize>, Arc<std_semaphore::Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                   data: &T, batch: &Vec<usize>, semaphore: Arc<std_semaphore::Semaphore>,
                                   error: Arc<Mutex<Option<String>>>, time: Arc<Mutex<SystemTime>>) -> Result<Vec<J>, VideoError> {
    if let Some(ref err) = *(error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    match function(&data, batch.clone(), semaphore.clone()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                let curr_time: SystemTime = SystemTime::now();
                let prev_time: SystemTime = {*time.lock().unwrap()};
                let time_diff: Duration = curr_time.duration_since(prev_time)?;
                if time_diff.as_secs() > 5 {
                    println!("slowing down...");
                    *time.lock().unwrap() = SystemTime::now();
                    try_algorithm_default(function, data, batch, semaphore, error, time)
                } else {
                    println!("not slowing down...");
                    semaphore.release();
                    try_algorithm_default(function, data, batch, semaphore, error, time)
                }
            } else {
                let mut terminate = error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}

pub fn try_algorithm_advanced<T, J>(function: &(Fn(&T,Vec<usize>, String, &SearchResult, Arc<std_semaphore::Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                    data: &T, batch: &Vec<usize>, algo: &str,
                                    json: &SearchResult, semaphore: Arc<std_semaphore::Semaphore>,
                                    error: Arc<Mutex<Option<String>>>, time: Arc<Mutex<SystemTime>>) -> Result<Vec<J>, VideoError> {
    if let Some(ref err) = *(error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    match function(&data, batch.clone(), algo.to_string(), &json, semaphore.clone()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                let curr_time: SystemTime = SystemTime::now();
                let prev_time: SystemTime = *time.lock().unwrap();
                let time_diff: Duration = curr_time.duration_since(prev_time)?;
                if time_diff.as_secs() > 5 {
                    println!("slowing down...");
                    *time.lock().unwrap() = curr_time;
                    try_algorithm_advanced(function, data, batch, algo, json, semaphore, error, time)
                }
                else {
                    println!("not slowing down...");
                    semaphore.release();
                    try_algorithm_advanced(function, data, batch, algo, json, semaphore, error, time)
                }
            } else {
                let mut terminate = error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}