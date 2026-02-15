# ⚙️ ksai_proc Documentation

Welcome to the detailed documentation for `ksai_proc`, a lightweight persistent process scheduler and manager written in Rust.

## Overview

`ksai_proc` is designed to manage long-running background processes (scripts, servers, etc.) with a simple terminal user interface. It handles process launching, monitoring, logging, and auto-revival upon crashes or system restarts.

## Table of Contents

### [Architecture](architecture/overview.md)
*   [Overview](architecture/overview.md)
*   [System Context Diagram](architecture/system_context.mmd)
*   [Data Flow Diagram](architecture/data_flow.mmd)

### [CLI Reference](cli/commands.md)
*   [Commands Usage](cli/commands.md)
*   [Common Workflows](cli/workflows.mmd)

### [Process Management](process_management/lifecycle.md)
*   [Lifecycle & States](process_management/lifecycle.md)
*   [State Machine Diagram](process_management/lifecycle_state_machine.mmd)
*   [Auto-Revival](process_management/revival.md)
*   [Revival Sequence Diagram](process_management/revival_sequence.mmd)

### [TUI Guide](tui/interface.md)
*   [Interface & Navigation](tui/interface.md)
*   [Event Loop Diagram](tui/event_loop.mmd)

## Quick Start

1.  **Build**: `cargo build --release`
2.  **Run**: `./target/release/ksai_proc`
3.  **Launch a Process**:
    ```bash
    ksai_proc run --name "my-server" -- python3 server.py
    ```
4.  **Stop a Process**:
    ```bash
    ksai_proc stop --name "my-server"
    ```
5.  **List Processes**:
    ```bash
    ksai_proc list
    ```
