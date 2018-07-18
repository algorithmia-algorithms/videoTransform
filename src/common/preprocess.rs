use algorithmia::Algorithmia;
use std::path::*;
use common::structs::ffmpeg::FFMpeg;
use uuid::Uuid;
use common::file_mgmt::clean_up;
use common::video_error::VideoError;

pub enum ExecutionStyle {
    Algo,
    ProdLocal,
    TestLocal
}


pub struct PreDefines{
    pub client: Algorithmia,
    pub scattered_working_directory: PathBuf,
    pub processed_working_directory: PathBuf,
    pub video_working_directory: PathBuf,
    pub data_api_work_directory: String,
    pub local_input_file: PathBuf,
    pub local_output_file: PathBuf,
    pub ffmpeg: FFMpeg,
    pub scatter_regex: String,
    pub process_regex: String,
    pub batch_size: usize,
    pub starting_threads: isize,
    pub max_threads: isize
}

impl PreDefines {
    pub fn create(format: ExecutionStyle,
                  batch_size: usize,
                  starting_threads: usize,
                  max_threads: usize,
                  output_file: &str,
                  input_file: &str,
                  has_image_compression: bool
    ) -> Result<PreDefines, VideoError> {
        let prod_key = "simA8y8WJtWGW+4h1hB0sLKnvb11";
        let test_key = "simA8y8WJtWGW+4h1hB0sLKnvb11";
        let test_api = "https://api.test.algorithmia.com";
        let session = String::from("data://.session");
        let not_session = String::from("data://.my/ProcessVideo");

        let (client, data_work_dir) = match format {
            ExecutionStyle::Algo => { (Algorithmia::default(), session) }
            ExecutionStyle::ProdLocal => { (Algorithmia::client(prod_key), not_session) }
            ExecutionStyle::TestLocal => { (Algorithmia::client_with_url(test_api, test_key), not_session) }
        };
        let ffmpeg_remote_url = "data://media/bin/ffmpeg-static.tar.gz";
        let ffmpeg_working_directory = PathBuf::from("/tmp/ffmpeg");
        let scattered_working_directory = PathBuf::from("/tmp/scattered_frames");
        let processed_working_directory = PathBuf::from("/tmp/processed_frames");
        let video_working_directory = PathBuf::from("/tmp/video");
        let local_output_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), output_file.split("/").last().unwrap().clone()));
        let local_input_file: PathBuf = PathBuf::from(format!("{}/{}", video_working_directory.display(), input_file.split("/").last().unwrap().clone()));
        let input_uuid = Uuid::new_v4();
        let output_uuid = Uuid::new_v4();
        let scatter_regex = if has_image_compression { format!("{}-%07d.jpg", input_uuid) } else { format!("{}-%07d.png", input_uuid) };
        let process_regex = if has_image_compression { format!("{}-%07d.jpg", output_uuid) } else { format!("{}-%07d.png", output_uuid) };
        clean_up(Some(&scattered_working_directory), Some(&processed_working_directory), &video_working_directory);
        let ffmpeg: FFMpeg = FFMpeg::create(ffmpeg_remote_url, &ffmpeg_working_directory, &client)?;
        Ok(PreDefines {
            client: client,
            scattered_working_directory: scattered_working_directory,
            processed_working_directory: processed_working_directory,
            data_api_work_directory: data_work_dir,
            video_working_directory: video_working_directory,
            local_input_file: local_input_file,
            local_output_file: local_output_file,
            ffmpeg: ffmpeg,
            scatter_regex: scatter_regex,
            process_regex: process_regex,
            batch_size: batch_size,
            starting_threads: starting_threads as isize,
            max_threads: max_threads as isize
        })
    }
}