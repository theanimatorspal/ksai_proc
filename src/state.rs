use std::{collections::HashMap, fs::{self, OpenOptions}, io::{self, BufReader, Seek, SeekFrom}, path::Path, time::{SystemTime, UNIX_EPOCH}};
use sysinfo::{Pid, System};
use fs2::FileExt; // Added for file locking
use crate::types::ProcessInfo;

// Helper for atomic state access (Read-Only)
pub fn read_state(state_file: &Path) -> HashMap<String, ProcessInfo> {
    if !state_file.exists() {
        return HashMap::new();
    }
    
    if let Ok(file) = OpenOptions::new().read(true).open(state_file) {
        // Shared lock for reading
        if file.lock_shared().is_ok() {
            let reader = BufReader::new(&file);
            let state = serde_json::from_reader(reader).unwrap_or_default();
            let _ = file.unlock();
            return state;
        }
    }
    // Fallback if locking fails (should rare) or file issues
    HashMap::new()
}

// Helper for atomic state access (Write-Only - typically not used directly to avoid race, but kept for compatibility if needed)
pub fn write_state(state_file: &Path, state: &HashMap<String, ProcessInfo>) {
    if let Ok(mut file) = OpenOptions::new().write(true).create(true).truncate(true).open(state_file) {
        if file.lock_exclusive().is_ok() {
            let _ = serde_json::to_writer_pretty(&file, state);
            let _ = file.unlock();
        }
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
    // Atomic update: Read -> Modify -> Write under exclusive lock
    if let Ok(mut file) = OpenOptions::new().read(true).write(true).create(true).open(state_file) {
        if file.lock_exclusive().is_ok() {
             // Read
            let _ = file.seek(SeekFrom::Start(0));
            let mut state: HashMap<String, ProcessInfo> = if file.metadata().unwrap().len() > 0 {
                serde_json::from_reader(BufReader::new(&file)).unwrap_or_default()
            } else {
                HashMap::new()
            };

            // Modify
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

            // Write
            let _ = file.seek(SeekFrom::Start(0));
            let _ = file.set_len(0);
            let _ = serde_json::to_writer_pretty(&file, &state);
            
            let _ = file.unlock();
        }
    }
}

pub fn reap_processes(state_file: &Path) -> Vec<(String, ProcessInfo)> {
    // Atomic update: Read -> Modify -> Write under exclusive lock
    let mut procs = Vec::new();

    if let Ok(mut file) = OpenOptions::new().read(true).write(true).create(true).open(state_file) {
        if file.lock_exclusive().is_ok() {
             // Read
            let _ = file.seek(SeekFrom::Start(0));
            let mut state: HashMap<String, ProcessInfo> = if file.metadata().unwrap().len() > 0 {
                serde_json::from_reader(BufReader::new(&file)).unwrap_or_default()
            } else {
                HashMap::new()
            };

            let mut sys = System::new();
            sys.refresh_all(); // Initial refresh
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64();

            for (pid_str, proc) in state.iter_mut() {
                if proc.status != "running" {
                    continue;
                }

                let pid: u32 = pid_str.parse().unwrap_or(0);
                
                // Direct existence check
                let exists = unsafe { libc::kill(pid as i32, 0) == 0 };
                
                let is_alive = if !exists {
                    false
                } else {
                    // It exists, check if it's a zombie. 
                    #[cfg(target_os = "linux")]
                    {
                        if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
                            if let Some(state_line) = status.lines().find(|l| l.starts_with("State:")) {
                                let state_part = state_line.split_whitespace().nth(1).unwrap_or("");
                                // Z = Zombie, X = Dead, t = Tracing stop
                                !(state_part.starts_with('Z') || state_part.starts_with('X'))
                            } else {
                                true 
                            }
                        } else {
                            true // If we can't read /proc but kill(0) succeeded, assume alive
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        let sys_pid = Pid::from_u32(pid);
                        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sys_pid]), true);
                        sys.process(sys_pid).map(|p| p.status() != sysinfo::ProcessStatus::Zombie).unwrap_or(false)
                    }
                };

                if let Some(timeout) = proc.timeout_sec {
                    if is_alive && (now - proc.start_time) > timeout {
                        unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
                        std::thread::sleep(std::time::Duration::from_millis(200)); 
                        proc.status = "killed (timeout)".to_string();
                        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&proc.log_file) {
                            use std::io::Write;
                            writeln!(f, "\n--- ❌ Timeout ({}s) reached. Killed by manager. ---", timeout).ok();
                        }
                    }
                }

                if !is_alive && proc.status == "running" {
                    // println!("DEBUG: reap_processes: PID {} marked as completed (is_alive=false)", pid_str);
                    proc.status = "completed".to_string();
                }
            }
            
            // Capture result before writing
            procs = state.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            // Write
            let _ = file.seek(SeekFrom::Start(0));
            let _ = file.set_len(0);
            let _ = serde_json::to_writer_pretty(&file, &state);
            
            let _ = file.unlock();
        }
    }
    
    procs.sort_by(|a, b| b.1.start_time.partial_cmp(&a.1.start_time).unwrap());
    procs
}

pub fn read_scheduled_jobs(path: &Path) -> Vec<crate::types::ScheduledJob> {
     // Shared lock for reading
    if !path.exists() {
        return Vec::new();
    }
    if let Ok(file) = OpenOptions::new().read(true).open(path) {
        if file.lock_shared().is_ok() {
             let reader = BufReader::new(&file);
             let jobs = serde_json::from_reader(reader).unwrap_or_default();
             let _ = file.unlock();
             return jobs;
        }
    }
    Vec::new()
}

pub fn write_scheduled_jobs(path: &Path, jobs: &Vec<crate::types::ScheduledJob>) {
    // Exclusive lock for writing
    if let Ok(mut file) = OpenOptions::new().write(true).create(true).truncate(true).open(path) {
        if file.lock_exclusive().is_ok() {
             let _ = serde_json::to_writer_pretty(&file, jobs);
             let _ = file.unlock();
        }
    }
}

pub fn update_state<F>(state_file: &Path, f: F) 
where F: FnOnce(&mut HashMap<String, ProcessInfo>)
{
    if let Ok(mut file) = OpenOptions::new().read(true).write(true).create(true).open(state_file) {
        if file.lock_exclusive().is_ok() {
             let _ = file.seek(SeekFrom::Start(0));
             let mut state: HashMap<String, ProcessInfo> = if file.metadata().unwrap().len() > 0 {
                 serde_json::from_reader(BufReader::new(&file)).unwrap_or_default()
             } else {
                 HashMap::new()
             };

             f(&mut state);

             let _ = file.seek(SeekFrom::Start(0));
             let _ = file.set_len(0);
             let _ = serde_json::to_writer_pretty(&file, &state);
             
             let _ = file.unlock();
        }
    }
}