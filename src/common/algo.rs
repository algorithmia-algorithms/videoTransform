
use algorithmia::prelude::*;
use algorithmia::algo::*;
use common::video_error::VideoError;
use std::thread;
use common::file_mgmt::{from_regex, create_directory};
use common::threading::*;
use algorithmia::data::{FileData, HasDataPath};
use std::time::Duration;
use std::io::copy;
use std::path::*;
use std::fs::File;
use serde_json::Value;

static SMART_VIDEO_DOWNLOADER: &'static str = "algo://media/SmartVideoDownloader/0.2.0";
static MAX_ATTEMPTS_DATA: usize = 7;
static MAX_ATTEMPTS_ALGO: usize = 5;

//gets any remote file, http/https or data connector
pub fn get_file_parallel(url: &str, local_path: &Path, client: &Algorithmia,
                         error_poll: Terminator) -> Result<PathBuf, VideoError> {
    let mut attempts = 0;
    let output;
    loop {
        if error_poll.check_signal().is_some() {return Err(format!("already receieved an error.").into())}
            else {
                let result = get_file_from_algorithmia(url, local_path, client);
                if result.is_ok() {
                    output = result.unwrap();
                    break;
                } else if attempts > MAX_ATTEMPTS_DATA {
                    let err = result.err().unwrap();
                    return Err(format!("failed {} times to download file {} : \n{}", attempts, url, err).into())
                } else {
                    thread::sleep(Duration::from_millis((1000 * attempts) as u64));
                    println!("failed {} times to download file\n{}", attempts, url.to_string());
                    attempts += 1;
                }
            }
    }
    Ok(output)
}


pub fn upload_file_parallel(url_dir: &str, local_file: &Path, client: &Algorithmia,
                            error_poll: Terminator) -> Result<String, VideoError> {
    if local_file.exists() {
        let mut attempts = 0;
        loop {
            if error_poll.check_signal().is_some() { return Err(format!("already receieved an error.").into()) } else {
                let file: File = File::open(local_file).map_err(|err| { format!("failed to open file: {}\n{}", local_file.display(), err) })?;
                let response: Result<(), VideoError> = client.file(url_dir).put(file).map_err(|err| { format!("upload failure for:{}\n{}", url_dir, err.description()).into() });
                if response.is_ok() {
                    let _ = response.unwrap();
                    break;
                } else if attempts > MAX_ATTEMPTS_DATA {
                    let err = response.err().unwrap();
                    return Err(format!("failed {} times to upload file {} : \n{}", attempts, local_file.display(), err).into())
                } else {
                    thread::sleep(Duration::from_millis((1000 * attempts) as u64));
                    attempts += 1;
                }
            }
        }
        Ok(url_dir.to_string())
    } else {
        Err(format!("file path: {} doesn't exist!, upload error.", local_file.display()).into())
    }
}

pub fn get_file(url: &str, local_path: &Path, remote_scratch: &str, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    let tmp_url = url.clone();
    let remote_file = format!("{}/temp.mp4", remote_scratch);
    let prefix: &str = tmp_url.split("://").next().unwrap().clone();
    let mut attempts = 0;
    let output;
    loop {
        let result = if prefix == "http" || prefix == "https" {
            get_file_from_html(url, local_path, &remote_file, client)
        } else {
            get_file_from_algorithmia(url, local_path, client)
        };
        if result.is_ok() {
            output = result.unwrap();
            break;
        } else if attempts > MAX_ATTEMPTS_DATA {
            let err = result.err().unwrap();
            return Err(format!("failed {} times to download file {} : \n{}", attempts, url, err).into())
        } else {
            thread::sleep(Duration::from_millis((1000 * attempts) as u64));
            println!("failed {} times to download file\n{}", attempts, url.to_string());
            attempts += 1;
        }
    }
    Ok(output)
}

