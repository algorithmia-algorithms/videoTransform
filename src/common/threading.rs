use std::time::{Duration, SystemTime};
use std::ops::*;
use std_semaphore::Semaphore;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, Ordering};
use common::video_error::*;
use common::structs::advanced_input::AdvancedInput;
static DURATION: u64 = 5;

pub type Default<T, J> = Fn(&Threadable<T>, Vec<usize>) -> Result<Vec<J>, VideoError> + Sync;
pub type Advanced<T, J> = Fn(&Threadable<T>, Vec<usize>, String, &AdvancedInput) -> Result<Vec<J>, VideoError> + Sync;
pub type Lockstep<T> = Arc<Mutex<T>>;

#[derive(Clone)]
pub struct Terminator {
    signal: Lockstep<Option<VideoError>>
}


#[derive(Clone)]
pub struct Threadable<J> where J:Clone{
    slowdown_signal: Arc<AtomicBool>,
    semaphore: Arc<Semaphore>,
    termination_signal: Terminator,
    time: Lockstep<SystemTime>,
    readonly_data: Arc<J>
}

impl Terminator {
    pub fn create() -> Terminator {
        Terminator{signal: Arc::new(Mutex::new(None))}
    }
    pub fn check_signal(&self) -> MutexGuard<Option<VideoError>> {
        self.signal.lock().unwrap()
    }

    pub fn set_signal(&self, error: VideoError) -> () {
        if self.check_signal().is_none() {
            *self.signal.lock().unwrap() = Some(error)
        } else {
            println!("signal already set, ignoring the set request.")
        }
    }

    pub fn get_signal(self) -> Option<VideoError> {
        println!("about to own the signal");
        let owned_signal = Arc::try_unwrap(self.signal).unwrap();
        let termination_message = owned_signal.into_inner().unwrap();
        if termination_message.is_some() {
            println!("we have an error");
            Some(termination_message.unwrap())
        } else {
            println!("we detected no errors");
            None
        }
    }
}

impl<J> Threadable<J> where J: Clone {
    pub fn create(starting_th: isize, max_th: isize, data: J) -> Threadable<J> {
        let slowdown = AtomicBool::new(false);
        let slowdown_signal: Arc<AtomicBool> = Arc::new(slowdown);
        let semaphore: Arc<Semaphore> = prepare_semaphore(starting_th, max_th);
        let termination_signal: Terminator = Terminator::create();
        let time: Lockstep<SystemTime> = Arc::new(Mutex::new(SystemTime::now()));
        let data = Arc::new(data);
        Threadable{slowdown_signal: slowdown_signal, semaphore:semaphore,
            termination_signal: termination_signal, time: time,  readonly_data: data}
    }

    pub fn arc_semaphore(&self) -> Arc<Semaphore> {self.semaphore.clone()}

    pub fn emergency_release(&self) -> () {self.semaphore.release(); println!("released semaphore due to emergency.")}

    pub fn arc_data(&self) -> Arc<J> {self.readonly_data.clone()}
    pub fn arc_term_signal(&self) -> Terminator {self.termination_signal.clone()}

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
    fn check_term_signal(&self) -> MutexGuard<Option<VideoError>> {
        self.termination_signal.check_signal()
    }
    fn set_term_signal(&self, message: VideoError) -> () {
        self.termination_signal.set_signal(message)
    }
    pub fn extract_term_signal(self) -> Option<VideoError> {
        self.termination_signal.get_signal()
    }

}


pub fn try_algorithm_default<T, J>(function: &Default<T, J>, batch: &Vec<usize>, threadable: &Threadable<T>) -> Result<Vec<J>, ()> where T: Clone {
    let current_time = SystemTime::now();
    match threading_strategizer(&threadable, current_time) {
        Ok(()) => {}
        Err(err) => {threadable.set_term_signal(err)}
    };
    if let &Some(ref err) = threadable.check_term_signal().deref() {
        return Err(())
    }
    match function(&threadable, batch.clone()) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            threadable.emergency_release();
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                threadable.slow_down();
                try_algorithm_default(function, batch, threadable)
            } else if threadable.check_term_signal().is_none() {
                let terminate_err = VideoError::MsgError(format!("algorithm thread failed, ending early: \n{}", err).into());
                threadable.set_term_signal(terminate_err);
                Err(())
            } else {
                    println!("already received an error!");
                    Err(())
                }
        }
    }
}

pub fn try_algorithm_advanced<T, J>(function: &Advanced<T, J>, batch: &Vec<usize>, algo: &str,
                                    json: &AdvancedInput, threadable: &Threadable<T>) -> Result<Vec<J>, ()> where T: Clone {
    let current_time = SystemTime::now();
    match threading_strategizer(&threadable, current_time) {
        Ok(()) => {}
        Err(err) => {threadable.set_term_signal(err)}
    };
    if let &Some(ref err) = threadable.check_term_signal().deref() {
        println!("failing early, already got an error");
        return Err(())
    }

    match function(&threadable, batch.clone(), algo.to_string(), &json) {
        Ok(result) => {
            Ok(result)
        },
        Err(err) => {
            threadable.emergency_release();
            if err.to_string().contains("algorithm hit max number of active calls per session") {
                threadable.slow_down();
                try_algorithm_advanced(function,  batch, algo, json,threadable)
            } else if threadable.check_term_signal().is_none() {
                let terminate_err = VideoError::MsgError(format!("algorithm thread failed, ending early: \n{}", err).into());
                threadable.set_term_signal(terminate_err);
                Err(())
            } else {
                println!("already received an error!");
                Err(())
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

fn threading_strategizer<J>(threadable: &Threadable<J>, current: SystemTime) -> Result<(), VideoError> where J: Clone {
    let time_diff: Duration = current.duration_since(threadable.acquire_time().clone())?;
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
    Ok(())
}
