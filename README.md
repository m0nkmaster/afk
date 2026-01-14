# afk 🛋️

> Let the AI work while you're away from the keyboard!

A tool-agnostic CLI for autonomous AI-driven software development. `afk` runs your AI coding tasks in a loop, spawning a **fresh agent instance** for each iteration — so context never overflows and the AI stays sharp.

## ✨ Why afk?

AI coding agents are powerful, but they have a fatal flaw: **context window exhaustion**. The longer an AI works on a problem, the more context it accumulates — past attempts, dead ends, outdated information. Eventually, it becomes bloated and confused, making worse decisions the longer it runs.

**afk solves this with the Ralph Wiggum pattern** — a kanban-style approach where each task gets a fresh AI instance with clean context. Think of it like a well-organised team: each developer picks up one ticket, completes it, and moves on. No cognitive overload. No stale context.

The result? AI that can work autonomously for hours without degrading.

## 🧠 The Ralph Wiggum Pattern

The technique is named after [Ralph Wiggum](https://en.wikipedia.org/wiki/Ralph_Wiggum) from *The Simpsons* — a character who approaches each moment with fresh-eyed obliviousness, unburdened by what came before.

In AI terms, it works like this:

1. **Fresh start every iteration** — Each loop spawns a brand new AI instance
2. **One task at a time** — Kanban-style: pick up a task, complete it, move on
3. **Memory through files, not context** — Progress persists via git commits, not the AI's memory

This means the AI never runs out of context, never gets confused by old attempts, and can work indefinitely without degradation.

### How Memory Persists

- 📝 **Git history** — Commits from previous iterations
- 📋 **progress.json** — Task status and per-task learnings (short-term memory)
- 📖 **AGENTS.md** — Project-wide conventions and patterns (long-term memory)

### Learn More

- 📰 [The Ralph Wiggum Technique](https://ghuntley.com/ralph/) by Geoffrey Huntley (the originator)
- 🔧 [snarktank/ralph](https://github.com/snarktank/ralph) by Ryan Carson (early implementation)
- 📚 [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic

## 🚀 Quick Start

### Step 1: Write your requirements in plain text

Create a requirements document describing what you want to build. This can be high-level or as detailed as you'd like. You don't need to break it down into granular tasks yourself:

```markdown
# Weather Dashboard

A simple web app that shows current weather for a given city.

Users should be able to enter a city name and see the temperature, conditions, and a 5-day forecast. Use the OpenWeather API.
The UI should be clean and mobile-friendly.
```

### Step 2: Generate tasks

```bash
afk import requirements.md
```

This runs your AI CLI to analyse the PRD and break it down into small, AI-sized tasks. The output goes to `.afk/tasks.json`.

### Step 3: Check and go!

```bash
afk list      # Review the tasks it generated
afk go        # Start the autonomous loop
```

That's it. afk works through the tasks one by one, committing as it goes.

### Already have a task list?

If you've already got tasks in a structured format, skip the import:

```bash
afk go TODO.md           # Markdown with checkboxes: - [ ] Task name
afk go tasks.json        # JSON with `afk-`style tasks array
afk go 20                # Run 20 iterations
afk go -u                # Run until all tasks complete
```

**Note:** These expect task lists, not raw PRDs. Use `afk import` to parse requirements into tasks.

## 📦 Installation

### One-liner (recommended)

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.ps1 | iex
```

Installs a standalone binary — no dependencies required. Updates with `afk update`.

### From source

```bash
git clone https://github.com/m0nkmaster/afk.git && cd afk
cargo build --release
# Binary at target/release/afk
```

### Cargo

```bash
cargo install --git https://github.com/m0nkmaster/afk
```

## 🎮 Commands

### Core Loop

| Command | Description |
|---------|-------------|
| `afk go` | Zero-config: auto-detect and run (10 iterations) |
| `afk go 20` | Run 20 iterations |
| `afk go -u` | Run until all tasks complete |
| `afk go TODO.md 5` | Use TODO.md as source, run 5 iterations |
| `afk go --init` | Re-run setup, then start loop |
| `afk go --fresh` | Clear session progress and start fresh |

### Task Management

| Command | Description |
|---------|-------------|
| `afk status` | Show current status and tasks |
| `afk status -v` | Verbose output with learnings |
| `afk list` | List tasks from current product requirements doc (PRD) |
| `afk task <id>` | Show details of a specific task |
| `afk done <id>` | Mark task complete |
| `afk fail <id>` | Mark task failed |
| `afk reset <id>` | Reset stuck task to pending |

### Import & Task Sources

| Command | Description |
|---------|-------------|
| `afk import <file>` | Import requirements doc into .afk/tasks.json |
| `afk tasks show` | Show current task list |
| `afk sync` | Sync from configured sources (alias: `afk tasks sync`) |
| `afk source add beads` | Add beads as task source |
| `afk source add markdown TODO.md` | Add markdown file source |
| `afk source list` | List configured sources |
| `afk source remove <index>` | Remove a source by index (1-based) |

### Quality & Debug

| Command | Description |
|---------|-------------|
| `afk verify` | Run quality gates (lint, test, types) |
| `afk prompt` | Preview next iteration's prompt |
| `afk prompt -c` | Copy prompt to clipboard |

### Session Management

| Command | Description |
|---------|-------------|
| `afk init` | Initialise afk (auto-detects project) |
| `afk archive` | Archive and clear session (ready for fresh work) |
| `afk archive list` | List archived sessions |
| `afk update` | Update afk to latest version |

## 🤖 Supported AI CLIs

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

## ⚠️ Task Size (Critical!)

Each task **must complete in a single AI context window**. Tasks that are too large cause context overflow and poor code quality.

**Right-sized** ✓
- Add a database column
- Create a UI component  
- Write tests for a module
- Fix a specific bug

**Too large** ✗
- "Build the dashboard"
- "Add authentication"
- "Refactor the API"

**When in doubt, split.** Five small tasks are better than one large task.

## 🔄 How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   📋 Load tasks from sources                                │
│      (beads, json, markdown, github)                        │
│                                                             │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   ✅ All tasks complete? ──────────────────────▶ EXIT ✓     │
│                                                             │
└───────────────────────────┬─────────────────────────────────┘
                            │ No
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   📝 Generate prompt with:                                  │
│      • Next task                                            │
│      • Context files                                        │
│      • Session learnings                                    │
│                                                             │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   🧠 Spawn FRESH AI instance                  ◀─────────┐   │
│      (clean context each time!)                         │   │
│                                                         │   │
└───────────────────────────┬─────────────────────────────│───┘
                            │                             │
                            ▼                             │
┌─────────────────────────────────────────────────────────│───┐
│                                                         │   │
│   💻 AI implements task                                 │   │
│      • Makes code changes                               │   │
│      • Records learnings                                │   │
│      • Updates AGENTS.md                                │   │
│                                                         │   │
└───────────────────────────┬─────────────────────────────│───┘
                            │                             │
                            ▼                             │
┌─────────────────────────────────────────────────────────│───┐
│                                                         │   │
│   🧪 Run quality gates                                  │   │
│      (lint, test, typecheck)                            │   │
│                                                         │   │
└───────────────────────────┬─────────────────────────────│───┘
                            │                             │
                    Pass?   │                             │
                   ┌────────┴────────┐                    │
                   │                 │                    │
              Yes  ▼            No   ▼                    │
         ┌─────────────┐    ┌─────────────┐              │
         │ Auto-commit │    │ Skip commit │              │
         └─────────────┘    └─────────────┘              │
                   │                 │                    │
                   └────────┬────────┘                    │
                            │                             │
                            └─────────────────────────────┘
```

## 📚 Documentation

- **[docs/user-guide.md](docs/user-guide.md)** — Complete command reference and workflow examples
- **[docs/architecture.md](docs/architecture.md)** — Technical overview for contributors
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — How to contribute

## 🙏 Inspired By

- [Ralph Wiggum pattern](https://ghuntley.com/ralph/) by Geoffrey Huntley
- [snarktank/ralph](https://github.com/snarktank/ralph) by Ryan Carson
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic
- [Beads](https://github.com/steveyegge/beads) by Steve Yegge

## 📄 License

MIT
