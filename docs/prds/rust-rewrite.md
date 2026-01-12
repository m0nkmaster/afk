# PRD: afk Rust Rewrite

## Executive Summary

**Objective**: Rewrite the `afk` CLI application from Python to Rust to achieve faster startup times, improved cross-platform performance, and simpler distribution as a single statically-linked binary.

**Current State**: Python 3.11+ application (~4,500 lines) using Click, Pydantic, Jinja2, Rich, and Watchdog. Distributed via pip/pipx or compiled standalone binary using Nuitka.

**Target State**: Native Rust application with equivalent functionality, sub-100ms startup time, single-binary distribution for Linux, macOS, and Windows.

**Version**: This PRD targets afk v1.0.0 (Rust edition), superseding the current Python v0.3.x series.

---

## Motivation

### Performance Issues with Current Implementation

1. **Slow Startup Time**: The Python application has noticeable startup latency (300-800ms), especially when using the Nuitka-compiled binary. This is felt on every `afk` command invocation.

2. **Cross-Platform Inconsistency**: Python's subprocess handling, path operations, and terminal output behave differently across platforms (particularly Windows).

3. **Distribution Complexity**: 
   - pip/pipx requires Python to be installed
   - Nuitka compilation produces large binaries (~50MB+) with slow build times
   - The compiled binary still depends on system libraries

4. **Runtime Dependencies**: Python's GIL and interpreter overhead add latency to file watching, subprocess spawning, and output streaming.

### Benefits of Rust

1. **Instant Startup**: Rust binaries typically start in <10ms
2. **Single Binary**: Statically linked, no runtime dependencies
3. **Small Size**: Expect 5-15MB binary (stripped, compressed)
4. **True Parallelism**: Native threading for file watching and output streaming
5. **Consistent Cross-Platform**: Same behaviour on Linux, macOS, Windows
6. **Memory Safety**: No runtime crashes from null pointers or memory issues
7. **Easier Distribution**: Just download and run — no Python, no pip

---

## Functional Requirements

### FR1: Full Feature Parity

The Rust implementation MUST support all existing commands and features:

| Command | Description | Priority |
|---------|-------------|----------|
| `afk go` | Zero-config autonomous loop | P0 |
| `afk go N` | Run N iterations | P0 |
| `afk go -u` | Run until complete | P0 |
| `afk run N` | Run N iterations | P0 |
| `afk run --until-complete` | Run until all tasks done | P0 |
| `afk run -b BRANCH` | Create feature branch | P1 |
| `afk start` | Init + run | P1 |
| `afk resume` | Continue from last session | P1 |
| `afk init` | Auto-detect project settings | P0 |
| `afk status` | Show configuration | P1 |
| `afk explain` | Show loop state | P0 |
| `afk verify` | Run quality gates | P0 |
| `afk next` | Preview next prompt | P1 |
| `afk done ID` | Mark task complete | P0 |
| `afk fail ID` | Mark task failed | P1 |
| `afk reset ID` | Reset stuck task | P1 |
| `afk source add TYPE PATH` | Add task source | P0 |
| `afk source list` | List sources | P1 |
| `afk source remove N` | Remove source | P1 |
| `afk prd parse FILE` | Parse PRD to JSON | P0 |
| `afk prd sync` | Sync from sources | P0 |
| `afk prd show` | Show PRD state | P1 |
| `afk sync` | Alias for prd sync | P1 |
| `afk archive create` | Archive session | P2 |
| `afk archive list` | List archives | P2 |
| `afk archive clear` | Clear session | P2 |
| `afk update` | Self-update | P1 |
| `afk completions SHELL` | Generate completions | P2 |

### FR2: Task Sources

Support all existing task source types:

| Source | Method | Priority |
|--------|--------|----------|
| Beads | `bd ready --json` subprocess | P0 |
| JSON PRD | File parsing | P0 |
| Markdown | Checkbox parsing | P0 |
| GitHub | `gh issue list --json` subprocess | P1 |

### FR3: AI CLI Support

Support all existing AI CLIs with their argument patterns:

