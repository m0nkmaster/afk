# afk

Autonomous AI coding loops — Ralph Wiggum style.

A tool-agnostic library for using the Ralph Wiggum approach for software development with AI. `afk` runs your AI coding tasks in a loop, letting it work autonomously while you step away from the keyboard.

## Installation

Install globally using [pipx](https://pipx.pypa.io/) (recommended):

```bash
# From GitHub
pipx install git+https://github.com/m0nkmaster/afk.git

# Or from PyPI (once published)
pipx install afk
```

Or with pip:

```bash
pip install git+https://github.com/m0nkmaster/afk.git
```

For development:

```bash
git clone https://github.com/m0nkmaster/afk.git
cd afk
pip install -e .
```

## Quick Start

```bash
# Initialize afk (auto-detects project settings)
afk init

# Parse a PRD into structured JSON (Anthropic harness pattern)
afk prd parse requirements.md     # Generates AI prompt to create prd.json
afk prd parse PRD.md --copy       # Copy prompt to clipboard

# Manage task sources
afk source add json prd.json      # Add JSON PRD file
afk source add markdown tasks.md  # Add markdown checklist
afk source add beads              # Add beads (bd) for issues
afk source add github             # Add GitHub issues
afk source list                   # List configured sources
afk source remove 1               # Remove source by index

# Check status
afk status

# Get the next iteration prompt
afk next              # Print to stdout
afk next --copy       # Copy to clipboard
afk next --file       # Write to .afk/prompt.md
afk next --limit 10   # Override max iterations

# After AI completes work
afk done <task-id>

# Run multiple iterations (with configured AI CLI)
afk run 5
```

## Initialization

The `init` command analyses your project and auto-configures afk:

```bash
afk init           # Interactive mode
afk init --yes     # Accept defaults
afk init --dry-run # Preview without changes
afk init --force   # Reconfigure existing setup
```

It detects:
- **Project type**: Python, Node.js, Rust, Go, Java (Maven/Gradle)
- **Available tools**: bd, gh, claude, aider
- **Task sources**: beads, PRD files, markdown task lists
- **Context files**: AGENTS.md, README.md, CONTRIBUTING.md

## Configuration

Configuration lives in `.afk/config.json`:

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
  "output": {
    "default": "clipboard",
    "file_path": ".afk/prompt.md"
  },
  "ai_cli": {
    "command": "claude",
    "args": ["-p"]
  },
  "prompt": {
    "template": "default",
    "context_files": ["AGENTS.md", "README.md"],
    "instructions": [
      "Follow the coding style in AGENTS.md",
      "Always run tests before marking done"
    ]
  },
  "git": {
    "auto_commit": true,
    "auto_branch": true,
    "branch_prefix": "afk/",
    "commit_message_template": "afk: {task_id} - {message}"
  },
  "archive": {
    "enabled": true,
    "directory": ".afk/archive",
    "on_branch_change": true
  }
}
```

## Task Source Formats

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

Uses `bd ready` to get available work from beads issue tracking.

### GitHub Issues

Uses `gh issue list` to pull open issues. Requires GitHub CLI.

## Modes

### HITL (Human-in-the-loop)

Run `afk next`, copy the prompt into your AI tool (Cursor, Claude, etc.), watch it work.

### Bootstrap (AI-driven)

```bash
afk next --bootstrap
```

Generates a prompt that teaches the AI to call `afk` commands itself, creating an autonomous loop.

### Wrapper (afk run)

```bash
afk run 5                        # Run 5 iterations
afk run 10 --branch my-feature   # Create feature branch first
afk run --until-complete         # Run until all tasks done
afk run --timeout 60             # 60 minute timeout
```

afk spawns your configured AI CLI directly and manages the loop. Each iteration gets **fresh context** — memory persists only via git history, `progress.json`, and task sources. This is essential to the Ralph Wiggum pattern.

## Git Integration

Enable automatic git operations in config:

```json
{
  "git": {
    "auto_commit": true,
    "auto_branch": true,
    "branch_prefix": "afk/",
    "commit_message_template": "afk: {task_id} - {message}"
  }
}
```

- **auto_commit**: Automatically commit after each task completion
- **auto_branch**: Create feature branches with `--branch` flag
- **branch_prefix**: Prefix for auto-created branches (default: `afk/`)

## Session Archiving

afk archives sessions for later reference:

```bash
afk archive create              # Manually archive current session
afk archive create -r "done"    # Archive with custom reason
afk archive list                # List all archives
afk archive clear               # Archive and clear current session
```

Archives are stored in `.afk/archive/` with timestamps and include:
- `progress.json` - Task completion state
- `prompt.md` - Last generated prompt
- `metadata.json` - Archive context (branch, reason, timestamp)

Archiving happens automatically:
- When starting a new `afk run` with existing progress
- On branch changes (if `on_branch_change: true`)
- When session completes

## Limits & Safety

- `max_iterations`: Stop after N iterations (default: 20)
- `max_task_failures`: Skip task after N failures (default: 3)
- `timeout_minutes`: Stop after N minutes (default: 120)

When limits are reached, `afk next` returns `AFK_LIMIT_REACHED` or `AFK_COMPLETE`.

## Inspired By

- [Ralph Wiggum](https://www.aihero.dev/tips-for-ai-coding-with-ralph-wiggum) by Matt Pocock
- [ghuntley.com/ralph](https://ghuntley.com/ralph/)
- [snarktank/ralph](https://github.com/snarktank/ralph) - Ryan Carson's Ralph implementation
- [Beads](https://github.com/steveyegge/beads)
- [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) by Anthropic

## License

MIT
