use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings{
    pub top_windows_to_save: u32,
    pub minutes_to_save: Vec<u32>
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            top_windows_to_save: 5,
            minutes_to_save: vec![5, 20, 35, 50]
        }
    }

    pub fn save_to_file(&self) {
        let path = Path::new("settings.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        let json = serde_json::to_string(&self).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    pub fn load_from_file() -> Settings {
        let path = Path::new("settings.json");
        if !path.exists() {
            let settings = Settings::new();
            settings.save_to_file();
            return settings;
        }

        let mut file = File::open(path).unwrap();
        let mut json = String::new();
        file.read_to_string(&mut json).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}