use std::fs::{self, DirEntry};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub ui: UI,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UI {
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UI {
                theme: "Adwaita".to_string(),
            },
        }
    }
}

impl Config {
    pub fn get_or_default() -> Config {
        // read/parse
        if let Ok(file) = fs::read_to_string(Config::get_folder() + "/config.toml") {
            toml::from_str::<Config>(&file).unwrap()
        } else {
            Config::default()
        }
    }

    pub fn write(&self) {
        let file = toml::to_string_pretty(self).unwrap();
        match fs::create_dir(Config::get_folder()) {
            Ok(_) => (), // ok
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                } else {
                    panic!()
                }
            }
        }; // ensure folder exists
        fs::write(Config::get_folder() + "/config.toml", file).unwrap();
    }

    // folder (if you don't use a unix-like os sucks to be you)
    fn get_folder() -> String {
        std::env::var("XDG_CONFIG_HOME").unwrap_or(format!(
            "{}/.config",
            std::env::home_dir().unwrap().to_str().unwrap()
        )) + "/spikedit"
    }
}
