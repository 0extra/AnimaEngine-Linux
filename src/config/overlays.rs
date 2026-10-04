use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayData {
    pub source: String,
    pub name: String,
    pub size: i32,
    pub speed: f64,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OverlaysFile {
    #[serde(default)]
    pub items: Vec<OverlayData>,
}

impl OverlaysFile {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anima-linux")
            .join("overlays.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        if !p.exists() {
            return Self::default();
        }
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(p, s)?;
        Ok(())
    }
}