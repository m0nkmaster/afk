//! Team TUI application state and event handling.
//!
//! Provides a multi-agent dashboard with overview and focus modes.
//! Each agent gets its own output buffer and state tracking.

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

use super::team_ui;
use crate::runner::worker::{WorkerEvent, WorkerStatus};

/// Commands sent from the TUI back to the orchestrator.
#[derive(Debug, Clone)]
pub enum TeamCommand {
    /// Pause a specific agent.
    PauseAgent(usize),
    /// Resume a specific agent.
    ResumeAgent(usize),
    /// Kill a specific agent.
    KillAgent(usize),
    /// Trigger merge of next completed agent.
    MergeNext,
    /// Quit all agents.
    QuitAll,
}

/// Per-agent state tracked by the TUI.
#[derive(Debug, Clone)]
pub struct AgentState {
    /// Agent display name (e.g., "🔧 Builder").
    pub name: String,
    /// Current task ID.
    pub task_id: String,
    /// Current task title.
    pub task_title: String,
    /// Current status.
    pub status: WorkerStatus,
    /// Current iteration number.
    pub iteration_current: u32,
    /// Max iterations.
    pub iteration_max: u32,
    /// Output buffer (scrolling log).
    pub output_lines: VecDeque<String>,
    /// Recent file changes.
    pub recent_files: VecDeque<(String, String)>,
    /// Recent tool calls.
    pub recent_tools: VecDeque<String>,
    /// Tool call count.
    pub tool_calls: u32,
    /// Files touched count.
    pub files_touched: u32,
    /// Iteration start time.
    pub iteration_start: Instant,
}

/// Maximum output lines per agent.
const MAX_AGENT_OUTPUT_LINES: usize = 200;

impl AgentState {
    /// Create a new agent state.
    pub fn new(name: String, task_id: String, task_title: String) -> Self {
        Self {
            name,
            task_id,
            task_title,
            status: WorkerStatus::Setting,
            iteration_current: 0,
            iteration_max: 0,
            output_lines: VecDeque::with_capacity(MAX_AGENT_OUTPUT_LINES),
            recent_files: VecDeque::with_capacity(8),
            recent_tools: VecDeque::with_capacity(8),
            tool_calls: 0,
            files_touched: 0,
            iteration_start: Instant::now(),
        }
    }

    /// Add an output line.
    pub fn add_output_line(&mut self, line: String) {
        for l in line.lines() {
            self.output_lines.push_back(l.to_string());
        }
        while self.output_lines.len() > MAX_AGENT_OUTPUT_LINES {
            self.output_lines.pop_front();
        }
    }

    /// Get a short status string.
    pub fn status_str(&self) -> &str {
        match &self.status {
            WorkerStatus::Setting => "setting up",
            WorkerStatus::Working => "working",
            WorkerStatus::Paused => "paused",
            WorkerStatus::Done => "done",
            WorkerStatus::Failed(_) => "failed",
            WorkerStatus::Killed => "killed",
        }
    }

    /// Check if the agent is still active.
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkerStatus::Setting | WorkerStatus::Working | WorkerStatus::Paused
        )
    }
}

/// TUI view mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    /// Overview: all agents side-by-side.
    Overview,
    /// Focus: single agent full-screen output.
    Focus(usize),
}

/// Team TUI application state.
#[derive(Debug)]
pub struct TeamTuiState {
    /// Per-agent states.
    pub agents: Vec<AgentState>,
    /// Current view mode.
    pub view_mode: ViewMode,
    /// Global start time.
    pub start_time: Instant,
    /// Total tasks in queue (initial).
    pub total_tasks: u32,
    /// Tasks completed so far.
    pub tasks_complete: u32,
    /// Tasks pending.
    pub tasks_pending: u32,
    /// Whether the session is complete.
    pub session_complete: bool,
    /// Session result: (iterations, tasks, duration, reason).
    pub session_result: Option<(u32, u32, f64, String)>,
    /// Spinner frame for animation.
    pub spinner_frame: usize,
    /// Scroll offset for focus mode.
    pub scroll_offset: u16,
    /// Auto-scroll in focus mode.
    pub auto_scroll: bool,
    /// Pending key for two-key combos (e.g., 'p' then '1').
    pub pending_key: Option<char>,
}

impl TeamTuiState {
    /// Create a new team TUI state.
    pub fn new(num_agents: usize) -> Self {
        Self {
            agents: Vec::with_capacity(num_agents),
            view_mode: ViewMode::Overview,
            start_time: Instant::now(),
            total_tasks: 0,
            tasks_complete: 0,
            tasks_pending: 0,
            session_complete: false,
            session_result: None,
            spinner_frame: 0,
            scroll_offset: 0,
            auto_scroll: true,
            pending_key: None,
        }
    }

