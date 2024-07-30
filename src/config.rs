use core::panic;
use std::{fs::File, io::Read};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub server: Server,
}

#[derive(Deserialize)]
pub struct Server {
    pub addr: String,
    pub port: u16,
    pub request_timeout: i64,
}

impl Config {
    pub fn load_config() -> Self {
        match File::open("config.yaml") {
            Ok(mut yaml_file) => {
                let mut buf = String::new();
                yaml_file.read_to_string(&mut buf).expect("Failed to read to string.");
                serde_yaml::from_str::<Config>(&buf).unwrap()
            }
            Err(_) => panic!("Failed to load config"),
        }
    }
}