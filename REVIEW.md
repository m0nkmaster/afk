# Code review: afk (Rust CLI)

**Scope:** Repository-wide static review of architecture, maintainability, safety, and testing.  
**Date:** 2026-04-04  
**Branch reviewed:** `code-review-ada-20260404-160020` (aligned with `main` at merge of performance work)  
**Note:** `cargo clippy` / `cargo test` were not executed in this review environment (Rust toolchain unavailable). CI and local runs should be treated as the source of truth for build health.

---

## Overall impressions

**afk** is a well-scoped Rust CLI with a clear domain model (config, PRD/tasks, progress, runner loop, pluggable sources). The layout matches the mental model described in `AGENTS.md`: thin `main.rs`, library crate with focused modules, and integration tests that exercise the binary. The project shows intentional investment in **documentation** (`#![deny(missing_docs)]` on the library), **testability** (e.g. `CliResult` / `ExitCode` instead of scattering `process::exit` in command handlers), and **operational ergonomics** (sleep guard, archive, multi-model selection, optional PTY).

The codebase reads as production-minded: errors are often surfaced to the user, external tools (`git`, `gh`, source-specific CLIs) are invoked via `std::process::Command` with structured arguments rather than shell strings, and there is **no `unsafe` usage** in `src/`.

---

## Strengths

1. **Module boundaries** — Separation between `cli` (parsing and dispatch), `config` (serde models + validation), `runner` (loop + iteration + quality gates), `sources` (adapters), `prd` / `progress` (state), and `parser` / `watcher` (I/O and side channels) is coherent and easy to navigate.

2. **Documentation culture** — Library-wide `missing_docs` denial and module-level docs make onboarding and public API clarity a first-class concern.

3. **CLI design** — `clap` derive API, subcommands, and centralized `handle_result` keep behaviour consistent. Deprecation of raw exits in favour of `CliResult` improves testability.

4. **Performance awareness** — `PrdCache` in `runner/controller.rs` avoids re-reading `tasks.json` on every loop iteration by keying off file `mtime`, which is appropriate for a hot path.

5. **Concurrency model for sources** — `aggregate_tasks` uses a fast path for a single source and spawns threads for multiple sources while preserving declaration order on join. Panics in worker threads are contained (logged, empty vec), avoiding a full process abort.

6. **Test surface** — Large `tests/cli_integration.rs` plus `tests/prompt_snapshots.rs` and extensive `#[cfg(test)]` blocks provide regression coverage for CLI and templates. `AGENTS.md` documents macOS `notify` constraints and single-threaded test execution.

7. **Feature flags** — Optional `pty` feature isolates heavier dependencies (`portable-pty`, `vte`) from default builds.

8. **Self-update** — `cli/update.rs` uses structured errors (`thiserror`), platform-specific asset naming, and a clear story for pip-installed binaries.

---

## Areas for improvement

### 1. Unused or misleading API surface

- **`LoopController::run` takes `_resume: bool`** but the parameter does not affect control flow (see `src/runner/controller.rs`). The doc comment says “continue from last session,” yet nothing in `run` / `run_main_loop` branches on it. **Risk:** Callers assume resume semantics that are not implemented here, or resume is handled only indirectly via progress files elsewhere without an explicit contract.

**Recommendation:** Either wire `resume` to observable behaviour (e.g. skip clearing session state, adjust initial iteration count) or remove/rename the parameter and document where “resume” actually happens (`progress.json`, CLI flags, etc.).

### 2. Silent degradation in source loaders

- **`aggregate_tasks` and `load_from_source`** — Individual loaders return empty vectors on failure (by design, per module docs). That keeps the CLI resilient but can **hide misconfiguration** (wrong path, `gh` not installed, beads errors).

**Recommendation:** Consider structured logging or a verbose/diagnostic mode that records per-source load outcomes (success, empty, error with message) without changing default quiet behaviour.

### 3. Duplication and cloning in the runner

- **`LoopController::with_feedback`** builds two `OutputHandler` instances with identical feedback settings (one for the controller, one cloned config path for `IterationRunner`). This is understandable (separate streams) but worth a short comment or a small factory to avoid drift if thresholds change in one place only.

### 4. Threading vs. workload

- **One OS thread per source** in `aggregate_tasks` — Fine for a handful of sources; for many sources, a thread pool or scoped parallelism with a cap might be preferable. Not urgent unless users regularly configure large source lists.

