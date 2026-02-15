use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub cmd_str: String,
    pub log_file: String,
    pub status: String,
    pub start_time: f64,
    pub timeout_sec: Option<f64>,
    pub script_name: String,
    pub working_dir: String,
    pub display_name: String,
}

pub struct PendingLaunch {
    pub script_args: Vec<String>,
    pub timeout_sec: Option<f64>,
    pub working_dir: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledJob {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub frequency: String, // e.g., "1m", "1h", "1d"
    pub start_at: u64, // Unix timestamp
    pub working_dir: String,
    pub last_run: Option<u64>, // Timestamp of last run
    pub enabled: bool,
}

#[derive(PartialEq)]
pub enum Mode {
    Navigate,
    Input,
}