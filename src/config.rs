use std::fs::{self};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub ui: UI,
    pub sidebar: Sidebar,
    pub lang_server: Vec<LanguageServer>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct UI {
    pub theme: String,
    pub theme_variant: ThemeVariant,
    pub show_minimap: bool,
    pub hl_line: bool,
    pub grid_bg: bool,
    pub word_wrap: bool,
    pub flat_corners: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Sidebar {
    pub show_hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ThemeVariant {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LanguageServer {
    pub file_extension: String,
    pub command: String,
    pub language: String,
}

pub fn default_lang_servers() -> Vec<LanguageServer> {
    vec![
        LanguageServer {
            file_extension: ".rs".into(),
            command: "rust-analyzer".into(),
            language: "rust".into(),
        },
        LanguageServer {
            file_extension: ".blp".into(),
            command: "blueprint-compiler lsp".into(),
            language: "blueprint".into(),
        },
    ]
}

impl Default for UI {
    fn default() -> Self {
        Self {
            theme: "Adwaita".to_string(),
            theme_variant: ThemeVariant::System,
            show_minimap: true,
            hl_line: true,
            grid_bg: false,
            word_wrap: false,
            flat_corners: false,
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self { show_hidden: false }
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
        )) + "/kyanite"
    }
}
