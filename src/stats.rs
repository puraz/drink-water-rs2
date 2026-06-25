use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single day's record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayRecord {
    pub date: NaiveDate,
    pub count: u64,
}

/// Daily hydration statistics with multi-day history
#[derive(Debug, Serialize, Deserialize)]
pub struct DrinkStats {
    records: Vec<DayRecord>,
}

/// Old single-day format (for migration)
#[derive(Deserialize)]
struct OldStats {
    date: NaiveDate,
    count: u64,
}

impl DrinkStats {
    /// Path to the stats file
    fn path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("drink-water-rs2");
        path.push("stats.json");
        path
    }

    /// Load all records from disk, migrating old format if needed.
    pub fn load() -> Self {
        let path = Self::path();

        if let Ok(content) = std::fs::read_to_string(&path) {
            // Try new multi-record format first
            if let Ok(stats) = serde_json::from_str::<Self>(&content) {
                return stats;
            }
            // Try old single-record format and migrate
            if let Ok(old) = serde_json::from_str::<OldStats>(&content) {
                let stats = Self {
                    records: vec![DayRecord {
                        date: old.date,
                        count: old.count,
                    }],
                };
                stats.save();
                log::info!("统计数据已迁移到新版格式");
                return stats;
            }
            log::warn!("统计数据文件解析失败，重新开始");
        }

        Self {
            records: Vec::new(),
        }
    }

    /// Write stats to disk, pruning records older than 7 days
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Prune old records before saving
        let today = Local::now().date_naive();
        let cutoff = today - chrono::Duration::days(7);
        let pruned: Vec<DayRecord> = self
            .records
            .iter()
            .filter(|r| r.date >= cutoff)
            .cloned()
            .collect();

        if let Ok(content) = serde_json::to_string_pretty(&DrinkStats { records: pruned }) {
            let _ = std::fs::write(&path, content);
        }
    }

    /// Record one drink for today. Returns today's total count.
    pub fn record_drink(&mut self) -> u64 {
        let today = Local::now().date_naive();
        if let Some(record) = self.records.iter_mut().find(|r| r.date == today) {
            record.count += 1;
        } else {
            self.records.push(DayRecord {
                date: today,
                count: 1,
            });
        }
        self.save();
        self.today_count()
    }

    /// Get today's count
    pub fn today_count(&self) -> u64 {
        let today = Local::now().date_naive();
        self.records
            .iter()
            .find(|r| r.date == today)
            .map(|r| r.count)
            .unwrap_or(0)
    }

    /// Get records for the last 7 days (including today), with zeros for
    /// days that have no data. Returns oldest-first.
    pub fn week_history(&self) -> Vec<DayRecord> {
        let today = Local::now().date_naive();
        let mut result = Vec::with_capacity(7);
        for i in (0..7).rev() {
            let date = today - chrono::Duration::days(i);
            let count = self
                .records
                .iter()
                .find(|r| r.date == date)
                .map(|r| r.count)
                .unwrap_or(0);
            result.push(DayRecord { date, count });
        }
        result
    }

    /// Maximum count in the last 7 days (for chart scaling)
    pub fn week_max(&self) -> u64 {
        let today = Local::now().date_naive();
        self.records
            .iter()
            .filter(|r| r.date >= today - chrono::Duration::days(7))
            .map(|r| r.count)
            .max()
            .unwrap_or(0)
    }
}
