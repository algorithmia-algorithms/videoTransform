use std::time::{Duration, SystemTime};
use std::ops::*;
use std_semaphore::Semaphore;
use std::sync::{Arc, Mutex};
use common::video_error::VideoError;
use common::json_utils::AdvancedInput;
static DURATION:u64 = 3;

pub fn try_algorithm_default<T, J>(function: &(Fn(&T, Vec<usize>, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                   data: &T, batch: &Vec<usize>, semaphore: Arc<Semaphore>,
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
                let prev_time: SystemTime = *time.lock().unwrap();
                let time_diff: Duration = curr_time.duration_since(prev_time)?;
                if time_diff.as_secs() > DURATION {
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

pub fn try_algorithm_advanced<T, J>(function: &(Fn(&T,Vec<usize>, String, &AdvancedInput, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                    data: &T, batch: &Vec<usize>, algo: &str,
                                    json: &AdvancedInput, semaphore: Arc<Semaphore>,
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
                if time_diff.as_secs() > DURATION {
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