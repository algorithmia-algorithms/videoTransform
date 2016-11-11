use algorithmia;
use std;
use std::fmt::Display;
use std::fmt;
use rayon;
wrapped_enum!{
    #[derive(Debug)]
    /// Document your pub enums
    pub enum VideoError {
        /// Variants too
        IOError(std::io::Error),
        /// algorithmia error
        AlgorithmError(algorithmia::error::Error),
        ///Message Error
        MsgError(String),
        /// conversion error
        Utf8Error(std::string::FromUtf8Error),
        /// parse float error
        FloatError(std::num::ParseFloatError),
        /// parse int error
        IntError(std::num::ParseIntError),
        /// rayon error
        RayonError(rayon::InitError),
    }
}

impl Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            VideoError::IOError(ref err) => Display::fmt(err, f),
            VideoError::AlgorithmError(ref err) => Display::fmt(err, f),
            VideoError::MsgError(ref err) => Display::fmt(err, f),
            VideoError::Utf8Error(ref err) => Display::fmt(err, f),
            VideoError::FloatError(ref err) => Display::fmt(err, f),
            VideoError::RayonError(ref err) => Display::fmt(err, f),
            VideoError::IntError(ref err) => Display::fmt(err, f)
        }
    }
}