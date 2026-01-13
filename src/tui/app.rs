//! TUI application state and event handling.

use std::collections::VecDeque;
use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use super::ui;

/// Events that can be sent to the TUI.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// New line of output from AI.
    OutputLine(String),
    /// Tool call detected.
    ToolCall(String),
    /// File changed.
    FileChange { path: String, change_type: String },
    /// Error detected.
    Error(String),
    /// Warning detected.
    Warning(String),
    /// Iteration started.
    IterationStart { current: u32, max: u32 },
    /// Iteration complete.
    IterationComplete { duration_secs: f64 },
    /// Task info updated.
    TaskInfo { id: String, title: String },
    /// Session complete.
    SessionComplete {
        iterations: u32,
        tasks: u32,
        duration: f64,
        reason: String,
    },
    /// Quit the TUI.
    Quit,
}

/// Statistics tracked by the TUI.
#[derive(Debug, Clone, Default)]
pub struct TuiStats {
    pub tool_calls: u32,
    pub files_changed: u32,
    pub files_created: u32,
    #[allow(dead_code)]
    pub lines_added: u32,
    #[allow(dead_code)]
    pub lines_removed: u32,
    pub errors: u32,
    pub warnings: u32,
}

/// TUI application state (separate from terminal for borrowing).
#[derive(Debug)]
pub struct TuiState {
    /// Output buffer (scrolling log).
    pub output_lines: VecDeque<String>,
    /// Maximum output lines to keep.
    pub max_output_lines: usize,
    /// Recent file changes.
    pub recent_files: VecDeque<(String, String)>,
    /// Recent tool calls.
    pub recent_tools: VecDeque<String>,
    /// Current iteration.
    pub iteration_current: u32,
    /// Max iterations.
    pub iteration_max: u32,
    /// Current task ID.
    pub task_id: Option<String>,
    /// Current task title.
    pub task_title: Option<String>,
    /// Start time.
    pub start_time: Instant,
    /// Iteration start time.
    pub iteration_start: Instant,
    /// Statistics.
    pub stats: TuiStats,
    /// Spinner frame index.
    pub spinner_frame: usize,
    /// Whether session is complete.
    pub session_complete: bool,
    /// Session result info.
    pub session_result: Option<(u32, u32, f64, String)>,
    /// Scroll offset for output.
    pub scroll_offset: u16,
    /// Auto-scroll enabled.
    pub auto_scroll: bool,
}

impl TuiState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            output_lines: VecDeque::with_capacity(1000),
            max_output_lines: 500,
            recent_files: VecDeque::with_capacity(10),
            recent_tools: VecDeque::with_capacity(10),
            iteration_current: 0,
            iteration_max: 0,
            task_id: None,
            task_title: None,
            start_time: now,
            iteration_start: now,
            stats: TuiStats::default(),
            spinner_frame: 0,
            session_complete: false,
            session_result: None,
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    /// Add an output line, respecting max buffer size.
    pub fn add_output_line(&mut self, line: String) {
        for l in line.lines() {
            self.output_lines.push_back(l.to_string());
        }
        while self.output_lines.len() > self.max_output_lines {
            self.output_lines.pop_front();
        }
        if self.auto_scroll {
            self.scroll_offset = 0;
        }
    }

    /// Scroll output up.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    /// Scroll output down.
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
        } else {
            self.auto_scroll = true;
        }
    }

    /// Scroll to top.
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.output_lines.len() as u16;
    }

    /// Scroll to bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    /// Get elapsed time in seconds.
    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Get iteration elapsed time in seconds.
    pub fn iteration_elapsed_secs(&self) -> f64 {
        self.iteration_start.elapsed().as_secs_f64()
    }
}

/// TUI application.
pub struct TuiApp {
    /// Terminal instance.
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Event receiver.
    rx: Receiver<TuiEvent>,
    /// Event sender (cloneable for external use).
    tx: Sender<TuiEvent>,
    /// Application state.
    state: TuiState,
    /// Last tick time.
    last_tick: Instant,
}

