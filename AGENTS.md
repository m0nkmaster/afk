# Agent Instructions

**afk** — Autonomous AI coding loops, Ralph Wiggum style.

## Project Overview

This is a Rust CLI tool that implements the Ralph Wiggum pattern for autonomous AI coding. It aggregates tasks from multiple sources and generates prompts for AI coding tools.

## Development Setup

```bash
cargo build --release
cargo test -- --test-threads=1
```

## Issue Tracking

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Code Quality

Before committing, run:

```bash
cargo fmt -- --check        # Formatting
cargo clippy                # Linting
cargo test -- --test-threads=1  # Run tests
```

## Testing

This project maintains comprehensive tests. Tests are required for all changes.

Write tests first. Follow TDD where practical.

### Running Tests

**Important:** Tests must run single-threaded due to the `notify` crate's FSEvents backend on macOS. Parallel test execution can cause hangs.

```bash
cargo test -- --test-threads=1      # Run all tests (recommended)
RUST_TEST_THREADS=1 cargo test      # Alternative via env var
cargo test --release -- --test-threads=1  # Release mode
cargo test config:: -- --test-threads=1   # Run tests for config module
```

### Test Modules

Tests are inline with modules (`#[cfg(test)] mod tests`). Key test coverage:

| Module | Description |
|--------|-------------|
| `cli` | CLI parsing and command structure |
| `config` | Configuration loading and serialisation |
| `progress` | Session and task progress tracking |
| `bootstrap` | Project analysis and AI CLI detection |
| `prompt` | Tera template rendering |
| `prd` | PRD document parsing and management |
| `sources` | All source adapters (beads, json, markdown, github) |
| `parser` | Output parsing with regex patterns |
| `feedback` | Metrics collection and ASCII art |
| `watcher` | File system monitoring |
| `runner` | Loop controller and iteration runner |
| `cli_integration` | End-to-end CLI command tests (in tests/) |

### Writing Tests

1. **Use tempfile** for temporary directories
2. **Mock external calls** via Command patterns where appropriate
3. **Test edge cases** — empty inputs, missing files, error conditions
4. **Keep tests focused** — one behaviour per test

## Architecture

```
src/
├── main.rs              # Entry point
├── lib.rs               # Library exports
├── path_matcher.rs      # Shared utility for ignore patterns
├── bootstrap/
│   └── mod.rs           # Project analysis, AI CLI detection
├── cli/
│   ├── mod.rs           # Clap CLI - commands and argument handling
│   ├── output.rs        # Output formatting utilities
│   ├── update.rs        # Self-update functionality
│   └── commands/
│       ├── mod.rs       # Command module exports
│       ├── archive.rs   # Archive session management
│       ├── completions.rs # Shell completions
│       ├── config.rs    # Config show/set commands
│       ├── go.rs        # Main loop command
│       ├── import.rs    # Import PRD/tasks
│       ├── init.rs      # Project initialisation
│       ├── progress_cmd.rs # Progress display
│       ├── prompt.rs    # Prompt preview
│       ├── source.rs    # Source management
│       ├── status.rs    # Status display
│       ├── task.rs      # Task management (done/fail/reset)
│       ├── use_cli.rs   # AI CLI switching
│       └── verify.rs    # Quality gate verification
├── config/
│   ├── mod.rs           # Serde models for .afk/config.json
│   ├── field.rs         # Config field definitions
│   ├── metadata.rs      # Config metadata handling
│   └── validation.rs    # Config validation rules
├── feedback/
│   ├── mod.rs           # Module exports
│   ├── art.rs           # ASCII art spinners and mascots
│   ├── celebration.rs   # Task/session completion displays
│   ├── display.rs       # Progress display panels
│   ├── metrics.rs       # Iteration metrics collection
│   └── spinner.rs       # Inline spinner animations
├── git/
│   └── mod.rs           # Git operations (commit, archive)
├── parser/
│   ├── mod.rs           # AI CLI output parsing (regex patterns)
│   └── stream_json.rs   # Streaming JSON parser for AI CLI output
├── prd/
│   ├── mod.rs           # PRD document model
│   ├── parse.rs         # PRD parsing
│   └── store.rs         # PRD persistence and sync
├── progress/
│   ├── mod.rs           # Session and task progress tracking
│   ├── archive.rs       # Archive logic for sessions
│   └── limits.rs        # Iteration limits and constraints
├── prompt/
│   ├── mod.rs           # Tera template rendering
│   └── template.rs      # Template utilities
├── runner/
│   ├── mod.rs           # Module exports
│   ├── controller.rs    # Loop lifecycle management
│   ├── iteration.rs     # Single iteration execution
│   ├── output_handler.rs # Console output
│   └── quality_gates.rs # Lint, test, type checks
├── sources/
│   ├── mod.rs           # aggregate_tasks() dispatcher
│   ├── beads.rs         # Beads (bd) integration
│   ├── github.rs        # GitHub issues via gh CLI
│   ├── json.rs          # JSON PRD files
│   └── markdown.rs      # Markdown checklists
├── tui/
│   ├── mod.rs           # Module exports
│   ├── app.rs           # TUI application state
│   └── ui.rs            # Ratatui UI rendering
└── watcher/
    └── mod.rs           # File system monitoring (notify crate)
```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | Serialisation |
| `tera` | Template rendering |
| `regex` | Output pattern matching |
| `notify` | File system watching |
| `chrono` | Timestamps |
| `arboard` | Clipboard access |
| `ctrlc` | Signal handling |
| `ratatui` / `crossterm` | Terminal UI |
| `tokio` | Async runtime |

