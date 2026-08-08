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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph, Sparkline},
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

const HISTORY_LIMIT: usize = 180;

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
    let title = latest
        .map(|s| format!(" {}  ·  PID {} ", s.name, s.pid))
        .unwrap_or_else(|| " UniProc · waiting for first sample ".into());
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            status,
            Style::default()
                .fg(if paused { Color::Yellow } else { Color::Green })
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("UNIPROC MONITOR"),
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);
    frame.render_widget(header, rows[0]);

    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    let cpu = latest.map_or(0.0, |s| s.cpu_percent);
    let memory = latest.map_or(0, |s| s.memory_bytes);
    let memory_ratio = latest
        .filter(|sample| sample.system_memory_bytes > 0)
        .map_or(0.0, |sample| {
            sample.memory_bytes as f32 / sample.system_memory_bytes as f32
        });
    frame.render_widget(
        metric_gauge(
            "CPU",
            cpu.min(100.0) / 100.0,
            format!("{cpu:.1}%"),
            Color::Cyan,
        ),
        cards[0],
    );
    frame.render_widget(
        metric_gauge(
            "RESIDENT MEMORY",
            memory_ratio,
            format_bytes(memory),
            Color::Magenta,
        ),
        cards[1],
    );

    let charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(rows[2]);
    render_cpu_chart(frame, charts[0], samples);
    let spark: Vec<u64> = samples
        .iter()
        .map(|sample| sample.memory_bytes / (1024 * 1024))
        .collect();
    frame.render_widget(
        Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("MEMORY HISTORY (MiB)"),
            )
            .data(&spark)
            .style(Style::default().fg(Color::Magenta)),
        charts[1],
    );

    let footer = latest
        .map(|s| {
            format!(
                "Disk: ↓ {} ↑ {}   ·   System network: ↓ {} ↑ {}",
                format_bytes(s.disk_read_bytes),
                format_bytes(s.disk_written_bytes),
                format_bytes(s.network_received_bytes),
                format_bytes(s.network_transmitted_bytes),
            )
        })
        .unwrap_or_else(|| "Collecting process metrics…".into());
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" p / space pause · c clear · q / esc quit "),
        ),
        rows[3],
    );
}

fn metric_gauge(title: &str, ratio: f32, label: String, color: Color) -> Gauge<'_> {
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .ratio(ratio.clamp(0.0, 1.0) as f64)
        .label(label)
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
        .style(Style::default().fg(Color::Cyan))
        .data(&data);
    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title("CPU HISTORY"))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_upper])
                .labels([Line::from("now")]),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, upper])
                .labels([Line::from("0%"), Line::from(format!("{upper:.0}%"))]),
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
