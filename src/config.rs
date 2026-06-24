use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Reminder interval in minutes
    pub interval_minutes: u64,
    /// Snooze duration in minutes
    pub snooze_minutes: u64,
    /// Water amount per drink in ml
    pub water_amount_ml: u64,
    /// Hour (0-23) to start reminding
    pub start_hour: u8,
    /// Hour (0-23) to stop reminding
    pub end_hour: u8,
    /// Whether running time display
    pub show_running_time: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            snooze_minutes: 5,
            water_amount_ml: 250,
            start_hour: 9,
            end_hour: 22,
            show_running_time: true,
        }
    }
}

impl Config {
    /// Path to config file
    pub fn path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("drink-water-rs2");
        path.push("config.json");
        path
    }

    /// Load config from disk, or create default
    pub fn load() -> Self {
        let path = Self::path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => {
                        log::info!("配置已加载 {:?}", path);
                        return config;
                    }
                    Err(e) => log::warn!("配置文件解析失败: {e}，使用默认值"),
                },
                Err(e) => log::warn!("配置文件读取失败: {e}，使用默认值"),
            }
        }
        let config = Config::default();
        config.save();
        config
    }

    /// Save config to disk
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(content) => match std::fs::write(&path, content) {
                Ok(_) => log::info!("配置已保存 {:?}", path),
                Err(e) => log::error!("配置保存失败: {e}"),
            },
            Err(e) => log::error!("配置序列化失败: {e}"),
        }
    }
}