| CLI | Command | Args | Priority |
|-----|---------|------|----------|
| Claude Code | `claude` | `--dangerously-skip-permissions -p` | P0 |
| Cursor Agent | `agent` | `-p --force` | P0 |
| Codex | `codex` | `--approval-mode full-auto -q` | P1 |
| Aider | `aider` | `--yes` | P1 |
| Amp | `amp` | `--dangerously-allow-all` | P2 |
| Kiro | `kiro` | `--auto` | P2 |

### FR4: Configuration

Maintain exact compatibility with existing `.afk/config.json` schema:

```json
{
  "sources": [{"type": "beads"}, {"type": "json", "path": "prd.json"}],
  "feedback_loops": {"types": "mypy .", "lint": "ruff check .", "test": "pytest"},
  "limits": {"max_iterations": 200, "max_task_failures": 50, "timeout_minutes": 120},
  "output": {"default": "stdout", "file_path": ".afk/prompt.md"},
  "ai_cli": {"command": "claude", "args": ["--dangerously-skip-permissions", "-p"]},
  "prompt": {"template": "default", "custom_path": null, "context_files": [], "instructions": []},
  "git": {"auto_commit": true, "auto_branch": false, "branch_prefix": "afk/", "commit_message_template": "afk: {task_id} - {message}"},
  "archive": {"enabled": true, "directory": ".afk/archive", "on_branch_change": true},
  "feedback": {"enabled": true, "mode": "full", "show_files": true, "show_metrics": true, "show_mascot": true, "refresh_rate": 0.1}
}
```

### FR5: PRD Document Format

Maintain exact compatibility with existing `.afk/prd.json` schema:

```json
{
  "project": "project-name",
  "branchName": "feature/branch",
  "description": "Description",
  "userStories": [
    {
      "id": "story-id",
      "title": "Story title",
      "description": "Story description",
      "acceptanceCriteria": ["Criterion 1", "Criterion 2"],
      "priority": 1,
      "passes": false,
      "source": "beads",
      "notes": ""
    }
  ],
  "lastSynced": "2025-01-12T00:00:00.000000"
}
```

### FR6: Progress Tracking

Maintain exact compatibility with existing `.afk/progress.json` schema:

```json
{
  "started_at": "2025-01-12T00:00:00.000000",
  "iterations": 5,
  "tasks": {
    "task-id": {
      "id": "task-id",
      "source": "beads",
      "status": "in_progress",
      "started_at": "2025-01-12T00:00:00.000000",
      "completed_at": null,
      "failure_count": 0,
      "commits": [],
      "message": null,
      "learnings": ["Learning 1", "Learning 2"]
    }
  }
}
```

### FR7: Prompt Generation

Support Jinja2-compatible templating with the same default template and variables:
- `iteration`, `max_iterations`
- `completed_count`, `total_count`
- `next_story` (with `.id`, `.priority`, `.title`)
- `context_files` (list)
- `feedback_loops` (dict)
- `custom_instructions` (list)
- `bootstrap` (bool)
- `stop_signal` (string or null)

### FR8: Output Parsing

Detect the same patterns from AI CLI output streams:

| Pattern | AI CLI | Event Type |
|---------|--------|------------|
| `Calling tool: X` | Claude | Tool call |
| `Writing to: X` | Claude | File change |
| `Reading: X` | Claude | File read |
| `⏺ ToolName(` | Cursor | Tool call |
| `Edited X` | Cursor | File modified |
| `Created X` | Cursor | File created |
| `Deleted X` | Cursor | File deleted |
| `Applied edit to X` | Aider | File modified |
| `Wrote X` | Aider | File modified |
| `Traceback (most recent call last):` | Any | Error |
| `Error:`, `Exception:` | Any | Error |
| `Warning:` | Any | Warning |

### FR9: Real-Time Feedback Display

Implement equivalent terminal UI:
- Full mode: Multi-panel display with activity, files, mascot, task panels
- Minimal mode: Single-line status bar
- Spinner animations with activity states (active, thinking, stalled)
- Progress tracking with elapsed time
- Celebration animations on task completion

### FR10: File System Watching

Monitor working directory for file changes during iterations:
- Recursive directory watching
- Configurable ignore patterns (`.git`, `__pycache__`, `node_modules`, etc.)
- Deduplicated change events
- Integration with metrics collector

