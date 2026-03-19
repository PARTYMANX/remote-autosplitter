use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::remote::AutosplitterSettingValue;

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub server_address: String,
    pub autosplitter_filepath: String,
    pub autosplitter_settings: HashMap<String, AutosplitterSettingValue>,
}

#[derive(Debug)]
pub enum ProfileError {
    #[allow(dead_code)]
    IOError(std::io::Error),
    TomlError,
}

impl Profile {
    pub fn save(&self, file_path: &str) -> Result<(), ProfileError> {
        let toml = match toml::to_string_pretty(self) {
            Ok(v) => v,
            Err(_) => return Err(ProfileError::TomlError),
        };

        let mut output_file = match File::create(Path::new(file_path)) {
            Ok(v) => v,
            Err(e) => return Err(ProfileError::IOError(e)),
        };

        match output_file.write_all(&toml.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => return Err(ProfileError::IOError(e)),
        }
    }

    pub fn load(file_path: &str) -> Result<Self, ProfileError> {
        let mut input_file = match File::open(Path::new(file_path)) {
            Ok(v) => v,
            Err(e) => return Err(ProfileError::IOError(e)),
        };

        let mut input_file_buffer = Vec::new();
        match input_file.read_to_end(&mut input_file_buffer) {
            Ok(_) => {}
            Err(e) => return Err(ProfileError::IOError(e)),
        }

        match toml::from_slice(&input_file_buffer) {
            Ok(v) => Ok(v),
            Err(_) => Err(ProfileError::TomlError),
        }
    }
}
