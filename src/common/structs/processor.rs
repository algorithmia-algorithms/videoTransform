
use algorithmia::Algorithmia;
use common::video_error::VideoError;
use common::json_utils::AdvancedInput;
use std_semaphore::Semaphore;

struct ProcessorDefault<T, J> {
    algo: Fn(&T, Vec<usize>, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync,
    payload: T,
    batch: Vec<usize>,
    semaphore: Arc<Semaphore>,
}

struct ProcessorAdavnced<T, J> {
    algo: Fn(&T, Vec<usize>, String, &AdvancedInput, Arc<Semaphore>) -> Result<Vec<J>, VideoError> + Sync,
    algo_name: String,
    payload: T,
    batch: Vec<usize>,
    json: AdvancedInput,
    semaphore: Arc<Semaphore>,
}


trait Executable<J> {
    fn execute(&self) -> Result<Vec<J>, VideoError> + Sync;
}