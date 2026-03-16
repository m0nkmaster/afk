# afk Performance Review

> Rust CLI expert review focused on startup latency, I/O throughput, and parallelism opportunities.

---

## Executive Summary

`afk` is a well-structured Rust CLI tool, but its current architecture leaves meaningful performance on the table in three areas: **startup overhead from sequential I/O**, **inefficient process I/O streaming**, and **untapped parallelism** during multi-task runs. The recommendations below are prioritised by impact-to-effort ratio.

---

## 1. Current Performance Bottlenecks

### 1.1 Sequential Blocking I/O at Startup

Every `afk go` iteration calls `generate_prompt_with_root`, which performs three synchronous, sequential operations before any AI work begins:

```rust
// src/prompt/mod.rs – these run one after the other, blocking the thread
let mut progress = SessionProgress::load(progress_path.as_deref())?;  // disk read
let prd = PrdDocument::load(tasks_path.as_deref())?;                  // disk read
// ...
progress.save(progress_save_path.as_deref())?;                        // disk write
```

And in the main loop (`controller.rs`), `PrdDocument::load` is called **again** every iteration to detect task completion:

```rust
// src/runner/controller.rs – inside the hot loop
let mut current_prd = match PrdDocument::load(None) {
    Ok(p) => p,
    Err(_) => prd.clone(),
};
// ...
let updated_prd = PrdDocument::load(None).unwrap_or(current_prd.clone());
```

This means **3–4 JSON file reads per iteration** even for a simple one-task session.

### 1.2 No PTY — AI CLIs Lose Interactivity

The subprocess is spawned with `Stdio::piped()` on all three handles:

```rust
// src/runner/iteration.rs
cmd.args(&args)
    .arg(prompt)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
```

Many AI CLIs (Claude, Cursor) detect whether stdout is a terminal. When they detect a plain pipe, they:
- Disable colour output, progress bars, and streaming indicators
- Sometimes buffer their output more aggressively (full buffering vs line buffering)
- May switch to a lower-bandwidth output format

The result is that the current pipeline adds **artificial latency** between the AI producing output and `afk` processing it, and may disable AI CLI streaming features that would otherwise give faster perceived response.

### 1.3 Duplicate Output Streaming Logic

`controller.rs` contains a complete copy of the iteration streaming loop (`run_iteration_with_tui`) that is largely a reimplementation of what `IterationRunner::execute_command` in `iteration.rs` already does. This means bug fixes and performance improvements must be applied in two places, and the code paths diverge in subtle ways. Neither path currently uses async I/O.

### 1.4 Quality Gates Run Sequentially

```rust
// src/runner/quality_gates.rs
for (name, cmd) in gates {
    let gate_result = run_single_gate(&name, &cmd, verbose);
    result.add_gate(gate_result);
}
```

Lint, type-check, and test gates are completely independent — they can all run at the same time. On a project with `cargo clippy`, `cargo test`, and a custom `prettier --check`, sequential execution wastes wall-clock time equal to `(N-1)` gate durations.

### 1.5 Source Aggregation Is Sequential

```rust
// src/sources/mod.rs
pub fn aggregate_tasks(sources: &[SourceConfig]) -> Vec<UserStory> {
    sources.iter().flat_map(load_from_source).collect()
}
```

Each source (beads, GitHub, JSON) can involve subprocess calls (`bd`, `gh`) or network requests. These are fully independent and could run in parallel.

### 1.6 Tera Template Re-initialised Every Iteration

```rust
// src/prompt/mod.rs – called every iteration
let mut tera = Tera::default();
tera.add_raw_template("prompt", &template_str)?;
let prompt = tera.render("prompt", &context)?;
```

`Tera::default()` allocates and compiles the template every single iteration. Template compilation is not free — it involves parsing, AST construction, and bytecode generation.

### 1.7 Output Buffer: `Vec<String>` + `concat()` Anti-pattern

```rust
// src/runner/iteration.rs and controller.rs
let mut output_buffer = Vec::new();
// ...
output_buffer.push(format!("{line}\n"));
// ...
let output = output_buffer.concat();
```

This allocates one `String` per line of AI output, then concatenates them all into a second large `String`. For a 10,000-line response, this is ~10,000 small allocations. Prefer a single `String` grown with `push_str`.

### 1.8 reqwest Blocking Client in Binary

