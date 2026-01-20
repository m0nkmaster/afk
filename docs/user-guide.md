# afk User Guide

Complete reference for **afk** - autonomous AI coding loops, Ralph Wiggum style.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Commands Reference](#commands-reference)
- [Configuration](#configuration)
- [Task Sources](#task-sources)
- [AI CLI Support](#ai-cli-support)
- [Workflow Examples](#workflow-examples)
- [Debugging](#debugging)
- [File Structure](#file-structure)

## Quick Start

### Zero-Config (Recommended)

```bash
# Import a requirements document and start working
afk import requirements.md   # Creates .afk/tasks.json
afk go                           # Runs the loop (auto-detects everything)
```

### With Explicit Configuration

```bash
afk init                         # Auto-detect project settings
afk source add beads             # Add task source
afk go 10                        # Run 10 iterations
```

### How `afk go` Works

`afk go` is the zero-config entry point:

1. **If `.afk/tasks.json` exists with tasks** → uses it directly as the source of truth
2. **If no tasks but sources detected** (TODO.md, beads, etc.) → syncs from those
3. **If nothing found** → shows helpful error with next steps

This means you can just drop a `tasks.json` in `.afk/` and run `afk go` - no configuration needed.

## Core Concepts

### The Ralph Wiggum Pattern

Each iteration spawns a **fresh AI instance** with clean context. This prevents context overflow and ensures consistent behaviour. Memory persists only through:

| Persistence Layer | Purpose |
|-------------------|---------|
| **Git history** | Commits from previous iterations |
| **progress.json** | Task status and per-task learnings (short-term memory) |
| **AGENTS.md** | Project-wide conventions and patterns (long-term memory) |

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
| `openspec` | OpenSpec change proposals |

### Quality Gates

Quality gates (feedback loops) run after each task completion:

```json
{
  "feedback_loops": {
    "types": "cargo check",
    "lint": "cargo clippy",
    "test": "cargo test"
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

The AI reads these files directly and updates them as it works.

## Commands Reference

### Core Loop Commands

| Command | Description |
|---------|-------------|
| `afk go` | Zero-config: auto-detect and run |
| `afk go 20` | Run 20 iterations |
| `afk go -u` | Run until all tasks complete |
| `afk go --init` | Re-run setup, then run |
| `afk go --fresh` | Clear session progress and start fresh |
| `afk go TODO.md 5` | Use specific source, run 5 iterations |

### Task Management Commands

| Command | Description |
|---------|-------------|
| `afk tasks` | List tasks from current PRD |
| `afk tasks -p` | Show only pending tasks |
| `afk tasks -l 10` | Limit to 10 tasks |
| `afk tasks --complete` | Show only completed tasks |
| `afk task <id>` | Show details of a specific task |
| `afk done <task-id>` | Mark task complete |
| `afk done <id> -m "msg"` | Mark complete with message |
| `afk fail <task-id>` | Mark task failed |
| `afk reset <task-id>` | Reset stuck task to pending |

### Status and Debugging Commands

| Command | Description |
|---------|-------------|
| `afk status` | Show current status and tasks |
| `afk status -v` | Verbose: include learnings and session details |
| `afk prompt` | Preview next prompt (without running) |
| `afk prompt -c` | Copy prompt to clipboard |
| `afk verify` | Run quality gates |
| `afk verify -v` | Show full output from failed gates |

### Source Management Commands

| Command | Description |
|---------|-------------|
| `afk source add beads` | Add beads issue tracker |
| `afk source add json tasks.json` | Add JSON tasks file |
| `afk source add markdown TODO.md` | Add markdown checklist |
| `afk source add github` | Add GitHub issues |
| `afk source add openspec` | Add OpenSpec change proposals |
| `afk source list` | List configured sources |
| `afk source remove 1` | Remove source by index |

### PRD & Tasks Commands

| Command | Description |
|---------|-------------|
| `afk import requirements.md` | Import requirements into .afk/tasks.json |
| `afk import PRD.md --copy` | Copy prompt to clipboard |
| `afk import PRD.md -o custom.json` | Custom output path |
| `afk sync` | Sync from all sources (alias: `afk tasks sync`) |
| `afk tasks sync` | Sync from all sources |

### Session/Archive Commands

| Command | Description |
|---------|-------------|
| `afk init` | Initialise afk (auto-detects project settings) |
| `afk init -f` | Force re-initialise (re-prompts for AI CLI) |
| `afk use` | Interactively switch AI CLI |
| `afk use claude` | Switch to a specific AI CLI |
| `afk use --list` | List available AI CLIs with install status |
| `afk archive` | Archive and clear session (ready for fresh work) |
| `afk archive list` | List archived sessions |

**Note:** When you switch git branches and run `afk go`, you'll be prompted to archive the previous session automatically.

### Config Commands

| Command | Description |
|---------|-------------|
| `afk config show` | Show all config values |
| `afk config show -s limits` | Show only the limits section |
| `afk config get <key>` | Get value (e.g., `limits.max_iterations`) |
| `afk config set <key> <value>` | Set value (e.g., `limits.max_iterations 20`) |
| `afk config reset` | Reset all config to defaults |
| `afk config reset <key>` | Reset specific key to default |
| `afk config edit` | Open config in $EDITOR |
| `afk config explain` | List all keys with descriptions |
| `afk config explain <key>` | Show full docs for a key |
| `afk config keys` | List all valid config keys |

### Utility Commands

| Command | Description |
|---------|-------------|
| `afk update` | Update to latest version |
| `afk update --check` | Check for updates without installing |
| `afk completions bash` | Generate bash completions |
| `afk completions zsh` | Generate zsh completions |
| `afk completions fish` | Generate fish completions |

## Configuration

All config lives in `.afk/config.json`:

```json
{
  "sources": [
    {"type": "beads"},
    {"type": "json", "path": "tasks.json"}
  ],
  "feedback_loops": {
    "types": "cargo check",
    "lint": "cargo clippy",
    "test": "cargo test"
  },
  "limits": {
    "max_iterations": 20,
    "max_task_failures": 3,
    "timeout_minutes": 120
  },
  "ai_cli": {
    "command": "claude",
    "args": ["--dangerously-skip-permissions", "-p"],
    "models": ["sonnet", "opus"]
  },
  "prompt": {
    "has_frontend": true
  },
  "git": {
    "auto_commit": true,
    "commit_message_template": "afk: {task_id} - {message}"
  },
  "archive": {
    "enabled": true,
    "directory": ".afk/archive"
  }
}
```

### Configuration Options

#### Sources

```json
{
  "sources": [
    {"type": "beads"},
    {"type": "json", "path": "tasks.json"},
    {"type": "markdown", "path": "TODO.md"},
    {"type": "github", "labels": ["afk"]},
    {"type": "openspec"}
  ]
}
```

#### Feedback Loops

```json
{
  "feedback_loops": {
    "types": "cargo check",
    "lint": "cargo clippy",
    "test": "cargo test",
    "build": "cargo build --release",
    "custom": {
      "security": "cargo audit"
    }
  }
}
```

#### Limits

| Limit | Description | Default |
|-------|-------------|---------|
| `max_iterations` | Stop after N iterations | 200 |
| `max_task_failures` | Skip task after N failures | 50 |
| `timeout_minutes` | Stop after N minutes | 120 |
| `prevent_sleep` | Prevent system sleep during sessions | true |

**Sleep Prevention:** When enabled, afk prevents the system from sleeping during autonomous sessions using platform-specific tools:
- **macOS**: Uses `caffeinate` to prevent idle sleep
- **Linux**: Uses `systemd-inhibit` (or `gnome-session-inhibit` as fallback)

This ensures long-running sessions aren't interrupted when you step away. The lock is automatically released when the session ends.

#### Prompt

```json
{
  "prompt": {
    "has_frontend": true,
    "context_files": ["AGENTS.md", "README.md"],
    "instructions": ["Use British English", "Always run tests"]
  }
}
```

| Option | Description | Default |
|--------|-------------|---------|
| `has_frontend` | Enable browser testing instructions for UI stories | Auto-detected |
| `context_files` | Additional files to mention in prompts | `[]` |
| `instructions` | Custom instructions appended to prompts | `[]` |
| `custom_path` | Path to custom prompt template | `null` |

**Frontend detection:** During `afk init`, afk auto-detects frontend projects by checking for:
- Framework config files (next.config.js, vite.config.ts, etc.)
- Frontend dependencies in package.json (react, vue, svelte, etc.)
- Frontend file patterns (.tsx, .jsx, .vue, .svelte)

When `has_frontend` is enabled, the prompt includes browser testing instructions requiring visual verification of UI changes.

## Task Sources

### JSON PRD (Anthropic Style)

```json
{
  "project": "my-project",
  "userStories": [
    {
      "id": "auth-flow",
      "title": "Implement login flow",
      "description": "Implement user authentication",
      "priority": 1,
      "acceptanceCriteria": [
        "User can enter email/password",
        "Invalid credentials show error",
        "Successful login redirects to dashboard"
      ],
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

### OpenSpec

Reads tasks from [OpenSpec](https://github.com/Fission-AI/OpenSpec) change proposals. Add to your config manually:

```json
{
  "sources": [{"type": "openspec"}]
}
```

Scans `openspec/changes/<change-id>/tasks.md` for unchecked items and enriches them with context from proposals and specs.

## AI CLI Support

afk works with any CLI that accepts prompts as the final argument. On first run, `afk go` auto-detects installed CLIs and prompts you to select one.

### Switching AI CLIs

```bash
afk use              # Interactive selection from installed CLIs
afk use cursor       # Switch directly to Cursor agent
afk use claude       # Switch to Claude Code
afk use --list       # Show all known CLIs with install status
```

### Supported CLIs

| CLI | Configuration |
|-----|---------------|
| **Claude Code** | `{"command": "claude", "args": ["--dangerously-skip-permissions", "-p"]}` |
| **Cursor Agent** | `{"command": "agent", "args": ["-p", "--force"]}` |
| **Codex** | `{"command": "codex", "args": ["--approval-mode", "full-auto", "-q"]}` |
| **Aider** | `{"command": "aider", "args": ["--yes", "--message"]}` |
| **Amp** | `{"command": "amp", "args": ["--dangerously-allow-all"]}` |
| **Kiro** | `{"command": "kiro", "args": ["--auto"]}` |

**Note:** afk automatically appends streaming output flags (`--output-format stream-json`) for supported CLIs. The `args` above are the base configuration only. To disable streaming, set `"output_format": "text"` in your config.

### Multi-Model Rotation

Configure multiple models to rotate between them pseudo-randomly across iterations. Different models bring different strengths and problem-solving approaches - cycling through them helps avoid getting stuck in local optima.

```json
{
  "ai_cli": {
    "command": "claude",
    "args": ["--dangerously-skip-permissions", "-p"],
    "models": ["sonnet", "opus", "haiku"]
  }
}
```

When multiple models are configured:
- afk selects one with equal probability each iteration
- Passes `--model <selected>` to the AI CLI
- Displays which model was selected in the TUI

This works with any CLI that supports the `--model` flag (Claude Code, Cursor, Aider, Codex, etc.).

You can also set models via the CLI:

```bash
afk config set ai_cli.models "sonnet, opus, haiku"
```

### Completion Signals

The AI can signal task completion by outputting:

- `<promise>COMPLETE</promise>` (Ralph-compatible)
- `AFK_COMPLETE`
- `AFK_STOP`

When detected, afk terminates the current iteration gracefully.

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

# 2. Import into structured tasks
afk import requirements.md

# 3. Start the autonomous loop
afk go
```

### Working with Existing Issues

```bash
# Add beads as a source
afk source add beads

# Sync and show tasks
afk tasks sync
afk tasks

# Start working
afk go
```

### Resuming Work

```bash
# afk always continues from where you left off
afk go

# Run more iterations
afk go 20

# Run until complete
afk go -u

# To start fresh, use the --fresh flag
afk go --fresh

# Or archive and clear the session manually first
afk archive -y
afk go
```

### Working with Branches

afk manages commits but not branches. Create your own branch first, then use afk:

```bash
# Create your feature branch
git checkout -b my-feature

# Run afk on this branch
afk go 10

# When done, raise a PR as usual
```

#### Branch Change Detection

When you switch git branches and run `afk go`, it detects the change and prompts:

```
⚠  You're on branch main but the last session was on feature/add-auth.
   Archive the previous session? [Y/n]:
```

- **Press Enter or Y** - Archives the previous session and starts fresh
- **Press N** - Continues without archiving (updates stored branch)

This keeps your work organised - each feature branch gets its own clean session, and old sessions are automatically archived when you switch context.

## Debugging

### Check Current State

```bash
# What's the current status?
afk status

# Verbose: include learnings and details
afk status -v

# List all tasks
afk tasks

# Show details of a specific task
afk task <id>

# Preview next prompt without running
afk prompt
```

### Common Issues

**Loop not progressing**: Check `afk status -v` for stuck tasks. Use `afk reset <id>` to retry.

**Quality gates failing**: Run the gate commands manually to see errors:

```bash
cargo check
cargo clippy
cargo test
```

**Context overflow**: Tasks are too large. Split them via `afk import`.

### Inspect Files Directly

```bash
cat .afk/progress.json     # Task completion state and per-task learnings
cat .afk/config.json       # Configuration
cat .afk/tasks.json        # Current task list
cat AGENTS.md              # Long-term project knowledge
ls .afk/archive/           # Previous sessions
```

## File Structure

```
.afk/
├── config.json      # Configuration
├── tasks.json       # Current task list (source of truth)
├── progress.json    # Session state (iterations, task status, per-task learnings, last branch)
└── archive/         # Previous sessions
    └── 20260112_123000/
        ├── progress.json
        ├── tasks.json
        └── metadata.json   # Includes branch name, reason, stats

AGENTS.md            # Long-term project knowledge (at project root or in subfolders)
```

### Archive Metadata

Each archived session includes metadata:

```json
{
  "archived_at": "2026-01-12T12:30:00.000000",
  "branch": "feature/add-auth",
  "reason": "branch_change",
  "iterations": 5,
  "tasks_completed": 3,
  "tasks_pending": 2
}
```

The `reason` field indicates why the session was archived:
- `completed` - All tasks finished
- `branch_change` - User switched git branches
- `manual` - User ran `afk archive` manually

## Installation

```bash
# One-liner (recommended)
curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash

# From source
git clone https://github.com/m0nkmaster/afk.git && cd afk
cargo build --release
# Binary at target/release/afk
```
