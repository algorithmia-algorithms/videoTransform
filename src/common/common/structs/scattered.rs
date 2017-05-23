use std::path::*;
pub struct Scattered {
    fps: f64,
    original_video: PathBuf,
    num_frames: usize,
    frames_dir: PathBuf,
    frame_regex: String,
}

impl Scattered {
    pub fn fps(&self) -> f64 {self.fps}
    pub fn frames_dir(&self) -> &Path {self.frames_dir.as_ref()}
    pub fn regex(&self) -> &str {&self.frame_regex}
    pub fn num_frames(&self) -> usize {self.num_frames}
    pub fn original_video(&self) ->&Path {&self.original_video}
    pub fn new(frames_dir: PathBuf, num_frames: usize, original_video: PathBuf, fps: f64, regex: String) -> Scattered{
        Scattered {frames_dir: frames_dir, original_video: original_video, fps: fps, frame_regex: regex, num_frames: num_frames}
    }
    pub fn clone(&self) -> Scattered {self.clone()}
}