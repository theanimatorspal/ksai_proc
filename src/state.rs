use std::{collections::HashMap, fs, path::Path, time::{SystemTime, UNIX_EPOCH}};
use sysinfo::{Pid, System};
use crate::types::ProcessInfo;

pub fn read_state(state_file: &Path) -> HashMap<String, ProcessInfo> {
    if let Ok(content) = fs::read_to_string(state_file) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn write_state(state_file: &Path, state: &HashMap<String, ProcessInfo>) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        fs::write(state_file, json).ok();
    }
}

pub fn register_process(
    state_file: &Path,
    pid: u32,
    cmd_str: &str,
    timeout_sec: Option<f64>,
    log_file: &Path,
    script_name: &str,
    working_dir: &str,
    display_name: &str,
) {
    let mut state = read_state(state_file);
    state.insert(
        pid.to_string(),
        ProcessInfo {
            cmd_str: cmd_str.to_string(),
            log_file: log_file.to_string_lossy().to_string(),
            status: "running".to_string(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64(),
            timeout_sec,
            script_name: script_name.to_string(),
            working_dir: working_dir.to_string(),
            display_name: display_name.to_string(),
        },
    );
    write_state(state_file, &state);
}

pub fn reap_processes(state_file: &Path) -> Vec<(String, ProcessInfo)> {
    let mut state = read_state(state_file);
    let mut sys = System::new_all();
    sys.refresh_all();

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();

    for (pid_str, proc) in state.iter_mut() {
        if proc.status != "running" {
            continue;
        }

        let pid: u32 = pid_str.parse().unwrap_or(0);
        let is_alive = sys.process(Pid::from_u32(pid)).is_some();

        if let Some(timeout) = proc.timeout_sec {
            if is_alive && (now - proc.start_time) > timeout {
                unsafe { libc::kill(pid as i32, libc::SIGKILL); }
                proc.status = "killed (timeout)".to_string();
                if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&proc.log_file) {
                    use std::io::Write;
                    writeln!(f, "\n--- ❌ Timeout ({}s) reached. Killed by manager. ---", timeout).ok();
                }
            }
        }

        if !is_alive && proc.status == "running" {
            proc.status = "completed".to_string();
        }
    }

    write_state(state_file, &state);

    let mut procs: Vec<_> = state.into_iter().collect();
    procs.sort_by(|a, b| b.1.start_time.partial_cmp(&a.1.start_time).unwrap());
    procs
}