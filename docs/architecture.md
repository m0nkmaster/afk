# afk Architecture

Technical overview of the afk codebase for contributors and developers.

## Overview

afk is a Rust CLI tool that implements the Ralph Wiggum pattern for autonomous AI coding. It aggregates tasks from multiple sources and generates prompts for AI coding tools.

## Project Structure

```
src/
├── main.rs          # Entry point, CLI dispatch
├── lib.rs           # Library exports
├── cli/             # CLI layer
│   ├── mod.rs       # Clap CLI definitions
│   ├── commands/    # Subcommand implementations
│   │   ├── mod.rs   # Module exports
│   │   ├── archive.rs
│   │   ├── completions.rs
│   │   ├── config.rs
│   │   ├── go.rs
│   │   ├── import.rs
│   │   ├── init.rs
│   │   ├── progress_cmd.rs
│   │   ├── prompt.rs
│   │   ├── source.rs
│   │   ├── status.rs
│   │   ├── task.rs
│   │   ├── use_cli.rs
│   │   └── verify.rs
│   ├── output.rs    # Output formatting (clipboard, file, stdout)
│   └── update.rs    # Self-update logic
├── config/          # Configuration layer
│   ├── mod.rs       # Serde models for .afk/config.json
│   ├── field.rs     # Field-level access and manipulation
│   ├── metadata.rs  # Config metadata and documentation
│   └── validation.rs # Config validation
├── bootstrap/       # Project analysis
│   └── mod.rs       # Project type detection, AI CLI detection
├── progress/        # Session tracking
│   ├── mod.rs       # SessionProgress, TaskProgress models
│   ├── limits.rs    # Iteration limits and constraints
│   └── archive.rs   # Session archiving
├── prompt/          # Prompt generation
│   ├── mod.rs       # Tera template rendering
│   └── template.rs  # Template utilities
├── prd/             # PRD management
│   ├── mod.rs       # PrdDocument model
│   ├── parse.rs     # PRD parsing
│   └── store.rs     # PRD persistence and sync
├── runner/          # Execution engine
│   ├── mod.rs       # Module exports
│   ├── controller.rs # Loop lifecycle management
│   ├── iteration.rs  # Single iteration execution
│   ├── output_handler.rs # Console output
│   └── quality_gates.rs  # Lint, test, type checks
├── git/             # Git integration
│   └── mod.rs       # Commit and archive operations
├── feedback/        # User feedback
│   ├── mod.rs       # Module exports
│   ├── metrics.rs   # Iteration metrics collection
│   ├── display.rs   # Progress display
│   ├── art.rs       # ASCII art mascots
│   └── spinner.rs   # Spinner animations
├── parser/          # Output parsing
│   ├── mod.rs       # AI CLI output parsing (regex patterns)
│   └── stream_json.rs # Streaming JSON parser for AI CLI output
├── watcher/         # File watching
│   └── mod.rs       # File system monitoring (notify crate)
├── tui/             # Terminal UI
│   ├── mod.rs       # Module exports
│   ├── app.rs       # TUI application state
│   └── ui.rs        # Ratatui UI rendering
└── sources/         # Task sources
    ├── mod.rs       # aggregate_tasks() dispatcher
    ├── beads.rs     # Beads (bd) integration
    ├── json.rs      # JSON PRD files
    ├── markdown.rs  # Markdown checklists
    └── github.rs    # GitHub issues via gh CLI
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `serde` / `serde_json` | Serialisation for config and data files |
| `tera` | Jinja2-style template rendering for prompts |
| `regex` | Output pattern matching for completion signals |
| `notify` | File system watching for watch mode |
| `chrono` | Timestamps for progress and archives |
| `arboard` | Cross-platform clipboard access |
| `ctrlc` | Signal handling for graceful shutdown |
| `ratatui` / `crossterm` | Terminal UI framework for TUI mode |
| `tokio` | Async runtime |
| `reqwest` | HTTP client for self-update |
| `anyhow` / `thiserror` | Error handling |

## Data Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Task Sources  │────▶│  aggregate_tasks │────▶│   tasks.json    │
│ (beads, json,   │     │                 │     │ (unified tasks) │
│  markdown, gh)  │     └─────────────────┘     └─────────────────┘
└─────────────────┘                                      │
                                                         ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ progress.json   │◀────│   Runner        │◀────│ generate_prompt │
│ (session state) │     │ (loop control)  │     │ (Tera templates)│
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                      │
         ▼                      ▼
┌─────────────────┐     ┌─────────────────┐
│  Git commits    │     │  AI CLI spawn   │
│ (memory layer)  │     │ (clean context) │
└─────────────────┘     └─────────────────┘
```

## Key Patterns

### Configuration Loading

