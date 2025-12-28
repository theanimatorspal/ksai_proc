use std::{collections::VecDeque, env, fs::{self, File, OpenOptions}, io::{self, BufRead, Write}, os::unix::process::CommandExt, path::Path, process::{Command, Stdio}, time::Duration};
use chrono::Local;
use crossterm::{event::{self, Event, KeyCode}, execute, terminal::{disable_raw_mode, LeaveAlternateScreen}};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use crate::{process::*, state::*, types::*};

pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    state_file: &Path,
    log_dir: &Path,
) -> io::Result<()> {
    loop {
        if app.last_reap.elapsed().unwrap_or_default() > Duration::from_millis(500) {
            app.last_reap = std::time::SystemTime::now();
            app.processes = reap_processes(state_file);
            if app.selected_index >= app.processes.len() && !app.processes.is_empty() {
                app.selected_index = app.processes.len() - 1;
            }
        }

        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if !app.error_message.is_empty() {
                    app.error_message.clear();
                }

                if app.name_input_mode {
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(pending) = app.pending_launch.take() {
                                let name = if app.input_buffer.is_empty() {
                                    None
                                } else {
                                    Some(app.input_buffer.clone())
                                };
                                
                                let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
                                if let Err(e) = launch_process_with_name(&exe_dir, state_file, log_dir, &pending.script_args, pending.timeout_sec, name, Some(pending.working_dir)) {
                                    app.error_message = format!("Launch error: {}", e);
                                }
                                app.processes = reap_processes(state_file);
                            }
                            app.name_input_mode = false;
                            app.input_buffer.clear();
                            app.mode = Mode::Navigate;
                        }
                        KeyCode::Esc => {
                            app.name_input_mode = false;
                            app.input_buffer.clear();
                            app.pending_launch = None;
                            app.mode = Mode::Navigate;
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    }
                    continue;
                }

                match app.mode {
                    Mode::Navigate => {
                        if app.is_paused {
                            if key.code == KeyCode::Char('p') {
                                app.is_paused = false;
                            }
                            continue;
                        }

                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('p') => app.is_paused = true,
                            KeyCode::Char('j') => {
                                if !app.processes.is_empty() {
                                    app.selected_index = (app.selected_index + 1).min(app.processes.len() - 1);
                                }
                            }
                            KeyCode::Char('k') => {
                                if app.selected_index > 0 {
                                    app.selected_index -= 1;
                                }
                            }
                            KeyCode::Char('o') => {
                                app.mode = Mode::Input;
                                app.input_buffer.clear();
                            }
                            KeyCode::Char('x') => {
                                if let Some((pid_str, proc)) = app.processes.get(app.selected_index) {
                                    if proc.status == "running" {
                                        let pid: i32 = pid_str.parse().unwrap_or(0);
                                        unsafe { libc::kill(pid, libc::SIGKILL); }
                                        let mut state = read_state(state_file);
                                        if let Some(p) = state.get_mut(pid_str) {
                                            p.status = "killed (manual)".to_string();
                                        }
                                        write_state(state_file, &state);
                                        app.processes = reap_processes(state_file);
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                if let Some((_, proc)) = app.processes.get(app.selected_index) {
                                    let log_file = proc.log_file.clone();
                                    app.log_readers.remove(&log_file);
                                    app.log_cache.remove(&log_file);
                                    match fs::remove_file(&log_file) {
                                        Ok(_) => app.error_message = format!("Deleted log: {}", Path::new(&log_file).file_name().unwrap().to_string_lossy()),
                                        Err(e) => app.error_message = format!("Error deleting log: {}", e),
                                    }
                                }
                            }
                            KeyCode::Char('R') => {
                                if let Some((pid_str, proc)) = app.processes.get(app.selected_index).cloned() {
                                    let old_pid: i32 = pid_str.parse().unwrap_or(0);

                                    if proc.status == "running" {
                                        unsafe { libc::kill(old_pid, libc::SIGKILL); }
                                    }

                                    let mut state = read_state(state_file);
                                    state.remove(&pid_str);
                                    write_state(state_file, &state);

                                    if let Ok(mut f) = OpenOptions::new().append(true).open(&proc.log_file) {
                                        writeln!(f, "\n--- 🔄 RESTARTED (PID {}) @ {} ---", old_pid, Local::now()).ok();
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

                                    app.processes = reap_processes(state_file);
                                    app.selected_index = 0;
                                }
                            }
                            KeyCode::Char('X') => {
                                if let Some((pid_str, proc)) = app.processes.get(app.selected_index).cloned() {
                                    let pid: i32 = pid_str.parse().unwrap_or(0);

                                    if proc.status == "running" {
                                        unsafe { libc::kill(pid, libc::SIGKILL); }
                                    }

                                    app.log_readers.remove(&proc.log_file);
                                    app.log_cache.remove(&proc.log_file);
                                    fs::remove_file(&proc.log_file).ok();

                                    let mut state = read_state(state_file);
                                    state.remove(&pid_str);
                                    write_state(state_file, &state);

                                    app.processes = reap_processes(state_file);
                                    if app.selected_index > 0 {
                                        app.selected_index -= 1;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Mode::Input => match key.code {
                        KeyCode::Enter => {
                            if !app.input_buffer.is_empty() {
                                let args: Vec<String> = app.input_buffer.split_whitespace().map(|s| s.to_string()).collect();
                                app.pending_launch = Some(PendingLaunch {
                                    script_args: args,
                                    timeout_sec: None,
                                    working_dir: env::current_dir().unwrap().to_string_lossy().to_string(),
                                });
                                app.name_input_mode = true;
                                app.input_buffer.clear();
                            } else {
                                app.mode = Mode::Navigate;
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Navigate;
                            app.input_buffer.clear();
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
        .split(Rect {
            x: size.x,
            y: size.y,
            width: size.width,
            height: size.height.saturating_sub(2),
        });

    let items: Vec<ListItem> = app
        .processes
        .iter()
        .enumerate()
        .map(|(i, (_, proc))| {
            let status_color = match proc.status.as_str() {
                "running" => Color::Green,
                s if s.starts_with("killed") || s.starts_with("stopped") => Color::Yellow,
                _ => Color::Red,
            };

            let display_name = format!("[{}] {}", proc.status.to_uppercase(), proc.display_name);

            let style = if i == app.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(status_color)
            };

            ListItem::new(display_name).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Processes (j/k) "));
    f.render_widget(list, chunks[0]);

    if app.selected_index < app.processes.len() {
        let (_, proc) = &app.processes[app.selected_index];
        let log_file = proc.log_file.clone();

        if !app.log_readers.contains_key(&log_file) {
            if let Ok(file) = File::open(&log_file) {
                app.log_readers.insert(log_file.clone(), io::BufReader::new(file));
                app.log_cache.insert(log_file.clone(), VecDeque::with_capacity(1000));
            }
        }

        if let Some(reader) = app.log_readers.get_mut(&log_file) {
            let cache = app.log_cache.entry(log_file.clone()).or_insert_with(|| VecDeque::with_capacity(1000));

            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                cache.push_back(line.trim_end().to_string());
                if cache.len() > 1000 {
                    cache.pop_front();
                }
                line.clear();
            }

            let display_height = (chunks[1].height.saturating_sub(2)) as usize;
            let lines: Vec<Line> = cache
                .iter()
                .rev()
                .take(display_height)
                .rev()
                .map(|l| Line::from(l.clone()))
                .collect();

            let title = format!(" STDOUT Log (tail -f) | CWD: {} ", proc.working_dir);
            let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(paragraph, chunks[1]);
        }
    }

    let footer_area = Rect {
        x: size.x,
        y: size.height.saturating_sub(2),
        width: size.width,
        height: 2,
    };

    let footer_text = if app.name_input_mode {
        format!("Enter process name (or press Enter for default): {}", app.input_buffer)
    } else if app.mode == Mode::Input {
        format!("cmd: {}", app.input_buffer)
    } else if app.is_paused {
        "--- PAUSED (Press 'p' to resume) ---".to_string()
    } else if !app.error_message.is_empty() {
        app.error_message.clone()
    } else {
        "[o]pen cmd | [x]kill | [X]remove | [c]lear logs | [j/k]navigate | [q]uit | [R]restart | [p]pause".to_string()
    };

    let footer_style = if !app.error_message.is_empty() {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::White).fg(Color::Black)
    };

    let footer = Paragraph::new(footer_text).style(footer_style);
    f.render_widget(footer, footer_area);
}