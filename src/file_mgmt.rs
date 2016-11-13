use algorithmia::{Algorithmia};
use std;
use std::path::*;
use std::fs::{File, ReadDir, read_dir, create_dir_all, remove_dir_all, metadata};
use std::io::{Read, Write};
use hyper::Client;
use hyper::header::Connection;
use regex::Regex;
use video_error::VideoError;
use rustc_serialize::json::{Json};

//gets any remote file, http/https or data connector
pub fn get_file(url: &str, local_path: &Path, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    println!("{:?}", local_path.display());
    let local_dir = local_path.parent().unwrap();
    create_directory(local_dir);
    let tmp_url = url.clone();
    let prefix: &str = tmp_url.split("://").next().unwrap().clone();
    if prefix == "http" || prefix == "https" {
        get_file_from_html(url.to_string(), local_path)
    }
    else {
        get_file_from_algorithmia(url.to_string(), local_path, client)
    }
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
    let mut remote_file = try!(client.file(&url).get().map_err(|err| format!("couldn't download file from url: {} \n{}", &url, err)));
    let mut local_file = try!(File::create(local_path).map_err(|err| format!("couldn't create local file: {} \n{}", local_path.to_str().unwrap(), err)));
    try!(std::io::copy(&mut remote_file, &mut local_file).map_err(|err| format ! ("couldn't copy remote file to local: {} \n{}", local_path.to_str().unwrap(), err)));
    Ok(PathBuf::from(local_path))
}

pub fn create_directory(directory: &Path) -> () {
    create_dir_all(directory);
    ()
}

pub fn get_filesize_mb(file: &Path) -> Result<u64, VideoError> {
    let meta = try!(metadata(file));
    Ok(meta.len() / 1000000u64)
}

pub fn clean_up(original_dir: &Path, process_dir: &Path) -> Result<(), VideoError> {
    match (remove_dir_all(original_dir), remove_dir_all(process_dir)) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), _) => Err(err.into()),
        (_, _) => Err("failed to clean all directories!".to_string().into())
    }
}

pub fn upload_file(url_dir: &str, local_file: &Path, client: &Algorithmia) -> Result<String, VideoError> {

    match local_file.exists() {
        true => {
            let mut file = try!(File::open(local_file).map_err(|err| {format!("failed to open file: {}\n{}",local_file.display(), err)}));
            let response =try!(client.file(url_dir).put(&mut file).map_err(|err| {format!("upload failure for:{}\n{}", url_dir, err)}));
            Ok(response.result)
        }
        false => {Err(format!("file path: {} doesn't exist!, upload error.", local_file.display()).into())}
    }
}

pub fn get_files_and_sort(frames_path: &Path) -> Vec<PathBuf> {
    let paths: ReadDir = read_dir(frames_path).unwrap();
    let mut files: Vec<PathBuf> = paths.filter_map(|entry| {
        entry.ok().map(|e| e.path())}).collect();
    files.sort();
    files
}

pub fn json_to_file(json: &Json, json_path: &Path) -> Result<PathBuf, VideoError> {
    let mut local_file: File = try!(File::create(json_path).map_err(|err| {format!("failed to create local json file {}\n{}", json_path.display(), err)}));
    let mut writer = std::io::BufWriter::new(local_file);
    try!(writer.write_all(json.to_string().as_bytes()));
    Ok(PathBuf::from(json_path))

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