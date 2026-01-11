# afk Usage Guide

**afk** — Autonomous AI coding loops, Ralph Wiggum style.

A tool-agnostic CLI for autonomous AI-driven software development. Run your AI coding tasks in a loop, letting the AI work autonomously while you step away from the keyboard.

---

## Table of Contents

- [How It Works](#how-it-works)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Commands Reference](#commands-reference)
- [Configuration](#configuration)
- [Task Sources](#task-sources)
- [Workflow Examples](#workflow-examples)
- [Debugging](#debugging)
- [AI CLI Support](#ai-cli-support)

---

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                        afk run                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │  Load tasks from sources      │
              │  (beads, json, markdown, gh)  │
              └───────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │  All tasks complete?          │──── Yes ───▶ EXIT ✓
              └───────────────────────────────┘
                              │ No
                              ▼
              ┌───────────────────────────────┐
              │  Generate prompt with:        │
              │  • Next task                  │
              │  • Context files              │
              │  • Session learnings          │
              └───────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │  Spawn FRESH AI instance      │◀──────────┐
              │  (clean context each time)    │           │
              └───────────────────────────────┘           │
                              │                           │
                              ▼                           │
              ┌───────────────────────────────┐           │
              │  AI implements task           │           │
              │  • Code changes               │           │
              │  • Records learnings          │           │
              │  • Updates AGENTS.md          │           │
              │  • Runs `afk done <id>`       │           │
              └───────────────────────────────┘           │
                              │                           │
                              ▼                           │
              ┌───────────────────────────────┐           │
              │  Run quality gates            │           │
              │  (lint, test, typecheck)      │           │
              └───────────────────────────────┘           │
                              │                           │
                      Pass?   │                           │
                     ┌────────┴────────┐                  │
                     │ Yes             │ No               │
                     ▼                 ▼                  │
              ┌─────────────┐   ┌─────────────┐           │
              │ Auto-commit │   │ Skip commit │           │
              └─────────────┘   └─────────────┘           │
                     │                 │                  │
                     └────────┬────────┘                  │
                              │                           │
                              └───────────────────────────┘
```

### The Ralph Wiggum Pattern

Each iteration spawns a **fresh AI instance** with clean context. This prevents context overflow and ensures consistent behaviour. Memory persists only through:

| Persistence Layer | Purpose |
|-------------------|---------|
| **Git history** | Commits from previous iterations |
| **progress.json** | Task status and per-task learnings (short-term memory) |
| **AGENTS.md** | Project-wide conventions and patterns (long-term memory) |

---

## Quick Start

### Zero-Config (Recommended)

```bash
# Parse a requirements document and start working
afk prd parse requirements.md    # Creates .afk/prd.json
afk go                           # Runs the loop (auto-detects everything)
```

### With Explicit Configuration

```bash
afk init                         # Auto-detect project settings
afk source add beads             # Add task source
afk run 10                       # Run 10 iterations
```

### How `afk go` Works

`afk go` is the zero-config entry point:

1. **If `.afk/prd.json` exists with tasks** → uses it directly as the source of truth
2. **If no PRD but sources detected** (TODO.md, beads, etc.) → syncs from those
3. **If nothing found** → shows helpful error with next steps

This means you can just drop a `prd.json` in `.afk/` and run `afk go` — no configuration needed.

---

## Core Concepts

### Tasks

Tasks are atomic units of work that the AI completes in a single iteration. Each task should:

- Complete in a **single AI context window**
- Have clear **acceptance criteria**
- Be **independently testable**

#### Right-sized tasks ✓

- Add a database column and migration
- Add a UI component to an existing page
- Update a server action with new logic
- Add a new API endpoint
- Write tests for a module

#### Too large (split these) ✗

- "Build the entire dashboard" → layout, navigation, each widget
- "Add authentication" → login form, session handling, protected routes
- "Refactor the API" → each endpoint separately

**When in doubt, split.** Five small tasks are better than one large task.

### Sources

Sources define where tasks come from:

| Source | Description |
|--------|-------------|
| `beads` | Beads issue tracker (`bd ready`) |
| `json` | JSON PRD file |
| `markdown` | Markdown checklist (TODO.md) |
| `github` | GitHub issues via `gh` CLI |

### Quality Gates

Quality gates (feedback loops) run after each task completion:

```json
{
  "feedback_loops": {
    "types": "mypy .",
    "lint": "ruff check .",
    "test": "pytest -x"
  }
}
```

The AI auto-commits only when all gates pass.

### Learnings

Learnings are recorded in two places:

| Location | Scope | Purpose |
|----------|-------|---------|
| **progress.json** | Per-task | Short-term memory for the current session |
| **AGENTS.md** | Project-wide | Long-term conventions that benefit future sessions |

The AI reads these files directly and updates them as it works. Task-specific learnings go in `progress.json` as an array on each task entry. Project-wide discoveries go in `AGENTS.md` at the project root (or in a subfolder's `AGENTS.md` for localised knowledge).

---

## Commands Reference

### Core Loop Commands

| Command | Description |
|---------|-------------|
| `afk go` | Zero-config: auto-detect and run 10 iterations |
| `afk go 20` | Run 20 iterations |
| `afk go -u` | Run until all tasks complete |
| `afk go TODO.md 5` | Use specific source, run 5 iterations |
| `afk start [N]` | Init if needed + run N iterations (default: 10) |
| `afk run [N]` | Run N iterations with configured AI CLI |
| `afk run --until-complete` | Run until all tasks done |
| `afk run -b feature-name` | Create feature branch first |
| `afk resume` | Continue from last session |

### Task Management Commands

| Command | Description |
|---------|-------------|
| `afk done <task-id>` | Mark task complete |
| `afk done <id> -m "msg"` | Mark complete with message |
| `afk fail <task-id>` | Mark task failed |
| `afk reset <task-id>` | Reset stuck task to pending |

### Debugging Commands

| Command | Description |
|---------|-------------|
| `afk status` | Show configuration |
| `afk explain` | Show current loop state |
| `afk explain -v` | Verbose: include learnings and failures |
| `afk next` | Preview next prompt (without running) |

### Source Management Commands

| Command | Description |
|---------|-------------|
| `afk source add beads` | Add beads issue tracker |
| `afk source add json prd.json` | Add JSON PRD file |
| `afk source add markdown TODO.md` | Add markdown checklist |
| `afk source add github` | Add GitHub issues |
| `afk source list` | List configured sources |
| `afk source remove 1` | Remove source by index |

### PRD Commands

| Command | Description |
|---------|-------------|
| `afk prd parse requirements.md` | Generate parsing prompt |
| `afk prd parse PRD.md --copy` | Copy prompt to clipboard |
| `afk prd parse PRD.md -o tasks.json` | Custom output path |
| `afk prd sync` | Sync from all sources |
| `afk prd show` | Show current PRD state |
| `afk prd show --pending` | Show only pending stories |

### Session/Archive Commands

| Command | Description |
|---------|-------------|
| `afk archive create` | Archive current session |
| `afk archive list` | List archives |
| `afk archive clear` | Archive and reset |

---

## Configuration

All config lives in `.afk/config.json`:

```json
{
  "sources": [
    {"type": "beads"},
    {"type": "json", "path": "prd.json"}
  ],
  "feedback_loops": {
    "types": "mypy .",
    "lint": "ruff check .",
    "test": "pytest -x"
  },
  "limits": {
    "max_iterations": 20,
    "max_task_failures": 3,
    "timeout_minutes": 120
  },
  "ai_cli": {
    "command": "agent",
    "args": []
  },
  "git": {
    "auto_commit": true,
    "auto_branch": true,
    "branch_prefix": "afk/"
  },
  "archive": {
    "enabled": true,
    "directory": ".afk/archive",
    "on_branch_change": true
  }
}
```

### Configuration Options

#### Sources

```json
{
  "sources": [
    {"type": "beads"},
    {"type": "json", "path": "prd.json"},
    {"type": "markdown", "path": "TODO.md"},
    {"type": "github", "labels": ["afk"]}
  ]
}
```

#### Feedback Loops

```json
{
  "feedback_loops": {
    "types": "mypy .",
    "lint": "ruff check .",
    "test": "pytest -x",
    "build": "npm run build",
    "custom": {
      "security": "bandit -r src/"
    }
  }
}
```

#### Limits

```json
{
  "limits": {
    "max_iterations": 20,
    "max_task_failures": 3,
    "timeout_minutes": 120
  }
}
```

| Limit | Description | Default |
|-------|-------------|---------|
| `max_iterations` | Stop after N iterations | 20 |
| `max_task_failures` | Skip task after N failures | 3 |
| `timeout_minutes` | Stop after N minutes | 120 |

#### Git Integration

```json
{
  "git": {
    "auto_commit": true,
    "auto_branch": false,
    "branch_prefix": "afk/",
    "commit_message_template": "afk: {task_id} - {message}"
  }
}
```

---

## Task Sources

### JSON PRD (Anthropic Style)

```json
{
  "tasks": [
    {
      "id": "auth-flow",
      "description": "Implement user authentication",
      "priority": 1,
      "passes": false
    }
  ]
}
```

### Markdown Checklist

```markdown
- [ ] Implement user authentication
- [ ] [HIGH] Critical security fix
- [ ] task-id: Task with explicit ID
- [x] Completed task (skipped)
```

### Beads

Uses `bd ready` to get available work from your beads issue tracker.

### GitHub Issues

Uses `gh issue list`. Requires GitHub CLI to be installed and authenticated.

---

## Workflow Examples

### Starting a New Feature

```bash
# 1. Write your requirements
cat > requirements.md << 'EOF'
# User Authentication

## Requirements
- Users can sign up with email/password
- Users can log in
- Users can reset password
- Protected routes require authentication
EOF

# 2. Parse into structured tasks
afk prd parse requirements.md

# 3. Start the autonomous loop
afk go
```

### Working with Existing Issues

```bash
# Add beads as a source
afk source add beads

# Sync and show tasks
afk prd sync
afk prd show

# Start working
afk go
```

### Resuming Work

```bash
# Continue from where you left off
afk resume

# Or run more iterations
afk resume 20

# Or run until complete
afk resume --until-complete
```

### Creating a Feature Branch

```bash
# Create branch and run
afk run 10 --branch my-feature

# This creates: afk/my-feature
```

---

## Debugging

### Check Current State

```bash
# What's the loop doing?
afk explain

# What tasks exist? (verbose)
afk explain -v

# Preview next prompt without running
afk next
```

### Common Issues

**Loop not progressing**: Check `afk explain` for stuck tasks. Use `afk reset <id>` to retry.

**Quality gates failing**: Run the gate commands manually to see errors:

```bash
mypy .
ruff check .
pytest -x
```

**Context overflow**: Tasks are too large. Split them via `afk prd parse`.

### Inspect Files Directly

```bash
cat .afk/progress.json     # Task completion state and per-task learnings
cat .afk/config.json       # Configuration
cat .afk/prd.json          # Current task list
cat AGENTS.md              # Long-term project knowledge
ls .afk/archive/           # Previous sessions
```

---

## AI CLI Support

afk works with any CLI that accepts prompts as the final argument:

| CLI | Configuration |
|-----|---------------|
| **Cursor Agent** (default) | `{"command": "agent", "args": ["--force", "-p"]}` |
| **Claude CLI** | `{"command": "claude", "args": ["--dangerously-skip-permissions", "-p"]}` |
| **Codex** | `{"command": "codex", "args": ["--approval-mode", "full-auto", "-q"]}` |
| **Aider** | `{"command": "aider", "args": ["--yes"]}` |
| **Amp** | `{"command": "amp", "args": ["--dangerously-allow-all"]}` |
| **Custom** | `{"command": "your-cli", "args": [...]}` |

When you run `afk go` for the first time, it will auto-detect installed AI CLIs and prompt you to select one.

### Completion Signals

The AI can signal task completion by outputting:

- `<promise>COMPLETE</promise>`
- `AFK_COMPLETE`
- `AFK_STOP`

When detected, afk terminates the current iteration gracefully.

---

## File Structure

```
.afk/
├── config.json      # Configuration
├── prd.json         # Current task list (source of truth)
├── progress.json    # Session state (iterations, task status, per-task learnings)
├── prompt.md        # Generated prompt (if using file output)
└── archive/         # Previous sessions
    └── 2025-01-11-feature-x/
        ├── prd.json
        ├── progress.json
        └── metadata.json

AGENTS.md            # Long-term project knowledge (at project root or in subfolders)
```

---

## Installation

```bash
# Recommended: install globally with pipx
pipx install git+https://github.com/m0nkmaster/afk.git

# Or with pip
pip install git+https://github.com/m0nkmaster/afk.git

# Development
git clone https://github.com/m0nkmaster/afk.git && cd afk
pip install -e ".[dev]"
```

---

## Inspired By

- [Ralph Wiggum pattern](https://ghuntley.com/ralph/) by Geoffrey Huntley
- [snarktank/ralph](https://github.com/snarktank/ralph) by Ryan Carson
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic
- [Beads](https://github.com/steveyegge/beads) by Steve Yegge