    /// Register an agent.
    pub fn add_agent(&mut self, name: String, task_id: String, task_title: String) {
        self.agents.push(AgentState::new(name, task_id, task_title));
    }

    /// Get elapsed time in seconds.
    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Count active agents.
    pub fn active_agents(&self) -> usize {
        self.agents.iter().filter(|a| a.is_active()).count()
    }

    /// Scroll up in focus mode.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    /// Scroll down in focus mode.
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(3);
            if self.scroll_offset == 0 {
                self.auto_scroll = true;
            }
        } else {
            self.auto_scroll = true;
        }
    }
}

/// Team TUI application.
pub struct TeamTuiApp {
    /// Terminal instance.
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Worker event receiver.
    worker_rx: Receiver<WorkerEvent>,
    /// Worker event sender (given to workers).
    worker_tx: Sender<WorkerEvent>,
    /// Command sender (for orchestrator to read).
    cmd_tx: Sender<TeamCommand>,
    /// Command receiver (not used here, given to orchestrator).
    _cmd_rx: Receiver<TeamCommand>,
    /// Application state.
    state: TeamTuiState,
    /// Last tick time.
    last_tick: Instant,
}

impl TeamTuiApp {
    /// Create a new team TUI application.
    pub fn new(num_agents: usize) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let (worker_tx, worker_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel();

        Ok(Self {
            terminal,
            worker_rx,
            worker_tx,
            cmd_tx,
            _cmd_rx: cmd_rx,
            state: TeamTuiState::new(num_agents),
            last_tick: Instant::now(),
        })
    }

    /// Get a sender for worker events.
    pub fn worker_sender(&self) -> Sender<WorkerEvent> {
        self.worker_tx.clone()
    }

    /// Get a receiver for commands (give to orchestrator).
    pub fn command_receiver(&mut self) -> Receiver<TeamCommand> {
        let (new_tx, new_rx) = mpsc::channel();
        let old_rx = std::mem::replace(&mut self._cmd_rx, new_rx);
        self.cmd_tx = new_tx;
        old_rx
    }

    /// Register an agent with the TUI.
    pub fn add_agent(&mut self, name: String, task_id: String, task_title: String) {
        self.state.add_agent(name, task_id, task_title);
    }

    /// Set task counts.
    pub fn set_task_counts(&mut self, total: u32, pending: u32) {
        self.state.total_tasks = total;
        self.state.tasks_pending = pending;
    }

