//! Process sampling backed by `sysinfo`.
//!
//! Values are kept in raw bytes internally (e.g. `memory_bytes`); presentation
//! code is responsible for scaling and formatting them appropriately. This avoids
//! unit mismatch issues and prevents precision loss during data processing.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Networks, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// A single point-in-time snapshot of process and system resource utilization.
/// This struct is safely serializable to formats like JSON and CSV.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    /// Timestamp of when the sample was taken, in milliseconds since the UNIX epoch.
    pub timestamp_ms: u64,
    /// The unique Process ID assigned by the OS.
    pub pid: u32,
    /// The name of the process executable.
    pub name: String,
    /// Executable path for the process, when reported by the operating system.
    pub executable_path: Option<String>,
    /// Number of process tasks/threads, when reported by the operating system.
    pub thread_count: Option<usize>,
    /// Seconds the process has been running since it was launched.
    pub uptime_seconds: u64,
    /// CPU utilization percentage. (e.g. 100.0 means one full core is utilized).
    pub cpu_percent: f32,
    /// Resident Set Size (RSS) memory in bytes used by the process.
    pub memory_bytes: u64,
    /// Total available physical memory on the host system, in bytes.
    pub system_memory_bytes: u64,
    /// Total virtual memory in bytes allocated by the process.
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

/// A stateful sampler that retains OS handles and differential counters
/// to efficiently calculate metrics over time.
pub struct ProcessSampler {
    /// Target Process ID to monitor.
    pid: Pid,
    /// The persistent `sysinfo` System instance.
    system: System,
    /// The persistent `sysinfo` Networks instance for tracking global I/O.
    networks: Networks,
}

impl ProcessSampler {
    /// Initializes a new sampler targeted at a specific Process ID.
    ///
    /// The constructor performs an initial warm-up refresh to verify the process exists
    /// and to establish baseline metrics for differential calculations (like CPU usage).
    ///
    /// # Arguments
    /// * `pid` - The target Process ID.
    pub fn new(pid: u32) -> Result<Self, String> {
        let pid = Pid::from_u32(pid);
        let mut system = System::new();
        // Pre-fetch system memory totals
        system.refresh_memory();
        // Warm up process specific metrics
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

    /// Triggers a refresh of all internal OS counters and returns a calculated snapshot.
    ///
    /// Since metrics like CPU% require computing the delta since the last poll, this
    /// function mutates the internal state of the sampler.
    ///
    /// # Returns
    /// `Some(ProcessInfo)` if the process is still alive, otherwise `None`.
    pub fn sample(&mut self) -> Option<ProcessInfo> {
        // Refresh memory first to get updated system totals
        self.system.refresh_memory();
        // Refresh network counters (used for delta calculations)
        self.networks.refresh(true);
        // Refresh the specific process to get updated CPU and I/O deltas
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
            executable_path: process
                .exe()
                .map(|path| path.to_string_lossy().into_owned()),
            thread_count: process.tasks().map(|tasks| tasks.len()),
            uptime_seconds: process.run_time(),
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

/// Scans the system process table for exact name matches.
///
/// Returns all exact process-name matches. Callers intentionally decide how to
/// handle ambiguity instead of silently monitoring an arbitrary process if
/// multiple processes share the same name.
///
/// # Arguments
/// * `name` - The exact executable name to search for.
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
/// Retrieves raw metric tuples without constructing a full `ProcessInfo` object.
///
/// # Arguments
/// * `pid` - The target Process ID.
/// * `system` - A mutable reference to an existing `sysinfo::System`.
pub fn get_process_info(pid: u32, system: &mut System) -> Option<(f32, u64, sysinfo::DiskUsage)> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        ProcessRefreshKind::everything(),
    );
    let process = system.process(Pid::from_u32(pid))?;
    Some((process.cpu_usage(), process.memory(), process.disk_usage()))
}