### FR11: Git Operations

Support all existing git operations:
- Branch detection and creation
- Auto-commit with configurable message templates
- Session archiving
- Branch change detection

### FR12: Completion Signals

Detect iteration completion from AI output:
- `<promise>COMPLETE</promise>`
- `AFK_COMPLETE`
- `AFK_STOP`

---

## Non-Functional Requirements

### NFR1: Startup Performance

| Metric | Current (Python) | Target (Rust) |
|--------|------------------|---------------|
| Cold start | 300-800ms | <50ms |
| Warm start | 200-400ms | <10ms |
| Help display | 150-300ms | <20ms |

### NFR2: Binary Size

| Platform | Current (Nuitka) | Target (Rust) |
|----------|------------------|---------------|
| Linux x86_64 | ~55MB | <10MB |
| macOS ARM64 | ~52MB | <8MB |
| macOS x86_64 | ~54MB | <10MB |
| Windows x86_64 | ~60MB | <12MB |

### NFR3: Memory Usage

| Scenario | Target |
|----------|--------|
| Idle | <5MB RSS |
| Running iteration | <50MB RSS |
| File watching active | <20MB RSS |

### NFR4: Cross-Platform Support

| Platform | Architecture | Support Level |
|----------|--------------|---------------|
| Linux | x86_64 | Full |
| Linux | aarch64 | Full |
| macOS | x86_64 | Full |
| macOS | aarch64 (Apple Silicon) | Full |
| Windows | x86_64 | Full |

### NFR5: Build Time

| Metric | Target |
|--------|--------|
| Debug build | <30s |
| Release build | <3min |
| CI full build (all targets) | <15min |

---

## Technical Architecture

### Rust Crate Structure

```
afk/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Dependency lock
├── src/
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library root (for testing)
│   ├── cli/                # CLI layer
│   │   ├── mod.rs          # CLI root
│   │   ├── commands/       # Command implementations
│   │   │   ├── go.rs       # afk go
│   │   │   ├── run.rs      # afk run
│   │   │   ├── init.rs     # afk init
│   │   │   ├── source.rs   # afk source add/list/remove
│   │   │   ├── prd.rs      # afk prd parse/sync/show
│   │   │   ├── verify.rs   # afk verify
│   │   │   ├── explain.rs  # afk explain
│   │   │   ├── done.rs     # afk done/fail/reset
│   │   │   ├── archive.rs  # afk archive
│   │   │   └── update.rs   # afk update
│   │   └── output.rs       # Console output helpers
│   ├── config/             # Configuration
│   │   ├── mod.rs
│   │   └── models.rs       # Serde models for config.json
│   ├── prd/                # PRD handling
│   │   ├── mod.rs
│   │   ├── models.rs       # UserStory, PrdDocument
│   │   └── store.rs        # Load/save/sync
│   ├── progress/           # Session progress
│   │   ├── mod.rs
│   │   ├── models.rs       # TaskProgress, SessionProgress
│   │   └── limits.rs       # Limit checking
│   ├── sources/            # Task sources
│   │   ├── mod.rs          # aggregate_tasks()
│   │   ├── beads.rs
│   │   ├── json.rs
│   │   ├── markdown.rs
│   │   └── github.rs
│   ├── prompt/             # Prompt generation
│   │   ├── mod.rs
│   │   └── template.rs     # Tera templates
│   ├── runner/             # Autonomous loop
│   │   ├── mod.rs
│   │   ├── controller.rs   # LoopController
│   │   ├── iteration.rs    # IterationRunner
│   │   ├── output_handler.rs
│   │   └── quality_gates.rs
│   ├── feedback/           # Real-time display
│   │   ├── mod.rs
│   │   ├── display.rs      # FeedbackDisplay
│   │   ├── metrics.rs      # MetricsCollector
│   │   └── art.rs          # Spinners, mascots
│   ├── watcher/            # File system watching
│   │   ├── mod.rs
│   │   └── file_watcher.rs
│   ├── parser/             # Output parsing
│   │   ├── mod.rs
│   │   └── patterns.rs     # Regex patterns
│   ├── git/                # Git operations
│   │   ├── mod.rs
│   │   └── ops.rs
│   └── bootstrap/          # Project detection
│       ├── mod.rs
│       └── detect.rs
├── tests/                  # Integration tests
│   ├── cli_tests.rs
│   ├── config_tests.rs
│   └── runner_tests.rs
└── benches/                # Benchmarks
    └── startup.rs
```

