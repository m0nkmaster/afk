# afk Sandbox

A minimal TODO app for integration testing `afk` with a real AI CLI.

## Purpose

This sandbox provides a realistic but small Python project with intentional gaps (unimplemented methods) that `afk` can work on. Use it to verify the full `afk` workflow works correctly after making changes.

## Quick Start

From the project root:

```bash
# Set up the sandbox (init git, install deps, init afk)
make sandbox-setup

# Navigate to sandbox
cd examples/sandbox

# Parse the PRD (copy prompt to clipboard, paste to AI)
afk prd parse PRD.md --copy

# After the AI creates prd.json, run afk
afk go 3
```

## What's Here

```
sandbox/
├── PRD.md                 # Requirements with 5 small tasks
├── pyproject.toml         # Python package config
├── src/todo/
│   ├── __init__.py
│   ├── app.py             # CLI with some TODOs
│   └── models.py          # Data models with some TODOs
└── tests/
    └── test_todo.py       # Tests (some will fail initially)
```

## The Tasks

The PRD contains 5 small, well-defined tasks:

1. **Implement TaskList.remove()** - Remove a task by ID
2. **Implement TaskList.list_pending()** - Filter incomplete tasks
3. **Implement TaskList.list_by_priority()** - Filter by priority
4. **Implement save_tasks()** - Persist tasks to JSON
5. **Implement cmd_done()** - CLI command to complete a task

Each task has tests that will fail until implemented.

## Commands

### From project root

```bash
make sandbox-setup    # Initialise everything
make sandbox-run      # Run afk go 3
make sandbox-reset    # Reset to initial state
make sandbox-status   # Show current state
```

### From sandbox directory

```bash
# Parse PRD and create task list
afk prd parse PRD.md --copy

# Run iterations
afk go 3              # 3 iterations
afk go 10             # 10 iterations
afk go -u             # Until complete

# Check status
afk explain           # See what's next
afk prd show          # Show PRD state

# Run tests manually
pytest -v
```

## Reset & Retry

To start fresh:

```bash
make sandbox-reset
make sandbox-setup
```

This resets all files to their initial state and removes `.afk/`.

## Notes

- The sandbox uses its own `.git` repository (separate from the main project)
- Changes made by the AI are tracked in the sandbox's git history
- The `.afk/` directory is created during setup and removed on reset
- Tests are designed to fail initially - that's the point!
