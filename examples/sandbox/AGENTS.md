# Agent Instructions

This is a minimal TODO app for testing afk.

## Project Structure

- `src/todo/models.py` - Data models (Task, TaskList)
- `src/todo/app.py` - CLI commands
- `tests/test_todo.py` - All tests

## Development

```bash
pip install -e ".[dev]"   # Install
pytest                     # Run tests
ruff check .              # Lint
```

## Current State

There are several unimplemented methods marked with `# TODO`. Tests exist for all of them and will fail until implemented.

## What to Do

1. Read `.afk/prd.json` to see pending tasks
2. Implement the method/function described
3. Run `pytest` to verify
4. Mark the story as complete when tests pass
