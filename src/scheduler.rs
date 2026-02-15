use std::{path::Path, thread, time::Duration};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::state::{read_scheduled_jobs, read_state, write_scheduled_jobs};
use crate::process::launch_process_with_name;


pub fn start_scheduler_daemon(state_file: &Path, scheduled_file: &Path, log_dir: &Path) {
    let script_dir = log_dir.parent().unwrap().to_path_buf(); // Assuming log_dir is inside the base dir
    
    println!("Scheduler daemon started. Monitoring scheduled: {}", scheduled_file.display());
    println!("Scheduler daemon using state file: {}", state_file.display());
    println!("Scheduler daemon using log dir: {}", log_dir.display());

    loop {
        // Run loop every 1 second
        thread::sleep(Duration::from_secs(1));
        
        // Reap zombies (children that exited)
        unsafe {
            let mut status = 0;
            loop {
                let pid = libc::waitpid(-1, &mut status, libc::WNOHANG);
                if pid > 0 {
                    println!("Scheduler: Reaped zombie child PID: {}, status: {}", pid, status);
                } else {
                    break;
                }
            }
        }

        
        // Clean up dead processes first so we have accurate state
        use crate::state::reap_processes;
        let _ = reap_processes(state_file);

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut scheduled_jobs = read_scheduled_jobs(scheduled_file);
        let mut changed = false;

        let running_processes = read_state(state_file);

        for job in scheduled_jobs.iter_mut() {
            if !job.enabled {
                continue;
            }

            // Check if it's time to run
            // Logic:
            // 1. Must be past start_at
            // 2. If it is a "service" (implied by "continuous" description), we ensure it IS running if now > start_at.
            //    But user also said "frequency like 1m".
            //    User clarification: "if the service not running then it will rerun it, but if the service is already running then it will do nothing"
            //    This sounds like: Check every `frequency`.
            
            // Refined Logic based on user feedback:
            // "every minute schedular; if the service not running then it will rerun it"
            
            // So, check if (now >= start_at).
            
            if now < job.start_at {
                continue;
            }

            // Parse frequency
            let freq_secs = parse_frequency(&job.frequency).unwrap_or(60); // Default 1m

            // We should check this job every `freq_secs`.
            // Ideally, we want to align with the frequency windows, but simpler is:
            // If last_run + freq <= now, then we perform the check.
            
            let should_check = match job.last_run {
                Some(last) => now >= last + freq_secs,
                None => true, // Check immediately if never run (and past start_at)
            };

            if should_check {
                // Perform the check: Is it running?
                // We construct the "display name" or identify the process.
                // The user said: "S:<name>"
                
                let target_name = format!("S:{}", job.name);
                
                let is_running = running_processes.values().any(|p| p.display_name == target_name && p.status == "running");

                if is_running {
                    // Already running, do nothing (maybe log verbose?)
                    // println!("Job '{}' is already running.", job.name);
                } else {
                    // Not running, start it!
                    println!("Scheduler: Starting job '{}'...", job.name);
                    
                    // We need to construct script_args from command + args
                    let mut script_args = vec![job.command.clone()];
                    script_args.extend(job.args.clone());

                    // Launch
                    match launch_process_with_name(
                        &script_dir,
                        state_file,
                        log_dir,
                        &script_args,
                        None, // Timeout? User didn't specify. Assuming None.
                        Some(target_name.clone()),
                        Some(job.working_dir.clone()),
                    ) {
                        Ok(_) => println!("Scheduler: Successfully started '{}'.", target_name),
                        Err(e) => eprintln!("Scheduler: Failed to start '{}': {}", target_name, e),
                    }
                }

                // Update last_run regardless of whether we started it or it was already running,
                // because we "checked" and ensured it was running.
                // OR should we only update if we actually ran it?
                // User said: "every 1 minute... it should run the process... frequency 1m"
                // If it's a long running process, "run" might mean "ensure running".
                // If I update last_run, I won't check again for 1m.
                // If it crashes 10s later, it stays dead for 50s.
                // This seems consistent with "every 1 minute... check".
                
                job.last_run = Some(now);
                changed = true;
            }
        }

        if changed {
            write_scheduled_jobs(scheduled_file, &scheduled_jobs);
        }
    }
}

fn parse_frequency(freq: &str) -> Option<u64> {
    let len = freq.len();
    if len < 2 { return None; }
    
    let (num_str, unit) = freq.split_at(len - 1);
    let num: u64 = num_str.parse().ok()?;
    
    match unit {
        "s" => Some(num),
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 86400),
        _ => None,
    }
}