`Cargo.toml` pulls in `reqwest` with the `blocking` feature for the self-update path. The blocking client starts its own Tokio runtime internally. Since the binary already has a Tokio runtime (`tokio = { features = ["full"] }`), this creates two runtimes — one of which sits idle for the entire session.

---

## 2. Recommended Optimisations

### 2.1 (HIGH IMPACT) Use a PTY for AI CLI Subprocess

Replace `Stdio::piped()` with a PTY so AI CLIs see a real terminal. The `portable-pty` crate provides cross-platform PTY support.

**Why this matters:** Claude and Cursor CLIs stream output differently when attached to a TTY. Line buffering on the AI CLI side means lower latency between tokens being produced and `afk` reading them. Some CLIs also enable richer structured output only when a TTY is present.

```toml
# Cargo.toml
portable-pty = "0.8"
```

```rust
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

fn spawn_with_pty(cmd_parts: &[String], prompt: &str) -> Result<Box<dyn std::io::Read + Send>> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 50,
        cols: 220,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&cmd_parts[0]);
    cmd.args(&cmd_parts[1..]);
    cmd.arg(prompt);

    let _child = pair.slave.spawn_command(cmd)?;
    // pair.master implements Read — stream it exactly like Stdio::piped()
    Ok(pair.master.try_clone_reader()?)
}
```

The `slave` end acts as the AI CLI's stdin/stdout/stderr. Because the PTY presents as a TTY, the AI CLI enables interactive output. Read from `master` as normal.

**Caveat:** PTY output includes ANSI escape sequences. Strip them before passing to your NDJSON parser with a small state machine or the `strip-ansi-escapes` crate.

### 2.2 (HIGH IMPACT) Parallelise Quality Gates with Rayon or Threads

```rust
// src/runner/quality_gates.rs — replace the sequential for loop
use std::thread;

pub fn run_quality_gates(feedback_loops: &FeedbackLoopsConfig, verbose: bool) -> QualityGateResult {
    let gates = collect_gates(feedback_loops);

    if gates.is_empty() {
        println!("\x1b[2mNo quality gates configured.\x1b[0m");
        return QualityGateResult::new();
    }

    println!("\n\x1b[1mRunning quality gates in parallel...\x1b[0m\n");

    // Spawn all gates concurrently
    let handles: Vec<_> = gates
        .into_iter()
        .map(|(name, cmd)| {
            thread::spawn(move || run_single_gate(&name, &cmd, verbose))
        })
        .collect();

    let mut result = QualityGateResult::new();
    for handle in handles {
        let gate_result = handle.join().expect("gate thread panicked");
        let status = if gate_result.passed { "\x1b[32m✓\x1b[0m" } else { "\x1b[31m✗\x1b[0m" };
        println!("  {} {} ({:.1}s)", status, gate_result.name, gate_result.duration_seconds);
        result.add_gate(gate_result);
    }

    result
}
```

For three typical gates (clippy ~2 s, tests ~5 s, format ~0.5 s), this reduces wall time from **~7.5 s → ~5 s** (limited by the slowest gate).

### 2.3 (HIGH IMPACT) Cache the Tera Template Across Iterations

Move template compilation out of `generate_prompt_with_root` into a `OnceLock` or pass a compiled `Tera` instance through `AfkConfig`:

```rust
// src/prompt/mod.rs
use std::sync::OnceLock;

static COMPILED_TEMPLATE: OnceLock<(String, Tera)> = OnceLock::new();

fn get_compiled_tera(template_str: &str) -> Result<&'static Tera, tera::Error> {
    // Recompile only if template string changed (e.g., user edited custom template)
    // For simplicity, compile once per process lifetime
    let (_, tera) = COMPILED_TEMPLATE.get_or_try_init(|| {
        let mut tera = Tera::default();
        tera.add_raw_template("prompt", template_str)?;
        Ok::<_, tera::Error>((template_str.to_string(), tera))
    })?;
    Ok(tera)
}
```

For a 20-iteration session, this eliminates 19 template compilations.

### 2.4 (MEDIUM IMPACT) Parallelise Source Loading

```rust
// src/sources/mod.rs
use std::thread;

pub fn aggregate_tasks(sources: &[SourceConfig]) -> Vec<UserStory> {
    if sources.len() <= 1 {
        return sources.iter().flat_map(load_from_source).collect();
    }

    let handles: Vec<_> = sources
        .iter()
        .cloned()
        .map(|source| thread::spawn(move || load_from_source(&source)))
        .collect();

    handles
        .into_iter()
        .flat_map(|h| h.join().unwrap_or_default())
        .collect()
}
```

