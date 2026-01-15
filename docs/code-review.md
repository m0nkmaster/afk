# afk Codebase Review

## Executive Summary

**Overall Assessment: Well-structured, production-ready Rust CLI with good practices.**

The `afk` codebase is a well-organised Rust CLI tool implementing autonomous AI coding loops. The code demonstrates strong Rust idioms, comprehensive test coverage, and thoughtful architecture. There are opportunities for improvement in error handling consistency, reducing code duplication, and enhancing type safety in some areas.

| Category | Rating | Notes |
|----------|--------|-------|
| Architecture | ★★★★☆ | Clean module separation, clear responsibilities |
| Readability | ★★★★☆ | Good naming, adequate documentation |
| Test Coverage | ★★★★★ | Comprehensive inline tests |
| Error Handling | ★★★☆☆ | Inconsistent use of Result types |
| Type Safety | ★★★★☆ | Strong types, some stringly-typed areas |
| Code Duplication | ★★★☆☆ | Several opportunities to DRY |

---

## Architecture Analysis

### Strengths

1. **Clean Module Organisation**
   - Clear separation of concerns: `cli/`, `config/`, `runner/`, `sources/`, etc.
   - Each module has a single responsibility
   - Public API exposed via `mod.rs` files with explicit re-exports

2. **Configuration System**
   - Serde-based with sensible defaults
   - Good use of `#[serde(default)]` and `#[serde(skip_serializing_if)]`
   - Constants centralised (`AFK_DIR`, `CONFIG_FILE`, etc.)

3. **Error Types**
   - Per-module error types using `thiserror`
   - Clear error messages with context

4. **Builder Pattern**
   - `RunOptions` uses fluent builder pattern effectively
   - Enables composable configuration

### Areas for Improvement

1. **Circular Dependencies**
   - Some modules import heavily from each other (e.g., `prd` ↔ `sources`)
   - Consider introducing a shared types module

---

## Readability Assessment

### Positives

- Consistent naming conventions (snake_case for functions, CamelCase for types)
- Module-level documentation comments (`//!`) explain purpose
- Functions have doc comments with `# Arguments` and `# Returns` sections
- British English spelling consistency in comments

### Issues

1. **Magic Strings**
   Several magic strings appear throughout:

   ```rust
   // In iteration.rs and controller.rs:
   if error == "AFK_COMPLETE" { ... }
   if error == "AFK_LIMIT_REACHED" { ... }
   ```

   **Recommendation:** Extract to constants or use an enum:

   ```rust
   pub enum StopSignal {
       Complete,
       LimitReached,
   }
   ```

2. **ANSI Escape Codes Scattered**
   Hard-coded ANSI codes throughout:

   ```rust
   println!("\x1b[31mError:\x1b[0m {e}");
   ```

   **Recommendation:** Centralise colour formatting:

   ```rust
   mod colours {
       pub fn error(msg: &str) -> String { format!("\x1b[31m{}\x1b[0m", msg) }
       pub fn success(msg: &str) -> String { format!("\x1b[32m{}\x1b[0m", msg) }
   }
   ```

3. **Long Function Bodies**
   Several functions exceed 50 lines:
   - `GoCommand::execute()` - ~100 lines
   - `run_main_loop()` - ~120 lines
   - `run_loop_with_tui_sender()` - ~200 lines

   **Recommendation:** Extract logical sections into helper functions.

---

## Error Handling

### Current State

Mixed approaches to error handling:

| Pattern | Usage | Assessment |
|---------|-------|------------|
| `Result<T, E>` | Core library functions | ✓ Good |
| `.unwrap_or_default()` | Loading optional files | ✓ Acceptable |
| `std::process::exit(1)` | CLI commands | ⚠ Could be cleaner |
| `eprintln!` + return | Various places | ⚠ Inconsistent |

### Issues

1. **Exit Codes in Library Code**

   ```rust
   // In cli/mod.rs - mixing exit codes into what should be library logic
   if !path.exists() {
       eprintln!("\x1b[31mError:\x1b[0m Source file not found: {source_path}");
       std::process::exit(1);
   }
   ```

   **Recommendation:** Return `Result` types and handle exit in `main()`.

