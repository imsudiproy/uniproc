//! Process sampling backed by `sysinfo`.
//!
//! Values are kept in bytes internally; presentation code is responsible for
//! formatting them. This avoids the unit mismatch that existed in the first
//! implementation.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Networks, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub timestamp_ms: u64,
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub system_memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    /// Bytes read since the preceding process refresh (platform dependent).
    pub disk_read_bytes: u64,
    /// Bytes written since the preceding process refresh (platform dependent).
    pub disk_written_bytes: u64,
    /// System-wide network bytes received since the preceding refresh.
    pub network_received_bytes: u64,
    /// System-wide network bytes transmitted since the preceding refresh.
    pub network_transmitted_bytes: u64,
}

pub struct ProcessSampler {
    pid: Pid,
    system: System,
    networks: Networks,
}

impl ProcessSampler {
    pub fn new(pid: u32) -> Result<Self, String> {
        let pid = Pid::from_u32(pid);
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::everything(),
        );
        if system.process(pid).is_none() {
            return Err(format!("process with PID {pid} was not found"));
        }
        Ok(Self {
            pid,
            system,
            networks: Networks::new_with_refreshed_list(),
        })
    }

    pub fn sample(&mut self) -> Option<ProcessInfo> {
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::everything(),
        );
        let process = self.system.process(self.pid)?;
        let disk = process.disk_usage();
        Some(ProcessInfo {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_millis() as u64,
            pid: self.pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_percent: process.cpu_usage(),
            memory_bytes: process.memory(),
            system_memory_bytes: self.system.total_memory(),
            virtual_memory_bytes: process.virtual_memory(),
            disk_read_bytes: disk.read_bytes,
            disk_written_bytes: disk.written_bytes,
            network_received_bytes: self
                .networks
                .values()
                .map(|network| network.received())
                .sum(),
            network_transmitted_bytes: self
                .networks
                .values()
                .map(|network| network.transmitted())
                .sum(),
        })
    }
}

/// Returns all exact process-name matches. Callers intentionally decide how to
/// handle ambiguity instead of silently monitoring an arbitrary process.
pub fn find_processes_by_name(name: &str) -> Vec<(u32, String)> {
    let system = System::new_all();
    system
        .processes_by_exact_name(name.as_ref())
        .map(|process| {
            (
                process.pid().as_u32(),
                process.name().to_string_lossy().into_owned(),
            )
        })
        .collect()
}

/// Compatibility helper retained for consumers of the original public API.
pub fn get_process_info(pid: u32, system: &mut System) -> Option<(f32, u64, sysinfo::DiskUsage)> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        ProcessRefreshKind::everything(),
    );
    let process = system.process(Pid::from_u32(pid))?;
    Some((process.cpu_usage(), process.memory(), process.disk_usage()))
}
