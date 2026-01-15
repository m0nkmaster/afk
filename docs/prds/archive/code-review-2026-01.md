# Codebase Review Report: afk

**Date:** 2026-01-14  
**Reviewer:** AI Code Review  
**Version:** v0.4.10

## Executive Summary

**afk** is a well-structured Rust CLI tool implementing the "Ralph Wiggum pattern" for autonomous AI coding loops. The codebase demonstrates solid Rust practices with comprehensive test coverage, clean module organisation, and consistent patterns throughout.

**Overall Assessment: Good quality codebase with room for incremental improvements.**

| Category | Rating | Notes |
|----------|--------|-------|
| Code Quality | 🟢 Good | Clean architecture, consistent style |
| Test Coverage | 🟢 Good | Comprehensive unit tests across modules |
| Error Handling | 🟢 Good | Uses `Result`, `thiserror`, proper propagation |
| Documentation | 🟡 Moderate | Good module docs, some functions lack detail |
| Dead Code | 🟡 Moderate | 16 `#[allow(dead_code)]` annotations |
| Static Analysis | 🟢 Good | Clippy passes cleanly, no warnings |

---

## Strengths

### 1. Excellent Module Organisation

The codebase follows a logical, domain-driven structure:
- Clear separation between CLI layer (`cli/`), core business logic (`runner/`, `progress/`), and integrations (`sources/`)
- Proper use of `mod.rs` for module exports and submodule organisation
- Public API is well-defined in `lib.rs`

### 2. Comprehensive Test Coverage

- Unit tests inline with modules (`#[cfg(test)] mod tests`)
- Tests cover happy paths, edge cases, and error conditions
- All 219+ tests pass (verified with `cargo test -- --test-threads=1`)
- Good use of `tempfile` for filesystem tests

### 3. Clean Error Handling

- Consistent use of `Result<T, E>` return types
- Domain-specific error types via `thiserror` (`ConfigError`, `ProgressError`, `PrdError`, `SourceError`)
- Proper error propagation with `?` operator
- `unwrap()` calls are almost entirely confined to test code

### 4. Static Analysis Passes

- Clippy passes with no warnings
- `cargo check` succeeds
- No compiler warnings

### 5. Good CLI Design

- Uses `clap` derive macros effectively
- Well-structured command hierarchy
- Helpful help text and examples embedded in code

---

## Issues & Recommendations

### HIGH PRIORITY

#### 1. Large Files Need Splitting

Several files exceed reasonable size for maintainability:

| File | Lines | Recommendation |
|------|-------|----------------|
| `src/cli/mod.rs` | 2,006 | Split command implementations into `commands/` submodules |
| `src/config/mod.rs` | 1,656 | Extract validation logic into separate file |
| `src/progress/mod.rs` | 1,407 | Extract archive logic into `archive.rs` |
| `src/feedback/display.rs` | 1,034 | Consider splitting spinner/panel logic |

**Action:** Refactor `cli/mod.rs` to move each command's `execute()` implementation into `cli/commands/{command}.rs`. The file currently mixes Clap struct definitions with business logic.

#### 2. Dead Code Cleanup

16 items marked with `#[allow(dead_code)]`:

```
src/tui/ui.rs: 4 unused sidebar functions (draw_sidebar, draw_task_panel, etc.)
src/tui/app.rs: 3 unused TuiStats fields
src/runner/controller.rs: 2 unused fields
src/runner/iteration.rs: 1 unused field
src/runner/quality_gates.rs: 1 unused function
src/watcher/mod.rs: 1 unused method
src/git/mod.rs: 1 unused function
src/sources/beads.rs: 1 unused helper
src/cli/commands/config.rs: 1 unused helper
```

**Action:** Audit each `#[allow(dead_code)]`. Either:
- Remove truly dead code
- Mark as `#[cfg(feature = "...")]` if for future use
- Add documentation explaining why it's retained

### MEDIUM PRIORITY

#### 3. Inconsistent Error Return Patterns

