# Auto-Revival

One of the key features of `ksai_proc` is its ability to "revive" processes that have crashed or were terminated unexpectedly (e.g., due to a system reboot).

## How it Works

The logic resides in `process.rs::revive_dead_processes()`.

1.  **Detection**:
    *   It reads the state file.
    *   It filters for processes that are marked as **running** in the JSON file.
    *   It checks against the OS process table (`sysinfo`).
    *   If a process is marked **running** but its PID **does not exist**, it is considered "dead" or "crashed".

2.  **Action**:
    *   The dead process entry is removed from the state file.
    *   A new process is spawned using the *exact same command, arguments, and working directory* as recorded in the old entry.
    *   The old log file is appended with a revival message: `b"--- 🔄 AUTO-REVIVED (was PID X) ---"`.
    *   A new entry is written to the state file with the new PID and the same metadata.

## Triggering Revival

Revival is triggered implicitly whenever `ksai_proc` runs any command or starts the TUI.
*   Running `ksai_proc list` will check and revive processes before showing the list.
*   Running `ksai_proc run ...` will launch the new process and *then* check/revive others.
*   Running `ksai_proc revive` explicitly triggers this check.

## Limitations

*   If a process crashes immediately upon start (e.g., syntax error), `ksai_proc` might attempt to revive it in a loop if the check frequency is high enough, though `ksai_proc` itself is not a daemon, so this loop only happens when you interact with the tool.
*   It does not differentiate between a crash and a `kill -9` from an external source (other than `ksai_proc stop`).

Visualized in [Revival Sequence Diagram](revival_sequence.mmd).
