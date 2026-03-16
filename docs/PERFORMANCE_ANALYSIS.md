# Performance Optimization Analysis for afk

**Date:** 2026-03-16  
**Author:** Code Review (Rust/CLI Expert)  
**Status:** Analysis Complete - Recommendations Below

---

## Executive Summary

This document analyzes the `afk` CLI for performance optimization opportunities, focusing on three specific areas:
1. Git worktrees for parallel task execution
2. PTY pseudo-terminals for improved AI CLI interaction
3. Other CLI performance optimizations

**Key Finding:** The current sequential Ralph Wiggum pattern is intentional and correct for the design. However, there are several optimization opportunities that could significantly improve performance, especially around PTY usage and I/O efficiency.

---

## 1. Git Worktrees - Analysis

### Current State
- **Not implemented.** The codebase does not use git worktrees at all.
- All iterations run in the same working directory.

### Can Worktrees Speed Up Parallel Execution?

**Short answer: Yes, but with caveats.**

The Ralph Wiggum pattern is fundamentally **sequential by design** - each task gets a fresh AI instance with clean context. This is the core innovation and shouldn't be changed lightly.

However, worktrees could enable **optional parallel execution** for independent tasks:

```
Worktree Use Case:
├── main (main branch - holds completed work)
├── afk-task-001 (worktree for independent task)
├── afk-task-002 (worktree for independent task)  
└── afk-task-003 (worktree for independent task)
```

### Recommendations

| Priority | Recommendation | Rationale |
|----------|---------------|-----------|
| **Low** | Consider worktrees as an **opt-in parallel mode** | Would require significant architectural changes. Only beneficial for truly independent tasks. |
| **Medium** | Document that worktrees are not currently used | Users may assume they're needed; clarify current design |

### Risk Assessment
- **Complexity:** High - would require significant refactoring
- **Breaking change:** No - would be opt-in
- **ROI:** Medium - only helps with parallel workloads

---

## 2. PTY Pseudo-Terminals - Analysis

### Current State
**Not implemented.** In `src/runner/iteration.rs`:

```rust
let mut cmd = Command::new(command);
cmd.args(&args)
    .arg(prompt)
    .stdin(Stdio::null())       // ← No PTY
    .stdout(Stdio::piped())     // ← Piped, not PTY
    .stderr(Stdio::piped());    // ← Piped, not PTY
```

### Why PTY Matters

Many AI CLIs (Claude Code, Cursor, OpenAI CLI) **behave differently when running without a TTY**:

| CLI | TTY Behavior |
|-----|--------------|
| Claude Code | Enables enhanced output formatting, progress indicators |
| Cursor Agent | Enables streaming output, interactive prompts |
| OpenAI CLI | Enables spinner animations, real-time feedback |
| Aider | Enables terminal control sequences |

### Benefits of PTY Implementation

1. **Better output streaming** - AI CLIs often buffer output when not connected to a terminal
2. **Interactive features** - Some CLIs offer TTY-specific enhancements
3. **Signal handling** - PTY enables proper SIGINT/SIGTSTP propagation
4. **Terminal detection** - Many CLIs check `isatty()` and behave differently

### Implementation Approach

Using the `portable-pty` crate (cross-platform):

```rust
use portable_pty::{native_pty_system, CommandBuilder, MasterPty};

pub fn spawn_with_pty(command: &str, args: &[&str]) -> Result<Box<dyn Child>> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty()?;
    
    let mut cmd = CommandBuilder::new(command);
    cmd.args(args);
    cmd.stdin(&pair.slave);
    cmd.stdout(&pair.slave);
    cmd.stderr(&pair.slave);
    
    // Set PTY size
    pair.master.resize(pty::PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    
    let child = pair.slave.spawn_command(cmd)?;
    Ok(child)
}
```

### Recommendations

| Priority | Recommendation | Rationale |
|----------|---------------|-----------|
| **High** | Add optional PTY mode via config flag | Low risk, high potential benefit |
| **Medium** | Detect TTY availability and auto-enable | Better UX |
| **Medium** | Use `portable-pty` crate for cross-platform support | Already used byCursor, etc. |

### Risk Assessment
- **Complexity:** Medium - requires handling PTY master/slave
- **Breaking change:** No - opt-in via flag
- **ROI:** High - improves output handling for most AI CLIs

---

## 3. Other CLI Performance Optimizations

### 3.1 Prompt Caching

**Current:** Fresh prompt generated every iteration (`src/prompt/mod.rs`)

