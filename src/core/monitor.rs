//! Lifecycle and scheduling logic for process monitoring.
//!
//! Provides the primary `Monitor` loop controller, which queries OS-level datasources
//! on a specific cadence until an optional duration limit is reached.

use crate::datasources::cpu_mem::{ProcessInfo, ProcessSampler, find_processes_by_name};
use std::thread;
use std::time::{Duration, Instant};

/// Indicates how the user identified the process they wish to monitor.
#[derive(Debug, Clone)]
pub enum Target {
    /// Exact Process ID
    Pid(u32),
    /// Process executable name (e.g., "python3")
    Name(String),
}

/// Resolves a `Target` identifier into a concrete OS Process ID (PID).
///
/// If resolving by name, this function ensures safety by verifying that exactly
/// *one* process matches the name. If multiple processes share the name, it aborts
/// to prevent ambiguous or accidental monitoring.
///
/// # Arguments
/// * `target` - Target specification enum.
///
/// # Returns
/// A valid PID `u32` or an error string describing the failure.
pub fn resolve_target(target: &Target) -> Result<u32, String> {
    match target {
        Target::Pid(pid) => Ok(*pid),
        Target::Name(name) => {
            let matches = find_processes_by_name(name);
            match matches.as_slice() {
                [] => Err(format!("no running process exactly named {name:?}")),
                [(pid, _)] => Ok(*pid),
                _ => Err(format!(
                    "{name:?} matches multiple processes ({}); select one with --pid: {}",
                    matches.len(),
                    matches
                        .iter()
                        .map(|(pid, _)| pid.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
    }
}

/// Orchestrates the recurring collection of process telemetry data.
///
/// Wraps the underlying `ProcessSampler` and dictates timing logic, such as ensuring
/// sleep durations are respected and handling maximum duration timeouts.
pub struct Monitor {
    /// Stateful OS hook that calculates metrics over time (e.g. CPU deltas).
    sampler: ProcessSampler,
    /// Delay between metric collection passes.
    interval: Duration,
    /// Absolute time when the monitor was initialized.
    started_at: Instant,
    /// Optional limit on total collection time.
    duration: Option<Duration>,
}

impl Monitor {
    /// Initializes a new `Monitor` for the given PID.
    ///
    /// The monitor immediately primes its internal samplers when instantiated.
    ///
    /// # Arguments
    /// * `pid` - Target process ID.
    /// * `interval` - Rest period between samples.
    /// * `duration` - Maximum time to monitor before automatic shutdown.
    pub fn new(pid: u32, interval: Duration, duration: Option<Duration>) -> Result<Self, String> {
        if interval.is_zero() {
            return Err("--interval must be at least 1 ms".to_owned());
        }
        Ok(Self {
            sampler: ProcessSampler::new(pid)?,
            interval,
            started_at: Instant::now(),
            duration,
        })
    }

    /// Triggers a single immediate sampling action.
    ///
    /// Returns `None` if the target process is no longer alive.
    pub fn sample(&mut self) -> Option<ProcessInfo> {
        self.sampler.sample()
    }
    
    /// Gets the configured sleep interval duration.
    pub fn interval(&self) -> Duration {
        self.interval
    }
    
    /// Evaluates if the time since instantiation has exceeded the specified duration limit.
    /// Always returns false if no duration limit was configured.
    pub fn is_expired(&self) -> bool {
        self.duration
            .is_some_and(|duration| self.started_at.elapsed() >= duration)
    }

    /// Performs blocking continuous monitoring.
    ///
    /// This runs in a tight loop and puts the thread to sleep between samples.
    /// Suitable for headless data accumulation (e.g., CSV export) without a UI.
    ///
    /// Returns a full vector of collected samples when the duration expires, or
    /// an error if the process dies prematurely.
    pub fn collect(mut self) -> Result<Vec<ProcessInfo>, String> {
        let mut samples = Vec::new();
        loop {
            if self.is_expired() {
                break;
            }
            let sample = self.sample().ok_or("the monitored process exited")?;
            samples.push(sample);
            thread::sleep(self.interval);
        }
        Ok(samples)
    }
}