    /// Run the TUI event loop.
    pub fn run(&mut self) -> io::Result<()> {
        let tick_rate = Duration::from_millis(100);

        loop {
            // Draw UI
            let state = &self.state;
            self.terminal.draw(|f| team_ui::draw(f, state))?;

            // Handle input events
            let timeout = tick_rate.saturating_sub(self.last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && self.handle_key(key.code) {
                        break;
                    }
                }
            }

            // Process worker events (non-blocking)
            let mut got_events = false;
            while let Ok(event) = self.worker_rx.try_recv() {
                got_events = true;
                self.handle_worker_event(event);
            }

            // If no events and no active agents, session is done
            if !got_events
                && self.state.active_agents() == 0
                && !self.state.agents.is_empty()
                && !self.state.session_complete
            {
                self.state.session_complete = true;
                let elapsed = self.state.elapsed_secs();
                let reason = if self
                    .state
                    .agents
                    .iter()
                    .all(|a| a.status == WorkerStatus::Done)
                {
                    "All tasks completed"
                } else {
                    "Agents finished"
                };
                self.state.session_result = Some((
                    self.state.agents.iter().map(|a| a.iteration_current).sum(),
                    self.state.tasks_complete,
                    elapsed,
                    reason.to_string(),
                ));
            }

            // Tick — update spinner
            if self.last_tick.elapsed() >= tick_rate {
                self.state.spinner_frame = self.state.spinner_frame.wrapping_add(1);
                self.last_tick = Instant::now();
            }

            // If session complete, wait for any key press to exit
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

    /// Handle a key press. Returns true if should quit.
    fn handle_key(&mut self, code: KeyCode) -> bool {
        // Check for pending two-key combo
        if let Some(pending) = self.state.pending_key.take() {
            if let KeyCode::Char(c) = code {
                if let Some(digit) = c.to_digit(10) {
                    let idx = digit.saturating_sub(1) as usize;
                    if idx < self.state.agents.len() {
                        match pending {
                            'p' => {
                                let _ = self.cmd_tx.send(TeamCommand::PauseAgent(idx));
                                self.state.agents[idx].status = WorkerStatus::Paused;
                            }
                            'r' => {
                                let _ = self.cmd_tx.send(TeamCommand::ResumeAgent(idx));
                                self.state.agents[idx].status = WorkerStatus::Working;
                            }
                            'k' => {
                                let _ = self.cmd_tx.send(TeamCommand::KillAgent(idx));
                                self.state.agents[idx].status = WorkerStatus::Killed;
                            }
                            _ => {}
                        }
                    }
                }
            }
            return false;
        }

        match &self.state.view_mode {
            ViewMode::Overview => self.handle_overview_key(code),
            ViewMode::Focus(_) => self.handle_focus_key(code),
        }
    }

    /// Handle keys in overview mode. Returns true if should quit.
    fn handle_overview_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('q') => {
                let _ = self.cmd_tx.send(TeamCommand::QuitAll);
                return true;
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u32 - '1' as u32) as usize;
                if idx < self.state.agents.len() {
                    self.state.view_mode = ViewMode::Focus(idx);
                    self.state.scroll_offset = 0;
                    self.state.auto_scroll = true;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('r') | KeyCode::Char('k') => {
                self.state.pending_key = Some(match code {
                    KeyCode::Char(c) => c,
                    _ => unreachable!(),
                });
            }
            KeyCode::Char('m') => {
                let _ = self.cmd_tx.send(TeamCommand::MergeNext);
            }
            KeyCode::Esc => {
                // If session complete, quit
                if self.state.session_complete {
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Handle keys in focus mode. Returns true if should quit.
    fn handle_focus_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc => {
                self.state.view_mode = ViewMode::Overview;
            }
            KeyCode::Char('q') => {
                let _ = self.cmd_tx.send(TeamCommand::QuitAll);
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.scroll_down();
            }
            KeyCode::Char('p') => {
                if let ViewMode::Focus(idx) = self.state.view_mode {
                    if idx < self.state.agents.len() {
                        let _ = self.cmd_tx.send(TeamCommand::PauseAgent(idx));
                        self.state.agents[idx].status = WorkerStatus::Paused;
                    }
                }
            }
            KeyCode::Char('K') => {
                if let ViewMode::Focus(idx) = self.state.view_mode {
                    if idx < self.state.agents.len() {
                        let _ = self.cmd_tx.send(TeamCommand::KillAgent(idx));
                        self.state.agents[idx].status = WorkerStatus::Killed;
                    }
                }
            }
            KeyCode::Char(' ') => {
                self.state.auto_scroll = !self.state.auto_scroll;
            }
            KeyCode::Char('g') => {
                self.state.auto_scroll = false;
                if let ViewMode::Focus(idx) = self.state.view_mode {
                    if let Some(agent) = self.state.agents.get(idx) {
                        self.state.scroll_offset = agent.output_lines.len() as u16;
                    }
                }
            }
            KeyCode::Char('G') => {
                self.state.scroll_offset = 0;
                self.state.auto_scroll = true;
            }
            _ => {}
        }
        false
    }

    /// Handle a worker event.
    fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::StatusChange { worker_id, status } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.status = status;
                }
            }
            WorkerEvent::Output { worker_id, line } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.add_output_line(line);
                }
            }
            WorkerEvent::TaskComplete { worker_id, task_id } => {
                self.state.tasks_complete += 1;
                if self.state.tasks_pending > 0 {
                    self.state.tasks_pending -= 1;
                }
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.status = WorkerStatus::Done;
                    agent.add_output_line(format!("✓ Task {} completed", task_id));
                }
            }
            WorkerEvent::TaskFailed {
                worker_id,
                task_id,
                reason,
            } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.status = WorkerStatus::Failed(reason.clone());
                    agent.add_output_line(format!("✗ Task {} failed: {}", task_id, reason));
                }
            }
            WorkerEvent::FileChange {
                worker_id,
                path,
                change_type,
            } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.files_touched += 1;
                    agent.recent_files.push_front((path, change_type));
                    if agent.recent_files.len() > 8 {
                        agent.recent_files.pop_back();
                    }
                }
            }
            WorkerEvent::ToolCall { worker_id, tool } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.tool_calls += 1;
                    agent.recent_tools.push_front(tool);
                    if agent.recent_tools.len() > 8 {
                        agent.recent_tools.pop_back();
                    }
                }
            }
            WorkerEvent::IterationStart {
                worker_id,
                current,
                max,
            } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.iteration_current = current;
                    agent.iteration_max = max;
                    agent.iteration_start = Instant::now();
                    agent.add_output_line(format!(
                        "━━━ Iteration {}/{} ━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                        current, max
                    ));
                }
            }
            WorkerEvent::IterationComplete {
                worker_id,
                duration_secs,
            } => {
                if let Some(agent) = self.state.agents.get_mut(worker_id) {
                    agent.add_output_line(format!(
                        "━━━ Iteration complete ({:.1}s) ━━━━━━━━━━━━━━━━━━━━",
                        duration_secs
                    ));
                }
            }
        }
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

