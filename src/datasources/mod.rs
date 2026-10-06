//! Low-level operating system telemetry hooks.
//!
//! Provides the data acquisition layers that interface directly with sysinfo
//! to extract real-time metrics about running processes (CPU, Memory, Network).

pub mod cpu_mem;