**Issue:** Prompt generation involves:
- Loading PRD from disk
- Parsing Tera templates
- Rendering context (project analysis, AGENTS.md, etc.)

**Optimization:** Cache prompts when PRD hasn't changed:

```rust
// Pseudocode - add caching layer
fn get_cached_prompt(config: &AfkConfig, prd: &PrdDocument) -> String {
    let prd_hash = hash(prd);
    if let Some(cached) = prompt_cache.get(&prd_hash) {
        return cached;
    }
    let prompt = generate_prompt(config, prd);
    prompt_cache.insert(prd_hash, prompt);
    prompt
}
```

**Priority:** Medium - 10-30% iteration time reduction estimated

### 3.2 Incremental PRD Reloading

**Current:** Full PRD reload every iteration in `controller.rs`:

```rust
// Reload PRD to check completion
let mut current_prd = match PrdDocument::load(None) {
    Ok(p) => p,
    Err(_) => prd.clone(),
};
```

**Optimization:** Track file modification time, only reload if changed:

```rust
fn load_prd_if_changed() -> Option<PrdDocument> {
    let path = Path::new(".afk/tasks.json");
    let mtime = path.metadata()?.modified().ok()?;
    if mtime > last_loaded_mtime {
        last_loaded_mtime = mtime;
        Some(PrdDocument::load(None)?)
    } else {
        None
    }
}
```

**Priority:** Low-Medium - depends on file system performance

### 3.3 Async I/O for File Operations

**Current:** Synchronous file I/O throughout

**Opportunity:** Use `tokio::fs` for async file operations (already using tokio):

```rust
// Current
let prd = PrdDocument::load(Some(path))?;

// Potential (async)
let prd = tokio::fs::read_to_string(path)
    .await?
    .parse::<PrdDocument>()?;
```

**Priority:** Low - limited benefit unless many concurrent file ops

### 3.4 Process Group for Signal Handling

**Current:** Direct child process management

**Improvement:** Use process groups for better signal propagation:

```rust
use std::os::unix::process::CommandExt;

let mut cmd = Command::new(command);
cmd.process_group(0); // Create new process group
// This ensures Ctrl+C propagates to all child processes
```

**Priority:** Medium - improves interrupt handling

### 3.5 Connection Pooling for HTTP (GitHub API)

**Current:** Likely creates new connection per GitHub API call

**Improvement:** Use `reqwest` with connection pooling:

```rust
// In sources/github.rs
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_github_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(5)
            .build()
            .unwrap()
    })
}
```

**Priority:** Low-Medium - only matters with frequent GitHub API calls

### 3.6 Parallel Quality Gates (When Implemented)

If quality gates (lint, test, type check) are run sequentially:

```rust
// Current (sequential)
run_linter()?;
run_type_checker()?;
run_tests()?;

// Potential (parallel with tokio)
tokio::try_join!(
    tokio::spawn(run_linter()),
    tokio::spawn(run_type_checker()),
    tokio::spawn(run_tests())
)?;
```

**Priority:** Low - quality gates may already be parallel

---

## Summary of Recommendations

### High Priority

| # | Recommendation | Est. Impact | Complexity |
|---|----------------|-------------|------------|
| 1 | Add optional PTY mode | High (output quality) | Medium |
| 2 | Add TTY auto-detection | Medium (UX) | Low |

### Medium Priority

| # | Recommendation | Est. Impact | Complexity |
|---|----------------|-------------|------------|
| 3 | Implement prompt caching | 10-30% time | Medium |
| 4 | Incremental PRD reloading | 5-15% I/O | Medium |
| 5 | Process group for signals | Reliability | Low |

### Low Priority

| # | Recommendation | Est. Impact | Complexity |
|---|----------------|-------------|------------|
| 6 | Async file I/O | Minimal | High |
| 7 | HTTP connection pooling | Minimal | Low |
| 8 | Worktree-based parallelism | Varies | High |

---

## Next Steps

1. **Start with PTY implementation** - Highest ROI, lowest risk
2. **Add prompt caching** - Significant speedup for repeated iterations
3. **Consider worktrees only if explicitly requested** - Major architectural change

---

## Appendix: Code References

- Iteration execution: `src/runner/iteration.rs`
- Loop controller: `src/runner/controller.rs`
- PTY would be added in: `execute_command()` method
- Prompt generation: `src/prompt/mod.rs`
- PRD handling: `src/prd/mod.rs`