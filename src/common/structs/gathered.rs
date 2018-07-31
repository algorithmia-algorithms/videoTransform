use std::path::*;

#[derive(Debug, Clone)]
pub struct Gathered {
    video_file: PathBuf
}

impl Gathered {
//    pub fn fps(&self) -> f64 {self.fps}
    pub fn video_file(&self) -> &PathBuf {&self.video_file}
    pub fn new(video_file: PathBuf) -> Gathered {
        Gathered {video_file: video_file}
    }
}