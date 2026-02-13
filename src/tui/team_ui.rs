//! Team TUI rendering with ratatui.
//!
//! Two modes:
//! - **Overview**: all agents side-by-side in a grid
//! - **Focus**: single agent full-screen with scrollable output

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};

use super::team_app::{TeamTuiState, ViewMode};
use crate::runner::worker::WorkerStatus;

/// Spinner frames for animation.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Draw the team TUI.
pub fn draw(f: &mut Frame, state: &TeamTuiState) {
    // Session complete screen
    if state.session_complete {
        if let Some((iterations, tasks, duration, ref reason)) = state.session_result {
            draw_session_complete(f, f.area(), iterations, tasks, duration, reason);
            return;
        }
    }

    let area = f.area();

    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header
            Constraint::Min(10),   // Body
            Constraint::Length(2), // Footer
        ])
        .split(area);

    draw_header(f, chunks[0], state);

    match &state.view_mode {
        ViewMode::Overview => draw_overview(f, chunks[1], state),
        ViewMode::Focus(idx) => draw_focus(f, chunks[1], state, *idx),
    }

    draw_footer(f, chunks[2], state);
}

/// Draw the header bar.
fn draw_header(f: &mut Frame, area: Rect, state: &TeamTuiState) {
    let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];

    let elapsed = state.elapsed_secs();
    let mins = (elapsed / 60.0) as u32;
    let secs = (elapsed % 60.0) as u32;

    let active = state.active_agents();

    let mut spans = vec![
        Span::styled(
            " ◉ ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "afk team",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            spinner,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
    ];

    // Agent count
    spans.push(Span::styled(
        format!("{}", active),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        format!("/{} agents", state.agents.len()),
        Style::default().fg(Color::DarkGray),
    ));

    // Task counts
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        "Tasks: ",
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::styled(
        format!("{}", state.tasks_pending),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        " pending, ",
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::styled(
        format!("{}", state.tasks_complete),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" done", Style::default().fg(Color::DarkGray)));

    // Time
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        format!("{:02}:{:02}", mins, secs),
        Style::default().fg(Color::Blue),
    ));

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_set(symbols::border::ROUNDED),
    );
    f.render_widget(header, area);
}

/// Draw overview mode: all agents in columns.
fn draw_overview(f: &mut Frame, area: Rect, state: &TeamTuiState) {
    if state.agents.is_empty() {
        let msg = Paragraph::new("No agents registered yet...")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .border_set(symbols::border::ROUNDED),
            );
        f.render_widget(msg, area);
        return;
    }

    let num = state.agents.len();

    // Split into equal columns
    let constraints: Vec<Constraint> = (0..num).map(|_| Constraint::Ratio(1, num as u32)).collect();

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, agent) in state.agents.iter().enumerate() {
        draw_agent_card(f, columns[i], agent, i, state.spinner_frame);
    }
}

/// Draw a single agent card in overview mode.
fn draw_agent_card(
    f: &mut Frame,
    area: Rect,
    agent: &super::team_app::AgentState,
    idx: usize,
    spinner_frame: usize,
) {
    let status_color = match &agent.status {
        WorkerStatus::Setting => Color::Blue,
        WorkerStatus::Working => Color::Green,
        WorkerStatus::Paused => Color::Yellow,
        WorkerStatus::Done => Color::Cyan,
        WorkerStatus::Failed(_) => Color::Red,
        WorkerStatus::Killed => Color::DarkGray,
    };

    let status_icon = match &agent.status {
        WorkerStatus::Setting => "⏳",
        WorkerStatus::Working => SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()],
        WorkerStatus::Paused => "⏸",
        WorkerStatus::Done => "✓",
        WorkerStatus::Failed(_) => "✗",
        WorkerStatus::Killed => "☠",
    };

    // Title: agent name + key hint
    let title = format!(" {} {} [{}] ", status_icon, agent.name, idx + 1);

    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(status_color))
        .border_set(symbols::border::ROUNDED);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // Card content layout
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Task title
            Constraint::Length(1), // Progress bar
            Constraint::Length(1), // Stats line
            Constraint::Min(1),    // Recent activity
        ])
        .split(inner);

    // Task title (truncated)
    let task_display = truncate_str(&agent.task_title, inner.width as usize - 2);
    let task_line = Paragraph::new(Line::from(vec![
        Span::styled(
            &agent.task_id,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": ", Style::default().fg(Color::DarkGray)),
        Span::styled(task_display, Style::default().fg(Color::White)),
    ]));
    f.render_widget(task_line, card_chunks[0]);

    // Progress bar
    if agent.iteration_max > 0 {
        let ratio = agent.iteration_current as f64 / agent.iteration_max as f64;
        let label = format!("iter {}/{}", agent.iteration_current, agent.iteration_max);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(status_color))
            .ratio(ratio.min(1.0))
            .label(label);
        f.render_widget(gauge, card_chunks[1]);
    } else {
        let waiting = Paragraph::new(Span::styled(
            "waiting...",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(waiting, card_chunks[1]);
    }

    // Stats line
    let stats_line = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{}", agent.tool_calls),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(" calls ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", agent.files_touched),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(" files │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("[{}]", agent.status_str()),
            Style::default().fg(status_color),
        ),
    ]));
    f.render_widget(stats_line, card_chunks[2]);

    // Recent activity (file changes + tools)
    let activity_height = card_chunks[3].height as usize;
    let mut items: Vec<ListItem> = Vec::new();

    for (path, change_type) in agent.recent_files.iter().take(activity_height) {
        let icon = match change_type.as_str() {
            "created" => "✦",
            "modified" => "→",
            "deleted" => "✗",
            "read" => "◇",
            _ => "·",
        };
        let color = match change_type.as_str() {
            "created" => Color::Green,
            "modified" => Color::Yellow,
            "deleted" => Color::Red,
            "read" => Color::DarkGray,
            _ => Color::White,
        };
        let display_path = truncate_str(path, inner.width as usize - 4);
        items.push(ListItem::new(Span::styled(
            format!("{} {}", icon, display_path),
            Style::default().fg(color),
        )));
    }

    let activity = List::new(items);
    f.render_widget(activity, card_chunks[3]);
}

