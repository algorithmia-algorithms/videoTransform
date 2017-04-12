use rayon::prelude::*;
use rayon;
use std::{thread, time};
use std::ops::*;
use std::io::{self, Write};
use std_semaphore;
use serde_json::Value;
use std::sync::{Arc, Mutex, RwLock};
use common::video_error::VideoError;
use common::json_utils::SearchResult;
static DURATION:u64 = 5;
use std::thread::sleep;

pub fn try_algorithm_default<T, J>(function: &(Fn(&T, Vec<usize>) -> Result<Vec<J>, VideoError> + Sync),
                                data: &T, batch: &Vec<usize>, locked_threads: Arc<RwLock<isize>>, starting_threads: isize,
                                semaphore: Arc<RwLock<std_semaphore::Semaphore>>, error: Arc<Mutex<Option<String>>>) -> Result<Vec<J>, VideoError> {
    if let Some(ref err) = *(error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    (*semaphore).read().unwrap().acquire();
    match function(&data, batch.clone()) {
        Ok(result) => Ok(result),
        Err(err) => {
            if err.to_string().contains("429") {
                println!("caught exception 429, slowing down...");
                let cur: isize = (*locked_threads).read().unwrap().clone();
                *(*locked_threads).write().unwrap() = if cur > 1 { cur - 1 } else { 1 };
                (*semaphore).write().unwrap().update(starting_threads - *(*locked_threads).read().unwrap());
                (*semaphore).read().unwrap().release();
                sleep(time::Duration::from_secs(DURATION));
                try_algorithm_default(function, data, batch, locked_threads, starting_threads, semaphore, error)
            } else {
                let mut terminate = error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}

pub fn try_algorithm_advanced<T, J>(function: &(Fn(&T,Vec<usize>, String, &SearchResult) -> Result<Vec<J>, VideoError> + Sync),
                                 data: &T, batch: &Vec<usize>, algo: &str,
                                 json: &SearchResult, locked_threads: Arc<RwLock<isize>>, starting_threads: isize,
                                 semaphore: Arc<RwLock<std_semaphore::Semaphore>>,
                                 error: Arc<Mutex<Option<String>>>) -> Result<Vec<J>, VideoError> {
    if let Some(ref err) = *(error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    io::stdout().write(b"waiting for semaphore.\n")?;
    (*semaphore).read().unwrap().acquire();
    io::stdout().write(b"acquired semaphore\n")?;
    match function(&data, batch.clone(), algo.to_string(), &json) {
        Ok(result) => {
            (*semaphore).read().unwrap().release();
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("429") {
                let cur: isize = (*locked_threads).read().unwrap().clone();
                *(*locked_threads).write().unwrap() = if cur > 1 { cur - 1 } else { 1 };
                (*semaphore).write().unwrap().update(starting_threads - *(*locked_threads).read().unwrap());
                (*semaphore).read().unwrap().release();
                thread::sleep(time::Duration::from_secs(DURATION));
                try_algorithm_advanced(function, data, batch, algo, json, locked_threads, starting_threads, semaphore, error)
            } else {
                let mut terminate = error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}