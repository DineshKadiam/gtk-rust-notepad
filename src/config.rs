use std::fs::{self};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub ui: UI,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct UI {
    pub theme: String,
    pub show_minimap: bool,
    pub hl_line: bool,
    pub grid_bg: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { ui: UI::default() }
    }
}

impl Default for UI {
    fn default() -> Self {
        Self {
            theme: "Adwaita".to_string(),
            show_minimap: true,
            hl_line: true,
            grid_bg: false,
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
