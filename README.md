<h1 align="center">⚙️ ksai_proc</h1>
<p align="center"><i>A Persistent Process Scheduler and Manager for Linux & macOS</i></p>
<p align="center"><b>Platform Support:</b> Linux ✅ | macOS ✅ | Windows ⚠️ (WSL/Experimental)</p>

<p align="center">
  <img src="https://img.shields.io/badge/build-stable-green?style=flat-square&logo=github" />
  <img src="https://img.shields.io/badge/bugs-features%20welcomed-informational?style=flat-square&logo=visualstudiocode" />
  <img src="https://img.shields.io/badge/api-rust-orange?style=flat-square&logo=rust" />
  <img src="https://img.shields.io/badge/docs-wiki%20hunt-yellow?style=flat-square&logo=readthedocs" />
  <img src="https://img.shields.io/badge/memory-rust--safe-blue?style=flat-square&logo=rust" />
  <img src="https://img.shields.io/badge/performance-lightweight-lightgrey?style=flat-square&logo=speedtest" />
</p>
---

## 🛠️ Building ksai_proc from Source

Follow these steps to build the `ksai_proc` binary. If it breaks, it’s probably your compiler (or Rust's).

### 🔧 Step 1: Install Rust

If you don't have Rust, install it via [rustup.rs](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

### 🌀 Step 2: Clone the Repo

```bash
git clone https://github.com/theanimatorspal/ksai_proc.git
cd ksai_proc
```

---

### 🧱 Step 3: Build with Cargo

#### 🐧 Linux / 🍎 macOS:

```bash
cargo build --release
```

#### 🧩 Final Setup

The binary will be located at `target/release/ksai_proc`. To run it globally:

1. Add the path to your `PATH` environment variable.
2. OR move it to a standard bin directory:
```bash
sudo mv target/release/ksai_proc /usr/local/bin/
```

---

## 🚀 Key Features

### 📡 Interactive TUI
Monitor all your processes in real-time with a sleek Terminal User Interface. Control your background jobs with simple keybindings. Launch the TUI by running `ksai_proc` with no arguments.

### 🔄 Auto-Revival
Never worry about a process crashing again. `ksai_proc` monitors your background jobs and automatically revives them if they die unexpectedly or if the system reboots.

### 📅 Persistent Scheduler (Cron-like)
Schedule scripts to run at specific intervals or times. The scheduler is persistent, resilient to system restarts, and handles job revival automatically.

```bash
ksai_proc schedule add --every "1h" --name "backup" "/bin/bash backup.sh"
```

### 📋 CLI Management
Simple, powerful commands to manage your process lifecycle:
- **Run**: `ksai_proc run --name "my-app" -- python3 app.py`
- **Stop**: `ksai_proc stop --name "my-app"`
- **Restart**: `ksai_proc restart --name "my-app"`
- **Logs**: `ksai_proc logs --name "my-app"`

---

## 🔧 ksai_proc in Action

### 🔬 Process Monitoring
Dedicated log management and status tracking for all your background scripts.

### 🎯 High-Reliability Tasks
Perfect for web scrapers, bots, and long-running simulations that need to stay alive 24/7.

### 📈 Scheduled Maintenance
Automate backups and cleanup tasks without the complexity of traditional crontabs.

---

## 📖 Documentation

Detailed documentation is available in the [`docs/`](docs/README.md) folder:
- [Architecture & Design](docs/architecture/overview.md)
- [CLI Reference](docs/cli/commands.md)
- [Process Lifecycle](docs/process_management/lifecycle.md)
- [Auto-Revival Mechanics](docs/process_management/revival.md)

---

> 🧠 **Want to contribute?**  
Explore the source code, fork the repo, and help us make process management even easier!
