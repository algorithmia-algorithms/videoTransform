use algorithmia::Algorithmia;
use std::path::*;

#[derive(Clone)]
pub struct Alter {
    client: Algorithmia,
    output_regex: String,
    input_regex: String,
    local_output_directory: PathBuf,
    local_input_directory: PathBuf,
    remote_working_directory: String,
}

impl Alter {
    pub fn new(client: Algorithmia,
               input_regex: &str,
               output_regex: &str,
               local_out_directory: &Path,
               local_input_directory: &Path,
               remote_working_directory: &str) -> Alter {
        Alter {
            client: client,
            output_regex: String::from(output_regex),
            input_regex: String::from(input_regex),
            local_input_directory: PathBuf::from(local_input_directory),
            local_output_directory: PathBuf::from(local_out_directory),
            remote_working_directory: String::from(remote_working_directory),
        }
    }

    pub fn client(&self) -> &Algorithmia {&self.client}
    pub fn input_regex(&self) -> &str {self.input_regex.as_ref()}
    pub fn output_regex(&self) -> &str {self.output_regex.as_ref()}
    pub fn local_input(&self) -> &Path {self.local_input_directory.as_path()}
    pub fn local_output(&self) -> &Path {self.local_output_directory.as_ref()}
    pub fn remote_working(&self) -> &str {self.remote_working_directory.as_ref()}
}

pub struct Altered {
    fps: f64,
    frames_dir: PathBuf,
    frames: Vec<PathBuf>,
    frame_regex: String,
}

impl Altered {
    pub fn fps(&self) -> f64 {self.fps}
//    pub fn frames(&self) -> &Vec<PathBuf> {self.frames.as_ref()}
    pub fn frames_dir(&self) -> &Path {self.frames_dir.as_ref()}
    pub fn regex(&self) -> &str {&self.frame_regex}
    pub fn new(frames_dir: PathBuf, frames: Vec<PathBuf>, fps: f64, frame_regex: String) -> Altered{
        Altered { frames_dir, fps, frame_regex, frames }
    }
}