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

#[derive(PartialEq)]
pub enum Mode {
    Navigate,
    Input,
}