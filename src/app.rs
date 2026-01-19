use std::{collections::{HashMap, VecDeque}, fs::File, io::BufReader, time::SystemTime};
use crate::types::{ProcessInfo, PendingLaunch, Mode};
use crate::monitor::Monitor;

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
    pub monitor: Monitor,
    pub show_resources: bool,
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
            monitor: Monitor::new(),
            show_resources: false,
        }
    }
}
