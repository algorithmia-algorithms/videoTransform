use algorithmia;
use std;
use rayon;
use serde_json;
use std::time;
quick_error!{
    #[derive(Debug)]
    /// Document your pub enums
    pub enum VideoError {
        /// Variants too
        IOError(err: std::io::Error) {
            from()
            cause(err)
        }
        /// algorithmia error
        AlgorithmError(err: algorithmia::error::Error) {
            from()
            cause(err)
        }
        ///Message Error
        MsgError(msg: String) {
            from()
            from(s: &'static str) -> (s.to_string())
            display("{}", msg)
        }
        ///Conversion error
        Utf8Error(err: std::string::FromUtf8Error) {
            from()
            cause(err)
        }
        ///Parse float error
        FloatError(err: std::num::ParseFloatError) {
            from()
            cause(err)
        }
        ///Parse int error
        IntError(err: std::num::ParseIntError) {
            from()
            cause(err)
        }
        ///Rayon error
        RayonError(err: rayon::InitError) {
            from()
            cause(err)
        }
        SerdeError(err: serde_json::Error) {
            from()
            cause(err)
        }
        SystemTimeError(err: time::SystemTimeError) {
            from()
            cause(err)
        }
    }
}