### 5. Panic usage

- **`panic!` in tests** — Present in `cli/mod.rs` tests and parser tests as match exhaustiveness helpers. That is idiomatic for tests. No issue for production paths from this scan.

### 6. Dependency and supply chain

- The project depends on a **modern Rust ecosystem stack** (clap 4, tokio for update, reqwest, ratatui, notify 6). Keeping `cargo audit` / Dependabot-style updates in CI is recommended (not verified here).

### 7. Regex and parsing maintenance

- **`parser/mod.rs`** — Regex-based detection of AI CLI output will require ongoing tuning as CLIs change. The parallel **stream JSON** path (`parser/stream_json.rs`) is the right direction for structured tools; continuing to invest in that path reduces fragility of regex fallbacks.

---

## Specific file and function reviews

### `src/lib.rs`

- Re-exports `aggregate_tasks` and sets `#![deny(missing_docs)]`. Clear public surface. Minor nit: comment “to be implemented in future stories” is stale relative to the current module set.

### `src/runner/controller.rs`

- **`PrdCache`** — Good encapsulation; failure to reload keeps last good PRD, which avoids flapping on transient IO errors.
- **`run_main_loop`** — Clear stop conditions (interrupt, timeout, max iterations, completion). Sync-from-sources when local work is done is a nice touch for multi-source workflows.
- **Archive on complete only** — Documented; aligns with not archiving on interrupt.

### `src/runner/iteration.rs`

- **Prompt short-circuit** for `AFK_COMPLETE` / `AFK_LIMIT_REACHED` — Magic strings coupling template/runner; acceptable for a convention-driven tool but centralizing constants would ease refactors.
- **Model selection** — `select_model()` before spawn is good for UX (display) and for diversity across iterations.

### `src/sources/mod.rs`

- **`aggregate_tasks`** — Ordering preserved via join order; good for deterministic task lists.
- **Panic in worker** — Logged to stderr; consider `tracing` if log volume or filtering becomes important.

### `src/config/mod.rs`

- **Serde models** — Rich enough for real projects; defaults via functions (`default_max_iterations`, etc.) are clear.
- **Doc reference to Python** — Comment mentions mirroring Python Pydantic; if Python code no longer exists in-repo, update comment to avoid confusion.

### `src/cli/update.rs`

- **GitHub API** — Rate-limit error is user-actionable (`gh auth login`). **Binary replacement** paths should remain careful about permissions and atomic replace (code not fully reviewed line-by-line here; worth periodic security review).

### `src/watcher/mod.rs`

- **Bounded `sync_channel(100)`** — Prevents unbounded memory growth from rapid FS events; good defensive choice.

### `src/git/mod.rs`

- Thin wrappers around `git` subprocess — Appropriate; avoids libgit2 dependency. Error handling is mostly boolean/Option; acceptable for CLI helpers.

### `tests/cli_integration.rs`

- **High value** for regression; file is very large. Long-term, splitting by command family (`go`, `config`, `archive`, …) could improve compile times and failure isolation.

---

## Actionable recommendations

| Priority | Action |
|----------|--------|
| **P1** | Resolve **`resume` parameter** semantics in `LoopController::run` vs progress/session state (implement, remove, or document precisely). |
| **P2** | Add **opt-in diagnostics** for source loading failures (per-source errors) without breaking existing “best effort” aggregation. |
| **P2** | Extract **magic strings** (`AFK_COMPLETE`, `AFK_LIMIT_REACHED`) to a single `const` module used by prompt generation and runner. |
| **P3** | Refresh **stale comments** in `lib.rs` and `config/mod.rs` (Python reference). |
| **P3** | Consider splitting **`tests/cli_integration.rs`** into submodules or files when touch churn becomes painful. |
| **P3** | Run **`cargo clippy --all-features`** and **`cargo test -- --test-threads=1`** before releases (documented in `AGENTS.md`); add CI job if not already present. |

---

## Summary

**afk** is a thoughtfully structured Rust CLI with strong modular boundaries, enforced documentation on the library crate, and pragmatic integration testing. The main technical debt visible from static review is **API/documentation alignment** around session resume, **observability** when task sources fail silently, and incremental **maintainability** improvements (constants, comment hygiene, test file size). None of these undermine the overall quality of the design; they are natural next steps as the tool grows.
