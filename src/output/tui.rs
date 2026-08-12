//! Interactive Ratatui dashboard.

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

const HISTORY_LIMIT: usize = 180;
const SURFACE: Color = Color::Rgb(10, 14, 20);
const PANEL: Color = Color::Rgb(16, 23, 34);
const BORDER: Color = Color::Rgb(53, 65, 83);
const TEXT: Color = Color::Rgb(222, 229, 237);
const MUTED: Color = Color::Rgb(132, 145, 164);
const CPU: Color = Color::Rgb(73, 190, 255);
const MEMORY: Color = Color::Rgb(198, 132, 255);
const DISK: Color = Color::Rgb(86, 211, 137);
const NETWORK: Color = Color::Rgb(255, 191, 87);

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
        if event::poll(timeout).map_err(|e| format!("cannot read terminal events: {e}"))? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
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

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(4),
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

    render_metric_card(
        frame,
        cards[0],
        "CPU",
        format!("{cpu:.1}%"),
        "processor load",
        Some(cpu.min(100.0) / 100.0),
        CPU,
    );
    render_metric_card(
        frame,
        cards[1],
        "Memory",
        format_bytes(memory),
        "resident set",
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

    let charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(rows[2]);
    render_cpu_chart(frame, charts[0], samples);
    let spark: Vec<u64> = samples
        .iter()
        .map(|sample| sample.memory_bytes / (1024 * 1024))
        .collect();
    frame.render_widget(
        Sparkline::default()
            .block(
                panel_block("Memory History")
                    .title_bottom(Line::from(" MiB ").style(Style::default().fg(MUTED))),
            )
            .data(&spark)
            .style(Style::default().fg(MEMORY).bg(PANEL)),
        charts[1],
    );

    let footer = "p / space pause   ·   c clear history   ·   q / esc quit";
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL))
            .block(panel_block("Controls")),
        rows[3],
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
        .map(|s| format!("{}  ·  PID {}", s.name, s.pid))
        .unwrap_or_else(|| "waiting for first sample".into());
    let status_color = if paused { NETWORK } else { DISK };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "UniProc",
                Style::default()
                    .fg(TEXT)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  process monitor", Style::default().fg(MUTED).bg(PANEL)),
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
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block("Dashboard"))
            .style(Style::default().bg(PANEL)),
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
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(value))
            .style(
                Style::default()
                    .fg(TEXT)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(subtitle.into())).style(Style::default().fg(MUTED).bg(PANEL)),
        rows[1],
    );
    if let Some(ratio) = gauge_ratio {
        frame.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(color).bg(Color::Rgb(31, 41, 55)))
                .ratio(ratio.clamp(0.0, 1.0) as f64)
                .label(""),
            rows[2],
        );
    } else {
        frame.render_widget(
            Sparkline::default()
                .data(&[1])
                .max(1)
                .style(Style::default().fg(color).bg(PANEL)),
            rows[2],
        );
    }
}

fn panel_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title(Line::from(format!(" {title} ")).style(Style::default().fg(MUTED).bg(PANEL)))
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
        .block(panel_block("CPU History"))
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