pub fn upload_file(url_dir: &str, local_file: &Path, client: &Algorithmia) -> Result<String, VideoError> {
    if local_file.exists() {
        let mut attempts = 0;
        loop {
            let file: File = File::open(local_file).map_err(|err| { format!("failed to open file: {}\n{}", local_file.display(), err) })?;
            let response: Result<(), VideoError> = client.file(url_dir).put(file).map_err(|err| { format!("upload failure for:{}\n{}", url_dir, err.description()).into() });
            if response.is_ok() {
                let _ = response.unwrap();
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

fn get_file_from_html(url: &str, local_path: &Path, remote_file: &str, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    let local_dir = local_path.parent().unwrap();
    create_directory(local_dir);
    let input = json!({
    "source": url,
    "output": remote_file
    });
    let _response = client.algo(SMART_VIDEO_DOWNLOADER).pipe(input).map_err(|err| format!("smart video downloader failed: {}\n{}", url, err))?;
    get_file_from_algorithmia(remote_file, local_path, client)

}

pub fn get_file_from_algorithmia(url: &str, local_path: &Path, client: &Algorithmia) -> Result<PathBuf, VideoError> {
    let local_dir = local_path.parent().unwrap();
    create_directory(local_dir);
    let file = client.file(&url);
    match file.exists() {
        Ok(true) => {
            let mut remote_file: FileData = file.get().map_err(|err| format!("couldn't download file from url: {} \n{}", &url, err))?;
            let mut local_file = File::create(local_path).map_err(|err| format!("couldn't create local file: {} \n{}", local_path.to_str().unwrap(), err))?;
            thread::sleep(Duration::from_secs(2));
            copy(&mut remote_file, &mut local_file).map_err(|err| format!("couldn't copy remote file to local: {} \n{}", local_path.to_str().unwrap(), err))?;
            Ok(PathBuf::from(local_path))
        }
        Ok(false) => {Err(format!("file not ready").into())}
        Err(error) => {
            Err(format!("recieved an error trying to download {}\n{}", url, error).into())
        }
    }
}

//exits early if the or if the output path is invalid.
pub fn early_exit(client: &Algorithmia, output_path: &str) -> Result<(), VideoError> {
    //try to upload a 0 size file to the output path, then delete it. if both succeed then the path is valid.
    let r: Result<_, VideoError> = client.file(output_path).put("").map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to upload.\n{}", output_path, err).into());
    let j: Result<_, VideoError> = client.file(output_path).delete().map_err(|err| format!("early exit: \n output path {} invalid, or invalid permissions, unable to delete.\n{}", output_path, err).into());
    r?;j?;
    Ok(())
}


pub fn batch_file_path(batch: &Vec<usize>, regex: &str, directory: &str) -> Result<Vec<String>, VideoError>
{
    let regexed = batch.iter().map(|iter| {
        from_regex(regex, iter.clone())
    }).collect::<Result<Vec<String>, VideoError>>()?;
    Ok(regexed.iter().map(|filename| {
        format!("{}/{}", directory, filename)
    }).collect::<Vec<String>>())
}

//retry 3 times, if it fails 3 times we exit hard.
pub fn batch_upload_file(local_files: &Vec<PathBuf>, remote_files: &Vec<String>,
                         client: &Algorithmia,
                         error_poll: Terminator) -> Result<(), VideoError>
{
    for (local_file, remote_file) in local_files.iter().zip(remote_files.iter()) {
        upload_file_parallel(&remote_file, &local_file, client, error_poll.clone())?;
    }
    Ok(())
}

pub fn batch_get_file(local_file_save_locations: &Vec<PathBuf>, remote_file_get_locations: &Vec<String>,
                      client: &Algorithmia, error_poll: Terminator) -> Result<Vec<PathBuf>, VideoError>
{
    let mut output: Vec<PathBuf> = Vec::new();
    for (local_file, remote_file) in local_file_save_locations.iter().zip(remote_file_get_locations.iter()) {
        output.push(get_file_parallel(&remote_file, &local_file, client, error_poll.clone())?);
    }
    Ok(output)
}

//fail fast if the exception contains '429'
pub fn try_algorithm(client: &Algorithmia, algorithm: &str, input: &Value,
                     error_poll: Terminator) -> Result<AlgoResponse, VideoError> {
    let mut attempts = 0;
    let mut final_result;
    loop {
        if error_poll.check_signal().is_some(){return Err(format!("already receieved an error.").into())}
        else {
            match client.algo(algorithm).timeout(500).pipe(input.clone()) {
                Ok(result) => {
                    final_result = result;
                    break;
                },
                Err(ref err) if attempts < MAX_ATTEMPTS_ALGO && !err.to_string().contains("algorithm hit max number of active calls per session") => {
                    println!("failed.");
                    thread::sleep(Duration::from_millis((1000 * attempts) as u64));
                    attempts += 1;
                },
                Err(ref err) => {
                    println!("failed hard.");
                    return Err(format!("algorithm {} failed: \n{}", &algorithm, err).into())
                }
            }
        }
    }
    Ok(final_result)
}
