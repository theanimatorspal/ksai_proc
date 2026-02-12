# Architecture Overview

`ksai_proc` follows a simple, monolithic architecture designed for reliability and ease of use. It consists of three main logical components that interact with the host system.

## Components

### 1. CLI (Command Line Interface)
*   **Entry Point**: `main.rs`
*   **Responsibility**: Parses user commands (run, list, stop, etc.), invokes process management functions, or launches the TUI.
*   **Interaction**: Direct invocation by the user.

### 2. State Management
*   **Storage**: `state.rs`, `types.rs`
*   **Responsibility**: Persists process information (PID, command, status, start time) to a JSON file (`state_file`).
*   **Interaction**: Read/written by CLI commands and the TUI loop. It acts as the "database" for the application.

### 3. Process Manager
*   **Logic**: `process.rs`, `monitor.rs`
*   **Responsibility**:
    *   Spawns new processes.
    *   Monitors process health (CPU, RAM).
    *   Handles signal sending (SIGKILL).
    *   Manages log files (stdout/stderr redirection).
    *   Performs auto-revival of crashed processes.

### 4. TUI (Terminal User Interface)
*   **Logic**: `ui.rs`, `app.rs`
*   **Responsibility**: Renders the visual interface using `ratatui` and `crossterm`.
*   **Interaction**: Polls the `state_file` to update the display and accepts user input to control processes.

## Key Design Principles

*   **Stateless Execution**: The manager itself (`ksai_proc`) does not need to run as a daemon. It relies on the `state_file` and system process table (`/proc`) to understand the world. This means you can kill `ksai_proc` and the managed processes will keep running.
*   **Persistence**: Process state is saved to disk, allowing `ksai_proc` to "remember" processes even after a system reboot (if configured to run on startup).
*   **Sync vs Async**: The codebase primarily uses synchronous blocking I/O for simplicity, given the low frequency of events. The TUI uses a polling loop.
