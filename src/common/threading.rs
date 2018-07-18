use std::time::{Duration, SystemTime};
use std::ops::*;
use std_semaphore::Semaphore;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use common::video_error::VideoError;
use common::structs::advanced_input::AdvancedInput;
static DURATION: u64 = 5;

pub type Default<T, J> = Fn(&T, Vec<usize>, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync;
pub type Advanced<T, J> = Fn(&T,Vec<usize>, String, &AdvancedInput, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync;
pub type Lockstep<T> = Arc<Mutex<T>>;

#[derive(Clone)]
pub struct Threadable<J> where J:Clone{
    slowdown_signal: Arc<AtomicBool>,
    semaphore: Arc<Semaphore>,
    termination_signal: Lockstep<Option<String>>,
    time: Lockstep<SystemTime>,
    readonly_data: Arc<J>
}

impl<J> Threadable<J> where J: Clone {
    pub fn create(starting_th: isize, max_th: isize, data: J) -> Threadable<J> {
        let slowdown = AtomicBool::new(false);
        let slowdown_signal: Arc<AtomicBool> = Arc::new(slowdown);
        let semaphore: Arc<Semaphore> = prepare_semaphore(starting_th, max_th);
        let termination_signal: Lockstep<Option<String>> = Arc::new(Mutex::new(None));
        let time: Lockstep<SystemTime> = Arc::new(Mutex::new(SystemTime::now()));
        let data = Arc::new(data);
        Threadable{slowdown_signal: slowdown_signal, semaphore:semaphore,
            termination_signal: termination_signal, time: time,  readonly_data: data}
    }

    fn arc_semaphore(&self) -> Arc<Semaphore> {self.semaphore.clone()}
//    fn arc_time(&self) -> Lockstep<SystemTime> {self.time.clone()}
    fn arc_slow_signal(&self) -> Arc<AtomicBool> {self.slowdown_signal.clone()}
    fn arc_data(&self) -> Arc<J> {self.readonly_data.clone()}
    pub fn arc_term_signal(&self) -> Lockstep<Option<String>> {self.termination_signal.clone()}

    fn acquire_time(&self) -> MutexGuard<SystemTime> {
        self.time.lock().unwrap()
    }

    fn acquire_slow_signal(&self) -> &AtomicBool {self.slowdown_signal.deref()}

    fn slow_down(&self) -> () {
        (*self.slowdown_signal).store(true, Ordering::Relaxed);
    }

    fn set_time(&self, time: SystemTime) -> () {
        *self.time.lock().unwrap() = time;
    }

    fn check_term_signal(&self) -> MutexGuard<Option<String>> {
        self.termination_signal.lock().unwrap()
    }
    fn set_err(&self, message: String) -> () {
        let mut terminate = self.termination_signal.lock().unwrap();
        *terminate = Some(message);
    }

}


pub fn try_algorithm_default<T, J>(function: &Default<T, J>, batch: &Vec<usize>, threadable: Threadable<T>) -> Result<Vec<J>, VideoError> where T: Clone {
    let current_time = SystemTime::now();
    threading_strategizer(&threadable, current_time);
    if let Some(ref err) = threadable.check_term_signal().deref() {
        return Err(err.to_string().into())
    }
    match function(&threadable.arc_data(), batch.clone(), threadable.arc_semaphore()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                threadable.slow_down();
                try_algorithm_default(function, batch, threadable)
            } else {
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                threadable.set_err(terminate_msg.clone());
                Err(terminate_msg.into())
            }
        }
    }
}

pub fn try_algorithm_advanced<T, J>(function: &Advanced<T, J>, batch: &Vec<usize>, algo: &str,
                                    json: &AdvancedInput, threadable: Threadable<T>) -> Result<Vec<J>, VideoError> where T: Clone {
    let current_time = SystemTime::now();
    threading_strategizer(&threadable, current_time);
    if let Some(ref err) = threadable.check_term_signal().deref() {
        return Err(err.to_string().into())
    }
    match function(&threadable.arc_data(), batch.clone(), algo.to_string(), &json, threadable.arc_semaphore()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                threadable.slow_down();
                try_algorithm_advanced(function,  batch, algo, json,threadable)
            } else {
                let terminate_msg: String = format!("algorithm thread failed, ending early: \n{}", err);
                threadable.set_err(terminate_msg.clone());
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

fn threading_strategizer<J>(threadable: &Threadable<J>, current: SystemTime) -> () where J: Clone {
    let time_diff: Duration = current.duration_since(threadable.acquire_time().clone()).unwrap();
    // println!("time difference is... {}", time_diff.as_secs());
    if time_diff.as_secs() > DURATION {
        threadable.set_time(current);
        // println!("time check...");
        let sem = threadable.arc_semaphore();
        let slow = threadable.acquire_slow_signal();
        if slow.load(Ordering::Relaxed) == true {
            // println!("We're slowing down.");
            sem.acquire();
        } else {
            // println!("No need to slow down, lets speed up.");
            sem.release();
        }
        //maybe we don't want to reset this to
        slow.store(false, Ordering::Relaxed);
    }
}
