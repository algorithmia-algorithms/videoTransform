use algorithmia::{Algorithmia};
use algorithmia::data::FileData;
use std::prelude::*;
use std;
use std::path::*;
use std::time;
use std::fs::{File, ReadDir, read_dir, create_dir_all, remove_dir_all, metadata};
use std::io::{Read, Write};
use hyper::Client;
use serde_json::value::*;
use serde_json::to_string;
use hyper::header::Connection;
use regex::Regex;
use common::video_error::VideoError;
use std::time::Duration;
use std::thread;
use std::error::Error as StdError;
static MAX_ATTEMPTS_DATA: usize = 5;

//gets any remote file, http/https or data connector
pub fn get_file(url: &str, local_path: &Path, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    println!("{:?}", local_path.display());
    println!("{}", url.to_string());
    let local_dir = local_path.parent().unwrap();
    create_directory(local_dir);
    let tmp_url = url.clone();
    let prefix: &str = tmp_url.split("://").next().unwrap().clone();
    let mut attempts = 0;
    let mut output;
    loop {
        let result = if prefix == "http" || prefix == "https" {
            get_file_from_html(url.to_string(), local_path)
        } else {
            get_file_from_algorithmia(url.to_string(), local_path, client)
        };
        if result.is_ok() {
            output = result.unwrap();
            break;
        }
            else if attempts > MAX_ATTEMPTS_DATA {
                let err = result.err().unwrap();
                return Err(format!("failed {} times to download file {} : \n{}", attempts, url, err).into())
            }
                else {
                    thread::sleep(Duration::from_millis((1000*attempts) as u64));
                    println!("failed {} times to download file\n{}", attempts, url.to_string());
                    attempts += 1;
                }
    }
    Ok(output)
}

fn get_file_from_html(url: String, local_path: &Path) -> Result<PathBuf, VideoError> {
    let client = Client::new();
    let mut response = try!(client.get(&url).header(Connection::close()).send().map_err(|err| format!("couldn't download file from url: {} \n{}", url, err)));
    let local_file = try!(File::create(local_path).map_err(|err| format!("couldn't create local file: {:?} \n{}", local_path.to_str().unwrap(), err)));
    let mut body = String::new();
    response.read_to_string(&mut body).unwrap();
    let mut writer = std::io::BufWriter::new(&local_file);
    writer.write_all(body.as_bytes());
    Ok(PathBuf::from(local_path))
}

fn get_file_from_algorithmia(url: String, local_path: &Path, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    println!("getting from algorithmia");
    let file = client.file(&url);
    let mut remote_file: FileData =file.get().map_err(|err| format!("couldn't download file from url: {} \n{}", &url, err))?;
    let mut local_file = File::create(local_path).map_err(|err| format!("couldn't create local file: {} \n{}", local_path.to_str().unwrap(), err))?;
    thread::sleep(Duration::from_secs(2));
    std::io::copy(&mut remote_file, &mut local_file).map_err(|err| format ! ("couldn't copy remote file to local: {} \n{}", local_path.to_str().unwrap(), err))?;
    Ok(PathBuf::from(local_path))
}

pub fn upload_file(url_dir: &str, local_file: &Path, client: &Algorithmia) -> Result<String, VideoError> {
    if local_file.exists() {
        let mut attempts = 0;
        let mut output;
        loop {
            let file: File = File::open(local_file).map_err(|err| { format!("failed to open file: {}\n{}", local_file.display(), err)})?;
            let response: Result< (), VideoError> = client.file(url_dir).put(file).map_err(|err| { format!("upload failure for:{}\n{}", url_dir, err.description()).into()});
            if response.is_ok() {
                output = response.unwrap();
                break;
            } else if attempts > MAX_ATTEMPTS_DATA {
                let err = response.err().unwrap();
                return Err(format!("failed {} times to upload file {} : \n{}", attempts, local_file.display(), err).into())
            } else {
                thread::sleep(Duration::from_millis((1000 * attempts) as u64));
                attempts += 1;
            }
        }
        Ok(url_dir.to_string())
    } else {
        Err(format!("file path: {} doesn't exist!, upload error.", local_file.display()).into())
    }
}

pub fn create_directory(directory: &Path) -> () {
    create_dir_all(directory);
}

pub fn get_filesize_mb(file: &Path) -> Result<u64, VideoError> {
    let meta = try!(metadata(file));
    Ok(meta.len() / 1000000u64)
}

pub fn clean_up(original_dir: Option<&Path>, process_dir: Option<&Path>, video_dir: &Path) -> () {
    original_dir.map(|dir| { remove_dir_all(dir)});
    process_dir.map(|dir| { remove_dir_all(dir)});
    remove_dir_all(video_dir);
}


pub fn get_files_and_sort(frames_path: &Path) -> Vec<PathBuf> {
    let paths: ReadDir = read_dir(frames_path).unwrap();
    let mut files: Vec<PathBuf> = paths.filter_map(|entry| {
        entry.ok().map(|e| e.path())}).collect();
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
