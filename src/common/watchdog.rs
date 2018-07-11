
use std::time::{Duration, SystemTime};
use std::ops::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use common::video_error::VideoError;


static MAX_TIME: f64 = 3000f64;
static ADJUSTMENT_TIME: f64 = 60f64;
pub fn watchdog_thread(completion_rx: Receiver<usize>, terminate_tx: Arc<Mutex<Option<String>>>,  total_jobs: usize) -> Result<(), VideoError> {
    let mut finished_jobs: f64 = 0f64;
    let start_time = SystemTime::now();
    let mut check_time = SystemTime::now();
    loop {
        let _ = completion_rx.recv().unwrap_or(return Ok(()));
        let current_time = SystemTime::now();
        finished_jobs = finished_jobs + 1f64;
        let current_time_delta = current_time.duration_since(start_time.clone())?;
        let check_time_delta = current_time.duration_since(check_time.clone())?;
        if (check_time_delta.as_secs() > 10) {
            let time_estimate = time_estimate(current_time_delta, finished_jobs, total_jobs as f64);
            if time_estimate as f64 >= MAX_TIME * 1.5 {
                let error_msg = format!("watchdog thread detected.\nMax algo run time: {}s\nAnticipated runtime: {}s\
                \nTerminated early to avoid expense", MAX_TIME, time_estimate);
                let mut terminate = terminate_tx.lock().unwrap();
                *terminate = Some(error_msg.clone());
                return Ok(())
            }
                else {
                    check_time = SystemTime::now()
                }
        }
    }
}

fn time_estimate(current_time_delta: Duration, finished_jobs: f64, total_jobs: f64) -> f64 {
    let delta_secs = current_time_delta.as_secs() as f64;
    if(delta_secs >= ADJUSTMENT_TIME){
        let remaining_jobs = total_jobs - finished_jobs;
        let jobs_per_sec: f64 = finished_jobs / delta_secs;
        println!("jobs per second is {}", jobs_per_sec);
        let remaining_time_estimate = remaining_jobs * jobs_per_sec;
        remaining_time_estimate + delta_secs
    } else {0f64}
}