//! Main entry point for the `uniproc` application.
//!
//! Handles command-line argument parsing, target process resolution, and
//! routes execution to either the interactive TUI dashboard or the headless
//! data collection exporters (CSV/JSON).

use clap::{ArgGroup, Parser};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use uniproc::{
    core::monitor::{Monitor, Target, resolve_target},
    output,
};

/// Command-line arguments definition using `clap`.
/// Provides structured configuration for target selection, sampling, and output methods.
#[derive(Parser, Debug)]
#[command(
    name = "uniproc",
    version,
    about = "An interactive process resource monitor"
)]
#[command(group(ArgGroup::new("target").required(true).args(["pid", "name"])))]
struct Cli {
    /// Process ID to monitor.
    #[arg(long, group = "target")]
    pid: Option<u32>,
    /// Exact process name to monitor. Fails safely if multiple processes match.
    #[arg(long, group = "target")]
    name: Option<String>,
    /// Sampling interval in milliseconds.
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(1..))]
    interval: u64,
    /// Stop after this many seconds. Required when exporting data.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    duration: Option<u64>,
    /// Write captured samples as CSV instead of starting the dashboard.
    #[arg(long, value_name = "PATH")]
    csv: Option<PathBuf>,
    /// Write captured samples as formatted JSON instead of starting the dashboard.
    #[arg(long, value_name = "PATH")]
    json: Option<PathBuf>,
    /// Track the entire process tree (aggregate metrics of all child processes).
    #[arg(long, short = 't')]
    tree: bool,
}

/// Program execution starts here.
/// Evaluates the top-level application logic and gracefully converts `Result` types
/// into standardized process exit codes.
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("uniproc: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Core execution flow.
///
/// 1. Resolves the requested target process.
/// 2. Validates export constraints (e.g., duration must exist for file exports).
/// 3. Instantiates the `Monitor` tracker.
/// 4. Either runs the interactive TUI or headless data collection.
/// 5. Optionally flushes collected samples to CSV or JSON formats.
///
/// # Arguments
/// * `cli` - Parsed command-line arguments.
fn run(cli: Cli) -> Result<(), String> {
    // Construct target specifier from command line
    let target = match (cli.pid, cli.name) {
        (Some(pid), _) => Target::Pid(pid),
        (_, Some(name)) => Target::Name(name),
        _ => unreachable!("clap validates the target"),
    };

    // Attempt to locate the live PID corresponding to the target
    let pid = resolve_target(&target)?;
    let exporting = cli.csv.is_some() || cli.json.is_some();

    // When exporting without a TUI, a duration is mandatory so the collector knows when to stop.
    if exporting && cli.duration.is_none() {
        return Err(
            "--duration is required with --csv or --json so collection has a defined end".into(),
        );
    }

    // Initialize the main resource monitor
    let monitor = Monitor::new(
        pid,
        Duration::from_millis(cli.interval),
        cli.duration.map(Duration::from_secs),
        cli.tree,
    )?;

    // Run the monitor blockingly (either silently or visibly via TUI)
    let samples = if exporting {
        monitor.collect()?
    } else {
        output::tui::run(monitor)?
    };

    // Post-processing: write accumulated samples to disk if requested
    if let Some(path) = cli.csv {
        output::csv::write(&path, &samples)?;
        println!("Wrote {} samples to {}", samples.len(), path.display());
    }
    if let Some(path) = cli.json {
        output::json::write(&path, &samples)?;
        println!("Wrote {} samples to {}", samples.len(), path.display());
    }
    Ok(())
}
