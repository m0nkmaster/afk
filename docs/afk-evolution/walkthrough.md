# AFK Evolution — Phase 1 Walkthrough

Phase 1 of the AFK evolution is complete, bringing GSD-inspired methodologies (SOPs, planning, and verification) to AFK's tool-agnostic framework.

## Completed

### Phase 1A: SOP-Driven Prompt Templates

Enhanced [default.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/default.md) with standardized protocols:

- **Deviation Rules** — Handling surprises mid-task
- **Commit Protocol** — Atomic, conventional commits
- **Self-Check Before Completing** — Final verification checklist

### Phase 1B: Interactive Planning (`afk plan`)

Implemented a new CLI command for smarter task decomposition.

- **Improved Prompting**: Uses [sop_planning.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/sop_planning.md) to generate better structured tasks.
- **Verification-Ready**: Automatically includes `verifyCommand` and `doneCriteria`.
- **Review Step**: Displays generated tasks for user review before execution.

### Phase 1C: Per-Task Verification

Added optional `verify_command` and `done_criteria` fields so tasks can define their own verification.

#### Files changed

| File                                                                                              | Change                                                    |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| [mod.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prd/mod.rs)                      | Added fields to `UserStory`, `Default`, `from_json_value` |
| [mod.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/mod.rs)                   | Extended `NextStoryContext` with new fields               |
| [default.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/default.md)           | Template renders verify command & done criteria           |
| [plan.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/cli/commands/plan.rs)           | **[NEW]** `afk plan` command implementation               |
| [sop_planning.md](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/sop_planning.md) | **[NEW]** Planning prompt template                        |
| [template.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/prompt/template.rs)         | Updated test struct                                       |
| [main.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/main.rs)                        | Registered `afk plan` dispatch                            |
| [mod.rs](file:///Users/rob.macdonald/Documents/code/misc/afk/src/cli/mod.rs)                      | Registered `PlanCommand` struct and variant               |

#### Usage example (`afk plan`)

```bash
afk plan requirements.md
```

The AI will decompose `requirements.md` into `.afk/tasks.json` with entries like:

```json
{
  "id": "add-auth",
  "title": "Add authentication",
  "verifyCommand": "cargo test --lib auth",
  "doneCriteria": [
    "JWT tokens are validated",
    "Unauthorized requests return 401"
  ]
}
```

## Validation

All **733 tests pass** (`cargo test -- --test-threads=1`).

- Included 8 new tests for planning and verification logic.

## Summary of Accomplishments

1. **Smarter Execution**: `afk go` now follows a robust SOP (Standard Operating Procedure).
2. **Better Decompostion**: `afk plan` produces right-sized, verifiable tasks.
3. **Rigorous Verification**: Per-task verification commands ensure higher quality results.
