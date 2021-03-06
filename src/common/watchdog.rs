
use std::time::{Duration, SystemTime};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender, channel};
use common::threading::{Terminator, Lockstep};
use common::video_error::VideoError;
use std::thread;
use std::thread::JoinHandle;

static MAX_TIME: f64 = 3000f64;
static ADJUSTMENT_TIME: f64 = 60f64;

#[derive(Clone)]
pub struct WatchdogComms {
    watchdog_rx: Lockstep<Receiver<usize>>,
    watchdog_tx: Lockstep<Sender<usize>>,
    terminate_tx: Terminator,
    total_jobs: usize,
}

pub struct Watchdog {
    watchdog_comms: WatchdogComms,
    callback: JoinHandle<()>
}

impl Watchdog {
    pub fn create(term_obj: Terminator, total_jobs: usize) -> Watchdog {
        let wdc = WatchdogComms::create(term_obj, total_jobs);
        let wdcc = wdc.clone();
        println!("starting up watchdog thread.");
        let callback = thread::spawn(move || {
            wdcc.watchdog_thread_inner();
        });
        Watchdog{watchdog_comms: wdc, callback: callback}
    }

    pub fn get_comms(&self) -> WatchdogComms {
        self.watchdog_comms.clone()
    }

    pub fn terminate(self) -> () {
        println!("terminating watchdog thread");
        let _ = self.watchdog_comms.watchdog_tx.lock().unwrap().send(0);
        let _ = self.callback.join();
    }
}

impl WatchdogComms {
    fn create(term_obj: Terminator, total_jobs: usize) -> WatchdogComms {
        let (s, r) = channel();
        let locked_s = Arc::new(Mutex::new(s));
        let locked_r = Arc::new(Mutex::new(r));
        WatchdogComms { watchdog_rx:locked_r, watchdog_tx:locked_s, terminate_tx: term_obj, total_jobs: total_jobs}
    }

    pub fn send_success_signal(&self) -> () {
        let _ = self.watchdog_tx.lock().unwrap().send(1);
    }

    fn watchdog_thread_inner(&self) -> () {

        fn failure_mgmt(wd: &WatchdogComms, message: String) -> () {
            println!("failing with message: {}", message);
            let err = VideoError::MsgError(message);
            wd.terminate_tx.set_signal(err)
        }

        let mut finished_jobs: f64 = 0f64;
        let start_time = SystemTime::now();
        let mut signal = 0;
        loop {
            let mut cont = true;
            while cont {
//                println!("waiting...");
                match self.watchdog_rx.lock().unwrap().try_recv() {
                    Ok(val) => {signal = val; cont = false}
                    Err(t) => {thread::sleep(Duration::new(0, 500000000))}
                }
            }
            if signal == 1 {
                let current_time = SystemTime::now();
                finished_jobs = finished_jobs + 1f64;
                let current_time_delta = current_time.duration_since(start_time.clone())
                    .map_err(|d| { return failure_mgmt(&self, format!("failed to check time: {}", d))}).unwrap();
                println!("current time is: {}", current_time_delta.as_secs());

                if current_time_delta.as_secs() > 10 {
                    let time_estimate = time_estimate(current_time_delta, finished_jobs, self.total_jobs as f64);
                    println!("time estimate is: {} secs", time_estimate);
                    if time_estimate as f64 >= MAX_TIME {
                        let error_msg = format!("watchdog thread detected.\nMax algo run time: {}s\
                        \nAnticipated runtime: {}s\
                        \nTerminated early to avoid expense", MAX_TIME, time_estimate);
                        failure_mgmt(&self,error_msg);
                        println!("terminating watchdog_thread...");
                        return ()
                    }
                }
            } else {
                break
            }
        }
        println!("terminated watchdog loop");
    }

}

fn time_estimate(current_time_delta: Duration, finished_jobs: f64, total_jobs: f64) -> f64 {
    let delta_secs = current_time_delta.as_secs() as f64;
    let remaining_jobs = total_jobs - finished_jobs;
    let jobs_per_sec: f64 = finished_jobs / delta_secs;
    println!("jobs per second is {}", jobs_per_sec);
    println!("remaining jobs is {}", remaining_jobs);
    let remaining_time_estimate = remaining_jobs / jobs_per_sec;
    println!("remaining time estimate: {}", remaining_time_estimate);
    if delta_secs >= ADJUSTMENT_TIME {
        remaining_time_estimate + delta_secs
    } else {0f64}
}