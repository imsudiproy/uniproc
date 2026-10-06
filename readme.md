# UniProc

UniProc is a terminal-first process resource monitor built with Rust, Ratatui, Crossterm, and sysinfo. It monitors one running process by PID or exact process name, displays an interactive TUI dashboard, and can export captured samples as CSV or pretty JSON for scripts, reports, and investigations.

## Why UniProc

Use UniProc when you want a lightweight, focused view of one process instead of a full-system monitor. It is useful for watching local services, CLI tools, development servers, stress tests, and short profiling sessions where you care about CPU, memory, disk I/O, and network activity over time.

## Features

- Interactive Ratatui dashboard with CPU, resident memory, disk I/O, network, uptime, executable path, thread count, and history views
- Modern Grafana-style layout with Tailwind-inspired colors
- Safe target selection by PID or exact process name
- Process Tree / Child Tracking: Monitor a process and all its descendants with the `--tree` flag
- Ambiguous process-name protection, with a prompt to select a PID when multiple processes match
- Pause, clear-history, and quit controls in the dashboard
- Bounded in-memory history for long-running TUI sessions
- CSV export for spreadsheet and shell workflows
- Pretty JSON export for automation and downstream tooling
- Byte-accurate raw fields in export files
- Reliable terminal cleanup when leaving the alternate screen
- Comprehensively documented codebase using rustdocs

## Status

UniProc is early but usable. The command-line interface and export fields are intentionally small and stable for the current release, but the project is still pre-1.0, so future versions may refine the dashboard and output formats.

## Platform Support

UniProc is built on cross-platform Rust libraries and should work on common Unix-like systems and Windows.

Current support notes:

- macOS: expected to work
- Linux: expected to work
- Windows: not tested yet, but it should ideally work because the underlying libraries support Windows

Some metric values are platform dependent. In particular, disk I/O comes from process refresh data exposed by `sysinfo`, executable paths can be unavailable when the operating system cannot report them, thread counts are currently reported only where `sysinfo` exposes process tasks, and network traffic is reported system-wide because portable per-process network I/O is not available through the current implementation.

## Requirements

- Rust toolchain with Cargo
- A terminal that supports alternate-screen TUI applications
- Permission to inspect the target process

The crate uses Rust edition `2024`, so use a recent stable Rust toolchain.

## Installation

From the repository root:

```bash
cargo build --release
```

The optimized binary will be created at:

```bash
target/release/uniproc
```

You can also run directly during development:

```bash
cargo run -- --pid 1234
```

## Quick Start

Monitor a process by PID:

```bash
cargo run -- --pid 1234
```

Monitor a process by exact name:

```bash
cargo run -- --name my-service
```

Monitor a process and all its descendants (Tree Mode):

```bash
cargo run -- --name chrome --tree
```

Sample more frequently:

```bash
cargo run -- --pid 1234 --interval 500
```

Capture CSV for a fixed duration:

```bash
cargo run -- --pid 1234 --duration 60 --csv metrics.csv
```

Capture JSON for a fixed duration:

```bash
cargo run -- --pid 1234 --duration 60 --json metrics.json
```

## CLI Reference

```text
uniproc --pid <PID> [OPTIONS]
uniproc --name <NAME> [OPTIONS]
```

Exactly one target is required:

| Option | Description |
| --- | --- |
| `--pid <PID>` | Monitor the process with this process ID. |
| `--name <NAME>` | Monitor a process by exact process name. If multiple processes match, UniProc exits and prints the matching PIDs so you can choose one explicitly. |

General options:

| Option | Default | Description |
| --- | --- | --- |
| `-t, --tree` | false | Aggregate CPU, Memory, Disk, and Threads for the target and all its descendants. **Note:** Memory is calculated as the sum of Resident Set Size (RSS). This heavily overcounts shared libraries used by child processes. |
| `--interval <MS>` | `1000` | Sampling interval in milliseconds. Must be at least `1`. |
| `--duration <SECONDS>` | none | Stop collection after this many seconds. Required when using `--csv` or `--json`. |
| `--csv <PATH>` | none | Write captured samples as CSV instead of starting the interactive dashboard. |
| `--json <PATH>` | none | Write captured samples as pretty JSON instead of starting the interactive dashboard. |
| `--help` | none | Print command help. |
| `--version` | none | Print the UniProc version. |

Export modes require `--duration` so the command has a defined end. Without `--csv` or `--json`, UniProc starts the interactive dashboard.

## Dashboard Controls

The dashboard header shows the target process name, PID, process uptime, thread count, and executable path when those details are available. Unsupported or unavailable thread counts are shown as `threads n/a`; unavailable executable paths are shown as `executable path unavailable`.

