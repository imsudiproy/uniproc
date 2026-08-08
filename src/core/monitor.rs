use crate::datasources::cpu_mem::{ProcessInfo, ProcessSampler, find_processes_by_name};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Target {
    Pid(u32),
    Name(String),
}

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

pub struct Monitor {
    sampler: ProcessSampler,
    interval: Duration,
    started_at: Instant,
    duration: Option<Duration>,
}

impl Monitor {
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

    pub fn sample(&mut self) -> Option<ProcessInfo> {
        self.sampler.sample()
    }
    pub fn interval(&self) -> Duration {
        self.interval
    }
    pub fn is_expired(&self) -> bool {
        self.duration
            .is_some_and(|duration| self.started_at.elapsed() >= duration)
    }

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
