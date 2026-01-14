# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.3] - 2026-01-14

### Added

- **Real-time AI output** — Parse NDJSON stream-json from Cursor and Claude CLIs
- **Unified stream parser** — Auto-detects CLI format and normalises events
- **Live progress display** — Shows assistant messages, tool calls, and file changes as they happen
- **Simplified TUI layout** — Compact header bar with stats, full-width scrollable output

### Changed

- Default output format is now `stream-json` (configurable via `output_format: "text"` for legacy behaviour)
- AI CLI args auto-include streaming flags based on detected CLI type

### Technical

- New `StreamJsonParser` in `src/parser/stream_json.rs`
- Added `AiOutputFormat` enum and `full_args()` helper to `AiCliConfig`
- TUI sidebar removed in favour of header-integrated stats

## [1.0.0-rc1] - 2026-01-12

### Added

- **Complete Rust rewrite** — Native binary with no runtime dependencies
- **Zero-config mode** — `afk go` auto-detects project type, AI CLI, and task sources
- **Multiple task sources** — Beads, JSON PRD, Markdown checklists, GitHub issues
- **Quality gates** — Configurable lint, test, typecheck, and build commands
- **Session management** — Progress tracking, archiving, and resume support
- **Self-update** — `afk update` downloads the latest release
- **Shell completions** — Bash, Zsh, and Fish support via `afk completions`

### Commands

- `afk go [N]` — Zero-config entry point, runs N iterations (default 10)
- `afk run [N]` — Run N iterations with configured settings
- `afk start` — Init if needed, then run
- `afk resume` — Continue from last session
- `afk init` — Analyse project and generate config
- `afk status` — Show current configuration
- `afk explain` — Debug current loop state
- `afk next` — Preview next prompt
- `afk verify` — Run quality gates
- `afk done <id>` — Mark task complete
- `afk fail <id>` — Mark task failed
- `afk reset <id>` — Reset stuck task
- `afk prd parse <file>` — Parse requirements into JSON
- `afk prd sync` — Sync from all sources
- `afk prd show` — Display current tasks
- `afk source add|list|remove` — Manage task sources
- `afk archive create|list|clear` — Session archives
- `afk update` — Self-update to latest version
- `afk completions <shell>` — Generate shell completions

### Supported AI CLIs

- Claude Code (`claude`)
- Cursor Agent (`agent`)
- Codex (`codex`)
- Aider (`aider`)
- Amp (`amp`)
- Kiro (`kiro`)

### Technical

- Built with Rust 1.85 (2024 edition)
- Cross-platform: Linux (x86_64, arm64), macOS (Intel, Apple Silicon), Windows
- Single static binary, no dependencies
- ~580 unit and integration tests

## [Unreleased]

### Planned

- Watch mode for continuous development
- Plugin system for custom sources
- Web UI for monitoring
