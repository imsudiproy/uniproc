//! Interactive Ratatui dashboard.
//!
//! This module handles the terminal user interface (TUI) for the `uniproc` process monitor.
//! It uses the `ratatui` crate to render a responsive dashboard with live charts,
//! gauges, and statistics about a running process.

use crate::core::monitor::Monitor;
use crate::datasources::cpu_mem::ProcessInfo;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, BorderType, Borders, Chart, Dataset, Gauge, GraphType, Paragraph, Sparkline,
    },
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

// Maximum number of data points to keep in memory for rendering historical charts.
const HISTORY_LIMIT: usize = 300; // Increased for better horizontal scrolling on wide screens

// UI Color Palette
const SURFACE: Color = Color::Rgb(9, 9, 11);
const PANEL: Color = Color::Rgb(24, 24, 27);
const BORDER: Color = Color::Rgb(63, 63, 70);
const TEXT: Color = Color::Rgb(244, 244, 245);
const MUTED: Color = Color::Rgb(161, 161, 170);

const CPU: Color = Color::Rgb(56, 189, 248);
const MEMORY: Color = Color::Rgb(192, 132, 252);
const DISK: Color = Color::Rgb(52, 211, 153);
const NETWORK: Color = Color::Rgb(250, 204, 21);

pub fn run(monitor: Monitor) -> Result<Vec<ProcessInfo>, String> {
    enable_raw_mode().map_err(|e| format!("cannot enable terminal raw mode: {e}"))?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(format!("cannot enter alternate screen: {error}"));
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            let mut cleanup_stdout = io::stdout();
            let _ = execute!(cleanup_stdout, LeaveAlternateScreen);
            return Err(format!("cannot initialize terminal: {error}"));
        }
    };

    let result = run_dashboard(&mut terminal, monitor);

    let cleanup = restore_terminal(&mut terminal);
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (result, Ok(())) => result,
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), String> {
    disable_raw_mode().map_err(|e| format!("cannot restore terminal mode: {e}"))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("cannot leave alternate screen: {e}"))?;
    terminal
        .show_cursor()
        .map_err(|e| format!("cannot restore cursor: {e}"))
}

fn run_dashboard(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut monitor: Monitor,
) -> Result<Vec<ProcessInfo>, String> {
    let mut samples = Vec::new();
    let mut paused = false;
    let mut status = String::from("LIVE");

    samples.push(monitor.sample().ok_or("the monitored process exited")?);
    let mut last_tick = Instant::now();

    loop {
        terminal
            .draw(|frame| draw(frame.area(), frame, &samples, paused, &status))
            .map_err(|e| format!("cannot draw dashboard: {e}"))?;

        if monitor.is_expired() {
            status = String::from("DURATION COMPLETE");
            terminal
                .draw(|frame| draw(frame.area(), frame, &samples, true, &status))
                .map_err(|e| e.to_string())?;
            return Ok(samples);
        }

        let elapsed = last_tick.elapsed();
        let timeout = monitor
            .interval()
            .saturating_sub(elapsed)
            .min(Duration::from_millis(100));

        if event::poll(timeout).map_err(|e| format!("cannot read terminal events: {e}"))?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(samples),
                KeyCode::Char('p') | KeyCode::Char(' ') => {
                    paused = !paused;
                    status = if paused { "PAUSED" } else { "LIVE" }.to_owned();
                }
                KeyCode::Char('c') => {
                    samples.clear();
                    status = if paused {
                        "PAUSED · HISTORY CLEARED"
                    } else {
                        "LIVE · HISTORY CLEARED"
                    }
                    .to_owned();
                }
                _ => {}
            }
        }

        if !paused && last_tick.elapsed() >= monitor.interval() {
            match monitor.sample() {
                Some(sample) => {
                    samples.push(sample);
                    if samples.len() > HISTORY_LIMIT {
                        samples.remove(0);
                    }
                    status = String::from("LIVE");
                    last_tick = Instant::now();
                }
                None => return Err("the monitored process exited".to_owned()),
            }
        }
    }
}

