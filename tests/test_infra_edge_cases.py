import pytest
import os
import time
import json
import re
import threading
import sys

def test_zombie_handling(test_env):
    """Test standard zombie process handling (external kill)."""
    # Start a process
    test_env["run"](["run", "--name", "zombie_candidate", "--no-tui", "/bin/sleep", "100"])
    time.sleep(1)
    
    # Get PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        pid = [k for k, v in state.items() if v["display_name"] == "zombie_candidate"][0]
        
    # Kill externally (SIGKILL)
    os.kill(int(pid), 9)
    
    # Wait a moment for system to register death
    time.sleep(1)
    
    # Run a CLI command to trigger `reap_processes`
    res = test_env["run"](["list"])
    
    # Wait for file lock and write
    time.sleep(0.5)
    
    # Verify status in state file
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        if pid in state:
            status = state[pid]["status"].lower()
            assert "running" not in status
    
def test_invalid_inputs(test_env):
    """Test invalid input handling."""
    # Invalid timeout
    res = test_env["run"](["run", "--timeout", "invalid", "/bin/ls"])
    assert res.returncode != 0
    assert re.search(r"invalid value 'invalid'|invalid timeout", res.stderr.lower())
    
    # Stop missing name
    res = test_env["run"](["stop", "--name", "nonexistent"])
    assert "Process nonexistent not found" in res.stdout or "not found" in res.stdout
    
    # Schedule invalid date
    res = test_env["run"](["schedule", "add", "--name", "bad_date", "--every", "1m", "--start-at", "bad-date-format", "/bin/ls"])
    assert "Error: Invalid date format" in res.stdout

# Stress test / Lock Contention
# This is tricky with `subprocess.run` overhead but we can try basic concurrency.
def test_lock_contention(test_env):
    """Stress test concurrent commands to verify locking prevents state corruption."""
    
    # Start a base process
    test_env["run"](["run", "--name", "base", "--no-tui", "/bin/sleep", "100"])
    
    def run_command(i):
        # alternate between list, logs, stop (fail), run (short)
        if i % 4 == 0:
            test_env["run"](["list"])
        elif i % 4 == 1:
            test_env["run"](["run", "--name", f"stress_{i}", "--no-tui", "/bin/sleep", "1"])
        elif i % 4 == 2:
            test_env["run"](["stop", "--name", "nonexistent"])
        else:
             # Just read logs of base
             # Need PID? let's just list again
             test_env["run"](["list"])
             
    threads = []
    for i in range(20): # 20 concurrent commands
        t = threading.Thread(target=run_command, args=(i,))
        threads.append(t)
        t.start()
        
    for t in threads:
        t.join()
        
    # Verify state file is valid JSON and not corrupted
    try:
        with open(test_env["state_file"], 'r') as f:
            json.load(f)
    except json.JSONDecodeError:
        pytest.fail("State file corrupted after concurrent access!")
