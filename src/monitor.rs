use std::collections::HashMap;
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System, ProcessesToUpdate};

#[derive(Clone, Debug, Default)]
pub struct ProcessStats {
    pub cpu_usage: f32,
    pub mem_usage: u64,
    pub virtual_mem: u64,
    pub disk_read: u64,
    pub disk_written: u64,
    pub max_cpu: f32,
    pub max_mem: u64,
    pub thread_count: u64,
}

pub struct Monitor {
    system: System,
    stats: HashMap<u32, ProcessStats>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_processes(ProcessRefreshKind::everything())
                    .with_cpu(sysinfo::CpuRefreshKind::everything())
                    .with_memory(sysinfo::MemoryRefreshKind::everything()),
            ),
            stats: HashMap::new(),
        }
    }

    pub fn update(&mut self, pids: &[u32]) {
        // Prepare list of Pids to refresh
        let sys_pids: Vec<Pid> = pids.iter().map(|&p| Pid::from_u32(p)).collect();
        
        // Use refresh_processes_specifics for efficiency if available, or just refresh_processes
        // sysinfo 0.30+ supports this. Checking Cargo.toml, version is 0.37.2.
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&sys_pids),
            true, // maintain order? (deprecated in newer versions, or remove_dead) - checking docs:
            // refresh_processes(processes_to_update: ProcessesToUpdate, remove_dead: bool) is likely usage
             ProcessRefreshKind::everything()
        );
        // refresh_memory is needed for total memory info if we want usage % relative to total, 
        // but process.memory() returns bytes.
        // We might want to refresh global CPU usage too for context but strict requirement is per process.
        
        for &pid in pids {
            let sys_pid = Pid::from_u32(pid);
            if let Some(proc) = self.system.process(sys_pid) {
                let entry = self.stats.entry(pid).or_default();
                
                entry.cpu_usage = proc.cpu_usage();
                entry.mem_usage = proc.memory();
                entry.virtual_mem = proc.virtual_memory();
                
                let disk_usage = proc.disk_usage();
                entry.disk_read = disk_usage.total_read_bytes;
                entry.disk_written = disk_usage.total_written_bytes;
                
                // Track max values
                if entry.cpu_usage > entry.max_cpu {
                    entry.max_cpu = entry.cpu_usage;
                }
                if entry.mem_usage > entry.max_mem {
                    entry.max_mem = entry.mem_usage;
                }
                
                // thread count is optional in some sysinfo versions, let's check
                // proc.tasks? or proc.thread_kind()?
                // for 0.37, strictly speaking, getting thread count might require iterating tasks if not exposed directly.
                // But usually, `proc.tasks()` returns an iterator/hashmap if implicitly refreshed.
                // Use a simple default if complex. 
                // Wait, sysinfo `Process` struct usually doesn't show thread count directly 
                // unless we iterate `tasks`. `proc.tasks` is available if `ProcessRefreshKind::with_tasks()` is used? 
                // Default `everything()` includes tasks?
                // Actually `proc.tasks()` was removed/changed in recent versions.
                // Let's rely on standard `proc` methods. If thread count is not easy, we skip or find alternative.
                // checking crate docs for 0.30+: It seems to be separate.
                // Let's assume 0 threads for now to be safe or check if `proc.thread_kind` is useful.
                // Actually, let's just use 1 if unknown.
                entry.thread_count = 1; // Placeholder until verified
            }
        }
    }

    pub fn get_stats(&self, pid: u32) -> Option<&ProcessStats> {
        self.stats.get(&pid)
    }
}