All settings live in `.afk/config.json`. The `AfkConfig` struct uses Serde for serialisation with sensible defaults:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfkConfig {
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub feedback_loops: FeedbackLoopsConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    // ...
}
```

### Task Aggregation

Sources implement a common pattern returning `Vec<UserStory>`:

```rust
pub fn aggregate_tasks(sources: &[SourceConfig]) -> Vec<UserStory> {
    let mut all_tasks = Vec::new();
    for source in sources {
        match load_from_source(source) {
            Ok(tasks) => all_tasks.extend(tasks),
            Err(e) => eprintln!("Warning: Failed to load from {:?}: {}", source.source_type, e),
        }
    }
    all_tasks
}
```

### Prompt Generation

Prompts use Tera templates with context injection:

```rust
pub fn generate_prompt(config: &AfkConfig, bootstrap: bool, limit: Option<u32>) 
    -> Result<PromptResult, PromptError> 
{
    let mut context = Context::new();
    context.insert("story", &next_story);
    context.insert("learnings", &learnings);
    context.insert("iteration", &iteration);
    // ...
    let prompt = tera.render("prompt.tera", &context)?;
    Ok(PromptResult { prompt, iteration, all_complete })
}
```

### The Ralph Wiggum Loop

The runner implements the core pattern:

```rust
pub fn run_loop(config: &AfkConfig, iterations: Option<u32>, ...) -> LoopResult {
    loop {
        // 1. Load current state
        let prd = PrdDocument::load(None)?;
        let mut progress = SessionProgress::load(None)?;
        
        // 2. Check stop conditions
        if progress.iterations >= max_iterations { break; }
        if prd.all_complete() { break; }
        
        // 3. Generate prompt
        let prompt = generate_prompt(config, false, None)?;
        
        // 4. Spawn fresh AI CLI
        let output = spawn_ai_cli(&config.ai_cli, &prompt)?;
        
        // 5. Run quality gates
        let gates = run_quality_gates(&config.feedback_loops)?;
        
        // 6. Auto-commit if gates pass
        if gates.all_passed {
            git_commit(&story.id)?;
        }
        
        // 7. Update progress
        progress.iterations += 1;
        progress.save(None)?;
    }
}
```

### Quality Gates

Gates run sequentially and report results:

```rust
pub fn run_quality_gates(config: &FeedbackLoopsConfig, verbose: bool) -> GatesResult {
    let mut results = Vec::new();
    
    if let Some(ref cmd) = config.types {
        results.push(run_gate("types", cmd, verbose));
    }
    if let Some(ref cmd) = config.lint {
        results.push(run_gate("lint", cmd, verbose));
    }
    // ...
    
    GatesResult {
        all_passed: results.iter().all(|r| r.passed),
        results,
    }
}
```

## Testing Strategy

Tests are inline with modules using `#[cfg(test)] mod tests`. Key categories:

| Category | Location | Description |
|----------|----------|-------------|
| Unit tests | `src/*/mod.rs` | Per-module tests |
| Integration tests | `tests/cli_integration.rs` | Full CLI invocation |
| Prompt snapshots | `tests/prompt_snapshots.rs` | Template output verification |
| Benchmarks | `benches/benchmarks.rs` | Performance testing |

### Test Execution

Tests must run single-threaded due to the `notify` crate's FSEvents backend on macOS:

```bash
cargo test -- --test-threads=1
```

## Error Handling

The codebase uses a combination of:

- `anyhow::Result` for application-level errors
- `thiserror` for domain-specific error types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Source file not found: {0}")]
    FileNotFound(String),
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    // ...
}
```

## Adding Features

### New Command

1. Add args struct and variant to `cli/mod.rs`
2. Implement `execute()` method on the struct
3. Add match arm in `main.rs`
4. Write tests

### New Task Source

1. Create `src/sources/newsource.rs`
2. Implement `load_newsource_tasks() -> Result<Vec<UserStory>, SourceError>`
3. Add variant to `SourceType` enum in `config/mod.rs`
4. Add match arm to `load_from_source()` in `sources/mod.rs`
5. Add `mod newsource;` to `sources/mod.rs`
6. Write tests

### New Quality Gate

1. Add field to `FeedbackLoopsConfig` in `config/mod.rs`
2. Add check in `run_quality_gates()` in `runner/quality_gates.rs`
3. Update documentation

## Performance Considerations

- **Startup time**: Rust provides fast cold start (~10ms)
- **Memory**: Single-pass processing, no persistent runtime
- **I/O**: Async file operations via tokio where beneficial
- **Binary size**: ~5-10MB depending on platform and optimisation

## Cross-Platform Support

Builds target:

- Linux (x86_64, arm64)
- macOS (Intel, Apple Silicon)  
- Windows (x86_64)

Platform-specific code is minimised; most functionality uses cross-platform crates.