### Key Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `clap` | CLI framework | Derive API, subcommands |
| `serde` + `serde_json` | JSON serialisation | Config, PRD, progress |
| `tokio` | Async runtime | File watching, subprocess |
| `tera` | Templating | Jinja2-compatible |
| `console` + `indicatif` | Terminal UI | Progress bars, colours |
| `ratatui` | TUI | Optional: full feedback display |
| `notify` | File watching | Cross-platform |
| `regex` | Pattern matching | Output parsing |
| `reqwest` | HTTP client | Self-update |
| `chrono` | Date/time | Timestamps |
| `toml` | TOML parsing | pyproject.toml reading |
| `arboard` | Clipboard | Cross-platform clipboard |
| `which` | Command lookup | AI CLI detection |
| `directories` | Standard paths | Config, cache |

### Alternative Crate Choices

| Purpose | Option A | Option B | Recommendation |
|---------|----------|----------|----------------|
| CLI | `clap` | `argh` | `clap` — more features, better help |
| Templating | `tera` | `minijinja` | `tera` — more Jinja2 compatible |
| Terminal UI | `console`/`indicatif` | `ratatui` | Start with `console`, add `ratatui` if needed |
| File watching | `notify` | `watchman` | `notify` — pure Rust, simpler |
| Async | `tokio` | `async-std` | `tokio` — larger ecosystem |

---

## Migration Strategy

### Phase 1: Core Infrastructure (Week 1-2)

**Goal**: Working CLI skeleton with configuration loading

1. Project scaffolding with Cargo workspace
2. Configuration models (serde, JSON load/save)
3. PRD document models
4. Progress tracking models
5. Basic CLI structure with clap
6. `afk --version` and `afk --help` working

**Deliverables**:
- Compiling project
- Config loading from `.afk/config.json`
- PRD loading from `.afk/prd.json`
- Progress loading from `.afk/progress.json`

### Phase 2: Task Sources (Week 2-3)

**Goal**: All four task sources working

1. JSON PRD source
2. Markdown source
3. Beads source (subprocess to `bd`)
4. GitHub source (subprocess to `gh`)
5. Source aggregation
6. `afk source add/list/remove` commands
7. `afk prd sync` and `afk prd show`

**Deliverables**:
- All sources loading tasks
- Sources can be added/removed via CLI
- PRD sync working

### Phase 3: Prompt Generation (Week 3-4)

**Goal**: Prompt system with templates

1. Tera template engine integration
2. Default template (exact parity with Python)
3. Custom template loading
4. Context variable population
5. `afk next` command
6. `afk prd parse` command

**Deliverables**:
- Identical prompt output to Python version
- Template customisation working

### Phase 4: Runner Core (Week 4-5)

**Goal**: Basic iteration execution

1. IterationRunner — subprocess spawning
2. Output streaming with line-by-line reading
3. Completion signal detection
4. LoopController — iteration loop
5. `afk run N` command
6. `afk go` command (basic version)

**Deliverables**:
- Can spawn AI CLI and stream output
- Iterations complete and loop continues
- Completion signals detected

### Phase 5: Quality Gates (Week 5-6)

**Goal**: Feedback loop execution

1. Quality gate runner
2. Gate pass/fail detection
3. Gate output capture
4. `afk verify` command
5. Integration with runner (skip commit on fail)

**Deliverables**:
- All configured gates run
- Pass/fail reported correctly
- Runner respects gate results

### Phase 6: Git Operations (Week 6)

**Goal**: Full git integration

1. Branch detection
2. Branch creation
3. Auto-commit
4. Session archiving
5. Archive management commands

**Deliverables**:
- `afk run -b BRANCH` creates branch
- Auto-commit on gate pass
- Archives created on session end

### Phase 7: Output Parsing (Week 7)

