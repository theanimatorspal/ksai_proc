import pytest
import os
import shutil
import uuid
import subprocess
import time

# Helper to find the binary
KSAI_PROC_BIN = os.path.abspath("target/debug/ksai_proc")

@pytest.fixture(scope="function")
def test_env(request):
    """
    Sets up a temporary directory and environment variables for a test function.
    Cleans up after the test.
    """
    # Create unique temp dir
    run_id = str(uuid.uuid4())[:8]
    test_dir = os.path.abspath(f"tests/temp_test_{run_id}")
    logs_dir = os.path.join(test_dir, "logs")
    state_file = os.path.join(logs_dir, "runningscripts.json")
    schedule_file = os.path.join(logs_dir, "scheduledscripts.json")
    
    if os.path.exists(test_dir):
        shutil.rmtree(test_dir)
    os.makedirs(logs_dir)
    
    # Set env vars
    env = os.environ.copy()
    env["KSAI_PROC_LOG_JSON"] = state_file
    env["KSAI_PROC_SCHEDULE_JSON"] = schedule_file
    env["KSAI_PROC_LOG_DIR"] = logs_dir
    
    # Build a run helper
    def run_ksai(args):
        cmd = [KSAI_PROC_BIN] + args
        return subprocess.run(cmd, capture_output=True, text=True, env=env, cwd=test_dir)

    yield {
        "run": run_ksai,
        "test_dir": test_dir,
        "logs_dir": logs_dir,
        "state_file": state_file,
        "schedule_file": schedule_file,
        "env": env
    }
    
    # Cleanup
    # Kill any processes started by this test (if possible to track, otherwise basic pkill by pattern if unique)
    # Since we use unique state files, the daemons started *should* be isolated by state file,
    # BUT `ksai_proc` doesn't strictly isolate binaries if they are just `target/debug/ksai_proc`.
    # However, `metrics` or `scheduler` might be running.
    # Best effort cleanup of processes tracked in the state file.
    
    try:
        if os.path.exists(state_file):
             # We can use our own binary to stop them?
             run_ksai(["stop", "--name", "all"]) # Not implemented yet?
             # Or just kill by reading state file
             import json
             with open(state_file, 'r') as f:
                 state = json.load(f)
                 for pid, proc in state.items():
                     try:
                         os.kill(int(pid), 9)
                     except:
                         pass
    except:
        pass

    if os.path.exists(test_dir):
        shutil.rmtree(test_dir)
