use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub file_path: String,
    pub position_x: i32,
    pub position_y: i32,
    pub width: i32,
    pub height: i32,
    pub remove_bg: bool,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default)]
    pub ai_input_size: Option<u32>,
    #[serde(default)]
    pub models_dir: Option<String>,
}

fn default_ai_model() -> String {
    "u2net".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: "assets/default.gif".to_string(),
            position_x: 100,
            position_y: 100,
            width: 300,
            height: 300,
            remove_bg: false,
            ai_model: default_ai_model(),
            ai_input_size: None,
            models_dir: None,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&s)?)
    }
}