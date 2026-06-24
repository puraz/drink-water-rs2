use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daily hydration statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct DrinkStats {
    pub date: NaiveDate,
    pub count: u64,
}

impl DrinkStats {
    /// Path to the stats file
    fn path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("drink-water-rs2");
        path.push("stats.json");
        path
    }

    /// Load today's stats, or create a fresh one
    pub fn load_today() -> Self {
        let today = Local::now().date_naive();
        let path = Self::path();

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(stats) = serde_json::from_str::<Self>(&content) {
                if stats.date == today {
                    return stats;
                }
            }
        }

        // No stats for today → start fresh
        let stats = DrinkStats {
            date: today,
            count: 0,
        };
        stats.save();
        stats
    }

    /// Write stats to disk
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }

    /// Record one drink for today
    pub fn record_drink(&mut self) -> u64 {
        let today = Local::now().date_naive();
        if self.date != today {
            // New day — reset
            self.date = today;
            self.count = 0;
        }
        self.count += 1;
        self.save();
        self.count
    }

    /// Get today's count
    pub fn today_count(&self) -> u64 {
        let today = Local::now().date_naive();
        if self.date == today {
            self.count
        } else {
            0
        }
    }
}
