import os
import sys
import subprocess
import time
import json
import shutil
from datetime import datetime, timedelta

KSAI_PROC_BIN = os.path.abspath("target/debug/ksai_proc")
# Generate a unique directory for this run to avoid conflicts with rogue daemons
import uuid
RUN_ID = str(uuid.uuid4())[:8]
TEST_DIR = os.path.abspath(f"tests/temp_scheduler_test_{RUN_ID}")
LOGS_DIR = os.path.join(TEST_DIR, "logs")
STATE_FILE = os.path.join(LOGS_DIR, "runningscripts.json")
SCHEDULE_FILE = os.path.join(LOGS_DIR, "scheduledscripts.json")

def kill_existing_daemons():
    # Kill any process named "ksai_proc internal-scheduler"
    try:
        subprocess.run(["pkill", "-f", "ksai_proc internal-scheduler"], check=False)
    except:
        pass

def setup_env():
    kill_existing_daemons()
    if os.path.exists(TEST_DIR):
        shutil.rmtree(TEST_DIR)
    os.makedirs(LOGS_DIR)
    
    # Set env var for ksai_proc to use this test dir
    os.environ["KSAI_PROC_LOG_JSON"] = STATE_FILE
    os.environ["KSAI_PROC_SCHEDULE_JSON"] = SCHEDULE_FILE
    os.environ["KSAI_PROC_LOG_DIR"] = LOGS_DIR

    # Clean legacy target logs usage just in case
    target_logs = os.path.join(os.path.dirname(KSAI_PROC_BIN), "logs")
    if os.path.exists(target_logs):
        shutil.rmtree(target_logs)
    os.makedirs(target_logs, exist_ok=True)

def run_ksai(args):
    """Run ksai_proc with given args."""
    cmd = [KSAI_PROC_BIN] + args
    result = subprocess.run(cmd, capture_output=True, text=True, env=os.environ, cwd=TEST_DIR)
    return result

def check_process_running(name):
    """Check if a process with display name or command containing `name` is running."""
    # We can check via `ksai_proc list` or reading the json state file directly.
    # Reading JSON is more robust for tests.
    if not os.path.exists(STATE_FILE):
        return False
    
    try:
        with open(STATE_FILE, 'r') as f:
            state = json.load(f)
            for pid, proc in state.items():
                if name in proc.get('display_name', '') or name in proc.get('cmd_str', ''):
                    if proc['status'] == 'running':
                        return True
    except:
        pass
    return False

def test_schedule_immediate():
    setup_env()
    print("--- Test: Schedule Immediate ---")
    
    # 1. Schedule a job that runs immediately
    res = run_ksai(["schedule", "add", "--name", "job_imm", "--every", "5s", "--start-at", "now", "/bin/sleep", "100"])
    if res.returncode != 0:
        print("Failed to schedule:", res.stderr)
        return False
    
    print("Scheduled 'job_imm'. Waiting for scheduler to pick it up...")
    time.sleep(3) # Wait for scheduler logic (it polls every 1s)

    # 2. Check if running
    assert check_process_running("S:job_imm"), "job_imm should be running"
    print("SUCCESS: 'job_imm' is running.")

    # 3. Stop it
    run_ksai(["schedule", "stop", "job_imm"])
    time.sleep(2)

def test_schedule_future():
    setup_env()
    print("\n--- Test: Schedule Future ---")
    
    # 1. Start scheduler daemon implicitly by running header command
    run_ksai(["list"])
    
    # 2. Schedule for 5 seconds later
    future_time = (datetime.now() + timedelta(seconds=5)).strftime("%Y-%m-%d %H:%M:%S")
    run_ksai(["schedule", "add", "--name", "job_fut", "--every", "10s", "--start-at", future_time, "/bin/sleep", "100"])
    
    print(f"Scheduled 'job_fut' at {future_time}. Should NOT run yet.")
    time.sleep(2)
    
    if check_process_running("S:job_fut"):
        print("FAILURE: 'job_fut' started too early!")
        return False
        
    print("Waiting for start time...")
    time.sleep(5)
    
    assert check_process_running("S:job_fut"), "job_fut should start on time"
    print("SUCCESS: 'job_fut' started on time.")

def test_revival():
    setup_env()
    print("\n--- Test: Persistence/Revival ---")
    
    # 1. Schedule immediate
    run_ksai(["schedule", "add", "--name", "job_rev", "--every", "5s", "/bin/sleep", "100"])
    time.sleep(3)
    
    if not check_process_running("S:job_rev"):
        print("FAILURE: Initial start failed.")
        if os.path.exists(STATE_FILE):
             with open(STATE_FILE, 'r') as f:
                 print("State File Content:", f.read())
        
        scheduler_log = os.path.join(LOGS_DIR, "scheduler.log")
        if os.path.exists(scheduler_log):
             with open(scheduler_log, 'r') as f:
                 print("Scheduler Log Content:\n", f.read())
        return False
        
    # 2. Kill the process manually (simulating crash)
    # We need the PID.
    with open(STATE_FILE, 'r') as f:
        state = json.load(f)
        pid_to_kill = None
        for pid, proc in state.items():
            if "S:job_rev" in proc.get('display_name', ''):
                pid_to_kill = pid
                break
    
    if pid_to_kill:
        print(f"Killing PID {pid_to_kill}...")
        import signal
        try:
            os.kill(int(pid_to_kill), signal.SIGKILL)
        except:
            print("Failed to kill process?")
    
    # 3. Wait. The scheduler loop (every 1s) should detect it's not running and restart it
    # because frequency is 5s.
    # Logic: "if the service not running then it will rerun it"
    # Logic check: `should_check` is true if `now >= last_run + freq`.
    # BUT, if I just started it 3s ago, `last_run` was 3s ago. Freq is 5s.
    # It will NOT restart it immediately. It will wait until 5s window passes.
    
    print("Waiting for frequency window (5s)...")
    time.sleep(7)
    
    if check_process_running("S:job_rev"):
        # Verify it's a NEW pid
        with open(STATE_FILE, 'r') as f:
            state = json.load(f)
            new_pid = None
            for pid, proc in state.items():
                if "S:job_rev" in proc.get('display_name', ''):
                    new_pid = pid
                    break
        if new_pid != pid_to_kill:
            print(f"SUCCESS: revived with new PID {new_pid}.")
        else:
            print("FAILURE: PID matches old PID? That's impossible if killed.")
            if os.path.exists(STATE_FILE):
                 with open(STATE_FILE, 'r') as f:
                     print("State File Content:", f.read())
            
            scheduler_log = os.path.join(LOGS_DIR, "scheduler.log")
            if os.path.exists(scheduler_log):
                 with open(scheduler_log, 'r') as f:
                     print("Scheduler Log Content:\n", f.read())
            return False
    else:
        print("FAILURE: Did not revive.")
        if os.path.exists(STATE_FILE):
             with open(STATE_FILE, 'r') as f:
                 print("State File Content:", f.read())
        
        scheduler_log = os.path.join(LOGS_DIR, "scheduler.log")
        if os.path.exists(scheduler_log):
             with open(scheduler_log, 'r') as f:
                 print("Scheduler Log Content:\n", f.read())
        return False

    return True

if __name__ == "__main__":
    try:
        if test_schedule_immediate() and test_schedule_future() and test_revival():
            print("\nALL TESTS PASSED")
            sys.exit(0)
        else:
            print("\nSOME TESTS FAILED")
            sys.exit(1)
    except Exception as e:
        print(f"An error occurred: {e}")
        import traceback
        traceback.print_exc()
