use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, VecDeque}, fs::File, io::BufReader, time::SystemTime};

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

pub struct App {
    pub processes: Vec<(String, ProcessInfo)>,
    pub selected_index: usize,
    pub mode: Mode,
    pub input_buffer: String,
    pub error_message: String,
    pub log_cache: HashMap<String, VecDeque<String>>,
    pub log_readers: HashMap<String, BufReader<File>>,
    pub is_paused: bool,
    pub last_reap: SystemTime,
    pub name_input_mode: bool,
    pub pending_launch: Option<PendingLaunch>,
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

impl App {
    pub fn new(processes: Vec<(String, ProcessInfo)>) -> Self {
        Self {
            processes,
            selected_index: 0,
            mode: Mode::Navigate,
            input_buffer: String::new(),
            error_message: String::new(),
            log_cache: HashMap::new(),
            log_readers: HashMap::new(),
            is_paused: false,
            last_reap: SystemTime::now(),
            name_input_mode: false,
            pending_launch: None,
        }
    }
}