mod state;
mod process;
mod ui;
mod types;

use std::{env, fs, path::PathBuf, time::Duration};
use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, Terminal};
use crate::{process::*, state::*, ui::*, types::*};

fn main() {
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let state_file = env::var("KSAI_PROC_LOG_JSON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| exe_dir.join("logs/runningscripts.json"));
    let log_dir = exe_dir.join("logs");

    fs::create_dir_all(&log_dir).ok();

    let args: Vec<String> = env::args().collect();
    let mut timeout_sec = None;
    let mut script_args = Vec::new();
    let mut i = 1;

    while i < args.len() {
        if args[i] == "--for" && i + 1 < args.len() {
            timeout_sec = parse_timeout(&args[i + 1]);
            i += 2;
        } else {
            script_args = args[i..].to_vec();
            break;
        }
    }

    if !script_args.is_empty() {
        if let Err(e) = launch_process(&exe_dir, &state_file, &log_dir, &script_args, timeout_sec) {
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