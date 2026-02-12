# Process Lifecycle & States

`ksai_proc` tracks the state of each managed process. This document describes the various states a process can be in and how transitions occur.

## Process States

*   **`running`**: The process is actively executing and has a valid PID in the system.
*   **`killed (manual)`**: The process was explicitly stopped by a user command (`stop`, or `x` in TUI).
*   **`killed (timeout)`**: The process exceeded its configured time limit and was terminated by the monitor.
*   **`completed`**: The process exited on its own (successfully or with an error code), and was detected as no longer running by the monitor.

## State Management

The state is stored in `state.rs` within a `HashMap<String, ProcessInfo>`.
*   **Key**: PID (as string).
*   **Value**: `ProcessInfo` struct containing command string, log path, status, start time, etc.

When `ksai_proc` starts (or the TUI loop runs), it calls `reap_processes()`. This function iterates through all known processes in the state file:
1.  Check if PID exists in `/proc`.
2.  If **running** but PID not found -> Mark as **completed**.
3.  If **running** and timeout exceeded -> Kill process -> Mark as **killed (timeout)**.

Visualized in [Lifecycle State Machine](lifecycle_state_machine.mmd).
