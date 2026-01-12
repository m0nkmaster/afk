# Contributing to afk

Thank you for your interest in contributing to afk! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- **Rust 1.85+** — Install via [rustup](https://rustup.rs/)
- **Git** — For version control

### Getting Started

```bash
# Clone the repository
git clone https://github.com/robo-mac/afk.git
cd afk

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the binary
./target/release/afk --help
```

## Code Quality

Before submitting a PR, ensure your code passes all quality checks:

```bash
# Format code
cargo fmt

# Check formatting (CI check)
cargo fmt -- --check

# Lint with Clippy
cargo clippy --all-targets --all-features

# Run tests (single-threaded, required for macOS)
cargo test -- --test-threads=1
```

### Why Single-Threaded Tests?

Tests must run single-threaded due to the `notify` crate's FSEvents backend on macOS. Parallel test execution can cause hangs. Always use:

```bash
cargo test -- --test-threads=1
# or
RUST_TEST_THREADS=1 cargo test
```

## Project Structure

```
src/
├── main.rs          # Entry point
├── lib.rs           # Library exports
├── cli/             # CLI commands and argument handling
│   ├── mod.rs       # Clap CLI definitions
│   ├── commands/    # Subcommand implementations
│   ├── output.rs    # Output formatting
│   └── update.rs    # Self-update logic
├── config/          # Configuration models
├── bootstrap/       # Project analysis, AI CLI detection
├── progress/        # Session and task progress tracking
├── prompt/          # Tera template rendering
├── prd/             # PRD document model and parsing
├── runner/          # Loop controller and iteration runner
├── git/             # Git operations
├── feedback/        # Metrics and ASCII art
├── parser/          # AI CLI output parsing
├── watcher/         # File system monitoring
└── sources/         # Task source adapters
    ├── beads.rs     # Beads integration
    ├── json.rs      # JSON PRD files
    ├── markdown.rs  # Markdown checklists
    └── github.rs    # GitHub issues
```

## Writing Tests

Tests are inline with modules using `#[cfg(test)] mod tests`. Guidelines:

1. **Use tempfile** for temporary directories
2. **Mock external calls** via Command patterns where appropriate
3. **Test edge cases** — empty inputs, missing files, error conditions
4. **Keep tests focused** — one behaviour per test

### Example Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_loads_from_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".afk/config.json");
        
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, r#"{"sources": []}"#).unwrap();
        
        let config = AfkConfig::load(Some(&config_path)).unwrap();
        assert!(config.sources.is_empty());
    }
}
```

## Adding a New Task Source

1. Create `src/sources/newsource.rs`
2. Implement `load_newsource_tasks() -> Result<Vec<UserStory>, Error>`
3. Add variant to `SourceType` enum in `config/mod.rs`
4. Add match arm to `load_from_source()` in `sources/mod.rs`
5. Add `mod newsource;` to `sources/mod.rs`
6. Write tests inline with `#[cfg(test)] mod tests`

## Commit Messages

Use conventional commits with short, single-line messages:

```
feat: add GitHub issues source
fix: handle empty PRD gracefully
docs: update installation instructions
refactor: simplify prompt generation
test: add coverage for edge cases
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run quality checks (`cargo fmt && cargo clippy && cargo test -- --test-threads=1`)
5. Commit with a clear message
6. Push to your fork
7. Open a PR against `main`

## Issue Tracking

This project uses [beads](https://github.com/steveyegge/beads) for issue tracking:

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Code Style

- Use British English in comments and documentation
- Follow Rust idioms and best practices
- Prefer explicit error handling over `.unwrap()` in library code
- Document public APIs with `///` doc comments

## Questions?

Open an issue or discussion on GitHub. We're happy to help!
