use std::{env, fs::{self, OpenOptions}, io, os::unix::process::CommandExt, path::{Path, PathBuf}, process::{Command, Stdio}, time::{SystemTime, UNIX_EPOCH}};
use crate::state::*;
use crate::types::ProcessInfo;

pub fn parse_timeout(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.ends_with('s') {
        s[..s.len() - 1].parse().ok()
    } else {
        s.parse().ok()
    }
}

fn find_script(script_dir: &Path, name: &str) -> Option<PathBuf> {
    for ext in ["py", "sh", ""] {
        let path = if ext.is_empty() {
            script_dir.join("scripts").join(name)
        } else {
            script_dir.join("scripts").join(format!("{}.{}", name, ext))
        };
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn launch_process(
    script_dir: &Path,
    state_file: &Path,
    log_dir: &Path,
    script_args: &[String],
    timeout_sec: Option<f64>,
) -> io::Result<()> {
    launch_process_with_name(script_dir, state_file, log_dir, script_args, timeout_sec, None, None)
}

pub fn launch_process_with_name(
    script_dir: &Path,
    state_file: &Path,
    log_dir: &Path,
    script_args: &[String],
    timeout_sec: Option<f64>,
    display_name: Option<String>,
    working_dir: Option<String>,
) -> io::Result<()> {
    let script_name = &script_args[0];
    let args = &script_args[1..];
    let cwd = working_dir.unwrap_or_else(|| env::current_dir().unwrap().to_string_lossy().to_string());

    let (cmd, cmd_args, script_path) = if let Some(path) = find_script(script_dir, script_name) {
        if path.extension().and_then(|s| s.to_str()) == Some("py") {
            ("python3".to_string(), [vec![path.to_string_lossy().to_string()], args.to_vec()].concat(), path.to_string_lossy().to_string())
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = fs::metadata(&path)?;
                if metadata.permissions().mode() & 0o111 == 0 {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
                }
            }
            (path.to_string_lossy().to_string(), args.to_vec(), path.to_string_lossy().to_string())
        }
    } else {
        (script_name.clone(), args.to_vec(), script_name.clone())
    };

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let clean_name: String = script_name.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
    let log_file = log_dir.join(format!("{}_{}.log", clean_name, timestamp % 100000));

    let log_handle = OpenOptions::new().create(true).append(true).open(&log_file)?;


    let mut child = Command::new(&cmd);

    child
        .args(&cmd_args)
        .current_dir(&cwd)
        .stdout(Stdio::from(log_handle.try_clone()?))
        .stderr(Stdio::from(log_handle));

    unsafe {
            child
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
    };

    let child = child.spawn()?;

    let cmd_str = format!("{} {}", cmd, cmd_args.join(" "));
    let final_name = display_name.unwrap_or_else(|| script_name.clone());
    
    register_process(state_file, child.id(), &cmd_str, timeout_sec, &log_file, script_name, &cwd, &final_name);

    Ok(())
}

pub fn revive_dead_processes(script_dir: &Path, state_file: &Path, log_dir: &Path) {
    use std::io::Write;
    use chrono::Local;
    use sysinfo::{Pid, System};
    
    let mut state = read_state(state_file);
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let pids_to_revive: Vec<_> = state.iter()
        .filter_map(|(pid_str, proc)| {
            if proc.status != "running" {
                return None;
            }
            let pid: u32 = pid_str.parse().ok()?;
            if sys.process(Pid::from_u32(pid)).is_none() {
                Some((pid_str.clone(), proc.clone()))
            } else {
                None
            }
        })
        .collect();
    
    for (old_pid, proc) in pids_to_revive {
        state.remove(&old_pid);
        write_state(state_file, &state);
        
        if let Ok(mut f) = OpenOptions::new().append(true).open(&proc.log_file) {
            writeln!(f, "\n--- 🔄 AUTO-REVIVED (was PID {}) @ {} ---", old_pid, Local::now()).ok();
        }

        let parts: Vec<&str> = proc.cmd_str.split_whitespace().collect();
        if !parts.is_empty() {
            let log_handle = OpenOptions::new().create(true).append(true).open(&proc.log_file).unwrap();

            if let Ok(child) = unsafe {
                Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(&proc.working_dir)
                    .stdout(Stdio::from(log_handle.try_clone().unwrap()))
                    .stderr(Stdio::from(log_handle))
                    .pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    })
                    .spawn()
            } {
                register_process(state_file, child.id(), &proc.cmd_str, proc.timeout_sec, Path::new(&proc.log_file), &proc.script_name, &proc.working_dir, &proc.display_name);
            }
        }
    }
}