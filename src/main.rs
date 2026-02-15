use clap::{Parser, Subcommand};
use std::{env, fs, path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};
use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, Terminal};

mod state;
mod process;
mod ui;
mod types;
mod monitor;
mod app;
mod scheduler;

use crate::{process::*, state::*, ui::*, app::App, types::{ScheduledJob, ProcessInfo}};

#[derive(Parser)]
#[command(name = "ksai_proc")]
#[command(about = "A lightweight TUI process manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Timeout for the process (e.g. 10s, 1m)
    #[arg(long, value_parser = parse_timeout_clap)]
    timeout: Option<f64>,

    /// Display name for the process
    #[arg(short, long)]
    name: Option<String>,

    /// Launch a process directly (backwards compatibility)
    #[arg(allow_hyphen_values = true)]
    script_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a new process
    Run {
        /// Do not open the TUI after launching
        #[arg(long)]
        no_tui: bool,

        /// Display name for the process
        #[arg(short, long)]
        name: Option<String>,

        /// Timeout for the process (e.g. 10s, 1m)
        #[arg(long, value_parser = parse_timeout_clap)]
        timeout: Option<f64>,

        #[arg(allow_hyphen_values = true)]
        script_args: Vec<String>,
    },
    /// List all tracked processes
    List,
    /// Stop a running process by PID
    Stop {
        /// PID of the process
        pid: Option<String>,
        /// Name of the process
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove a process from tracking by PID
    Remove { pid: String },
    /// Restart a process by PID
    Restart { pid: String },
    /// View logs for a process by PID
    Logs {
        pid: String,
        /// Number of lines to show (default: 20)
        #[arg(short, long, default_value_t = 20)]
        lines: usize,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
    /// Clean up dead processes and old logs
    Prune,
    /// Revive processes that have crashed
    Revive,
    /// Scheduler commands
    Schedule {
         #[command(subcommand)]
         cmd: ScheduleCommands,
    },
    /// Internal scheduler daemon (hidden)
    #[command(hide = true)]
    InternalScheduler,
}

#[derive(Subcommand)]
enum ScheduleCommands {
    /// Schedule a new process
    Add {
        /// Unique name for the schedule
        #[arg(long)]
        name: String,
        /// Frequency (e.g., 1m, 1h, 1d)
        #[arg(long)]
        every: String,
        /// Start date (YYYY-MM-DD HH:MM:SS) or "now"
        #[arg(long, default_value = "now")]
        start_at: String,
        /// Command to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// List scheduled jobs
    List,
    /// Stop/Disable a scheduled job
    Stop {
        name: String,
    },
    /// Remove a scheduled job
    Remove {
        name: String,
    },
}

fn parse_timeout_clap(s: &str) -> Result<f64, String> {
    parse_timeout(s).ok_or_else(|| format!("Invalid timeout format: {}", s))
}

fn get_scheduled_file(exe_dir: &std::path::Path) -> PathBuf {
    env::var("KSAI_PROC_SCHEDULE_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe_dir.join("logs/scheduledscripts.json"))
}

fn ensure_scheduler_running(exe_dir: &std::path::Path, state_file: &std::path::Path) {
    // Check if scheduler is running by reaping first to get fresh status
    let procs = reap_processes(state_file);
    let scheduler_running = procs.iter().any(|(_, p)| p.display_name == "ksai_scheduler_daemon" && p.status == "running");
    
    // Debug
    // println!("DEBUG: ensure_scheduler_running: scheduler_running={}, procs={}", scheduler_running, procs.len());

    if !scheduler_running {
        // Spawn it
        let ksai_proc_exe = exe_dir.join("ksai_proc");
        let log_dir = env::var("KSAI_PROC_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| exe_dir.join("logs"));
        let log_file = log_dir.join("scheduler.log");
        
        let log_handle = std::fs::OpenOptions::new().create(true).append(true).open(&log_file).ok();

        if let Some(file) = log_handle {
             match std::process::Command::new(&ksai_proc_exe)
                .arg("internal-scheduler")
                .stdout(std::process::Stdio::from(file.try_clone().unwrap()))
                .stderr(std::process::Stdio::from(file))
                .spawn() {
                    Ok(child) => {
                         // Register it so we know it's running
                         register_process(state_file, child.id(), "ksai_proc internal-scheduler", None, &log_file, "scheduler", &exe_dir.to_string_lossy(), "ksai_scheduler_daemon");
                    },
                    Err(e) => eprintln!("Failed to start scheduler daemon: {}", e),
                }
        }
    }
}

fn main() {
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let state_file = env::var("KSAI_PROC_LOG_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe_dir.join("logs/runningscripts.json"));
    let log_dir = env::var("KSAI_PROC_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe_dir.join("logs"));
    let scheduled_file = get_scheduled_file(&exe_dir);

    fs::create_dir_all(&log_dir).ok();

    let cli = Cli::parse();

    if let Some(cmd) = &cli.command {
        match cmd {
            Commands::InternalScheduler => {}
            _ => ensure_scheduler_running(&exe_dir, &state_file),
        }
    }

    match cli.command {
        Some(Commands::Run { script_args, no_tui: _, name, timeout }) => {
            if !script_args.is_empty() {
                let final_timeout = timeout.or(cli.timeout);
                if let Err(e) = launch_process_with_name(&exe_dir, &state_file, &log_dir, &script_args, final_timeout, name, None) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                println!("Process launched successfully.");
            }
        }
        Some(Commands::List) => {
            let state = read_state(&state_file);
            let mut procs: Vec<_> = state.into_iter().collect();
            procs.sort_by(|a, b| b.1.start_time.partial_cmp(&a.1.start_time).unwrap());
            
            if procs.is_empty() {
                println!("No processes running.");
                return;
            }

            // Calculate widths
            let mut max_pid = 3; // "PID"
            let mut max_name = 4; // "Name"
            let mut max_status = 6; // "Status"
            let mut max_started = 19; // "YYYY-MM-DD HH:MM:SS"
            let mut max_dir = 9; // "Directory"
            let mut max_cmd = 7; // "Command"

            // Pre-calculate formatted strings to determine widths
            let rows: Vec<_> = procs.iter().map(|(pid, proc)| {
                let started = chrono::DateTime::from_timestamp(proc.start_time as i64, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                max_pid = max_pid.max(pid.len());
                max_name = max_name.max(proc.display_name.len());
                max_status = max_status.max(proc.status.len());
                max_started = max_started.max(started.len());
                max_dir = max_dir.max(proc.working_dir.len());
                max_cmd = max_cmd.max(proc.cmd_str.len());

                (pid, proc, started)
            }).collect();

            // Add some padding
            max_pid += 2;
            max_name += 2;
            max_status += 2;
            max_started += 2;
            max_dir += 2;
            
            // Header
            println!(
                "{:<w_pid$} {:<w_name$} {:<w_status$} {:<w_started$} {:<w_dir$} {:<w_cmd$}", 
                "PID", "Name", "Status", "Started", "Directory", "Command",
                w_pid = max_pid,
                w_name = max_name,
                w_status = max_status,
                w_started = max_started,
                w_dir = max_dir,
                w_cmd = max_cmd
            );
            
            // Separator
            let total_width = max_pid + max_name + max_status + max_started + max_dir + max_cmd;
            println!("{}", "-".repeat(total_width + 5));

            // Rows
            for (pid, proc, started) in rows {
                println!(
                    "{:<w_pid$} {:<w_name$} {:<w_status$} {:<w_started$} {:<w_dir$} {:<w_cmd$}", 
                    pid, proc.display_name, proc.status, started, proc.working_dir, proc.cmd_str,
                    w_pid = max_pid,
                    w_name = max_name,
                    w_status = max_status,
                    w_started = max_started,
                    w_dir = max_dir,
                    w_cmd = max_cmd
                );
            }
        }
        Some(Commands::Stop { pid, name }) => {
            let mut message = String::new();
            
            update_state(&state_file, |state| {
                let target_pid = if let Some(n) = &name {
                    let found = state.iter().find(|(_, p)| p.display_name == *n).map(|(pid, _)| pid.clone());
                    if found.is_none() {
                        message = format!("Process with name '{}' not found.", n);
                        return;
                    }
                    found
                } else if pid.is_some() {
                    pid.clone()
                } else {
                    None
                };

                if let Some(pid_str) = target_pid {
                    if let Some(proc) = state.get_mut(&pid_str) {
                         if proc.status == "running" {
                             let pid_val: i32 = pid_str.parse().unwrap_or(0);
                             unsafe { libc::kill(-pid_val, libc::SIGKILL); }
                             // Sleep inside lock to ensure status update reflects reality (process fully dead)
                             // before releasing lock? Or just mark it 'killed' immediately?
                             // 200ms is a bit long for a lock. 
                             // But if we don't sleep, `waitpid` might not have reaped it yet? 
                             // Actually `kill` returns immediately. 
                             // If we mark it "killed", the scheduler will see "killed" and ignore it (correct).
                             std::thread::sleep(std::time::Duration::from_millis(200));
                             proc.status = "killed (manual)".to_string();
                             message = format!("Process {} stopped.", pid_str);
                         } else {
                             message = format!("Process {} is not running (status: {}).", pid_str, proc.status);
                         }
                    } else {
                         message = format!("Process {} not found.", pid_str);
                    }
                } else {
                     message = "Error: You must specify either a PID or a valid --name.".to_string();
                }
            });
            println!("{}", message);
        }
        Some(Commands::Remove { pid }) => {
            let mut message = String::new();
             update_state(&state_file, |state| {
                if let Some(proc) = state.remove(&pid) {
                    if proc.status == "running" {
                        let pid_val: i32 = pid.parse().unwrap_or(0);
                        unsafe { libc::kill(-pid_val, libc::SIGKILL); }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    fs::remove_file(&proc.log_file).ok();
                    message = format!("Process {} removed and logs deleted.", pid);
                } else {
                    message = format!("Process {} not found.", pid);
                }
             });
             println!("{}", message);
        }
        Some(Commands::Restart { pid }) => {
            let mut proc_to_restart: Option<ProcessInfo> = None;
            let mut message = String::new();

             update_state(&state_file, |state| {
                if let Some(proc) = state.get(&pid).cloned() {
                    let old_pid: i32 = pid.parse().unwrap_or(0);
                    if proc.status == "running" {
                        unsafe { libc::kill(-old_pid, libc::SIGKILL); }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    
                    state.remove(&pid);
                    proc_to_restart = Some(proc);
                } else {
                    message = format!("Process {} not found.", pid);
                }
             });

            if let Some(proc) = proc_to_restart {
                let parts: Vec<&str> = proc.cmd_str.split_whitespace().collect();
                if !parts.is_empty() {
                    let log_handle = std::fs::OpenOptions::new().create(true).append(true).open(&proc.log_file).unwrap();
                    if let Ok(child) = unsafe {
                        use std::os::unix::process::CommandExt;
                        std::process::Command::new(parts[0])
                            .args(&parts[1..])
                            .current_dir(&proc.working_dir)
                            .stdout(std::process::Stdio::from(log_handle.try_clone().unwrap()))
                            .stderr(std::process::Stdio::from(log_handle))
                            .pre_exec(|| {
                                libc::setsid();
                                Ok(())
                            })
                            .spawn()
                    } {
                        let pid_val: u32 = child.id();
                        register_process(&state_file, pid_val, &proc.cmd_str, proc.timeout_sec, std::path::Path::new(&proc.log_file), &proc.script_name, &proc.working_dir, &proc.display_name);
                        println!("Process restarted with new PID {}.", pid_val);
                    }
                }
            } else {
                 if !message.is_empty() {
                     println!("{}", message);
                 }
            }
        }
        Some(Commands::Logs { pid, lines, follow }) => {
            let state = read_state(&state_file);
            if let Some(proc) = state.get(&pid) {
                if follow {
                    let mut cmd = std::process::Command::new("tail");
                    cmd.arg("-f").arg("-n").arg(lines.to_string()).arg(&proc.log_file);
                    cmd.status().ok();
                } else {
                    let mut cmd = std::process::Command::new("tail");
                    cmd.arg("-n").arg(lines.to_string()).arg(&proc.log_file);
                    cmd.status().ok();
                }
            } else {
                println!("Process {} not found.", pid);
            }
        }
        Some(Commands::Prune) => {
             let mut removed_count = 0;
            update_state(&state_file, |state| {
                let before = state.len();
                state.retain(|_, proc| proc.status == "running");
                removed_count = before - state.len();
            });
            println!("Pruned {} non-running processes.", removed_count);
        }
        Some(Commands::Revive) => {
            println!("Reviving crashed processes...");
            revive_dead_processes(&exe_dir, &state_file, &log_dir);
            println!("Revival check complete.");
        }
        Some(Commands::Schedule { cmd }) => {
            ensure_scheduler_running(&exe_dir, &state_file);
            match cmd {
                ScheduleCommands::Add { name, every, start_at, command } => {
                     // Check if name exists
                     let mut jobs = read_scheduled_jobs(&scheduled_file);
                     if jobs.iter().any(|j| j.name == name) {
                         println!("Error: Scheduled job with name '{}' already exists.", name);
                         return;
                     }
                     if command.is_empty() {
                         println!("Error: No command provided.");
                         return;
                     }

                     use chrono::{NaiveDateTime, Local, TimeZone};
                     let start_timestamp = if start_at == "now" {
                         SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
                     } else {
                         // Try parsing "YYYY-MM-DD HH:MM:SS" (Local)
                         if let Ok(dt) = NaiveDateTime::parse_from_str(&start_at, "%Y-%m-%d %H:%M:%S") {
                             Local.from_local_datetime(&dt).unwrap().timestamp() as u64
                         } else if let Ok(dt) = NaiveDateTime::parse_from_str(format!("{} 00:00:00", start_at).as_str(), "%Y-%m-%d %H:%M:%S") {
                             Local.from_local_datetime(&dt).unwrap().timestamp() as u64
                         } else {
                             println!("Error: Invalid date format. Use 'YYYY-MM-DD HH:MM:SS' or 'YYYY-MM-DD' or 'now'");
                             return;
                         }
                     };

                     let job = ScheduledJob {
                         name: name.clone(),
                         command: command[0].clone(),
                         args: command[1..].to_vec(),
                         frequency: every,
                         start_at: start_timestamp,
                         working_dir: env::current_dir().unwrap().to_string_lossy().to_string(),
                         last_run: None,
                         enabled: true,
                     };
                     
                     jobs.push(job);
                     write_scheduled_jobs(&scheduled_file, &jobs);
                     println!("Scheduled job '{}' added.", name);
                }
                ScheduleCommands::List => {
                    let jobs = read_scheduled_jobs(&scheduled_file);
                    if jobs.is_empty() {
                        println!("No scheduled jobs.");
                    } else {
                        println!("{:<15} {:<10} {:<20} {:<10} {:<20}", "Name", "Every", "Start At", "Enabled", "Command");
                        println!("{}", "-".repeat(80));
                        for job in jobs {
                            let start_at_str = chrono::DateTime::from_timestamp(job.start_at as i64, 0).map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or("?".to_string());
                            println!("{:<15} {:<10} {:<20} {:<10} {:<20}", job.name, job.frequency, start_at_str, job.enabled, job.command);
                        }
                    }
                }
                ScheduleCommands::Stop { name } => {
                    let mut jobs = read_scheduled_jobs(&scheduled_file);
                    if let Some(job) = jobs.iter_mut().find(|j| j.name == name) {
                        job.enabled = false;
                         write_scheduled_jobs(&scheduled_file, &jobs);
                        println!("Scheduled job '{}' disabled.", name);
                    } else {
                        println!("Scheduled job '{}' not found.", name);
                    }
                }
                ScheduleCommands::Remove { name } => {
                    let mut jobs = read_scheduled_jobs(&scheduled_file);
                    let len_before = jobs.len();
                    jobs.retain(|j| j.name != name);
                    if jobs.len() < len_before {
                        write_scheduled_jobs(&scheduled_file, &jobs);
                         println!("Scheduled job '{}' removed.", name);
                    } else {
                         println!("Scheduled job '{}' not found.", name);
                    }
                }
            }
        }
        Some(Commands::InternalScheduler) => {
            scheduler::start_scheduler_daemon(&state_file, &scheduled_file, &log_dir);
        }
        None => {
            if !cli.script_args.is_empty() {
                ensure_scheduler_running(&exe_dir, &state_file);
                if let Err(e) = launch_process_with_name(&exe_dir, &state_file, &log_dir, &cli.script_args, cli.timeout, cli.name, None) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                println!("Process launched successfully.");
                return;
            }
            
            ensure_scheduler_running(&exe_dir, &state_file);

            revive_dead_processes(&exe_dir, &state_file, &log_dir);

            if let Err(e) = run_tui(&state_file, &log_dir) {
                eprintln!("TUI error: {}", e);
            }
        }
    }
}

fn run_tui(state_file: &std::path::Path, log_dir: &std::path::Path) -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(reap_processes(state_file));
    let result = run_app(&mut terminal, &mut app, state_file, log_dir);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}