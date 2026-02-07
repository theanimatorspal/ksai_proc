use clap::{Parser, Subcommand};
use std::{env, fs, path::PathBuf, time::Duration};
use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, Terminal};

mod state;
mod process;
mod ui;
mod types;
mod monitor;
mod app;

use crate::{process::*, state::*, ui::*, app::App};

#[derive(Parser)]
#[command(name = "ksai_proc")]
#[command(about = "A lightweight TUI process manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Launch a process directly (backwards compatibility)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    script_args: Vec<String>,

    /// Timeout for the process (e.g. 10s, 1m)
    #[arg(long, value_parser = parse_timeout_clap)]
    timeout: Option<f64>,

    /// Display name for the process
    #[arg(short, long)]
    name: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a new process
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        script_args: Vec<String>,
        /// Do not open the TUI after launching
        #[arg(long)]
        no_tui: bool,
        /// Display name for the process
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List all tracked processes
    List,
    /// Stop a running process by PID
    Stop { pid: String },
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
}

fn parse_timeout_clap(s: &str) -> Result<f64, String> {
    parse_timeout(s).ok_or_else(|| format!("Invalid timeout format: {}", s))
}

fn main() {
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let state_file = env::var("KSAI_PROC_LOG_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe_dir.join("logs/runningscripts.json"));
    let log_dir = exe_dir.join("logs");

    fs::create_dir_all(&log_dir).ok();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { script_args, no_tui, name }) => {
            if !script_args.is_empty() {
                if let Err(e) = launch_process_with_name(&exe_dir, &state_file, &log_dir, &script_args, cli.timeout, name, None) {
                    eprintln!("Error: {}", e);
                    return;
                }
                println!("Process launched successfully.");
                if no_tui {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            
            revive_dead_processes(&exe_dir, &state_file, &log_dir);
            if let Err(e) = run_tui(&state_file, &log_dir) {
                eprintln!("TUI error: {}", e);
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
                max_status = max_status.max(proc.status.len());
                max_started = max_started.max(started.len());
                max_dir = max_dir.max(proc.working_dir.len());
                max_cmd = max_cmd.max(proc.cmd_str.len());

                (pid, proc, started)
            }).collect();

            // Add some padding
            max_pid += 2;
            max_status += 2;
            max_started += 2;
            max_dir += 2;
            // max_cmd is last, doesn't need strict padding limit if it's last, 
            // but if we put Directory before Command, Command is last.
            
            // Header
            println!(
                "{:<w_pid$} {:<w_status$} {:<w_started$} {:<w_dir$} {:<w_cmd$}", 
                "PID", "Status", "Started", "Directory", "Command",
                w_pid = max_pid,
                w_status = max_status,
                w_started = max_started,
                w_dir = max_dir,
                w_cmd = max_cmd
            );
            
            // Separator
            let total_width = max_pid + max_status + max_started + max_dir + max_cmd;
            println!("{}", "-".repeat(total_width + 5)); // +5 for extra safety margin

            // Rows
            for (pid, proc, started) in rows {
                println!(
                    "{:<w_pid$} {:<w_status$} {:<w_started$} {:<w_dir$} {:<w_cmd$}", 
                    pid, proc.status, started, proc.working_dir, proc.cmd_str,
                    w_pid = max_pid,
                    w_status = max_status,
                    w_started = max_started,
                    w_dir = max_dir,
                    w_cmd = max_cmd
                );
            }
        }
        Some(Commands::Stop { pid }) => {
            let mut state = read_state(&state_file);
            if let Some(proc) = state.get_mut(&pid) {
                if proc.status == "running" {
                    let pid_val: i32 = pid.parse().unwrap_or(0);
                    unsafe { libc::kill(-pid_val, libc::SIGKILL); }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    proc.status = "killed (manual)".to_string();
                    write_state(&state_file, &state);
                    println!("Process {} stopped.", pid);
                } else {
                    println!("Process {} is not running (status: {}).", pid, proc.status);
                }
            } else {
                println!("Process {} not found.", pid);
            }
        }
        Some(Commands::Remove { pid }) => {
            let mut state = read_state(&state_file);
            if let Some(proc) = state.remove(&pid) {
                if proc.status == "running" {
                    let pid_val: i32 = pid.parse().unwrap_or(0);
                    unsafe { libc::kill(-pid_val, libc::SIGKILL); }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                fs::remove_file(&proc.log_file).ok();
                write_state(&state_file, &state);
                println!("Process {} removed and logs deleted.", pid);
            } else {
                println!("Process {} not found.", pid);
            }
        }
        Some(Commands::Restart { pid }) => {
            let state = read_state(&state_file);
            if let Some(proc) = state.get(&pid).cloned() {
                let old_pid: i32 = pid.parse().unwrap_or(0);
                if proc.status == "running" {
                    unsafe { libc::kill(-old_pid, libc::SIGKILL); }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                
                let mut state = read_state(&state_file);
                state.remove(&pid);
                write_state(&state_file, &state);

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
                println!("Process {} not found.", pid);
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
            let mut state = read_state(&state_file);
            let before = state.len();
            state.retain(|_, proc| proc.status == "running");
            write_state(&state_file, &state);
            println!("Pruned {} non-running processes.", before - state.len());
        }
        Some(Commands::Revive) => {
            println!("Reviving crashed processes...");
            revive_dead_processes(&exe_dir, &state_file, &log_dir);
            println!("Revival check complete.");
        }
        None => {
            if !cli.script_args.is_empty() {
                if let Err(e) = launch_process_with_name(&exe_dir, &state_file, &log_dir, &cli.script_args, cli.timeout, cli.name, None) {
                    eprintln!("Error: {}", e);
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }

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