## Key Patterns

- **Config**: All settings in `.afk/config.json`, loaded via Serde models
- **Tasks File**: `.afk/tasks.json` is the working task list; used directly if no sources configured
- **Progress**: Session state in `.afk/progress.json`, tracks iterations, task status, and per-task learnings (short-term memory)
- **AGENTS.md**: Long-term learnings go in `AGENTS.md` at project root or in subfolders for folder-specific knowledge
- **Sources**: Pluggable adapters (beads, json, markdown, github) that sync into tasks.json
- **Prompts**: Tera templates, customisable via config
- **Runner**: Implements Ralph Wiggum pattern — spawns fresh AI CLI each iteration
- **Fresh Context**: Each iteration gets clean context; memory persists via git + progress.json + AGENTS.md
- **Quality Gates**: Feedback loops (lint, test, types) run before auto-commit
- **Archiving**: Sessions archived on completion or manually via `afk archive` (moves files to `.afk/archive/`, clears session)

## Key Commands

```bash
afk go                 # Zero-config: auto-detect PRD/sources and run
afk go 20              # Run 20 iterations
afk go --init          # Re-run setup (incl. AI CLI selection), then run
afk go --fresh         # Clear session progress and start fresh
afk status             # Show current status
afk status -v          # Verbose status with learnings
afk tasks              # List tasks from PRD
afk tasks -p           # Show only pending tasks
afk task <id>          # Show task details
afk sync               # Sync tasks from configured sources
afk verify             # Run quality gates (lint, test, types)
afk done <task-id>     # Mark task complete
afk fail <task-id>     # Mark task failed
afk reset <task-id>    # Reset stuck task
afk prompt             # Preview next prompt
afk use                # Interactively switch AI CLI
afk use claude         # Switch to a specific AI CLI
afk use --list         # List available AI CLIs
afk config show        # View current config
afk config set <key> <value>  # Set a config value
afk init --force       # Re-initialise with AI CLI selection
afk archive            # Archive session and clear (ready for fresh work)
afk archive list       # List archived sessions
```

## PRD Workflow

The recommended workflow for new projects:

```bash
afk import requirements.md  # Creates .afk/tasks.json
afk go                          # Starts working through tasks
```

When `.afk/tasks.json` exists with tasks and no sources are configured, `afk go` uses it directly as the source of truth — no configuration required.

## Adding a New Task Source

1. Create `src/sources/newsource.rs`
2. Implement `load_newsource_tasks() -> Result<Vec<UserStory>, Error>`
3. Add variant to `SourceType` enum in `config/mod.rs`
4. Add match arm to `load_from_source()` in `sources/mod.rs`
5. Add `mod newsource;` to `sources/mod.rs`
6. Write tests inline with `#[cfg(test)] mod tests`

## Releasing

Releases are automated via GitHub Actions. **Do NOT manually create releases.**

**Release workflow:**

```bash
# 1. Bump version in Cargo.toml
# 2. Commit the version bump
git add Cargo.toml Cargo.lock
git commit -m "release: X.Y.Z"

# 3. Push and create tag
git push
git tag vX.Y.Z
git push origin vX.Y.Z

# 4. WAIT for the workflow to complete (~6 minutes)
gh run watch  # Watch the release workflow progress
```

The release workflow builds binaries for all platforms and attaches them to the release. If you create the release manually with `gh release create`, users will see "no binary for this platform" until the workflow finishes.

**NEVER run `gh release create`** — let the workflow handle it.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** — Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) — Tests, linters, builds
3. **Update issue status** — Close finished work, update in-progress items
4. **PUSH TO REMOTE** — This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** — Clear stashes, prune remote branches
6. **Verify** — All changes committed AND pushed
7. **Hand off** — Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing — that leaves work stranded locally
- NEVER say "ready to push when you are" — YOU must push
- If push fails, resolve and retry until it succeeds