impl Drop for TeamTuiApp {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_new() {
        let agent = AgentState::new(
            "🔧 Builder".to_string(),
            "auth-001".to_string(),
            "Add login".to_string(),
        );
        assert_eq!(agent.name, "🔧 Builder");
        assert_eq!(agent.task_id, "auth-001");
        assert_eq!(agent.status_str(), "setting up");
        assert!(agent.is_active());
    }

    #[test]
    fn test_agent_state_output_buffer() {
        let mut agent = AgentState::new("A".to_string(), "t".to_string(), "T".to_string());
        for i in 0..250 {
            agent.add_output_line(format!("line {}", i));
        }
        // Should be capped at MAX_AGENT_OUTPUT_LINES
        assert_eq!(agent.output_lines.len(), MAX_AGENT_OUTPUT_LINES);
    }

    #[test]
    fn test_agent_state_status_str() {
        let mut agent = AgentState::new("A".to_string(), "t".to_string(), "T".to_string());
        assert_eq!(agent.status_str(), "setting up");
        agent.status = WorkerStatus::Working;
        assert_eq!(agent.status_str(), "working");
        agent.status = WorkerStatus::Paused;
        assert_eq!(agent.status_str(), "paused");
        agent.status = WorkerStatus::Done;
        assert_eq!(agent.status_str(), "done");
        agent.status = WorkerStatus::Failed("oops".to_string());
        assert_eq!(agent.status_str(), "failed");
        agent.status = WorkerStatus::Killed;
        assert_eq!(agent.status_str(), "killed");
    }

    #[test]
    fn test_agent_is_active() {
        let mut agent = AgentState::new("A".to_string(), "t".to_string(), "T".to_string());
        assert!(agent.is_active()); // Setting
        agent.status = WorkerStatus::Working;
        assert!(agent.is_active());
        agent.status = WorkerStatus::Paused;
        assert!(agent.is_active());
        agent.status = WorkerStatus::Done;
        assert!(!agent.is_active());
        agent.status = WorkerStatus::Failed("x".to_string());
        assert!(!agent.is_active());
        agent.status = WorkerStatus::Killed;
        assert!(!agent.is_active());
    }

    #[test]
    fn test_team_tui_state_new() {
        let state = TeamTuiState::new(3);
        assert!(state.agents.is_empty());
        assert_eq!(state.view_mode, ViewMode::Overview);
        assert_eq!(state.tasks_complete, 0);
        assert!(!state.session_complete);
    }

    #[test]
    fn test_team_tui_state_add_agent() {
        let mut state = TeamTuiState::new(3);
        state.add_agent(
            "Agent 1".to_string(),
            "t1".to_string(),
            "Task 1".to_string(),
        );
        state.add_agent(
            "Agent 2".to_string(),
            "t2".to_string(),
            "Task 2".to_string(),
        );
        assert_eq!(state.agents.len(), 2);
        assert_eq!(state.active_agents(), 2);
    }

    #[test]
    fn test_team_tui_state_scroll() {
        let mut state = TeamTuiState::new(1);
        assert!(state.auto_scroll);
        state.scroll_up();
        assert!(!state.auto_scroll);
        assert_eq!(state.scroll_offset, 3);
        state.scroll_down();
        assert_eq!(state.scroll_offset, 0);
        assert!(state.auto_scroll);
    }

    #[test]
    fn test_view_mode_eq() {
        assert_eq!(ViewMode::Overview, ViewMode::Overview);
        assert_eq!(ViewMode::Focus(0), ViewMode::Focus(0));
        assert_ne!(ViewMode::Overview, ViewMode::Focus(0));
        assert_ne!(ViewMode::Focus(0), ViewMode::Focus(1));
    }

    #[test]
    fn test_team_command_variants() {
        let _c = TeamCommand::PauseAgent(0);
        let _c = TeamCommand::ResumeAgent(0);
        let _c = TeamCommand::KillAgent(0);
        let _c = TeamCommand::MergeNext;
        let _c = TeamCommand::QuitAll;
    }
}