/// Draw focus mode: single agent full-screen output.
fn draw_focus(f: &mut Frame, area: Rect, state: &TeamTuiState, idx: usize) {
    let agent = match state.agents.get(idx) {
        Some(a) => a,
        None => return,
    };

    let status_color = match &agent.status {
        WorkerStatus::Working => Color::Green,
        WorkerStatus::Paused => Color::Yellow,
        WorkerStatus::Done => Color::Cyan,
        WorkerStatus::Failed(_) => Color::Red,
        _ => Color::White,
    };

    // Layout: info bar + output
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Agent info bar
            Constraint::Min(5),    // Output
        ])
        .split(area);

    // Info bar
    let iter_str = if agent.iteration_max > 0 {
        format!(
            "Iteration {}/{}",
            agent.iteration_current, agent.iteration_max
        )
    } else {
        "Waiting...".to_string()
    };

    let info_spans = vec![
        Span::styled(
            format!(" {} ", agent.name),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &agent.task_id,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": ", Style::default().fg(Color::DarkGray)),
        Span::styled(&agent.task_title, Style::default().fg(Color::White)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            iter_str,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("[{}]", agent.status_str()),
            Style::default().fg(status_color),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} calls", agent.tool_calls),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(", ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} files", agent.files_touched),
            Style::default().fg(Color::Magenta),
        ),
    ];

    let info_bar = Paragraph::new(Line::from(info_spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_set(symbols::border::ROUNDED),
    );
    f.render_widget(info_bar, chunks[0]);

    // Output panel
    let output_area = chunks[1];
    let output_lines = &agent.output_lines;
    let visible_height = output_area.height.saturating_sub(2) as usize;
    let total_lines = output_lines.len();
    let scroll_offset = state.scroll_offset as usize;

    let start = if total_lines > visible_height {
        total_lines
            .saturating_sub(visible_height)
            .saturating_sub(scroll_offset)
    } else {
        0
    };
    let end = total_lines.saturating_sub(scroll_offset);

    let items: Vec<ListItem> = output_lines
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|line| {
            let style = if line.contains("ERROR") || line.contains("❌") || line.contains("✗") {
                Style::default().fg(Color::Red)
            } else if line.contains("WARN") || line.contains("⚠") {
                Style::default().fg(Color::Yellow)
            } else if line.starts_with("━━━") {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if line.contains("✓") || line.contains("complete") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            let max_w = output_area.width.saturating_sub(4) as usize;
            ListItem::new(Span::styled(truncate_str(line, max_w).into_owned(), style))
        })
        .collect();

    let title = if state.auto_scroll {
        " Output [auto-scroll] "
    } else {
        " Output [scroll: ↑/↓ or j/k] "
    };

    let output = List::new(items).block(
        Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_set(symbols::border::ROUNDED),
    );
    f.render_widget(output, output_area);

    // Scrollbar
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total_lines).position(start);
        f.render_stateful_widget(
            scrollbar,
            output_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the footer with context-sensitive key hints.
fn draw_footer(f: &mut Frame, area: Rect, state: &TeamTuiState) {
    let spans = match &state.view_mode {
        ViewMode::Overview => {
            vec![
                Span::styled(
                    " 1-9",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" focus  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "p+N",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" pause  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "r+N",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" resume  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "k+N",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" kill  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "m",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" merge  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" quit", Style::default().fg(Color::DarkGray)),
            ]
        }
        ViewMode::Focus(_) => {
            vec![
                Span::styled(
                    " Esc",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" overview  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "↑↓",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" scroll  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "space",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" auto-scroll  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "p",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" pause  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "K",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" kill  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" quit", Style::default().fg(Color::DarkGray)),
            ]
        }
    };

    // Show pending key indicator
    let mut all_spans = if let Some(key) = state.pending_key {
        vec![Span::styled(
            format!(" [{}+?] ", key),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]
    } else {
        Vec::new()
    };
    all_spans.extend(spans);

    let footer = Paragraph::new(Line::from(all_spans)).block(
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
            Constraint::Percentage(30),
            Constraint::Length(12),
            Constraint::Percentage(30),
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

    let (border_color, title) = if reason.contains("complete") || reason.contains("Complete") {
        (Color::Green, " ✓ Team Session Complete ")
    } else {
        (Color::Cyan, " Team Session Ended ")
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let label_width = 16;

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Iterations:"), label_style),
            Span::styled(
                format!(" {}", iterations),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Tasks completed:"), label_style),
            Span::styled(
                format!(" {}", tasks),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Duration:"), label_style),
            Span::styled(
                format!(" {}", duration_str),
                Style::default().fg(Color::Blue),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Reason:"), label_style),
            Span::styled(format!(" {}", reason), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to exit...",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .border_set(symbols::border::DOUBLE),
        )
        .centered();
    f.render_widget(panel, center);
}

/// Truncate a string to fit width.
fn truncate_str(s: &str, max_width: usize) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= max_width {
        std::borrow::Cow::Borrowed(s)
    } else {
        let truncated: String = s.chars().take(max_width.saturating_sub(1)).collect();
        std::borrow::Cow::Owned(format!("{}…", truncated))
    }
}