Some execute methods use `eprintln!` + `std::process::exit()` directly instead of returning errors.

**Action:** Consider returning `Result<ExitCode, Error>` from execute methods for better testability and consistency. The main.rs entrypoint can convert to exit codes.

#### 4. Blocking I/O in Async Context

The runner uses `tokio` runtime but some operations are blocking:
- Child process spawning via `std::process::Command`
- Git operations in `src/git/mod.rs`

**Action:** For now this is acceptable since the loop is inherently sequential, but consider using `tokio::process::Command` if parallelism is needed in future.

#### 5. Magic Numbers and Hardcoded Values

Various hardcoded values could be constants or config:
- `max_output_lines` defaults to 500
- Activity thresholds (2s, 10s)

**Action:** Consider making these configurable via `AfkConfig` or at minimum consolidate into a single constants module.

#### 6. Progress Module unwrap() Pattern

The `set_task_status` method uses an unnecessary `unwrap()` after inserting a key.

**Action:** Return `&TaskProgress` from the `entry()` call directly instead of re-fetching.

### LOW PRIORITY

#### 7. Doc Comment Coverage

Most public functions have doc comments, but some are missing.

**Action:** Add `#[deny(missing_docs)]` to `lib.rs` and fill in gaps.

#### 8. Test Organisation

Integration tests in `tests/` are minimal. Most testing is via inline unit tests.

**Action:** Consider adding more end-to-end CLI tests using `assert_cmd` crate for full workflow verification.

#### 9. Duplicate Ignore Pattern Logic

File ignore patterns are duplicated between `watcher/mod.rs` and potentially other places.

**Action:** Consider extracting a shared `PathMatcher` utility.

---

## Architecture Notes

### What's Working Well

1. **PRD/Task Model** - Clean separation between `UserStory` (from sources) and `TaskProgress` (runtime state)
2. **Source Aggregation** - Pluggable adapter pattern for beads, json, markdown, github sources
3. **Template System** - Tera templates for prompt generation with sensible defaults
4. **TUI Integration** - Optional ratatui frontend cleanly separated from core loop logic

### Potential Future Improvements

1. **Plugin System** - Sources could become proper plugins loadable at runtime
2. **Parallel Task Execution** - Currently sequential; could parallelise independent tasks
3. **Metrics Persistence** - `MetricsCollector` data isn't persisted across sessions

---

## Concrete Action Items

### Immediate (This Session)

- [x] Verify tests pass: `cargo test -- --test-threads=1`
- [x] Verify clippy passes: `cargo clippy`
- [x] Document findings (this report)

### Short-Term (Next 1-2 Sessions)

| Priority | Task | Effort |
|----------|------|--------|
| High | Split `cli/mod.rs` - move execute() impls to commands/ | 2-3 hours |
| High | Audit and remove dead code (16 items) | 1 hour |
| Medium | Fix `set_task_status` unwrap pattern | 15 mins |
| Medium | Add `#[deny(missing_docs)]` and fill gaps | 1-2 hours |

### Medium-Term (Backlog)

| Priority | Task | Effort |
|----------|------|--------|
| Medium | Refactor execute() to return Result | 2-3 hours |
| Low | Extract PathMatcher utility | 30 mins |
| Low | Add more integration tests | 2-3 hours |

---

## Metrics Summary

- **Total Lines of Code:** ~23,112 (including tests)
- **Test Count:** 219+ unit tests, 4 doc-tests (ignored)
- **Modules:** 20+ source modules
- **Dependencies:** 16 direct dependencies
- **Clippy Warnings:** 0
- **Dead Code Annotations:** 16

---

## Conclusion

This is a well-maintained Rust codebase with good practices. The main areas for improvement are:

1. **File size** - Split the largest files for maintainability
2. **Dead code** - Remove or document the 16 suppressed items  
3. **Error patterns** - Consider more consistent Result-based returns from CLI

No critical bugs or security issues were identified. The codebase is in good shape for continued development.