fn draw(
    area: Rect,
    frame: &mut ratatui::Frame,
    samples: &[ProcessInfo],
    paused: bool,
    status: &str,
) {
    let latest = samples.last();

    frame.render_widget(Block::default().style(Style::default().bg(SURFACE)), area);

    // Main vertical layout structure:
    // Row 1: Header (Title, target info)
    // Row 2: Metric Cards (Current point-in-time stats)
    // Row 3: CPU Line Chart
    // Row 4: Memory Sparkline
    // Row 5: Footer (Keybindings)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header
            Constraint::Length(6), // Metric Cards
            Constraint::Min(10),   // CPU Chart
            Constraint::Length(7), // Memory Sparkline
            Constraint::Length(3), // Footer
        ])
        .margin(1)
        .split(area);

    render_header(frame, rows[0], latest, samples.len(), paused, status);

    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(rows[1]);

    let cpu = latest.map_or(0.0, |s| s.cpu_percent);
    let memory = latest.map_or(0, |s| s.memory_bytes);
    let memory_ratio = latest
        .filter(|sample| sample.system_memory_bytes > 0)
        .map_or(0.0, |sample| {
            sample.memory_bytes as f32 / sample.system_memory_bytes as f32
        });
    let disk_read = latest.map_or(0, |s| s.disk_read_bytes);
    let disk_written = latest.map_or(0, |s| s.disk_written_bytes);
    let network_received = latest.map_or(0, |s| s.network_received_bytes);
    let network_transmitted = latest.map_or(0, |s| s.network_transmitted_bytes);

    let is_tree = latest.map_or(false, |s| s.name.ends_with("(tree)"));
    let memory_title = if is_tree { "Memory (Σ RSS)" } else { "Memory" };
    let memory_subtitle = if is_tree {
        "overcounts shared libs"
    } else {
        "resident set"
    };

    render_metric_card(
        frame,
        cards[0],
        "CPU",
        format!("{cpu:.1}%"),
        "load",
        Some(cpu.min(100.0) / 100.0),
        CPU,
    );
    render_metric_card(
        frame,
        cards[1],
        memory_title,
        format_bytes(memory),
        memory_subtitle,
        Some(memory_ratio),
        MEMORY,
    );
    render_metric_card(
        frame,
        cards[2],
        "Disk I/O",
        format!("↓ {}", format_bytes(disk_read)),
        format!("↑ {}", format_bytes(disk_written)),
        None,
        DISK,
    );
    render_metric_card(
        frame,
        cards[3],
        "Network",
        format!("↓ {}", format_bytes(network_received)),
        format!("↑ {}", format_bytes(network_transmitted)),
        None,
        NETWORK,
    );

    // Render CPU Chart (Line Graph)
    render_cpu_chart(frame, rows[2], samples);

    // Render Memory Sparkline
    let spark: Vec<u64> = samples
        .iter()
        .map(|sample| sample.memory_bytes / (1024 * 1024))
        .collect();

    let max_mem_mib = spark.iter().copied().max().unwrap_or(1);

    // Scale the maximum upper bound of the sparkline to 150% of the peak memory usage.
    // This provides vertical headroom so that the visual representation naturally aligns
    // at the 66% height mark during steady-state memory utilization, rather than filling
    // the entire block height. A minimum scale of 10 MiB is enforced for tiny processes.
    let spark_max = ((max_mem_mib as f64 * 1.5) as u64).max(10);

    frame.render_widget(
        Sparkline::default()
            .block(
                panel_block("Memory Allocation (MiB)").title_bottom(
                    Line::from(format!(" Peak: {} MiB ", max_mem_mib))
                        .style(Style::default().fg(MUTED)),
                ),
            )
            .data(&spark)
            .max(spark_max)
            .style(Style::default().fg(MEMORY).bg(PANEL)),
        rows[3],
    );

    let footer = "p / space pause   ·   c clear history   ·   q / esc quit";
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        rows[4],
    );
}

