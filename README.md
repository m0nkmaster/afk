# afk

Autonomous AI coding loops — Ralph Wiggum style.

A tool-agnostic CLI for autonomous AI-driven software development. `afk` runs your AI coding tasks in a loop, letting it work autonomously while you step away from the keyboard.

## The Ralph Wiggum Pattern

Each iteration spawns a **fresh AI instance** with clean context. This prevents context overflow and ensures consistent behaviour. Memory persists only through:

- **Git history** — Commits from previous iterations
- **progress.json** — Task status and per-task learnings (short-term memory)
- **AGENTS.md** — Project-wide conventions and patterns (long-term memory)

## Quick Start

```bash
# Zero-config: parse a PRD and go
afk prd parse requirements.md    # Creates .afk/prd.json
afk go                           # Runs the loop (auto-detects everything)
```

Or with explicit configuration:

```bash
afk init                         # Auto-detect project settings
afk source add beads             # Add task source
afk run 10                       # Run 10 iterations
```

## Installation

### One-liner (recommended)

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.ps1 | iex
```

This installs a standalone binary — no Python required. Updates with `afk update`.

### From source (development)

```bash
git clone https://github.com/m0nkmaster/afk.git && cd afk
pip install -e ".[dev]"
```

### With pip/pipx

```bash
pip install afk      # or: pipx install afk
```

## Key Commands

| Command | Description |
|---------|-------------|
| `afk go` | Zero-config: auto-detect and run |
| `afk go 20` | Run 20 iterations |
| `afk run -u` | Run until all tasks complete |
| `afk explain` | Show current loop state |
| `afk verify` | Run quality gates |
| `afk done <id>` | Mark task complete |
| `afk next` | Preview next prompt |

## Supported AI CLIs

afk works with any CLI that accepts prompts as the final argument:

| CLI | Command | Notes |
|-----|---------|-------|
| Claude Code | `claude` | Anthropic's terminal agent |
| Cursor Agent | `agent` | Cursor's CLI agent |
| Codex | `codex` | OpenAI's CLI |
| Aider | `aider` | AI pair programming |
| Amp | `amp` | Sourcegraph's agent |
| Kiro | `kiro` | Amazon's AI CLI |

On first run, `afk go` auto-detects installed CLIs and prompts you to select one.

## Documentation

- **[USAGE.md](USAGE.md)** — Complete command reference, configuration options, and workflow examples
- **[AGENTS.md](AGENTS.md)** — Developer guide for contributing to afk

## Task Size (Critical!)

Each task **must complete in a single AI context window**. Tasks that are too large cause context overflow and poor code quality.

**Right-sized** ✓ — Add a database column, add a UI component, write tests for a module

**Too large** ✗ — "Build the dashboard", "Add authentication", "Refactor the API"

**When in doubt, split.** Five small tasks are better than one large task.

## How It Works

```
                    ┌─────────────────────────────┐
                    │     Load tasks from         │
                    │   sources (beads, json,     │
                    │      markdown, github)      │
                    └─────────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────────────────────┐
                    │   All tasks complete?       │─── Yes ──▶ EXIT ✓
                    └─────────────────────────────┘
                                  │ No
                                  ▼
                    ┌─────────────────────────────┐
                    │   Generate prompt with:     │
                    │   • Next task               │
                    │   • Context files           │
                    │   • Session learnings       │
                    └─────────────────────────────┘
                                  │
                                  ▼
                    ┌─────────────────────────────┐
                    │   Spawn FRESH AI instance   │◀────────┐
                    │   (clean context each time) │         │
                    └─────────────────────────────┘         │
                                  │                         │
                                  ▼                         │
                    ┌─────────────────────────────┐         │
                    │   AI implements task        │         │
                    │   • Code changes            │         │
                    │   • Records learnings       │         │
                    │   • Updates AGENTS.md       │         │
                    └─────────────────────────────┘         │
                                  │                         │
                                  ▼                         │
                    ┌─────────────────────────────┐         │
                    │   Run quality gates         │         │
                    │   (lint, test, typecheck)   │         │
                    └─────────────────────────────┘         │
                                  │                         │
                          Pass?   │                         │
                         ┌────────┴────────┐                │
                         │                 │                │
                    Yes  ▼            No   ▼                │
               ┌─────────────┐    ┌─────────────┐           │
               │ Auto-commit │    │ Skip commit │           │
               └─────────────┘    └─────────────┘           │
                         │                 │                │
                         └────────┬────────┘                │
                                  │                         │
                                  └─────────────────────────┘
```

## Inspired By

- [Ralph Wiggum pattern](https://ghuntley.com/ralph/) by Geoffrey Huntley
- [snarktank/ralph](https://github.com/snarktank/ralph) by Ryan Carson
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic
- [Beads](https://github.com/steveyegge/beads) by Steve Yegge

## License

MIT
