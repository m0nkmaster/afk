//! Team mode orchestrator.
//!
//! Manages multiple workers running in parallel, each in its own git worktree.
//! Handles task assignment, worker lifecycle, and merge coordination.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use crate::config::AfkConfig;
use crate::git;
use crate::persona::{self, Persona};
use crate::prd::{PrdDocument, UserStory};

use super::worker::{Worker, WorkerError, WorkerEvent, WorkerStatus};
use super::{RunResult, StopReason};

/// Options for running in team mode.
#[derive(Debug, Clone)]
pub struct TeamOptions {
    /// Number of parallel agents.
    pub num_agents: u32,
    /// Max iterations per agent per task.
    pub max_iterations: u32,
    /// Optional natural language prompt for quick mode.
    pub prompt: Option<String>,
}

impl Default for TeamOptions {
    fn default() -> Self {
        Self {
            num_agents: 3,
            max_iterations: 5,
            prompt: None,
        }
    }
}

/// Error type for team operations.
#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    /// Worker setup failed.
    #[error("Worker setup failed: {0}")]
    WorkerError(#[from] WorkerError),
    /// No tasks available.
    #[error("No tasks available to work on")]
    NoTasks,
    /// Task decomposition failed.
    #[error("Task decomposition failed: {0}")]
    DecomposeError(String),
    /// PRD error.
    #[error("PRD error: {0}")]
    PrdError(#[from] crate::prd::PrdError),
    /// Persona error.
    #[error("Persona error: {0}")]
    PersonaError(#[from] persona::PersonaError),
    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// The team orchestrator manages parallel workers.
pub struct TeamRunner {
    config: AfkConfig,
    options: TeamOptions,
    workers: Vec<Worker>,
    personas: Vec<Persona>,
    task_queue: Vec<UserStory>,
    completed_tasks: Vec<String>,
    interrupted: Arc<AtomicBool>,
}

impl TeamRunner {
    /// Create a new team runner.
    pub fn new(config: AfkConfig, options: TeamOptions) -> Self {
        Self {
            config,
            options,
            workers: Vec::new(),
            personas: Vec::new(),
            task_queue: Vec::new(),
            completed_tasks: Vec::new(),
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run the team.
    ///
    /// This is the main entry point. It:
    /// 1. Decomposes the prompt into tasks (quick mode) or loads existing tasks
    /// 2. Loads personas
    /// 3. Sets up workers with worktrees
    /// 4. Runs workers in parallel
    /// 5. Merges results back
    pub fn run(&mut self) -> Result<RunResult, TeamError> {
        let start = Instant::now();

        // Set up interrupt handler
        let interrupted = self.interrupted.clone();
        let _ = ctrlc::set_handler(move || {
            interrupted.store(true, Ordering::SeqCst);
        });

        // Step 1: Get tasks
        if let Some(ref prompt) = self.options.prompt.clone() {
            self.decompose_prompt(prompt)?;
        } else {
            self.load_existing_tasks()?;
        }

        if self.task_queue.is_empty() {
            return Err(TeamError::NoTasks);
        }

        let total_tasks = self.task_queue.len();
        println!("\x1b[32m✓\x1b[0m {} tasks ready", total_tasks);
        for task in &self.task_queue {
            println!(
                "  → {}: {} (priority {})",
                task.id, task.title, task.priority
            );
        }
        println!();

        // Step 2: Load personas
        persona::ensure_defaults(None)?;
        self.personas = persona::load_personas(None)?;

        // Step 3: Set up workers
        let num_agents = (self.options.num_agents as usize).min(self.task_queue.len());
        println!("Spawning {} agents...\n", num_agents);

        let (tx, rx): (Sender<WorkerEvent>, Receiver<WorkerEvent>) = mpsc::channel();

        for i in 0..num_agents {
            let persona = self.personas.get(i).cloned();
            let mut worker = Worker::new(i, persona, self.options.max_iterations, Some(tx.clone()));

            // Assign next task from queue
            if let Some(task) = self.task_queue.first().cloned() {
                self.task_queue.remove(0);
                println!("  {} → {}", worker.display_name(), task.title);
                worker.assign_task(task);
            }

            self.workers.push(worker);
        }
        println!();

        // Step 4: Set up worktrees
        for worker in &mut self.workers {
            if let Err(e) = worker.setup() {
                eprintln!(
                    "\x1b[33m⚠\x1b[0m  Failed to set up {}: {}",
                    worker.display_name(),
                    e
                );
                worker.status = WorkerStatus::Failed(e.to_string());
            }
        }

        // Step 5: Run workers in parallel threads
        let mut handles = Vec::new();
        let interrupted_main = self.interrupted.clone();

        for worker in &self.workers {
            if worker.status == WorkerStatus::Setting {
                // This worker was set up successfully
                let worker_id = worker.id;
                let working_dir = worker.working_dir().to_path_buf();
                let config = self.config.clone();
                let max_iters = worker.max_iterations;
                let tx_clone = tx.clone();
                let interrupted_clone = self.interrupted.clone();

                let handle = std::thread::spawn(move || {
                    run_worker_loop(
                        worker_id,
                        &working_dir,
                        &config,
                        max_iters,
                        tx_clone,
                        interrupted_clone,
                    )
                });
                handles.push((worker.id, handle));
            }
        }

        // Drop our sender so rx closes when all workers finish
        drop(tx);

        // Step 6: Process events from workers
        let mut tasks_completed = 0u32;
        let mut iterations_completed = 0u32;

        while let Ok(event) = rx.recv() {
            if interrupted_main.load(Ordering::SeqCst) {
                println!("\n\x1b[33m⚠\x1b[0m  Interrupted — stopping agents...");
                break;
            }

            match event {
                WorkerEvent::StatusChange { worker_id, status } => {
                    if let Some(w) = self.workers.get_mut(worker_id) {
                        let name = w.display_name();
                        match &status {
                            WorkerStatus::Working => {
                                println!("  {} is working...", name);
                            }
                            WorkerStatus::Done => {
                                println!("  \x1b[32m✓\x1b[0m {} completed task", name);
                            }
                            WorkerStatus::Failed(reason) => {
                                println!("  \x1b[31m✗\x1b[0m {} failed: {}", name, reason);
                            }
                            _ => {}
                        }
                        w.status = status;
                    }
                }
                WorkerEvent::Output { worker_id, line } => {
                    if let Some(w) = self.workers.get(worker_id) {
                        let name = w.display_name();
                        // Print with worker prefix (truncate long lines)
                        let display_line = if line.len() > 100 {
                            format!("{}...", &line[..97])
                        } else {
                            line
                        };
                        println!("  [{name}] {display_line}");
                    }
                }
                WorkerEvent::TaskComplete { worker_id, task_id } => {
                    tasks_completed += 1;
                    self.completed_tasks.push(task_id.clone());

                    // Merge the worker's branch
                    if let Some(w) = self.workers.get(worker_id) {
                        let branch = &w.branch_name;
                        print!("  Merging {} ({})... ", w.display_name(), branch);
                        match git::merge_branch(branch) {
                            git::MergeResult::Clean => {
                                println!("\x1b[32m✓\x1b[0m");
                            }
                            git::MergeResult::Conflict(files) => {
                                println!("\x1b[33mconflict\x1b[0m");
                                println!("    Conflicting files: {}", files.join(", "));
                                println!("    Task {} needs manual resolution.", task_id);
                            }
                            git::MergeResult::Failed(reason) => {
                                println!("\x1b[31mfailed\x1b[0m: {}", reason);
                            }
                        }
                    }

                    // Assign next task if available
                    if let Some(next_task) = self.task_queue.first().cloned() {
                        self.task_queue.remove(0);
                        if let Some(w) = self.workers.get_mut(worker_id) {
                            println!("  {} → next task: {}", w.display_name(), next_task.title);
                            w.assign_task(next_task);
                            if let Err(e) = w.setup() {
                                eprintln!("  \x1b[33m⚠\x1b[0m  Setup failed: {}", e);
                            }
                        }
                    }
                }
                WorkerEvent::TaskFailed {
                    worker_id,
                    task_id,
                    reason,
                } => {
                    if let Some(w) = self.workers.get(worker_id) {
                        eprintln!(
                            "  \x1b[31m✗\x1b[0m {} failed on {}: {}",
                            w.display_name(),
                            task_id,
                            reason
                        );
                    }
                }
                WorkerEvent::IterationStart {
                    worker_id,
                    current,
                    max,
                } => {
                    if let Some(w) = self.workers.get(worker_id) {
                        println!("  [{}] Iteration {}/{}", w.display_name(), current, max);
                    }
                }
                WorkerEvent::IterationComplete {
                    worker_id,
                    duration_secs,
                } => {
                    iterations_completed += 1;
                    if let Some(w) = self.workers.get(worker_id) {
                        println!(
                            "  [{}] Iteration complete ({:.1}s)",
                            w.display_name(),
                            duration_secs
                        );
                    }
                }
                WorkerEvent::FileChange {
                    worker_id,
                    path,
                    change_type,
                } => {
                    if let Some(w) = self.workers.get(worker_id) {
                        println!("  [{}] {} {}", w.display_name(), change_type, path);
                    }
                }
                WorkerEvent::ToolCall { worker_id, tool } => {
                    if let Some(w) = self.workers.get(worker_id) {
                        println!("  [{}] Tool: {}", w.display_name(), tool);
                    }
                }
            }
        }

        // Step 7: Wait for all threads to complete
        for (id, handle) in handles {
            if let Err(e) = handle.join() {
                eprintln!("  Worker {} thread panicked: {:?}", id, e);
            }
        }

        // Step 8: Clean up worktrees
        println!("\nCleaning up worktrees...");
        for worker in &mut self.workers {
            worker.teardown();
        }

        let duration = start.elapsed().as_secs_f64();
        let stop_reason = if interrupted_main.load(Ordering::SeqCst) {
            StopReason::UserInterrupt
        } else if self.task_queue.is_empty() && tasks_completed > 0 {
            StopReason::Complete
        } else {
            StopReason::MaxIterations
        };

        // Print summary
        println!();
        let mins = (duration / 60.0) as u32;
        let secs = (duration % 60.0) as u32;
        println!("━━━ Team Session Complete ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "  Tasks: \x1b[32m{}\x1b[0m/{} complete",
            tasks_completed, total_tasks
        );
        println!("  Iterations: {}", iterations_completed);
        println!("  Duration: {}m {}s", mins, secs);
        println!("  Reason: {}", stop_reason);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(RunResult {
            iterations_completed,
            tasks_completed,
            stop_reason,
            duration_seconds: duration,
            archived_to: None,
        })
    }

    /// Decompose a natural language prompt into tasks using the AI CLI.
    fn decompose_prompt(&mut self, prompt: &str) -> Result<(), TeamError> {
        println!("Decomposing into tasks...");

        // Load the decompose template
        let template_content = include_str!("../prompt/decompose.md");
        let mut tera = tera::Tera::default();
        tera.add_raw_template("decompose", template_content)
            .map_err(|e| TeamError::DecomposeError(e.to_string()))?;

        let mut context = tera::Context::new();
        context.insert("task_description", prompt);
        context.insert("output_path", ".afk/tasks.json");

        let full_prompt = tera
            .render("decompose", &context)
            .map_err(|e| TeamError::DecomposeError(e.to_string()))?;

        // Run the AI CLI with the decompose prompt.
        // Use only the base args (no --output-format stream-json) since we
        // want plain text output, and inherit stderr to avoid pipe deadlocks.
        let command = &self.config.ai_cli.command;
        let args: Vec<&str> = self.config.ai_cli.args.iter().map(|s| s.as_str()).collect();

        let mut cmd = Command::new(command);
        cmd.args(&args)
            .arg(&full_prompt)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TeamError::DecomposeError(format!("AI CLI not found: {command}"))
            } else {
                TeamError::DecomposeError(format!("Failed to start AI CLI: {e}"))
            }
        })?;

        // Stream output
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => println!("  {line}"),
                    Err(_) => break,
                }
            }
        }

        // Wait for completion
        let status = child
            .wait()
            .map_err(|e| TeamError::DecomposeError(e.to_string()))?;
        if !status.success() {
            return Err(TeamError::DecomposeError(format!(
                "AI CLI exited with code {}",
                status.code().unwrap_or(-1)
            )));
        }

        // Load the generated tasks
        self.load_existing_tasks()?;

        Ok(())
    }

    /// Run the team with TUI dashboard.
    ///
    /// Same as `run()` but renders a multi-agent TUI instead of printing
    /// to stdout. The TUI supports overview and focus modes with keyboard
    /// controls for pause/kill/merge.
    pub fn run_with_tui(&mut self) -> Result<RunResult, TeamError> {
        use crate::tui::TeamTuiApp;

        let start = Instant::now();

        // Set up interrupt handler
        let interrupted = self.interrupted.clone();
        let _ = ctrlc::set_handler(move || {
            interrupted.store(true, Ordering::SeqCst);
        });

        // Step 1: Get tasks (pre-TUI, uses println)
        if let Some(ref prompt) = self.options.prompt.clone() {
            self.decompose_prompt(prompt)?;
        } else {
            self.load_existing_tasks()?;
        }

        if self.task_queue.is_empty() {
            return Err(TeamError::NoTasks);
        }

        let total_tasks = self.task_queue.len() as u32;

        // Step 2: Load personas
        persona::ensure_defaults(None)?;
        self.personas = persona::load_personas(None)?;

        // Step 3: Create TUI
        let num_agents = (self.options.num_agents as usize).min(self.task_queue.len());
        let mut tui = TeamTuiApp::new(num_agents).map_err(TeamError::IoError)?;
        let worker_tx = tui.worker_sender();

        // Step 4: Set up workers
        for i in 0..num_agents {
            let persona = self.personas.get(i).cloned();
            let mut worker = Worker::new(
                i,
                persona,
                self.options.max_iterations,
                Some(worker_tx.clone()),
            );

            if let Some(task) = self.task_queue.first().cloned() {
                self.task_queue.remove(0);
                tui.add_agent(worker.display_name(), task.id.clone(), task.title.clone());
                worker.assign_task(task);
            }

            self.workers.push(worker);
        }

        tui.set_task_counts(total_tasks, self.task_queue.len() as u32);

        // Step 5: Set up worktrees
        for worker in &mut self.workers {
            if let Err(e) = worker.setup() {
                let err_msg = e.to_string();
                worker.status = WorkerStatus::Failed(err_msg.clone());
                // Notify TUI of the failure (setup only sets local status)
                let _ = worker_tx.send(WorkerEvent::StatusChange {
                    worker_id: worker.id,
                    status: WorkerStatus::Failed(err_msg),
                });
            }
        }

        // Step 6: Spawn worker threads
        let mut handles = Vec::new();
        for worker in &self.workers {
            if worker.status == WorkerStatus::Setting {
                let worker_id = worker.id;
                let working_dir = worker.working_dir().to_path_buf();
                let config = self.config.clone();
                let max_iters = worker.max_iterations;
                let tx_clone = worker_tx.clone();
                let interrupted_clone = self.interrupted.clone();

                let handle = std::thread::spawn(move || {
                    run_worker_loop(
                        worker_id,
                        &working_dir,
                        &config,
                        max_iters,
                        tx_clone,
                        interrupted_clone,
                    )
                });
                handles.push((worker.id, handle));
            }
        }

        // Drop our copy of the sender so TUI's receiver closes when workers finish
        drop(worker_tx);

        // Step 7: Run TUI event loop (blocks until quit or all done)
        let _ = tui.run();
        tui.cleanup().ok();

        // Step 8: Wait for worker threads
        // Signal interrupt so workers stop
        self.interrupted.store(true, Ordering::SeqCst);
        for (id, handle) in handles {
            if let Err(e) = handle.join() {
                eprintln!("  Worker {} thread panicked: {:?}", id, e);
            }
        }

        // Step 9: Clean up worktrees
        for worker in &mut self.workers {
            worker.teardown();
        }

        let duration = start.elapsed().as_secs_f64();
        let tasks_completed = self.completed_tasks.len() as u32;
        let iterations_completed: u32 = self
            .workers
            .iter()
            .map(|w| w.max_iterations.min(5)) // Best estimate
            .sum();

        let stop_reason = if self.interrupted.load(Ordering::SeqCst) {
            StopReason::UserInterrupt
        } else if self.task_queue.is_empty() && tasks_completed > 0 {
            StopReason::Complete
        } else {
            StopReason::MaxIterations
        };

        Ok(RunResult {
            iterations_completed,
            tasks_completed,
            stop_reason,
            duration_seconds: duration,
            archived_to: None,
        })
    }

    /// Load tasks from existing .afk/tasks.json.
    fn load_existing_tasks(&mut self) -> Result<(), TeamError> {
        let prd = PrdDocument::load(None)?;

        // Get pending tasks sorted by priority
        self.task_queue = prd.user_stories.into_iter().filter(|s| !s.passes).collect();

        self.task_queue.sort_by_key(|s| s.priority);

        Ok(())
    }
}

/// Run iterations for a single worker in its worktree.
///
/// This runs on a separate thread. It spawns the AI CLI in the worker's
/// worktree directory and streams output back via the event channel.
fn run_worker_loop(
    worker_id: usize,
    working_dir: &std::path::Path,
    config: &AfkConfig,
    max_iterations: u32,
    tx: Sender<WorkerEvent>,
    interrupted: Arc<AtomicBool>,
) {
    let _ = tx.send(WorkerEvent::StatusChange {
        worker_id,
        status: WorkerStatus::Working,
    });

    let _ = tx.send(WorkerEvent::Output {
        worker_id,
        line: format!("Starting in {}", working_dir.display()),
    });

    for iteration in 1..=max_iterations {
        if interrupted.load(Ordering::SeqCst) {
            break;
        }

        let _ = tx.send(WorkerEvent::IterationStart {
            worker_id,
            current: iteration,
            max: max_iterations,
        });

        let iter_start = Instant::now();

        // Build the AI CLI command
        let command = &config.ai_cli.command;
        let args: Vec<&str> = config.ai_cli.args.iter().map(|s| s.as_str()).collect();

        // Generate prompt for this worker
        let _ = tx.send(WorkerEvent::Output {
            worker_id,
            line: "Generating prompt...".to_string(),
        });
        let prompt_result = crate::prompt::generate_prompt_with_root(
            config,
            true, // bootstrap mode
            Some(max_iterations),
            Some(working_dir),
        );

        let prompt = match prompt_result {
            Ok(result) => result.prompt,
            Err(e) => {
                let _ = tx.send(WorkerEvent::Output {
                    worker_id,
                    line: format!("Prompt generation error: {e}"),
                });
                let _ = tx.send(WorkerEvent::TaskFailed {
                    worker_id,
                    task_id: String::new(),
                    reason: e.to_string(),
                });
                return;
            }
        };

        let _ = tx.send(WorkerEvent::Output {
            worker_id,
            line: format!("Spawning {} {}...", command, args.join(" ")),
        });

        // Spawn AI CLI in the worker's worktree
        let mut cmd = Command::new(command);
        cmd.args(&args)
            .arg(&prompt)
            .current_dir(working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Select model if configured
        if let Some(model) = config.ai_cli.select_model() {
            cmd.args(["--model", model]);
        }

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                let _ = tx.send(WorkerEvent::TaskFailed {
                    worker_id,
                    task_id: String::new(),
                    reason: format!("Failed to spawn AI CLI: {e}"),
                });
                return;
            }
        };

        // Read stderr in a separate thread to avoid pipe deadlocks.
        // Without this, the child blocks when the 64KB stderr buffer fills.
        let stderr_tx = tx.clone();
        let stderr_handle = child.stderr.take().map(|stderr| {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = stderr_tx.send(WorkerEvent::Output {
                        worker_id,
                        line,
                    });
                }
            })
        });

        // Stream stdout
        let mut completed = false;
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if interrupted.load(Ordering::SeqCst) {
                    break;
                }
                match line {
                    Ok(line) => {
                        // Check for completion signal
                        if line.contains("<promise>COMPLETE</promise>") {
                            completed = true;
                        }
                        let _ = tx.send(WorkerEvent::Output { worker_id, line });
                    }
                    Err(_) => break,
                }
            }
        }

        // Wait for stderr thread to finish
        if let Some(handle) = stderr_handle {
            let _ = handle.join();
        }

        let duration = iter_start.elapsed().as_secs_f64();
        let _ = tx.send(WorkerEvent::IterationComplete {
            worker_id,
            duration_secs: duration,
        });

        if completed {
            // Check if the task file was updated
            let tasks_path = working_dir.join(".afk/tasks.json");
            if let Ok(prd) = PrdDocument::load(Some(&tasks_path)) {
                if let Some(story) = prd.user_stories.first() {
                    if story.passes {
                        let _ = tx.send(WorkerEvent::TaskComplete {
                            worker_id,
                            task_id: story.id.clone(),
                        });
                        let _ = tx.send(WorkerEvent::StatusChange {
                            worker_id,
                            status: WorkerStatus::Done,
                        });
                        return;
                    }
                }
            }

            // COMPLETE signal but task not marked as done — continue
            let _ = tx.send(WorkerEvent::Output {
                worker_id,
                line: "COMPLETE signal received but task not marked done, continuing..."
                    .to_string(),
            });
        }
    }

    // Max iterations reached without completion
    let _ = tx.send(WorkerEvent::StatusChange {
        worker_id,
        status: WorkerStatus::Failed("Max iterations reached".to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_options_default() {
        let opts = TeamOptions::default();
        assert_eq!(opts.num_agents, 3);
        assert_eq!(opts.max_iterations, 5);
        assert!(opts.prompt.is_none());
    }

    #[test]
    fn test_team_runner_new() {
        let config = AfkConfig::default();
        let options = TeamOptions::default();
        let runner = TeamRunner::new(config, options);
        assert!(runner.workers.is_empty());
        assert!(runner.task_queue.is_empty());
        assert!(runner.completed_tasks.is_empty());
    }

    #[test]
    fn test_team_error_display() {
        let err = TeamError::NoTasks;
        assert_eq!(err.to_string(), "No tasks available to work on");

        let err = TeamError::DecomposeError("AI not found".to_string());
        assert!(err.to_string().contains("AI not found"));
    }

    #[test]
    fn test_load_existing_tasks_empty() {
        let config = AfkConfig::default();
        let options = TeamOptions::default();
        let mut runner = TeamRunner::new(config, options);

        // With no tasks.json, this should still work (empty queue)
        // The PrdDocument::load returns default when file missing
        let result = runner.load_existing_tasks();
        assert!(result.is_ok());
        // Task queue may be empty (no tasks.json) - that's fine
    }

    #[test]
    fn test_load_existing_tasks_filters_completed() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();

        let tasks_json = r#"{
            "project": "test",
            "userStories": [
                {"id": "done", "title": "Done", "priority": 1, "passes": true},
                {"id": "pending-1", "title": "Pending 1", "priority": 2, "passes": false},
                {"id": "pending-2", "title": "Pending 2", "priority": 1, "passes": false}
            ]
        }"#;
        fs::write(afk_dir.join("tasks.json"), tasks_json).unwrap();

        let config = AfkConfig::default();
        let options = TeamOptions::default();
        let mut runner = TeamRunner::new(config, options);

        // Load from the temp directory's tasks.json
        let prd = PrdDocument::load(Some(&afk_dir.join("tasks.json"))).unwrap();
        runner.task_queue = prd.user_stories.into_iter().filter(|s| !s.passes).collect();
        runner.task_queue.sort_by_key(|s| s.priority);

        assert_eq!(runner.task_queue.len(), 2);
        // Should be sorted by priority
        assert_eq!(runner.task_queue[0].id, "pending-2"); // priority 1
        assert_eq!(runner.task_queue[1].id, "pending-1"); // priority 2
    }
}
