import pytest
import os
import time
import json
import signal

def test_scheduler_revival_on_cli(test_env):
    """Test that the scheduler daemon is revived when a CLI command is run."""
    # 1. Start scheduler implicitly
    test_env["run"](["schedule", "list"])
    
    # 2. Get scheduler PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        scheduler_pid = None
        for pid, proc in state.items():
            if "ksai_scheduler_daemon" in proc["display_name"]:
                scheduler_pid = pid
                break
    
    assert scheduler_pid, "Scheduler should be running"
    
    # 3. Schedule a job to verify it continues working later
    test_env["run"](["schedule", "add", "--name", "persist_job", "--every", "10s", "/bin/sleep", "100"])
    
    # 4. Kill scheduler daemon
    os.kill(int(scheduler_pid), 9)
    
    # Wait a moment
    time.sleep(1)
    
    # 5. Run a CLI command (e.g., list)
    test_env["run"](["list"])
    
    # 6. Give the revived scheduler a moment to register itself in the state file
    time.sleep(2)
    
    # 7. Verify scheduler revived with NEW PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        new_scheduler_pid = None
        for pid, proc in state.items():
            if "ksai_scheduler_daemon" in proc["display_name"] and proc["status"] == "running":
                new_scheduler_pid = pid
                break
                
    assert new_scheduler_pid, "Scheduler should have revived"
    assert new_scheduler_pid != scheduler_pid, "Scheduler should have a new PID"

    # 7. Check if old PID entry is gone/marked killed?
    # Actually, `ksai_proc list` (triggered above) calls `ensure_scheduler_running`
    # AND `reap_processes`. So `reap_processes` should have marked the old scheduler as dead/completed.
    # The new scheduler instance is a separate process.
    
    # Optional: Verify scheduled job is still being monitored.
    # We can check if the new scheduler picked up the job state.
    # The job state is in `scheduledscripts.json`, which persists.
    # The scheduler reads it on startup.
    # So `persist_job` should eventually be run.
    time.sleep(1) # Scheduler startup
    res = test_env["run"](["schedule", "list"])
    assert "persist_job" in res.stdout