2. **Silent Failures**

   ```rust
   // In controller.rs
   let _ = mark_story_in_progress(&task.id);  // Failure silently ignored
   ```

   **Recommendation:** Log failures or handle explicitly.

3. **Error Propagation**

   Many functions use pattern:

   ```rust
   match result {
       Ok(()) => {}
       Err(e) => {
           eprintln!("\x1b[31mError:\x1b[0m {e}");
           std::process::exit(1);
       }
   }
   ```

   **Recommendation:** Use `?` operator with a top-level error handler.

---

## Code Duplication

### Identified Duplications

1. **Config/PRD/Progress Loading Pattern**

   Identical pattern in three modules:

   ```rust
   pub fn load(path: Option<&Path>) -> Result<Self, Error> {
       let path = path
           .map(PathBuf::from)
           .unwrap_or_else(|| PathBuf::from(DEFAULT_PATH));

       if !path.exists() {
           return Ok(Self::default());
       }

       let contents = fs::read_to_string(&path)?;
       let data: Self = serde_json::from_str(&contents)?;
       Ok(data)
   }
   ```

   **Recommendation:** Extract to a trait or generic function:

   ```rust
   pub trait JsonPersistence: Default + DeserializeOwned + Serialize {
       const DEFAULT_PATH: &'static str;

       fn load(path: Option<&Path>) -> Result<Self, PersistenceError> {
           // Generic implementation
       }

       fn save(&self, path: Option<&Path>) -> Result<(), PersistenceError> {
           // Generic implementation
       }
   }
   ```

2. **Task Status Updates**

   Similar patterns in `DoneCommand`, `FailCommand`, `ResetCommand`:

   ```rust
   let mut progress = match SessionProgress::load(None) {
       Ok(p) => p,
       Err(e) => {
           eprintln!("\x1b[31mError loading progress:\x1b[0m {e}");
           std::process::exit(1);
       }
   };
   ```

   **Recommendation:** Extract to helper function.

3. **Source Type Handling**

   Repeated in multiple files:

   ```rust
   match &source.source_type {
       SourceType::Beads => "beads".to_string(),
       SourceType::Json => format!("json: {}", source.path.as_deref().unwrap_or("?")),
       SourceType::Markdown => format!("markdown: {}", source.path.as_deref().unwrap_or("?")),
       SourceType::Github => format!("github: {}", source.repo.as_deref().unwrap_or("current repo")),
   }
   ```

   **Recommendation:** Implement `Display` for `SourceConfig`.

---

## Type Safety

### Good Practices

- Strong enums for states (`TaskStatus`, `StopReason`, `ChangeType`)
- Newtype patterns where appropriate
- `#[non_exhaustive]` could be added to public enums for future compatibility

### Improvement Areas

1. **Stringly-Typed IDs**

   ```rust
   pub fn set_task_status(&mut self, task_id: &str, ...) { ... }
   pub fn get_task(&self, task_id: &str) -> Option<&TaskProgress> { ... }
   ```

   **Recommendation:** Consider a `TaskId` newtype for compile-time safety:

   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
   pub struct TaskId(String);
   ```

2. **Priority as i32**

   ```rust
   pub priority: i32,  // In UserStory
   ```

   Priority is always 1-5. Consider:

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
   pub enum Priority {
       Critical = 1,
       High = 2,
       Medium = 3,
       Low = 4,
       Backlog = 5,
   }
   ```

3. **Feedback Mode Duplication**

   Two enums for feedback mode:
   - `FeedbackMode` in `config/mod.rs`
   - `FeedbackMode` in `runner/output_handler.rs`

   **Recommendation:** Consolidate to a single type.

---

## Test Quality

### Strengths

- **Comprehensive Coverage**: Every module has inline `#[cfg(test)] mod tests`
- **Test Isolation**: Good use of `tempfile::TempDir` for file system tests
- **Round-Trip Tests**: Serialisation/deserialisation tested thoroughly
- **Edge Cases**: Empty inputs, missing files, error conditions covered
- **Real-World Formats**: Tests include actual JSON formats from production

