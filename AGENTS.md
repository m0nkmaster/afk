# Agent Instructions

**afk** - Autonomous AI coding loops, Ralph Wiggum style.

## Project Overview

This is a Python CLI tool that implements the Ralph Wiggum pattern for autonomous AI coding. It aggregates tasks from multiple sources and generates prompts for AI coding tools.

## Development Setup

```bash
pip install -e ".[dev]"
```

## Code Quality

Before committing, run:

```bash
ruff check .                 # Linting
ruff format .                # Formatting  
mypy src/afk                 # Type checking
pytest                       # Tests (when added)
```

## Architecture

```
src/afk/
├── cli.py           # Click CLI - commands and argument handling
├── config.py        # Pydantic models for .afk/config.json
├── progress.py      # Session and task progress tracking
├── prompt.py        # Jinja2 prompt generation
├── output.py        # Output handlers (clipboard, file, stdout)
└── sources/         # Task source adapters
    ├── beads.py     # Beads (bd) integration
    ├── json_prd.py  # JSON PRD files
    ├── markdown.py  # Markdown checklists
    └── github.py    # GitHub issues via gh CLI
```

## Key Patterns

- **Config**: All settings in `.afk/config.json`, loaded via Pydantic models
- **Progress**: Session state in `.afk/progress.json`, tracks iterations and task status
- **Sources**: Pluggable adapters that return `List[Task]`
- **Prompts**: Jinja2 templates, customizable via config

## Adding a New Task Source

1. Create `src/afk/sources/newsource.py`
2. Implement `load_newsource_tasks() -> list[Task]`
3. Add to `SourceConfig.type` literal in `config.py`
4. Add case to `_load_from_source()` in `sources/__init__.py`

## Session Completion

When ending a work session, you MUST:

1. **Run quality gates** - `ruff check . && mypy src/afk`
2. **Commit and push**:
   ```bash
   git add -A
   git commit -m "descriptive message"
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```

Work is NOT complete until `git push` succeeds.
