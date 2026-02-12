# CLI Commands Reference

`ksai_proc` is primarily controlled via its command-line interface.

## Usage

```bash
ksai_proc [OPTIONS] [COMMAND]
```

## Options

*   `--timeout <TIMEOUT>`: Sets a timeout for the launched process (e.g., `10s`, `1m`). Process will be killed after this duration.
*   `--name <NAME>`: Assigns a custom display name to the process.
*   `--[no-]tui`: Only applicable to the `run` command. If present, suppresses the launching of the TUI after starting the process.

## Commands

### `run` (Default)
Launches a new process.

*   **Syntax**: `ksai_proc run [OPTIONS] -- <COMMAND> [ARGS...]`
*   **Example**: `ksai_proc run --name "Server" -- python3 server.py --port 8080`
*   **Notes**:
    *   The `--` separator is recommended to distinguish `ksai_proc` flags from the command's flags.
    *   Process names must be unique among running processes. Attempting to launch a process with a duplicate name will fail.

### `list`
Lists all currently tracked processes.

*   **Syntax**: `ksai_proc list`
*   **Output**: A formatted table showing PID, Status, Start Time, Working Directory, and Command.

### `stop`
Stops a running process by sending SIGKILL. You can specify either the PID or the unique process Name.

*   **Syntax**:
    *   `ksai_proc stop <PID>`
    *   `ksai_proc stop --name <NAME>`
*   **Examples**:
    *   `ksai_proc stop 12345`
    *   `ksai_proc stop --name "my-server"`

### `remove`
Removes a process from the tracking list. If the process is running, it will be stopped first. Also deletes the associated log file.

*   **Syntax**: `ksai_proc remove <PID>`
*   **Example**: `ksai_proc remove 12345`

### `restart`
Restarts a process by killing the old instance (if running) and spawning a new one with the same command and configuration.

*   **Syntax**: `ksai_proc restart <PID>`
*   **Example**: `ksai_proc restart 12345`

### `logs`
Views the logs (stdout/stderr) for a specific process.

*   **Syntax**: `ksai_proc logs <PID> [--lines <N>] [--follow]`
*   **Options**:
    *   `--lines <N>`: Number of lines to show (default: 20).
    *   `--follow`: Follow log output (like `tail -f`).

### `prune`
Removes all non-running (stopped, killed, completed) processes from the tracking list.

*   **Syntax**: `ksai_proc prune`

### `revive`
Checks for any processes that are marked as "running" in the state file but are not actually running in the system (e.g., due to a crash or reboot), and restarts them.

*   **Syntax**: `ksai_proc revive`
