# UniProc

UniProc is a terminal-first process monitor built with Rust and Ratatui. It monitors one process safely by PID or exact name, with a responsive dashboard and script-friendly exports.

## Features

- Live CPU, resident memory, disk I/O, and history charts
- System-wide network traffic shown alongside the selected process (per-process network I/O is not portable)
- Pause, clear-history, and quit controls
- Exact process-name lookup that refuses ambiguous matches
- Bounded in-memory history and reliable terminal cleanup
- CSV and pretty JSON export with byte-accurate fields

## Use

```bash
cargo run -- --pid 1234
cargo run -- --name my-service --interval 500
cargo run -- --pid 1234 --duration 60 --csv metrics.csv
cargo run -- --pid 1234 --duration 60 --json metrics.json
```

Use `p` or `Space` to pause the dashboard, `c` to clear its history, and `q` or `Esc` to quit. Export modes require `--duration` so they always finish predictably.

## Development

```bash
cargo test
cargo fmt --check
```