When using beads + GitHub + JSON simultaneously, this turns three sequential process/network calls into one parallel batch.

### 2.5 (MEDIUM IMPACT) Reduce Disk I/O in the Hot Loop

The main loop reads `tasks.json` twice per iteration. Introduce a simple change-detection mechanism using the file's last-modified timestamp:

```rust
// src/runner/controller.rs
use std::time::SystemTime;

struct PrdCache {
    prd: PrdDocument,
    last_modified: SystemTime,
    path: PathBuf,
}

impl PrdCache {
    fn refresh_if_changed(&mut self) -> &PrdDocument {
        let mtime = fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if mtime > self.last_modified {
            if let Ok(prd) = PrdDocument::load(Some(&self.path)) {
                self.prd = prd;
                self.last_modified = mtime;
            }
        }
        &self.prd
    }
}
```

This replaces `PrdDocument::load(None)` in the loop with a stat + conditional read, reducing the common case from a full JSON parse to a single `stat()` syscall.

### 2.6 (MEDIUM IMPACT) Switch Output Buffer to a Single Growing String

```rust
// src/runner/iteration.rs — replace Vec<String> accumulator
let mut output = String::with_capacity(64 * 1024); // 64 KB initial capacity

// In the loop, replace:
//   output_buffer.push(format!("{line}\n"));
// With:
output.push_str(&line);
output.push('\n');

// Remove the final:
//   let output = output_buffer.concat();
// `output` is already the complete string
```

This eliminates `O(N)` small allocations for an `N`-line response, replacing them with amortised `O(1)` growth of a single buffer.

### 2.7 (MEDIUM IMPACT) Git Worktrees for Parallel Task Execution

For projects where tasks are independent (e.g., different feature areas), `afk` could run multiple AI CLI instances simultaneously, each in its own git worktree:

```rust
// New module: src/runner/worktree.rs
use std::process::Command;
use std::path::PathBuf;

pub struct WorktreeGuard {
    pub path: PathBuf,
    branch: String,
}

impl WorktreeGuard {
    /// Create a new worktree at `<project>/.afk/worktrees/<branch>`.
    pub fn create(branch: &str, base_branch: &str) -> Result<Self, anyhow::Error> {
        let path = PathBuf::from(format!(".afk/worktrees/{}", branch));
        std::fs::create_dir_all(&path)?;

        Command::new("git")
            .args(["worktree", "add", "-b", branch, path.to_str().unwrap(), base_branch])
            .status()?;

        Ok(Self { path, branch: branch.to_string() })
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let _ = Command::new("git")
            .args(["worktree", "remove", "--force", self.path.to_str().unwrap()])
            .status();
        let _ = Command::new("git")
            .args(["branch", "-D", &self.branch])
            .status();
    }
}
```

A parallel runner would:
1. Split the pending task list across N worktrees
2. Run one `afk` iteration per worktree concurrently
3. Merge branches back (or open PRs) on completion

This is the most impactful option for large task lists, but requires careful conflict resolution. Start with independent tasks (e.g., tasks tagged `parallel-safe`).

**Example orchestration sketch:**

```rust
// src/runner/parallel.rs
pub fn run_parallel(config: &AfkConfig, tasks: Vec<UserStory>, degree: usize) -> Vec<RunResult> {
    let chunks: Vec<_> = tasks.chunks(tasks.len().div_ceil(degree)).collect();

    let handles: Vec<_> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let branch = format!("afk-parallel-{i}-{}", chrono::Utc::now().timestamp());
            let guard = WorktreeGuard::create(&branch, "main").expect("worktree create failed");
            let config = config.clone();
            let tasks = chunk.to_vec();

            std::thread::spawn(move || {
                let _guard = guard; // keep alive; drop removes worktree
                run_iteration_in_dir(&config, &tasks, &guard.path)
            })
        })
        .collect();

    handles.into_iter().map(|h| h.join().unwrap()).collect()
}
```

### 2.8 (LOW IMPACT) Eliminate the Blocking reqwest Runtime

Replace the blocking reqwest usage in `cli/update.rs` with an async call, or restructure so the binary only has one Tokio runtime:

