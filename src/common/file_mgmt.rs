use std::path::*;
use std::fs::{ReadDir, read_dir, create_dir_all, remove_dir_all, metadata};
use regex::Regex;
use common::video_error::VideoError;

pub fn create_directory(directory: &Path) -> () {
    let _ = create_dir_all(directory);
}

pub fn get_filesize_mb(file: &Path) -> Result<u64, VideoError> {
    let meta = try!(metadata(file));
    Ok(meta.len() / 1000000u64)
}

pub fn clean_up(original_dir: Option<&Path>, process_dir: Option<&Path>, video_dir: &Path) -> () {
    original_dir.map(|dir| { remove_dir_all(dir) });
    process_dir.map(|dir| { remove_dir_all(dir) });
    let _ = remove_dir_all(video_dir);
}


pub fn get_files_and_sort(frames_path: &Path) -> Vec<PathBuf> {
    let paths: ReadDir = read_dir(frames_path).unwrap();
    let mut files: Vec<PathBuf> = paths.filter_map(|entry| {
        entry.ok().map(|e| e.path())
    }).collect();
    files.sort();
    files
}

//used with Process to create file names from a regex filename containing a %07d & iteration number.
pub fn from_regex(regex: &str, iter: usize) -> Result<String, VideoError> {
    lazy_static! {
        static ref FINDER: Regex = Regex::new(r"%([0-9][0-9])d").unwrap();
    }
    let cap = FINDER.captures_iter(regex).next().unwrap();
    let num_digits = try!(cap.at(1).unwrap().parse::<usize>());
    Ok(regex.replace(&format!("%0{}d", num_digits), &format!("{:0width$}", iter, width = num_digits)))
}




//#[test]
//fn regex_test () {
//    let regex = "frame-%07d.png";
//    let iter: usize = 1343;
//    let file_name: String = from_regex(regex, iter));
//    println!("filename: {}", &file_name);
//    assert!("frame-0001343.png".eq(&file_name));
//}
