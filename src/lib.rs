//! Core libraries for `uniproc`.
//!
//! Exposes modules responsible for:
//! - `core`: Monitoring logic, target resolution, and sampling lifecycle.
//! - `datasources`: Operating system level hooks and metrics parsers (CPU, Memory, Network).
//! - `output`: Reporting interfaces such as the terminal UI (TUI) and data exporters (CSV/JSON).

pub mod core;
pub mod datasources;
pub mod output;