**Goal**: AI output event detection

1. Regex pattern definitions
2. OutputParser implementation
3. Event types (tool call, file change, error, warning)
4. Integration with MetricsCollector

**Deliverables**:
- All patterns from Python version detected
- Events flow to metrics

### Phase 8: Feedback Display (Week 7-8)

**Goal**: Real-time terminal UI

1. MetricsCollector
2. FeedbackDisplay (full mode)
3. Minimal mode (single line)
4. Spinner animations
5. Mascot ASCII art
6. Celebration animations
7. Activity state detection (active/thinking/stalled)

**Deliverables**:
- Visual parity with Python version
- Both display modes working

### Phase 9: File Watching (Week 8)

**Goal**: Backup file change detection

1. FileWatcher with notify
2. Ignore patterns
3. Change deduplication
4. Integration with metrics

**Deliverables**:
- File changes detected during iteration
- Changes appear in feedback display

### Phase 10: Project Bootstrap (Week 9)

**Goal**: Auto-configuration

1. Stack detection (Python, Node, Rust, Go, etc.)
2. AI CLI detection
3. Source inference
4. Context file detection
5. `afk init` command
6. First-run experience (AI CLI selection)

**Deliverables**:
- `afk init` generates sensible config
- `afk go` prompts for AI CLI on first run

### Phase 11: Polish & Edge Cases (Week 9-10)

**Goal**: Production readiness

1. All remaining commands (`resume`, `start`, `done`, `fail`, `reset`)
2. Self-update (`afk update`)
3. Shell completions
4. Error handling and user-friendly messages
5. Signal handling (Ctrl+C graceful shutdown)
6. Windows-specific fixes

**Deliverables**:
- All commands working
- Self-update functional
- Windows parity

### Phase 12: Testing & CI (Week 10-11)

**Goal**: Comprehensive test coverage

1. Unit tests for all modules
2. Integration tests for CLI commands
3. Snapshot tests for prompt output
4. Cross-platform CI (Linux, macOS, Windows)
5. Release workflow (GitHub Actions)

**Deliverables**:
- 80%+ test coverage
- CI green on all platforms
- Automated releases

### Phase 13: Documentation & Release (Week 11-12)

**Goal**: Ship v1.0.0

1. Update README for Rust version
2. Update USAGE.md
3. Update AGENTS.md
4. Update install scripts
5. Release v1.0.0-rc1 for testing
6. Gather feedback, fix issues
7. Release v1.0.0

**Deliverables**:
- Documentation complete
- v1.0.0 released

---

## Testing Strategy

### Unit Tests

- Config loading/saving
- PRD document operations
- Progress tracking
- Source parsing (JSON, markdown, beads output, GitHub output)
- Prompt template rendering
- Output pattern matching
- Path handling

### Integration Tests

- CLI command execution
- Full loop execution with mock AI CLI
- Quality gate execution
- Git operations (with test repo)
- File watching

### Snapshot Tests

- Prompt output (exact match with Python)
- Help text
- PRD sync output

### Property-Based Tests (Optional)

- Config round-trip (load → save → load)
- PRD round-trip
- Progress round-trip

### Benchmarks

- Startup time
- Config loading
- PRD sync (large file)
- Prompt generation

---

## Risk Assessment

### High Risk

| Risk | Mitigation |
|------|------------|
| Tera template differences from Jinja2 | Snapshot test all prompt outputs against Python |
| Windows subprocess behaviour | Test early and often on Windows CI |
| Terminal UI differences across platforms | Use battle-tested crates (console, indicatif) |

### Medium Risk

| Risk | Mitigation |
|------|------------|
| notify crate platform quirks | Test file watching on all platforms |
| Self-update on Windows (file locking) | Use rename-and-replace strategy |
| Clipboard access on Linux (X11 vs Wayland) | Use arboard with fallbacks |

### Low Risk

| Risk | Mitigation |
|------|------------|
| JSON serialisation compatibility | Extensive tests with real-world files |
| Regex pattern matching differences | Unit test all patterns |

---

## Success Criteria

### Performance

