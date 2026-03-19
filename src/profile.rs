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

impl Profile {
    pub fn save(&self, file_path: &str) {
        let toml = toml::to_string_pretty(self).unwrap();

        let mut output_file = match File::create(Path::new(file_path)) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to open {}: {}", file_path, e);
                //return PatcherResult::Failure;
                panic!()
            }
        };

        match output_file.write_all(&toml.as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to write {}: {}", file_path, e);
            }
        };
    }

    pub fn load(file_path: &str) -> Self {
        let mut input_file = match File::open(Path::new(file_path)) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to open {}: {}", file_path, e);
                //return PatcherResult::Failure;
                panic!()
            }
        };

        let mut input_file_buffer = Vec::new();
        match input_file.read_to_end(&mut input_file_buffer) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to read {}: {}", file_path, e);
            }
        }

        toml::from_slice(&input_file_buffer).unwrap()
    }
}
