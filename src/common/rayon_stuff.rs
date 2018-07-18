use std::time::{Duration, SystemTime};
use std::ops::*;
use std_semaphore::Semaphore;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use common::video_error::VideoError;
use common::structs::advanced_input::AdvancedInput;
static DURATION: u64 = 5;


pub fn try_algorithm_default<T, J>(function: &(Fn(&T, Vec<usize>, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                   data: &T, batch: &Vec<usize>, semaphore: Arc<Semaphore>,
                                    slowdown_signal:  Arc<AtomicBool>,
                                   catastrophic_error: Arc<Mutex<Option<String>>>, time: Arc<Mutex<SystemTime>>) -> Result<Vec<J>, VideoError> {
    let current_time = SystemTime::now();
    let slow = slowdown_signal.clone();
    threading_strategizer(time.clone(), current_time, slowdown_signal.clone(), semaphore.clone());
    if let Some(ref err) = *(catastrophic_error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    match function(&data, batch.clone(), semaphore.clone()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                (*slow).store(true, Ordering::Relaxed);
                try_algorithm_default(function, data, batch, semaphore, slowdown_signal, catastrophic_error, time)
            } else {
                let mut terminate = catastrophic_error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}

pub fn try_algorithm_advanced<T, J>(function: &(Fn(&T,Vec<usize>, String, &AdvancedInput, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync),
                                    data: &T, batch: &Vec<usize>, algo: &str,
                                    json: &AdvancedInput, semaphore: Arc<Semaphore>, slowdown_signal: Arc<AtomicBool>,
                                    catastrophic_error: Arc<Mutex<Option<String>>>, time: Arc<Mutex<SystemTime>>) -> Result<Vec<J>, VideoError> {
    let current_time = SystemTime::now();
    let slow = slowdown_signal.clone();
    threading_strategizer(time.clone(), current_time, slowdown_signal.clone(), semaphore.clone());
    if let Some(ref err) = *(catastrophic_error.lock().unwrap()) {
        return Err(err.to_string().into())
    }
    match function(&data, batch.clone(), algo.to_string(), &json, semaphore.clone()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                (*slow).store(true, Ordering::Relaxed);
                try_algorithm_advanced(function, data, batch, algo, json, semaphore, slowdown_signal, catastrophic_error, time)
            } else {
                let mut terminate = catastrophic_error.lock().unwrap();
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                *terminate = Some(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}

pub fn prepare_semaphore(starting_threads: isize, max_threads: isize) -> Arc<Semaphore> {
    let semaphore = Semaphore::new(max_threads);
    for _ in 0..(max_threads-starting_threads) {
        semaphore.acquire();
    }
    Arc::new(semaphore)
}

fn threading_strategizer(previous_time: Arc<Mutex<SystemTime>>, current: SystemTime, slowdown_signal: Arc<AtomicBool>, semaphore: Arc<Semaphore>) -> () {
    let time_diff: Duration = current.duration_since(*previous_time.lock().unwrap()).unwrap();
    // println!("time difference is... {}", time_diff.as_secs());
    if time_diff.as_secs() > DURATION {
        *previous_time.lock().unwrap() = current;
        // println!("time check...");
        if (*slowdown_signal).load(Ordering::Relaxed) == true {
            // println!("We're slowing down.");
            semaphore.acquire();
        } else {
            // println!("No need to slow down, lets speed up.");
            semaphore.release();
        }
        //maybe we don't want to reset this to
        (*slowdown_signal).store(false, Ordering::Relaxed);
    }
}