### Areas to Consider

1. **Integration Tests**
   - `tests/cli_integration.rs` exists but could cover more scenarios
   - Consider adding end-to-end workflow tests

2. **Property-Based Testing**
   - For complex parsing logic (PRD, markdown sources), consider `proptest`

3. **Mock External Commands**
   - Tests that run `git` or `bd` assume they're available
   - Consider mocking or feature-gating these tests

---

## Performance Considerations

### Current State

- File I/O is synchronous (appropriate for CLI)
- `tokio` dependency exists but appears unused in core logic
- File watcher uses bounded channels to prevent memory issues

### Potential Optimisations

1. **Remove Unused Async Runtime**

   `Cargo.toml` includes:

   ```toml
   tokio = { version = "1.43", features = ["full"] }
   ```

   But the codebase is synchronous. Either:
   - Remove tokio if unused
   - Or migrate to async where beneficial (HTTP, file I/O)

2. **Lazy Template Compilation**

   ```rust
   let mut tera = Tera::default();
   tera.add_raw_template("prompt", &template_str)?;
   ```

   Template is recompiled each iteration. Consider caching.

---

## Security Considerations

### Concerns

1. **Command Injection Risk**

   ```rust
   let process = Command::new(shell)
       .args([shell_arg, cmd])
       .spawn();
   ```

   User-provided commands are executed via shell. This is intentional (quality gates), but should be documented.

2. **Path Traversal**

   ```rust
   let path = source.path.as_deref();
   ```

   Source paths are user-controlled. Consider validation.

### Recommendations

- Add warnings in config documentation about command execution
- Validate paths stay within project directory

---

## Dependency Review

### Cargo.toml Analysis

| Crate | Purpose | Assessment |
|-------|---------|------------|
| `clap` | CLI framework | ✓ Standard choice |
| `serde` | Serialisation | ✓ Essential |
| `tera` | Templates | ✓ Well-maintained |
| `notify` | File watching | ✓ Note: FSEvents issues on macOS |
| `ratatui` | TUI | ✓ Good choice |
| `reqwest` | HTTP | ⚠ Only used for self-update |
| `tokio` | Async runtime | ⚠ Appears unused |
| `arboard` | Clipboard | ✓ Cross-platform |

### Recommendations

1. **Review tokio usage** - if truly unused, remove to reduce compile time
2. **Consider feature flags** - `reqwest` could be optional if self-update is rarely used

---

## Proposed Improvements

### Priority 1 (High Impact, Low Effort)

1. **Centralise colour formatting**
   - Create `src/cli/colours.rs`
   - Replace all `\x1b[...` with function calls

2. **Consolidate FeedbackMode enums**
   - Keep one definition in `config/mod.rs`
   - Re-export in `runner/mod.rs`

### Priority 2 (Medium Impact)

4. **Implement `Display` for `SourceConfig`**
   - Removes 4+ duplicated match blocks

5. **Extract JSON persistence trait**
   - Reduces ~100 lines of duplicated code
   - Improves consistency

6. **Add `TaskId` newtype**
   - Catches ID mismatches at compile time
   - Document ID format

### Priority 3 (Long-term)

7. **Async migration (optional)**
   - If keeping `tokio`, migrate file I/O
   - Otherwise remove the dependency

8. **Property-based tests**
   - Add `proptest` for parser modules

9. **Error type consolidation**
   - Create `AfkError` that wraps all module errors
   - Enable `?` propagation throughout

---

## Summary

The `afk` codebase is well-designed and maintainable. The main areas for improvement are:

1. **DRYing up repeated patterns** - persistence, error handling
2. **Improving type safety** - IDs, priorities, feedback modes
3. **Removing dead dependencies** - tokio if unused

The test coverage is excellent, and the code follows Rust best practices. CLI command implementations have been extracted to `cli/commands/`.

---

*Review conducted: January 2026*