fn render_header(
    frame: &mut ratatui::Frame,
    area: Rect,
    latest: Option<&ProcessInfo>,
    sample_count: usize,
    paused: bool,
    status: &str,
) {
    let target = latest
        .map(|s| {
            format!(
                "{}  ·  PID {}  ·  uptime {}  ·  threads {}",
                s.name,
                s.pid,
                format_duration(s.uptime_seconds),
                format_thread_count(s.thread_count)
            )
        })
        .unwrap_or_else(|| "waiting for first sample".into());

    let executable_path = latest
        .and_then(|s| s.executable_path.as_deref())
        .unwrap_or("executable path unavailable");

    let status_color = if paused { NETWORK } else { DISK };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " UniProc ",
                Style::default()
                    .fg(TEXT)
                    .bg(CPU)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  Process Telemetry",
                Style::default()
                    .fg(TEXT)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(target, Style::default().fg(CPU).bg(PANEL)),
            Span::styled(
                format!("   ·   {sample_count} samples   ·   "),
                Style::default().fg(MUTED).bg(PANEL),
            ),
            Span::styled(
                format!(" {status} "),
                Style::default()
                    .fg(status_color)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            executable_path.to_owned(),
            Style::default()
                .fg(MUTED)
                .bg(PANEL)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(PANEL)),
        area,
    );
}

fn render_metric_card(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &'static str,
    value: String,
    subtitle: impl Into<String>,
    gauge_ratio: Option<f32>,
    color: Color,
) {
    let block = panel_block(title);
    let inner = block.inner(area).inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(value)).style(
            Style::default()
                .fg(TEXT)
                .bg(PANEL)
                .add_modifier(Modifier::BOLD),
        ),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(subtitle.into())).style(Style::default().fg(MUTED).bg(PANEL)),
        rows[1],
    );

    if let Some(ratio) = gauge_ratio {
        frame.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(color).bg(Color::Rgb(39, 39, 42)))
                .ratio(ratio.clamp(0.0, 1.0) as f64)
                .use_unicode(true)
                .label(""),
            rows[2],
        );
    } else {
        frame.render_widget(
            Sparkline::default()
                .data([1])
                .max(1)
                .style(Style::default().fg(Color::Rgb(39, 39, 42)).bg(PANEL)),
            rows[2],
        );
    }
}

fn panel_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(BORDER))
        .title(
            Line::from(format!(" {title} ")).style(
                Style::default()
                    .fg(TEXT)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .style(Style::default().fg(TEXT).bg(PANEL))
}

fn render_cpu_chart(frame: &mut ratatui::Frame, area: Rect, samples: &[ProcessInfo]) {
    let data: Vec<(f64, f64)> = samples
        .iter()
        .enumerate()
        .map(|(i, sample)| (i as f64, sample.cpu_percent as f64))
        .collect();

    let upper = data
        .iter()
        .map(|(_, value)| *value)
        .fold(100.0_f64, f64::max)
        .ceil();
    let x_upper = data.len().max(2) as f64 - 1.0;

    let dataset = Dataset::default()
        .name("CPU %")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(CPU))
        .data(&data);

    let chart = Chart::new(vec![dataset])
        .block(panel_block("CPU Usage History"))
        .style(Style::default().bg(PANEL))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_upper])
                .style(Style::default().fg(MUTED).bg(PANEL))
                .labels([Line::from("now").style(Style::default().fg(MUTED))]),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, upper])
                .style(Style::default().fg(MUTED).bg(PANEL))
                .labels([
                    Line::from("0%").style(Style::default().fg(MUTED)),
                    Line::from(format!("{upper:.0}%")).style(Style::default().fg(MUTED)),
                ]),
        );

    frame.render_widget(chart, area);
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

fn format_thread_count(thread_count: Option<usize>) -> String {
    thread_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "n/a".to_owned())
}
