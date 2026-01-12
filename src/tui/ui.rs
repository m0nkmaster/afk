//! TUI rendering with ratatui.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::app::TuiState;

/// Spinner frames for animation.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Braille animation frames for activity indicator.
const ACTIVITY_FRAMES: &[&str] = &[
    "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
];

/// Draw the entire TUI.
pub fn draw(f: &mut Frame, state: &TuiState) {
    // Check if session is complete - show celebration screen
    if state.session_complete {
        if let Some((iterations, tasks, duration, ref reason)) = state.session_result {
            draw_session_complete(f, f.area(), iterations, tasks, duration, reason);
            return;
        }
    }

    let area = f.area();

    // Main layout: header, body, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Body
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(f, main_chunks[0], state);
    draw_body(f, main_chunks[1], state);
    draw_footer(f, main_chunks[2]);
}

/// Draw the header bar.
fn draw_header(f: &mut Frame, area: Rect, state: &TuiState) {
    let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];

    // Format elapsed time
    let elapsed = state.elapsed_secs();
    let mins = (elapsed / 60.0) as u32;
    let secs = (elapsed % 60.0) as u32;
    let time_str = format!("{:02}:{:02}", mins, secs);

    // Build header spans
    let mut spans = vec![
        Span::styled(" ◉ ", Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("afk", Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(spinner, Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" ", Style::default()),
    ];

    // Iteration info
    if state.iteration_max > 0 {
        let iter_str = if state.iteration_max == u32::MAX {
            format!("Iteration {}", state.iteration_current)
        } else {
            format!("Iteration {}/{}", state.iteration_current, state.iteration_max)
        };
        spans.push(Span::styled(iter_str, Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::BOLD)));
    }

    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(time_str, Style::default().fg(Color::Blue)));
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    // Activity bar
    for i in 0..8 {
        let frame_idx = (state.spinner_frame + i) % ACTIVITY_FRAMES.len();
        let chr = ACTIVITY_FRAMES[frame_idx];
        let color = match i {
            0..=2 => Color::Cyan,
            3..=5 => Color::Blue,
            _ => Color::DarkGray,
        };
        spans.push(Span::styled(chr, Style::default().fg(color)));
    }

    // Stats summary
    let stats = &state.stats;
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        format!("{}", stats.tool_calls),
        Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD),
    ));
    spans.push(Span::styled(" calls ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        format!("{}", stats.files_changed + stats.files_created),
        Style::default().fg(Color::Magenta).add_modifier(ratatui::style::Modifier::BOLD),
    ));
    spans.push(Span::styled(" files", Style::default().fg(Color::DarkGray)));

    if stats.errors > 0 {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("{} errors", stats.errors),
            Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    let header = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(header, area);
}

/// Draw the main body area.
fn draw_body(f: &mut Frame, area: Rect, state: &TuiState) {
    // Split into output (left) and sidebar (right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(40),      // Output (main)
            Constraint::Length(32),   // Sidebar
        ])
        .split(area);

    draw_output_panel(f, body_chunks[0], state);
    draw_sidebar(f, body_chunks[1], state);
}

/// Draw the AI output panel.
fn draw_output_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    let output_lines = &state.output_lines;
    let visible_height = area.height.saturating_sub(2) as usize;

    // Calculate scroll position
    let total_lines = output_lines.len();
    let scroll_offset = state.scroll_offset as usize;

    // Get visible lines
    let start = if total_lines > visible_height {
        total_lines.saturating_sub(visible_height).saturating_sub(scroll_offset)
    } else {
        0
    };
    let end = total_lines.saturating_sub(scroll_offset);

    let items: Vec<ListItem> = output_lines
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("❌") {
                Style::default().fg(Color::Red)
            } else if line.contains("WARN") || line.contains("⚠") {
                Style::default().fg(Color::Yellow)
            } else if line.starts_with("━━━") {
                Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)
            } else if line.contains("✓") || line.contains("complete") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(truncate_line(line, area.width as usize - 4), style))
        })
        .collect();

    let title = if state.auto_scroll {
        " AI Output [auto-scroll] "
    } else {
        " AI Output [scroll: ↑/↓ or j/k] "
    };

    let output = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(output, area);

    // Render scrollbar
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(start);

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the sidebar with stats and activity.
fn draw_sidebar(f: &mut Frame, area: Rect, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),   // Task info
            Constraint::Length(9),   // Stats
            Constraint::Min(5),      // Recent files
        ])
        .split(area);

    draw_task_panel(f, chunks[0], state);
    draw_stats_panel(f, chunks[1], state);
    draw_files_panel(f, chunks[2], state);
}