- [ ] `afk --version` completes in <20ms
- [ ] `afk go` (to first AI spawn) completes in <100ms
- [ ] Binary size <10MB on all platforms

### Functionality

- [ ] All P0 commands working with feature parity
- [ ] All P1 commands working
- [ ] Config, PRD, and progress files compatible with Python version

### Quality

- [ ] 80%+ test coverage
- [ ] CI passing on Linux, macOS, Windows
- [ ] No known crashes or panics
- [ ] Graceful error messages for all failure modes

### Distribution

- [ ] Single binary distribution
- [ ] Automated releases via GitHub Actions
- [ ] Self-update working

---

## Appendix A: Module Mapping (Python → Rust)

| Python Module | Lines | Rust Module(s) | Notes |
|---------------|-------|----------------|-------|
| `cli.py` | 1,130 | `cli/commands/*` | Split by command |
| `runner.py` | 1,068 | `runner/*` | Split into controller/iteration/output |
| `bootstrap.py` | 530 | `bootstrap/*` | |
| `feedback.py` | 360 | `feedback/*` | |
| `output_parser.py` | 285 | `parser/*` | |
| `prd_store.py` | 275 | `prd/*` | |
| `git_ops.py` | 219 | `git/*` | |
| `prompt.py` | 203 | `prompt/*` | |
| `progress.py` | 205 | `progress/*` | |
| `file_watcher.py` | 205 | `watcher/*` | |
| `config.py` | 135 | `config/*` | |
| `prd.py` | 139 | `prompt/*` (merge) | PRD parse template |
| `art.py` | 81 | `feedback/art.rs` | |
| `output.py` | 58 | `cli/output.rs` | |
| `sources/__init__.py` | 43 | `sources/mod.rs` | |
| `sources/beads.py` | 182 | `sources/beads.rs` | |
| `sources/github.py` | 128 | `sources/github.rs` | |
| `sources/json_prd.py` | 123 | `sources/json.rs` | |
| `sources/markdown.py` | 101 | `sources/markdown.rs` | |

**Total Python**: ~4,500 lines
**Estimated Rust**: ~3,500-4,500 lines (Rust tends to be similar or slightly more verbose)

---

## Appendix B: Crate Version Recommendations

```toml
[dependencies]
clap = { version = "4", features = ["derive", "cargo"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tera = "1"
console = "0.15"
indicatif = "0.17"
notify = "6"
regex = "1"
reqwest = { version = "0.11", features = ["blocking", "json"] }
chrono = { version = "0.4", features = ["serde"] }
toml = "0.8"
arboard = "3"
which = "6"
directories = "5"
thiserror = "1"
anyhow = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
mockall = "0.12"
criterion = "0.5"
```

---

## Appendix C: Decisions Log

| Decision | Rationale | Alternatives Considered |
|----------|-----------|-------------------------|
| Use `clap` for CLI | Industry standard, excellent derive API, auto-help | `argh` (simpler but fewer features) |
| Use `tera` for templates | Most Jinja2-compatible | `minijinja` (smaller but less compatible) |
| Use `tokio` runtime | Best async ecosystem, needed for notify | `async-std` (smaller but less ecosystem) |
| Use `console`/`indicatif` for TUI | Simpler than full TUI, sufficient for needs | `ratatui` (more powerful but complex) |
| Keep same JSON schemas | Zero migration needed for existing users | Breaking change with new schemas |
| Pure Rust (no FFI) | Simplest cross-compilation | C bindings for specific libraries |

---

## Appendix D: Open Questions

1. **Template Engine Choice**: Should we use `tera` (more Jinja2 compatible) or `minijinja` (smaller, faster)? → Recommend `tera` for compatibility.

2. **Async vs Sync**: Should the runner be fully async, or sync with async file watching only? → Recommend sync main loop with async file watching.

3. **TUI Framework**: Is `console`/`indicatif` sufficient, or do we need `ratatui` for the full feedback display? → Start with simpler crates, upgrade if needed.

4. **Python Interop**: Should we maintain a Python wrapper for users who want to extend afk? → No, focus on pure Rust. Extensions via custom templates.

5. **Config Migration**: Do we need to support migrating old config formats? → No, current schema is stable and should be kept.
