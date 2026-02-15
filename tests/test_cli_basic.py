import pytest
import os
import time
import json
import re

def test_run_basic(test_env):
    """Test running a simple process."""
    res = test_env["run"](["run", "--name", "test_sleep", "--no-tui", "/bin/sleep", "10"])
    assert res.returncode == 0
    assert "Process launched successfully" in res.stdout

    # Verify it shows up in list
    res_list = test_env["run"](["list"])
    assert "test_sleep" in res_list.stdout
    assert "sleep 10" in res_list.stdout

def test_run_with_timeout(test_env):
    """Test running a process with a timeout."""
    # Run a sleep 10 with timeout 2s
    res = test_env["run"](["run", "--name", "timeout_test", "--timeout", "2s", "--no-tui", "/bin/sleep", "10"])
    assert res.returncode == 0
    
    # Wait for registration
    time.sleep(1)
    
    # Check it runs
    res_list = test_env["run"](["list"])
    assert "timeout_test" in res_list.stdout
    
    # Wait for timeout (2s) + buffer
    time.sleep(4)
    
    # It should be killed
    res_list = test_env["run"](["list"])
    # It might still be in the list but status killed/completed
    # We need to parse the list output or check state file directly
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        found = False
        for pid, proc in state.items():
            if proc["display_name"] == "timeout_test":
                found = True
                assert proc["status"] == "killed (timeout)" or proc["status"] == "completed" # Depending on race
                # Actually our logic explicitly sets "killed (timeout)"
        assert found

def test_stop_by_name(test_env):
    """Test stopping a process by name."""
    test_env["run"](["run", "--name", "to_stop", "--no-tui", "/bin/sleep", "100"])
    time.sleep(1)
    
    res = test_env["run"](["stop", "--name", "to_stop"])
    assert "stopped" in res.stdout
    
    # Verify status
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        for pid, proc in state.items():
            if proc["display_name"] == "to_stop":
                assert "killed" in proc["status"]

def test_restart_process(test_env):
    """Test restarting a process."""
    test_env["run"](["run", "--name", "to_restart", "--no-tui", "/bin/sleep", "100"])
    time.sleep(1)
    
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        pid_1 = [k for k, v in state.items() if v["display_name"] == "to_restart"][0]

    res = test_env["run"](["restart", pid_1])
    assert "restarted with new PID" in res.stdout
    
    # Verify new PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        # Should still have 1 entry (or 2 if old one wasn't cleaned immediately? restart removes old)
        assert len(state) >= 1
        # Find the one with display_name
        pids = [p for p, v in state.items() if v["display_name"] == "to_restart"]
        assert len(pids) == 1
        assert pids[0] != pid_1

def test_remove_process(test_env):
    """Test removing a process."""
    test_env["run"](["run", "--name", "to_remove", "--no-tui", "/bin/sleep", "100"])
    time.sleep(1)
    
    # Get PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        pid = [k for k, v in state.items() if v["display_name"] == "to_remove"][0]
        
    res = test_env["run"](["remove", pid])
    assert "removed" in res.stdout
    
    # Verify it's gone
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        # Ensure "to_remove" is not in any display_name
        found = any("to_remove" in v["display_name"] for v in state.values())
        assert not found

def test_logs_creation(test_env):
    """Test that logs are created and retrievable."""
    # python script that prints something
    script = "print('Hello World'); import time; time.sleep(1)"
    test_env["run"](["run", "--name", "log_test", "--no-tui", "python3", "-u", "-c", script])
    time.sleep(1)
    
    # Get PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        pid = [k for k, v in state.items() if v["display_name"] == "log_test"][0]
    
    # Check logs command
    res = test_env["run"](["logs", pid])
    assert "Hello World" in res.stdout or "Hello World" in res.stderr # TUI logs might go to stderr? CLI command usually stdout
    # Actually `tail` goes to stdout.
    
    # Verify file content directly
    log_file = state[pid]["log_file"]
    with open(log_file, 'r') as f:
        content = f.read()
        assert "Hello World" in content