```toml
# Cargo.toml — remove "blocking" feature
reqwest = { version = "0.12", features = ["json"] }
```

```rust
// Use tokio::spawn / async fn for the update check
// or run the check lazily after the main loop completes
```

This reduces startup overhead by avoiding the creation of a second hidden thread pool.

### 2.9 (LOW IMPACT) Pre-size HashMaps with Known Capacity

```rust
// src/prompt/mod.rs — pre-size to avoid rehashing
let gate_count = [
    config.feedback_loops.types.is_some(),
    config.feedback_loops.lint.is_some(),
    config.feedback_loops.test.is_some(),
    config.feedback_loops.build.is_some(),
].iter().filter(|&&b| b).count() + config.feedback_loops.custom.len();

let mut feedback_loops: HashMap<String, String> = HashMap::with_capacity(gate_count);
```

### 2.10 (LOW IMPACT) Deduplicate Streaming Logic

`controller.rs` (`run_iteration_with_tui`) and `iteration.rs` (`execute_command`) contain near-identical output streaming loops. Merging them into a single `stream_output` function with a trait-based sink reduces binary size, compile time, and maintenance burden:

```rust
trait OutputSink: Send {
    fn on_line(&mut self, display: Option<String>, tui: Option<TuiEvent>);
}

struct ConsoleSink<'a>(&'a mut OutputHandler);
struct TuiSink(mpsc::Sender<TuiEvent>);

fn stream_output(
    stdout: impl Read,
    parser: Option<&mut StreamJsonParser>,
    sink: &mut dyn OutputSink,
) -> (bool, String) { /* unified streaming logic */ }
```

---

## 3. Prioritised Next Steps

| Priority | Change | Estimated Speedup | Effort |
|----------|--------|-------------------|--------|
| **P0** | PTY-based subprocess spawning (2.1) | Eliminates buffering latency; unlocks AI CLI streaming features | Medium (2–3 days) |
| **P0** | Parallel quality gates (2.2) | Saves `(N-1)` × avg gate time per iteration | Low (1 day) |
| **P1** | Cache compiled Tera template (2.3) | Removes template re-parse overhead on every iteration | Low (half day) |
| **P1** | Reduce hot-loop disk I/O via mtime cache (2.5) | Replaces 2 JSON parses per iteration with `stat()` | Low (1 day) |
| **P1** | Single-string output buffer (2.6) | Reduces allocator pressure for large AI responses | Low (hours) |
| **P2** | Parallel source loading (2.4) | Cuts source sync time when using multiple adapters | Low (1 day) |
| **P2** | Git worktrees for parallel task execution (2.7) | Can achieve N× throughput for independent tasks | High (1–2 weeks) |
| **P3** | Deduplicate streaming logic (2.10) | Compile-time improvement; correctness benefit | Medium (2–3 days) |
| **P3** | Async reqwest (2.8) | Removes hidden runtime overhead | Low (hours) |
| **P3** | Pre-size HashMaps (2.9) | Micro-optimisation | Low (minutes) |

---

## 4. Quick Win Checklist

These changes require < 30 minutes each and have no risk:

- [ ] Replace `Vec<String>` output buffer with `String::with_capacity(64 * 1024)`
- [ ] Add `HashMap::with_capacity` to feedback loops builder
- [ ] Extract `run_single_gate` calls into `thread::spawn` joinset
- [ ] Wrap `Tera` in `OnceLock` to avoid re-compilation

---

## 5. Measurement Plan

Before and after each change, benchmark with:

```bash
# Startup to first AI CLI spawn (use a no-op AI CLI mock)
hyperfine --warmup 3 'afk go --dry-run'

# Full iteration wall time (with a fast mock AI CLI that returns immediately)
time afk go 1

# Quality gate parallel speedup
hyperfine 'afk verify'
```

Add `criterion` benchmarks for `generate_prompt_with_root` and `aggregate_tasks` to track regressions in CI.

---

## References

- [`portable-pty`](https://crates.io/crates/portable-pty) — cross-platform PTY support
- [`strip-ansi-escapes`](https://crates.io/crates/strip-ansi-escapes) — ANSI escape stripping
- [`rayon`](https://crates.io/crates/rayon) — data parallelism for Rust iterators
- [Git worktrees documentation](https://git-scm.com/docs/git-worktree)
- [Tera template caching patterns](https://tera.netlify.app/docs/#global-functions)
