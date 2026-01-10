# afk

Autonomous AI coding loops — Ralph Wiggum style.

A tool-agnostic library for using the Ralph Wiggum approach for software development with AI. `afk` runs your AI coding tasks in a loop, letting it work autonomously while you step away from the keyboard.

## Installation

```bash
pip install -e .
```

## Quick Start

```bash
# Initialize in your project
afk init

# Add a task source
afk source add json prd.json      # JSON PRD file
afk source add markdown tasks.md  # Markdown checklist
afk source add beads              # Use beads (bd) for issues
afk source add github             # GitHub issues

# Check status
afk status

# Get the next iteration prompt
afk next              # Print to stdout
afk next --copy       # Copy to clipboard
afk next --file       # Write to .afk/prompt.md

# After AI completes work
afk done <task-id>

# Run multiple iterations (with configured AI CLI)
afk run 5
```

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
afk run 5
```

afk spawns your configured AI CLI directly and manages the loop.

## Limits & Safety

- `max_iterations`: Stop after N iterations (default: 20)
- `max_task_failures`: Skip task after N failures (default: 3)
- `timeout_minutes`: Stop after N minutes (default: 120)

When limits are reached, `afk next` returns `AFK_LIMIT_REACHED` or `AFK_COMPLETE`.

## Inspired By

- [Ralph Wiggum](https://www.aihero.dev/tips-for-ai-coding-with-ralph-wiggum) by Matt Pocock
- [ghuntley.com/ralph](https://ghuntley.com/ralph/)
- [Beads](https://github.com/steveyegge/beads)

## License

MIT
