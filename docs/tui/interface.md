# Terminal User Interface (TUI)

The TUI provides an interactive way to manage processes without endlessly typing CLI commands.

## Launching the TUI

You can launch the TUI by running `ksai_proc` without arguments, or by running `ksai_proc run ...` (unless `--no-tui` is specified).

## Interface Layout

The screen is divided into three main sections:

1.  **Process List (Left)**:
    *   Shows a list of all tracked processes.
    *   Columns: `[STATUS] DisplayName`.
    *   Status Colors:
        *   Green: Running
        *   Yellow: Killed/Stopped
        *   Red: Error/Unknown

2.  **Details Pane (Right)**:
    *   **Default View**: Shows the live stdout/stderr logs of the selected process (`tail -f`).
    *   **Resource View (`s`)**: Shows a table with CPU%, RAM usage, Disk I/O, and Thread count for all running processes.

3.  **Footer (Bottom)**:
    *   Displays current mode, input prompt, or keybinding hints.

## Keybindings

### Navigation
*   `j` / `k`: Move selection up/down in the process list.
*   `q`: Quit the TUI (processes keep running in the background).

### Process Control
*   `o`: **Open/Run** a new process. Enter command at the prompt.
*   `x`: **Kill** the selected process (sends SIGKILL).
*   `X` (Shift+x): **Remove** the selected process from the list (stops it first if running).
*   `R` (Shift+r): **Restart** the selected process (kills old instance, spawns new one).
*   `c`: **Clear** logs for the selected process (deletes the log file).

### View Control
*   `s`: **Swap** view between "Logs" and "Resources".
*   `p`: **Pause** the TUI updates (useful for reading fast-scrolling logs).

Visualized in [Event Loop Diagram](event_loop.mmd).