impl TuiApp {
    /// Create a new TUI application.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let (tx, rx) = mpsc::channel();

        Ok(Self {
            terminal,
            rx,
            tx,
            state: TuiState::new(),
            last_tick: Instant::now(),
        })
    }

    /// Get a sender for sending events to the TUI.
    pub fn sender(&self) -> Sender<TuiEvent> {
        self.tx.clone()
    }

    /// Run the TUI event loop.
    pub fn run(&mut self) -> io::Result<()> {
        let tick_rate = Duration::from_millis(100);

        loop {
            // Draw UI - borrow state separately
            let state = &self.state;
            self.terminal.draw(|f| ui::draw(f, state))?;

            // Handle input events with timeout
            let timeout = tick_rate.saturating_sub(self.last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Up | KeyCode::Char('k') => self.state.scroll_up(),
                            KeyCode::Down | KeyCode::Char('j') => self.state.scroll_down(),
                            KeyCode::Char('g') => self.state.scroll_to_top(),
                            KeyCode::Char('G') => self.state.scroll_to_bottom(),
                            KeyCode::Char(' ') => {
                                self.state.auto_scroll = !self.state.auto_scroll;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Process TUI events (non-blocking)
            while let Ok(event) = self.rx.try_recv() {
                if !self.handle_event(event) {
                    break;
                }
            }

            // Tick - update spinner
            if self.last_tick.elapsed() >= tick_rate {
                self.state.spinner_frame = self.state.spinner_frame.wrapping_add(1);
                self.last_tick = Instant::now();
            }

            // Check if session is complete
            if self.state.session_complete {
                std::thread::sleep(Duration::from_millis(100));
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(_) = event::read()? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a TUI event, returns false if should quit.
    fn handle_event(&mut self, event: TuiEvent) -> bool {
        match event {
            TuiEvent::OutputLine(line) => {
                self.state.add_output_line(line);
            }
            TuiEvent::ToolCall(tool) => {
                self.state.stats.tool_calls += 1;
                self.state.recent_tools.push_front(tool);
                if self.state.recent_tools.len() > 8 {
                    self.state.recent_tools.pop_back();
                }
            }
            TuiEvent::FileChange { path, change_type } => {
                match change_type.as_str() {
                    "created" => self.state.stats.files_created += 1,
                    _ => self.state.stats.files_changed += 1,
                }
                self.state.recent_files.push_front((path, change_type));
                if self.state.recent_files.len() > 8 {
                    self.state.recent_files.pop_back();
                }
            }
            TuiEvent::Error(msg) => {
                self.state.stats.errors += 1;
                self.state.add_output_line(format!("❌ ERROR: {}", msg));
            }
            TuiEvent::Warning(msg) => {
                self.state.stats.warnings += 1;
                self.state.add_output_line(format!("⚠️  WARN: {}", msg));
            }
            TuiEvent::IterationStart { current, max } => {
                self.state.iteration_current = current;
                self.state.iteration_max = max;
                self.state.iteration_start = Instant::now();
                self.state.add_output_line(format!(
                    "━━━ Iteration {}/{} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                    current, max
                ));
            }
            TuiEvent::IterationComplete { duration_secs } => {
                self.state.add_output_line(format!(
                    "━━━ Iteration complete ({:.1}s) ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                    duration_secs
                ));
            }
            TuiEvent::TaskInfo { id, title } => {
                self.state.task_id = Some(id);
                self.state.task_title = Some(title);
            }
            TuiEvent::SessionComplete {
                iterations,
                tasks,
                duration,
                reason,
            } => {
                self.state.session_complete = true;
                self.state.session_result = Some((iterations, tasks, duration, reason));
            }
            TuiEvent::Quit => {
                return false;
            }
        }
        true
    }

    /// Clean up and restore terminal.
    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
