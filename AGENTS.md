# Agent Instructions

**afk** - Autonomous AI coding loops, Ralph Wiggum style.

## Project Overview

This is a Python CLI tool that implements the Ralph Wiggum pattern for autonomous AI coding. It aggregates tasks from multiple sources and generates prompts for AI coding tools.

## Development Setup

```bash
pip install -e ".[dev]"
```

## Issue Tracking

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Code Quality

Before committing, run:

```bash
ruff check .                 # Linting
ruff format .                # Formatting  
mypy src/afk                 # Type checking
pytest                       # Tests with coverage
```

## Testing

This project maintains test coverage with pytest. Tests are required for all changes.

Write tests first. Follow the red-green-refactor test driven development process (TDD).

### Running Tests

```bash
pytest                       # Run all tests with coverage
pytest -v                    # Verbose output
pytest tests/test_config.py  # Run specific test file
pytest -k "test_load"        # Run tests matching pattern
```

### Test Structure

Tests are organised by module:
- `tests/test_config.py` - Configuration models
- `tests/test_progress.py` - Session and task progress
- `tests/test_bootstrap.py` - Project analysis
- `tests/test_prompt.py` - Prompt generation
- `tests/test_output.py` - Output handlers
- `tests/test_cli.py` - CLI commands
- `tests/test_git_ops.py` - Git operations
- `tests/test_runner.py` - Autonomous loop runner
- `tests/test_prd.py` - PRD parsing
- `tests/test_sources*.py` - Task source adapters
- `tests/test_art.py` - ASCII art spinners
- `tests/test_feedback.py` - Feedback display (WIP)
- `tests/test_file_watcher.py` - File system watcher (WIP)
- `tests/test_output_parser.py` - AI output parsing (WIP)

### Writing Tests

1. **Use fixtures** from `tests/conftest.py` for common setup (temp directories, sample data)
2. **Mock external calls** - subprocess, clipboard, file I/O where appropriate
3. **Test edge cases** - empty inputs, missing files, error conditions
4. **Keep tests focused** - one behaviour per test

### Coverage Requirements

- Coverage report generated as HTML in `htmlcov/`
- New code must include corresponding tests

## Architecture

```
src/afk/
├── cli.py           # Click CLI - commands and argument handling
├── config.py        # Pydantic models for .afk/config.json
├── bootstrap.py     # Project analysis and auto-configuration
├── progress.py      # Session and task progress tracking (includes per-task learnings)
├── prompt.py        # Jinja2 prompt generation
├── output.py        # Output handlers (clipboard, file, stdout)
├── prd.py           # PRD parsing prompt generation
├── prd_store.py     # PRD storage, sync, and task aggregation
├── runner.py        # Autonomous loop runner (Ralph pattern)
├── git_ops.py       # Git operations (branch, commit, archive)
├── art.py           # ASCII art and spinner animations
├── feedback.py      # Real-time feedback display (WIP)
├── file_watcher.py  # File system monitoring (WIP)
├── output_parser.py # AI CLI output parsing (WIP)
└── sources/         # Task source adapters
    ├── beads.py     # Beads (bd) integration
    ├── json_prd.py  # JSON PRD files
    ├── markdown.py  # Markdown checklists
    └── github.py    # GitHub issues via gh CLI
```

## Key Patterns

- **Config**: All settings in `.afk/config.json`, loaded via Pydantic models
- **PRD File**: `.afk/prd.json` is the working task list; used directly if no sources configured
- **Progress**: Session state in `.afk/progress.json`, tracks iterations, task status, and per-task learnings (short-term memory)
- **AGENTS.md**: Long-term learnings go in `AGENTS.md` at project root or in subfolders for folder-specific knowledge
- **Sources**: Pluggable adapters (beads, json, markdown, github) that sync into prd.json
- **Prompts**: Jinja2 templates, customizable via config
- **Runner**: Implements Ralph Wiggum pattern - spawns fresh AI CLI each iteration
- **Fresh Context**: Each iteration gets clean context; memory persists via git + progress.json + AGENTS.md
- **Quality Gates**: Feedback loops (lint, test, types) run before auto-commit
- **Archiving**: Sessions archived on completion, branch change, or manually

## Key Commands

```bash
afk go                 # Zero-config: auto-detect PRD/sources and run
afk go 20              # Run 20 iterations
afk start              # Init if needed + run loop
afk run N              # Run N iterations
afk explain            # Debug current loop state
afk verify             # Run quality gates (lint, test, types)
afk done <task-id>     # Mark task complete
afk fail <task-id>     # Mark task failed
afk reset <task-id>    # Reset stuck task
afk next               # Preview next prompt
```

## PRD Workflow

The recommended workflow for new projects:

```bash
afk prd parse requirements.md   # Creates .afk/prd.json
afk go                          # Starts working through tasks
```

When `.afk/prd.json` exists with tasks and no sources are configured, `afk go` uses it directly as the source of truth — no configuration required.

## Adding a New Task Source

1. Create `src/afk/sources/newsource.py`
2. Implement `load_newsource_tasks() -> list[UserStory]`
3. Add to `SourceConfig.type` literal in `config.py`
4. Add case to `_load_from_source()` in `sources/__init__.py`

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
