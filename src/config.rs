use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub focus_minutes: u32,
    pub break_minutes: u32,
    pub total_sessions: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            break_minutes: 5,
            total_sessions: 5,
        }
    }
}

impl Config {
    fn path() -> Option<PathBuf> {
        xdg::BaseDirectories::with_prefix("gpui-pomo")
            .ok()?
            .place_config_file("config.json")
            .ok()
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}