/// Draw task info panel.
fn draw_task_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    let mut lines = vec![];

    if let Some(ref task_id) = state.task_id {
        lines.push(Line::from(vec![
            Span::styled("Task: ", Style::default().fg(Color::DarkGray)),
            Span::styled(task_id.clone(), Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        ]));
    }

    if let Some(ref title) = state.task_title {
        let truncated = if title.len() > 28 {
            format!("{}...", &title[..25])
        } else {
            title.to_string()
        };
        lines.push(Line::from(Span::styled(
            truncated,
            Style::default().fg(Color::White).add_modifier(ratatui::style::Modifier::ITALIC),
        )));
    }

    // Iteration progress bar
    if state.iteration_max > 0 && state.iteration_max != u32::MAX {
        lines.push(Line::from(""));
        let progress = state.iteration_current as f64 / state.iteration_max as f64;
        let filled = (progress * 24.0) as usize;
        let empty = 24 - filled;
        let bar_filled = "█".repeat(filled);
        let bar_empty = "░".repeat(empty);
        lines.push(Line::from(vec![
            Span::styled("[", Style::default().fg(Color::DarkGray)),
            Span::styled(bar_filled, Style::default().fg(Color::Cyan)),
            Span::styled(bar_empty, Style::default().fg(Color::DarkGray)),
            Span::styled("]", Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Iteration timer
    let iter_elapsed = state.iteration_elapsed_secs();
    let iter_mins = (iter_elapsed / 60.0) as u32;
    let iter_secs = (iter_elapsed % 60.0) as u32;
    lines.push(Line::from(vec![
        Span::styled("Iter time: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:02}:{:02}", iter_mins, iter_secs),
            Style::default().fg(Color::Blue),
        ),
    ]));

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Current Task ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(panel, area);
}

/// Draw stats panel with animated numbers.
fn draw_stats_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    let stats = &state.stats;
    let frame = state.spinner_frame;

    // Pulse effect for non-zero stats
    let pulse_style = |value: u32, base_color: Color| -> Style {
        if value > 0 && frame % 10 < 5 {
            Style::default().fg(base_color).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(base_color)
        }
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Tool calls:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5}", stats.tool_calls),
                pulse_style(stats.tool_calls, Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Files:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5}", stats.files_changed + stats.files_created),
                pulse_style(stats.files_changed + stats.files_created, Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Created:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5}", stats.files_created),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Errors:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5}", stats.errors),
                if stats.errors > 0 {
                    Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Warnings:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5}", stats.warnings),
                if stats.warnings > 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
    ];

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Statistics ")
                .title_style(Style::default().fg(Color::Magenta).add_modifier(ratatui::style::Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(panel, area);
}

/// Draw recent files panel.
fn draw_files_panel(f: &mut Frame, area: Rect, state: &TuiState) {
    let recent_files = &state.recent_files;

    let items: Vec<ListItem> = recent_files
        .iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(path, change_type)| {
            let (icon, color) = match change_type.as_str() {
                "created" => ("+", Color::Green),
                "deleted" => ("-", Color::Red),
                _ => ("✎", Color::Yellow),
            };

            // Extract filename
            let filename = path.rsplit('/').next().unwrap_or(path);
            let truncated = if filename.len() > 24 {
                format!("{}...", &filename[..21])
            } else {
                filename.to_string()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(truncated, Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Recent Files ")
                .title_style(Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(list, area);
}

/// Draw the footer bar.
fn draw_footer(f: &mut Frame, area: Rect) {
    let help = vec![
        Span::styled(" q", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled("space", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" auto-scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled("g/G", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled(" top/bottom", Style::default().fg(Color::DarkGray)),
    ];

    let footer = Paragraph::new(Line::from(help))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .border_set(symbols::border::ROUNDED),
        );

    f.render_widget(footer, area);
}

/// Draw session complete screen.
fn draw_session_complete(
    f: &mut Frame,
    area: Rect,
    iterations: u32,
    tasks: u32,
    duration: f64,
    reason: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(15),
            Constraint::Percentage(25),
        ])
        .horizontal_margin(10)
        .split(area);

    let center = chunks[1];

    let duration_mins = (duration / 60.0) as u32;
    let duration_secs = (duration % 60.0) as u32;
    let duration_str = if duration_mins > 0 {
        format!("{}m {}s", duration_mins, duration_secs)
    } else {
        format!("{:.0}s", duration)
    };

    let (border_color, title) = if tasks > 0 || reason.contains("complete") {
        (Color::Green, " ✓ Session Complete ")
    } else if reason.contains("interrupt") {
        (Color::Yellow, " ⚠ Session Interrupted ")
    } else {
        (Color::Cyan, " Session Ended ")
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Iterations:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", iterations), Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Tasks completed:", Style::default().fg(Color::DarkGray)),
            Span::styled(format!(" {}", tasks), Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Duration:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(duration_str, Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("  Reason:         ", Style::default().fg(Color::DarkGray)),
            Span::styled(reason.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★ ★",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to exit...",
            Style::default().fg(Color::DarkGray).add_modifier(ratatui::style::Modifier::ITALIC),
        )),
    ];

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(border_color).add_modifier(ratatui::style::Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .border_set(symbols::border::DOUBLE),
        )
        .centered();

    f.render_widget(panel, center);
}

/// Truncate a line to fit within width.
fn truncate_line(line: &str, max_width: usize) -> String {
    if line.chars().count() <= max_width {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(max_width.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