| Key | Action |
| --- | --- |
| `p` | Pause or resume sampling. |
| `Space` | Pause or resume sampling. |
| `c` | Clear the in-memory dashboard history. |
| `q` | Quit and return collected samples to the caller. |
| `Esc` | Quit and return collected samples to the caller. |

## Metrics

UniProc currently collects these fields for each sample:

| Field | Unit | Notes |
| --- | --- | --- |
| `timestamp_ms` | milliseconds | Unix timestamp in milliseconds. |
| `pid` | process ID | Target process ID. |
| `name` | string | Process name reported by the operating system. |
| `executable_path` | path or null | Executable path for the process, when reported by the operating system. |
| `thread_count` | count or null | Number of process tasks/threads, when reported by the operating system. |
| `uptime_seconds` | seconds | How long the target process has been running. |
| `cpu_percent` | percent | CPU usage reported by `sysinfo`. |
| `memory_bytes` | bytes | Resident memory for the process. |
| `system_memory_bytes` | bytes | Total system memory at sample time. |
| `virtual_memory_bytes` | bytes | Virtual memory for the process. |
| `disk_read_bytes` | bytes | Bytes read since the preceding process refresh, platform dependent. |
| `disk_written_bytes` | bytes | Bytes written since the preceding process refresh, platform dependent. |
| `network_received_bytes` | bytes | System-wide network bytes received since the preceding refresh. |
| `network_transmitted_bytes` | bytes | System-wide network bytes transmitted since the preceding refresh. |

The dashboard formats byte and duration values for readability. Export files keep raw byte values and write uptime as raw seconds. Optional fields use `null` in JSON and an empty CSV cell when the operating system does not report them.

## CSV Output

CSV export writes a header row followed by one row per sample:

```text
timestamp_ms,pid,name,executable_path,thread_count,uptime_seconds,cpu_percent,memory_bytes,system_memory_bytes,virtual_memory_bytes,disk_read_bytes,disk_written_bytes,system_network_received_bytes,system_network_transmitted_bytes
```

Example:

```bash
cargo run -- --pid 1234 --duration 30 --csv metrics.csv
```

## JSON Output

JSON export writes a pretty-printed array of sample objects.

Example:

```bash
cargo run -- --pid 1234 --duration 30 --json metrics.json
```

Example shape:

```json
[
  {
    "timestamp_ms": 1760000000000,
    "pid": 1234,
    "name": "my-service",
    "executable_path": "/usr/local/bin/my-service",
    "thread_count": 8,
    "uptime_seconds": 3600,
    "cpu_percent": 12.5,
    "memory_bytes": 104857600,
    "system_memory_bytes": 17179869184,
    "virtual_memory_bytes": 4294967296,
    "disk_read_bytes": 4096,
    "disk_written_bytes": 8192,
    "network_received_bytes": 2048,
    "network_transmitted_bytes": 1024
  }
]
```

## Behavior and Limitations

- UniProc monitors one process per run.
- `--name` uses exact process-name matching, not substring matching.
- If the process exits while monitoring, UniProc exits with an error.
- Dashboard history is bounded to avoid unbounded memory growth.
- Export collection sleeps for the configured interval between samples.
- Disk I/O values are platform dependent.
- Executable paths may be unavailable because of operating-system permissions or platform limitations.
- Thread counts come from process task data and may be unavailable on platforms where `sysinfo` does not expose tasks.
- Network values are system-wide deltas, not per-process network usage.
- Export paths are overwritten if the target file already exists.

## Troubleshooting

`process with PID <pid> was not found`

The PID does not exist or the process exited before UniProc could start sampling.

`no running process exactly named "<name>"`

The process name did not match exactly. Check the process name shown by your operating system and try again.

`"<name>" matches multiple processes`

More than one process has the same exact name. Re-run UniProc with `--pid` and one of the PIDs printed in the error message.

`--duration is required with --csv or --json`

Export mode needs a fixed end time. Add `--duration <SECONDS>`.

Terminal display looks broken after exit

UniProc attempts to restore the terminal on exit. If the terminal is still in a bad state after an unexpected interruption, run:

```bash
reset
```

## Development

Run tests:

```bash
cargo test
```

Check formatting:

```bash
cargo fmt --check
```

Build the release binary:

```bash
cargo build --release
```

Run the dashboard against the current shell or another known process:

```bash
cargo run -- --pid 1234
```

Optional local stress-test setup on Debian or Ubuntu:

```bash
sudo apt install -y stress
stress --cpu 1 --timeout 250
```

## Project Structure

```text
src/main.rs                 CLI parsing and top-level command flow
src/core/monitor.rs         Target resolution and sample collection loop
src/datasources/cpu_mem.rs  Process sampling through sysinfo
src/output/tui.rs           Interactive Ratatui dashboard
src/output/csv.rs           CSV writer
src/output/json.rs          JSON writer
tests/                      Integration tests
doc/help.md                 Extra development notes
```

## License

See `LICENSE`.
