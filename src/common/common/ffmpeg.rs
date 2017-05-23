use algorithmia::Algorithmia;
use algorithmia::algo::*;
use algorithmia::data::*;
use std::process::Command;
use common::video_error::VideoError;
use std::path::*;
use common::file_mgmt;
use std::f64;
use std::ops::*;

pub struct FFMpeg{
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

pub fn new(ffmpeg_remote: &str, ffmpeg_directory: &Path, client: &Algorithmia) -> Result<FFMpeg, VideoError> {
    let ffmpeg_file: PathBuf = PathBuf::from(format!("{}/{}", ffmpeg_directory.display(), "ffmpeg.tar.gz"));
    let checker_file: PathBuf = PathBuf::from(format!("{}/{}", ffmpeg_directory.display(), "/ffmpeg-static/ffmpeg"));
    if !checker_file.exists() {
        let tar_file = file_mgmt::get_file_from_algorithmia(ffmpeg_remote, &ffmpeg_file, client)?;
        println!("got file.");
        let unzip = try!(Command::new("tar").args(&["-C", ffmpeg_directory.to_str().unwrap(), "-xf", &tar_file.to_str().unwrap()]).output());
        println!("unzipped file.");
    }
    Ok(
        FFMpeg{ffprobe_path: PathBuf::from(format!("{}/{}", ffmpeg_directory.display(), "/ffmpeg-static/ffprobe")),
            ffmpeg_path: PathBuf::from(format!("{}/{}", ffmpeg_directory.display(), "/ffmpeg-static/ffmpeg"))}
    )
}

impl FFMpeg {

    pub fn ffmpeg(&self) -> &str {self.ffmpeg_path.as_path().to_str().unwrap()}

    pub fn ffprobe(&self) -> &str {self.ffprobe_path.as_path().to_str().unwrap()}

    pub fn get_video_duration(&self, video_path: &Path) -> Result<f64, VideoError> {
        let response = try!(Command::new(self.ffprobe())
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg("format=duration")
            .arg("-of")
            .arg("default=noprint_wrappers=1:nokey=1")
            .arg(video_path.to_str().unwrap())
            .output());
        if response.stderr.is_empty() {
            let mut result = try!(String::from_utf8(response.stdout));
            result.pop();
            Ok(try!(result.parse::<f64>()))
        }
            else {
                Err(format!("ffprobe error, could not get duration: \n {}", String::from_utf8_lossy(&response.stderr)).into())
            }
    }
    //determines a basic jpeg compression ratio between 2-19 based on how big the file is.
    pub fn get_compression_factor(&self, video_file: &Path) -> Result<usize, VideoError> {
        let file_size: f64 = try!(file_mgmt::get_filesize_mb(video_file)) as f64;
        let logged:usize = (file_size/2f64).ln().floor() as usize;
        println!("compression factor: {}", logged );
        if logged >= 31usize {
            Ok(31usize)
        }
            else {
                Ok(logged )
            }
    }
    //gets the frames per second of the input video, using nb_frames and duration from ffprobe
    pub fn get_video_fps(&self, video_file: &Path) -> Result<f64, VideoError> {
        println!("getting fps");
        let response = Command::new(self.ffprobe())
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("stream=r_frame_rate")
            .arg("-of")
            .arg("default=noprint_wrappers=1:nokey=1")
            .arg(video_file.to_str().unwrap())
            .output().unwrap();
        if response.stderr.is_empty() {
            let mut result: String = try!(String::from_utf8(response.stdout));
            result.pop();
            println!("{:?}", result);
            let splits: Vec<&str> = result.splitn(2, '/').collect();
            let numerator = try!(splits.iter().next().unwrap().parse::<f64>());
            let denominator = try!(splits.iter().next_back().unwrap().parse::<f64>());
            let fps: f64 = numerator / denominator;
            println!("{}", fps);
            Ok(fps)
        } else {
            Err(format!("ffprobe error, could not get duration: \n {}", String::from_utf8_lossy(&response.stderr)).into())
        }
    }

    //re-attaches the audio track to the concatenated video file.
    pub fn attach_streams(&self, input_video: &Path, output_video: &Path, original_vvideo: &Path) -> Result<PathBuf, VideoError> {
        let response = try!(Command::new(self.ffmpeg())
            .args(&["-loglevel",
                "error",
                "-i", input_video.to_str().unwrap(),
                "-i", original_vvideo.to_str().unwrap(),
                "-c", "copy",
                "-map", "1",
                "-map", "-1:v",
                "-map", "0:v",
                output_video.to_str().unwrap(), "-y"]).output());
        if response.stderr.is_empty() {
            Ok(PathBuf::from(output_video))
        } else {
            Err(format!("ffmpeg error, could not re-attach streams: \n {}", String::from_utf8_lossy(&response.stderr)).into())
        }
    }

    pub fn cat_video(&self, output_file: &Path, directory: &Path, regex: &str, fps: f64, crf: Option<u64>) -> Result<PathBuf, VideoError> {
        let complete_regex = format!("{}/{}", directory.display(), regex);
        let response = if crf.is_some() {
            Command::new(self.ffmpeg())
                .args(&["-loglevel", "error",
                    "-framerate", &fps.to_string(),
                    "-i", &complete_regex,
                    "-c:v", "libx264",
                    "-pix_fmt", "yuv420p",
                    "-preset", "veryfast",
                    "-crf", &crf.unwrap().to_string(),
                    output_file.to_str().unwrap(), "-y"]).output()?
        } else {
            Command::new(self.ffmpeg())
                .args(&["-loglevel", "error",
                    "-framerate", &fps.to_string(),
                    "-i", &complete_regex,
                    "-c:v", "libx264",
                    "-pix_fmt", "yuv420p",
                    output_file.to_str().unwrap(), "-y"]).output()?
        };

        if response.stderr.is_empty() {
            Ok(PathBuf::from(output_file))
        } else {
            Err(format!("ffmpeg error, could not concat frames: \n {}", String::from_utf8_lossy(&response.stderr)).into())
        }
    }
    //splits a video into frames at a given fps using ffmpeg, if no quality we use jpeg image compression based on the input video filesize.
    pub fn split_video(&self, video_path: &Path, frames_path: &Path, regex: &str, fps: f64, compression_factor: &Option<u64>) -> Result<Vec<PathBuf>, VideoError> {
        let response = if compression_factor.is_some() {
            try!(Command::new(self.ffmpeg())
                .args(&["-loglevel", "error",
                    "-i", video_path.to_str().unwrap(),
                        "-q:v", &compression_factor.clone().unwrap().to_string(),
                    "-vf",
                    &format!("fps={}", fps),
                    regex, "-y"]).current_dir(frames_path).output())
        }
        else {
            try!(Command::new(self.ffmpeg())
                .args(&["-loglevel", "error",
                    "-i", video_path.to_str().unwrap(),
                    "-vf",
                    &format!("fps={}", fps),
                    regex, "-y"]).current_dir(frames_path).output())
        };

        if response.stderr.is_empty() {
            let frames: Vec<PathBuf> = file_mgmt::get_files_and_sort(frames_path);
            Ok(frames)
        } else {
            Err(format!("ffmpeg error, could not split video into frames: \n {}", String::from_utf8_lossy(&response.stderr)).into())
        }
    }
}