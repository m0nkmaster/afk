# AFK Evolution: Applying GSD Learnings (Tool-Agnostic)

## Honest Assessment: Is This Worth Doing?

### The Case For

1. **AFK has a genuine architectural advantage**: tool-agnosticism. GSD is locked to Claude Code (and recently Gemini/OpenCode). If Codex, Cursor Agent, Amp, or Kiro gain traction, GSD users are stranded. AFK users just `afk use codex`.
2. **The gap is in methodology, not runtime**: Everything GSD does _well_ (planning, research, parallel execution, SOPs) can be replicated by an external orchestrator. AFK's Rust loop is more reliable than prompts-as-control-flow.
3. **Market positioning**: GSD is "the Claude Code enhancement." AFK could be "the universal AI dev loop." These are different markets.

### The Case Against

1. **GSD's sub-agent model is native and elegant**. Claude Code's `Task()` primitive handles parallelism, context isolation, and result collection in a way that's hard to match externally. AFK would need to manage multiple OS processes, coordinate file locks, and merge results — significantly more complex.
2. **Diminishing returns on prompts**: GSD's SOPs work because Claude Code's inner loop _is_ Claude. When AFK pipes a prompt to `codex` or `aider`, those tools have their _own_ system prompts and behaviours. AFK's prompt injections may conflict or be ignored.
3. **Maintenance burden**: Every new feature doubles the surface area. AFK already supports 6 CLIs, 5 source adapters, TUI, quality gates, model rotation, branch detection, archiving. Adding planning/research/parallel execution is a _lot_ of new code.
4. **Community and momentum**: GSD has Discord, social media virality, a $GSD token (!), and "engineers at Amazon, Google, Shopify" endorsements. Features alone won't close this gap — distribution matters more.

### Verdict

**Worth doing selectively.** Don't try to clone GSD — you'll always be playing catch-up against a project with more social momentum. Instead, pick the 2-3 highest-impact features that leverage AFK's unique position (tool-agnostic, external, reliable) and execute them well.

> [!IMPORTANT]
> The recommended approach is **Phase 1 only** (SOP Prompts + Planning). This is the highest ROI with the lowest risk. Phases 2-3 are optional and should be validated by user demand.

---

## Holistic Interaction Model

Before diving into changes, here's how the commands should compose into **two clear workflows**:

### Workflow A: Full Planning (New Projects)

```
afk plan requirements.md     # Interview → research → decompose → review
  ↓ (writes .afk/tasks.json)
afk go                        # Execute with SOP prompts, per-task verify
```

`afk plan` **replaces** the current `afk import` as the recommended entry point. It adds an interactive element (the AI asks clarifying questions) and produces higher-quality task decomposition via the SOP planning prompt. `afk import` stays for backwards compatibility as the "quick/non-interactive" path.

### Workflow B: Quick Start (Existing Tasks)

```
afk go TODO.md               # Existing behaviour, now with better prompts
afk go tasks.json
afk go 20
```

No change to user behaviour. The improvement is invisible — better prompt templates produce better results.

### Command Relationship

| Command             | Purpose                                           | Interactive?            | Output             |
| ------------------- | ------------------------------------------------- | ----------------------- | ------------------ |
| `afk plan <file>`   | **New.** AI-assisted decomposition with interview | Yes (AI asks questions) | `.afk/tasks.json`  |
| `afk import <file>` | **Existing.** Static AI decomposition             | No                      | `.afk/tasks.json`  |
| `afk go`            | **Enhanced.** Execute loop with SOP prompts       | No                      | Commits + progress |
| `afk prompt`        | **Enhanced.** Preview the SOP-enriched prompt     | No                      | Stdout             |

The key insight: **`afk plan` is the "thick" path for greenfield work, `afk import` is the "thin" path for quick tasks.** Both feed into the same `afk go` loop, which now has better prompts regardless of how tasks were created.

---

## Proposed Changes

### Phase 1: Smarter Prompts & Planning (High Impact, Low Risk)

These changes improve what AFK _already does_ — generating prompts and running tasks — without adding new architectural complexity.

---

#### 1A. SOP-Driven Prompt Templates

**The Problem**: AFK's current [default.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/default.md) is a generic instruction sheet. It tells the AI _what_ to do but not _how_ to handle edge cases.

**The GSD Learning**: GSD's [gsd-executor.md](file:///Users/rob.macdonald/Documents/code/misc/get-shit-done/agents/gsd-executor.md) is a 400-line Standard Operating Procedure with explicit deviation rules, commit protocols, and checkpoint handling.

**What to Change**:

##### [MODIFY] [default.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/default.md)

Enhance the default prompt template with:

- **Deviation rules** (auto-fix bugs, auto-add missing critical functionality, ask about architectural changes) — directly inspired by GSD's Rules 1-4
- **Commit protocol** (stage files individually, use conventional commit types, never `git add .`)
- **Verification steps** (run quality gates before committing, verify files exist)
- **Self-check** (before marking a task complete, verify the acceptance criteria are met)

Keep this template tool-agnostic — no Claude-specific XML or `Task()` calls.

##### [NEW] `src/prompt/sop_planning.md`

A new template for `afk plan` that instructs the AI to:

- Ask clarifying questions about ambiguous requirements (in interactive mode)
- Break tasks into "single context window" sized units
- Add verification commands to each task
- Consider dependencies between tasks

##### [MODIFY] [template.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/template.rs)

Add tests for correctly rendering the new template.

---

#### 1B. Interactive Planning (`afk plan`)

**The Problem**: `afk import` takes a static file and passes it to the AI for task decomposition. There's no interactive step to clarify requirements.

**The GSD Learning**: `discuss-phase` interviews the user to eliminate ambiguity _before_ planning.

**What to Change**:

##### [NEW] `src/cli/commands/plan.rs`

New `afk plan` command that:

1. Takes a requirements file (or reads from stdin)
2. Spawns the AI CLI with a _planning-specific_ prompt (uses `sop_planning.md`)
3. The prompt instructs the AI to output a structured task list
4. AFK parses the output and writes to `.afk/tasks.json`
5. Displays the generated tasks for user review before `afk go`

This is essentially `afk import` but with a better prompt and a review step.

##### [MODIFY] [mod.rs (cli)](file:///Users/rob.macdonald/Documents/code/misc/afk/src/cli/mod.rs)

Register the new `plan` subcommand.

---

#### 1C. Per-Task Verification Commands

**The Problem**: AFK's quality gates are global (lint, test, build). There's no per-task verification.

**The GSD Learning**: Every GSD plan includes `<verify>` and `<done>` criteria per task.

**What to Change**:

##### [MODIFY] [UserStory in prd/mod.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prd/mod.rs)

Add optional `verify_command` and `done_criteria` fields to the `UserStory` struct.

##### [MODIFY] [default.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/default.md)

Include verification and done criteria in the prompt if present on the task.

##### [MODIFY] [NextStoryContext in prompt/mod.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/mod.rs)

Extended context for prompt rendering.

---

## Verification Plan

### Automated Tests

All changes are in Rust. AFK has comprehensive inline tests.

```bash
# Run all tests (single-threaded as required by the project)
cargo test -- --test-threads=1
```

For each feature:

- **SOP templates**: Test rendering in `template.rs`.
- **`afk plan`**: Test CLI parsing and prompt generation in `plan.rs`.
- **Per-task verification**: Test deserialization and prompt injection in `prd/mod.rs` and `prompt/mod.rs`.
