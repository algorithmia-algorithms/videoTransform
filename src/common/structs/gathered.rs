use std::path::*;

pub struct Gathered {
    fps: f64,
    video_file: PathBuf
}

impl Gathered {
    pub fn fps(&self) -> f64 {self.fps}
    pub fn video_file(&self) -> &PathBuf {&self.video_file}
    pub fn new(video_file: PathBuf, fps: f64) -> Gathered {
        Gathered {video_file: video_file, fps: fps}
    }
}