import pytest
import time
import json
import os
from datetime import datetime, timedelta

def test_schedule_immediate(test_env):
    """Test immediate scheduling."""
    # Implicitly start scheduler
    test_env["run"](["list"])
    
    test_env["run"](["schedule", "add", "--name", "imm_test", "--every", "10s", "--start-at", "now", "/bin/sleep", "5"])
    time.sleep(3)
    
    # Check if running
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        found = any("S:imm_test" in p["display_name"] for p in state.values())
        assert found

def test_schedule_future(test_env):
    """Test future scheduling."""
    test_env["run"](["list"])
    
    future_time = (datetime.now() + timedelta(seconds=5)).strftime("%Y-%m-%d %H:%M:%S")
    test_env["run"](["schedule", "add", "--name", "future_test", "--every", "1m", "--start-at", future_time, "/bin/sleep", "100"])
    
    time.sleep(2)
    # Should NOT be running
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        found = any("S:future_test" in p["display_name"] for p in state.values())
        assert not found
        
    time.sleep(10) # 5s + buffer + scheduler poll
    
    # Trigger reaper and scheduler revival check
    test_env["run"](["list"])
    
    # Should be running
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        found = any("S:future_test" in p["display_name"] and p["status"] == "running" for p in state.values())
        assert found

def test_schedule_revival(test_env):
    """Test revival of crashed scheduled process."""
    test_env["run"](["list"])
    test_env["run"](["schedule", "add", "--name", "crash_test", "--every", "5s", "--start-at", "now", "/bin/sleep", "100"])
    time.sleep(3)
    
    # Kill it
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        pid = [k for k, v in state.items() if "S:crash_test" in v["display_name"]][0]
    
    os.kill(int(pid), 9)
    
    # Wait for frequency window (5s) + buffer + scheduler poll
    time.sleep(8)
    
    # Trigger reaper and scheduler revival check multiple times if needed
    test_env["run"](["list"])
    time.sleep(2)
    test_env["run"](["list"]) # Second trigger to ensure revival registered
    
    # Verify new PID
    with open(test_env["state_file"], 'r') as f:
        state = json.load(f)
        new_pid = [k for k, v in state.items() if "S:crash_test" in v["display_name"] and v["status"] == "running"][0]
        assert new_pid != pid

def test_schedule_management(test_env):
    """Test list, stop, remove schedule commands."""
    test_env["run"](["schedule", "add", "--name", "mgmt_test", "--every", "1h", "/bin/sleep", "10"])
    
    # List
    res = test_env["run"](["schedule", "list"])
    assert "mgmt_test" in res.stdout
    assert "true" in res.stdout # Enabled
    
    # Stop (Disable)
    test_env["run"](["schedule", "stop", "mgmt_test"])
    res = test_env["run"](["schedule", "list"])
    assert "false" in res.stdout # Disabled
    
    # Remove
    test_env["run"](["schedule", "remove", "mgmt_test"])
    res = test_env["run"](["schedule", "list"])
    assert "mgmt_test" not in res.stdout
