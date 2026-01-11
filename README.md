# afk

Autonomous AI coding loops — Ralph Wiggum style.

A tool-agnostic CLI for autonomous AI-driven software development. `afk` runs your AI coding tasks in a loop, letting it work autonomously while you step away from the keyboard.

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

**Key principle**: Each iteration spawns a **fresh AI instance** with clean context. Memory persists only through:

- **Git history** - Commits from previous iterations
- **progress.json** - Task status and per-task learnings (short-term memory)
- **AGENTS.md** - Project-wide conventions and patterns (long-term memory)

## Quick Start

```bash
# Zero-config: parse a PRD and go
afk prd parse requirements.md    # Creates .afk/prd.json
afk go                           # Runs the loop (auto-detects everything)

# Or with explicit sources:
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

## Commands

### Core Loop

```bash
afk go                     # Zero-config: auto-detect and run 10 iterations
afk go 20                  # Run 20 iterations
afk go -u                  # Run until all tasks complete
afk go TODO.md 5           # Use specific source, run 5 iterations

afk start [N]              # Init if needed + run N iterations (default: 10)
afk run [N]                # Run N iterations with configured AI CLI
afk run --until-complete   # Run until all tasks done
afk run -b feature-name    # Create feature branch first
```

### Task Management

```bash
afk done <task-id>         # Mark task complete
afk done <id> -m "msg"     # With completion message
afk fail <task-id>         # Mark task failed
afk reset <task-id>        # Reset stuck task to pending
```

### Debugging

```bash
afk status                 # Show configuration
afk explain                # Show current loop state
afk explain -v             # Verbose: include learnings and failures
afk next                   # Preview next prompt (without running)
afk verify                 # Run quality gates (lint, test, types)
afk verify -v              # Show full output from failed gates
```

### Sources

```bash
afk source add beads       # Use beads issue tracker
afk source add json prd.json
afk source add markdown TODO.md
afk source add github
afk source list
afk source remove 1
```

### PRD Parsing

Convert a requirements document into structured tasks:

```bash
afk prd parse requirements.md        # Generate prompt, creates .afk/prd.json
afk prd parse PRD.md --copy          # Copy prompt to clipboard
afk prd parse PRD.md -o tasks.json   # Custom output path
```

After parsing, `.afk/prd.json` becomes the source of truth. Run `afk go` to start working through the tasks.

### Session Management

```bash
afk archive create         # Archive current session
afk archive list           # List archives
afk archive clear          # Archive and reset
```

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
    "auto_branch": true
  }
}
```

## Task Size (Critical!)

Each task **must complete in a single AI context window**. Tasks that are too large cause context overflow, incomplete features, and poor code quality.

### Right-sized tasks ✓

- Add a database column and migration
- Add a UI component to an existing page  
- Update a server action with new logic
- Add a new API endpoint
- Write tests for a module

### Too large (split these) ✗

- "Build the entire dashboard" → layout, navigation, each widget
- "Add authentication" → login form, session handling, protected routes
- "Refactor the API" → each endpoint separately

**When in doubt, split.** Five small tasks are better than one large task.

## Debugging

### Check current state

```bash
# What's the loop doing?
afk explain

# What tasks exist?
afk explain -v

# Preview next prompt without running
afk next
```

### Common issues

**Loop not progressing**: Check `afk explain` for stuck tasks. Use `afk reset <id>` to retry.

**Quality gates failing**: Run the gate commands manually to see errors:
```bash
mypy .
ruff check .
pytest -x
```

**Context overflow**: Tasks are too large. Split them via `afk prd parse`.

### Inspect files

```bash
cat .afk/progress.json     # Task status and per-task learnings
cat .afk/config.json       # Configuration
cat .afk/prd.json          # Current task list
cat AGENTS.md              # Long-term project knowledge
ls .afk/archive/           # Previous sessions
```

## Task Sources

### JSON PRD (Anthropic style)

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

### Markdown

```markdown
- [ ] Implement user authentication
- [ ] [HIGH] Critical security fix
- [ ] task-id: Task with explicit ID
- [x] Completed task (skipped)
```

### Beads

Uses `bd ready` to get available work.

### GitHub Issues

Uses `gh issue list`. Requires GitHub CLI.

## AI CLI Support

afk works with any CLI that accepts prompts as the final argument:

| CLI | Config |
|-----|--------|
| Claude CLI | `{"command": "claude", "args": ["--dangerously-skip-permissions"]}` |
| Cursor Agent | `{"command": "agent", "args": ["-p", "--force"]}` |
| Codex | `{"command": "codex", "args": ["--approval-mode", "full-auto"]}` |
| Aider | `{"command": "aider", "args": ["--yes", "--message"]}` |
| Amp | `{"command": "amp", "args": ["--dangerously-allow-all"]}` |
| Kiro | `{"command": "kiro", "args": ["--auto"]}` |
| Custom | `{"command": "your-cli", "args": [...]}` |

When you run `afk go` for the first time, it will auto-detect installed AI CLIs and prompt you to select one.

## Limits & Safety

- `max_iterations`: Stop after N iterations (default: 20)
- `max_task_failures`: Skip task after N failures (default: 3)  
- `timeout_minutes`: Stop after N minutes (default: 120)

## Inspired By

- [Ralph Wiggum pattern](https://ghuntley.com/ralph/) by Geoffrey Huntley
- [snarktank/ralph](https://github.com/snarktank/ralph) by Ryan Carson
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic
- [Beads](https://github.com/steveyegge/beads) by Steve Yegge

## License

MIT
