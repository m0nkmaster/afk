//! TUI rendering with ratatui.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Widget,
    },
    Frame,
};

use super::app::TuiState;
use crate::feedback;

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
    // Simplified layout with full-width output
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header (compact)
            Constraint::Min(10),   // Body (output)
            Constraint::Length(2), // Footer (compact)
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
        Span::styled(
            " ◉ ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            "afk",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            spinner,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
    ];

    // Iteration info
    if state.iteration_max > 0 {
        let iter_str = if state.iteration_max == u32::MAX {
            format!("Iteration {}", state.iteration_current)
        } else {
            format!(
                "Iteration {}/{}",
                state.iteration_current, state.iteration_max
            )
        };
        spans.push(Span::styled(
            iter_str,
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Task counts
    if state.tasks_pending > 0 || state.tasks_complete > 0 {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled("Tasks: ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("{}", state.tasks_pending),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        spans.push(Span::styled(" pending, ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("{}", state.tasks_complete),
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        spans.push(Span::styled(" complete", Style::default().fg(Color::DarkGray)));
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
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ));
    spans.push(Span::styled(
        " calls ",
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::styled(
        format!("{}", stats.total_files()),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ));
    spans.push(Span::styled(" files", Style::default().fg(Color::DarkGray)));

    if stats.errors > 0 {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            format!("{} errors", stats.errors),
            Style::default()
                .fg(Color::Red)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Add task info if available
    if let Some(ref task_id) = state.task_id {
        spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(
            task_id.clone(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        if let Some(ref title) = state.task_title {
            // Truncate title if too long
            let max_title_len = 30;
            let display_title = if title.len() > max_title_len {
                format!("{}...", &title[..max_title_len - 3])
            } else {
                title.clone()
            };
            spans.push(Span::styled(
                format!(": {}", display_title),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_set(symbols::border::ROUNDED),
    );

    f.render_widget(header, area);
}

/// Draw the main body area.
fn draw_body(f: &mut Frame, area: Rect, state: &TuiState) {
    // Full-width output panel (simplified layout)
    draw_output_panel(f, area, state);
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
            let style = if line.contains("ERROR") || line.contains("❌") {
                Style::default().fg(Color::Red)
            } else if line.contains("WARN") || line.contains("⚠") {
                Style::default().fg(Color::Yellow)
            } else if line.starts_with("━━━") {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else if line.contains("✓") || line.contains("complete") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(
                truncate_line(line, area.width as usize - 4),
                style,
            ))
        })
        .collect();

    let title = if state.auto_scroll {
        " AI Output [auto-scroll] "
    } else {
        " AI Output [scroll: ↑/↓ or j/k] "
    };

    let output = List::new(items).block(
        Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
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

        let mut scrollbar_state = ScrollbarState::new(total_lines).position(start);

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Draw the footer bar.
fn draw_footer(f: &mut Frame, area: Rect) {
    let help = vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "space",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" auto-scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "g/G",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" top/bottom", Style::default().fg(Color::DarkGray)),
    ];

    let footer = Paragraph::new(Line::from(help)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_set(symbols::border::ROUNDED),
    );

    f.render_widget(footer, area);
}

/// Animated starfield background widget.
struct StarfieldBackground {
    frame: usize,
}

impl Widget for StarfieldBackground {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create a moving starfield effect
        let frame = self.frame;

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                // Pseudo-random star placement based on position and frame
                let seed = (x as usize * 31 + y as usize * 17 + frame / 3) % 47;
                if seed < 2 {
                    // Animate star brightness based on frame
                    let star_idx = (seed + frame / 5) % feedback::STAR_CHARS.len();
                    let star = feedback::get_star_char(star_idx);

                    // Colour varies with position for depth effect
                    let color = match (x as usize + y as usize + frame / 4) % 4 {
                        0 => Color::DarkGray,
                        1 => Color::Gray,
                        2 => Color::Blue,
                        _ => Color::Cyan,
                    };

                    buf[(x, y)].set_char(star).set_fg(color);
                }
            }
        }
    }
}

/// A single firework explosion.
struct Firework {
    x: u16,
    y: u16,
    pattern_idx: usize,
    char_idx: usize,
    age: usize,
}

impl Firework {
    fn new(x: u16, y: u16, pattern_idx: usize, char_idx: usize) -> Self {
        Self {
            x,
            y,
            pattern_idx,
            char_idx,
            age: 0,
        }
    }

    /// Render firework particles to buffer.
    fn render(&self, area: Rect, buf: &mut Buffer, frame: usize) {
        let pattern = feedback::get_burst_pattern(self.pattern_idx);
        let base_char = feedback::get_firework_char(self.char_idx);

        // Explosion expands over time
        let scale = ((self.age + frame / 2) % 6) as i16;

        // Colours cycle for sparkle effect
        let colors = [
            Color::Yellow,
            Color::Magenta,
            Color::Cyan,
            Color::Red,
            Color::Green,
            Color::White,
        ];

        for (i, (dx, dy)) in pattern.iter().enumerate() {
            // Scale the pattern outward over time
            let scaled_dx = dx * (scale + 1) / 2;
            let scaled_dy = dy * (scale + 1) / 3;

            let px = self.x as i16 + scaled_dx;
            let py = self.y as i16 + scaled_dy;

            if px >= area.left() as i16
                && px < area.right() as i16
                && py >= area.top() as i16
                && py < area.bottom() as i16
            {
                let color = colors[(i + frame / 2) % colors.len()];
                // Fade particles as they age
                let char_to_use = if (self.age + frame) % 8 < 6 {
                    base_char
                } else {
                    '·'
                };
                buf[(px as u16, py as u16)]
                    .set_char(char_to_use)
                    .set_fg(color);
            }
        }
    }
}

/// Draw session complete screen with fireworks and moving background.
fn draw_session_complete(
    f: &mut Frame,
    area: Rect,
    iterations: u32,
    tasks: u32,
    duration: f64,
    reason: &str,
) {
    // Use a simple frame counter based on terminal size for animation
    // (In real usage, this would come from TuiState.spinner_frame)
    static FRAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let frame = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Render moving starfield background
    let starfield = StarfieldBackground { frame };
    f.render_widget(starfield, area);

    // Generate fireworks at pseudo-random positions based on frame
    let fireworks = generate_fireworks(area, frame);
    for fw in &fireworks {
        fw.render(area, f.buffer_mut(), frame);
    }

    // Centre panel layout
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

    // Animated celebration border characters
    let celebration_chars = ['★', '✦', '✧', '◆', '❋'];
    let border_char = celebration_chars[frame % celebration_chars.len()];
    let celebration_line = format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char,
        border_char
    );

    // Animated colours for celebration line
    let celebration_colors = [
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::Green,
        Color::Red,
    ];
    let line_color = celebration_colors[(frame / 2) % celebration_colors.len()];

    // Format stats with consistent alignment
    let label_style = Style::default().fg(Color::DarkGray);
    let label_width = 16;

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            celebration_line.clone(),
            Style::default()
                .fg(line_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Iterations:"), label_style),
            Span::styled(
                format!(" {}", iterations),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{:>label_width$}", "Tasks completed:"), label_style),
            Span::styled(
                format!(" {}", tasks),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
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
            celebration_line,
            Style::default()
                .fg(line_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to exit...",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(ratatui::style::Modifier::ITALIC),
        )),
    ];

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(border_color)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .border_set(symbols::border::DOUBLE),
        )
        .centered();

    f.render_widget(panel, center);
}

/// Generate fireworks at pseudo-random positions based on animation frame.
fn generate_fireworks(area: Rect, frame: usize) -> Vec<Firework> {
    let mut fireworks = Vec::new();

    // Generate 3-5 fireworks at different positions
    // Positions cycle to create continuous explosions
    let positions = [
        (0.15, 0.2),
        (0.85, 0.25),
        (0.1, 0.75),
        (0.9, 0.8),
        (0.5, 0.15),
    ];

    for (i, (x_pct, y_pct)) in positions.iter().enumerate() {
        // Stagger firework appearances
        let fw_frame = (frame + i * 7) % 30;
        if fw_frame < 20 {
            let x = (area.width as f64 * x_pct) as u16 + area.left();
            let y = (area.height as f64 * y_pct) as u16 + area.top();

            fireworks.push(Firework::new(x, y, i, (frame / 3 + i) % 10));
        }
    }

    fireworks
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
