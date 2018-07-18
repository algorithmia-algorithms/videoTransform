use algorithmia::Algorithmia;
use std::path::*;

#[derive(Clone)]
pub struct Extract {
    client: Algorithmia,
    input_regex: String,
    local_input_directory: PathBuf,
    remote_working_directory: String,
}

impl Extract {
    pub fn new(client: Algorithmia,
               input_regex: &str,
               local_input_directory: &Path,
               remote_working_directory: &str) -> Extract {
        Extract {
            client: client,
            input_regex: String::from(input_regex),
            local_input_directory: PathBuf::from(local_input_directory),
            remote_working_directory: String::from(remote_working_directory),
        }
    }

    pub fn client(&self) -> &Algorithmia {&self.client}
    pub fn input_regex(&self) -> &str {self.input_regex.as_ref()}
    pub fn local_input(&self) -> &Path {self.local_input_directory.as_path()}
    pub fn remote_working(&self) -> &str {self.remote_working_directory.as_ref()}
}