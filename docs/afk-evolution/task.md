# AFK Evolution - Phase 1

- [x] Phase 1A: SOP-Driven Prompt Templates
  - [x] Add Deviation Rules to `default.md`
  - [x] Add Commit Protocol to `default.md`
  - [x] Add Self-Check Before Completing to `default.md`
  - [x] Update tests in `template.rs`
  - [x] Verify all tests pass

- [x] Phase 1C: Per-Task Verification
  - [x] Add `verify_command` and `done_criteria` to `UserStory` struct
  - [x] Update `Default` impl and `from_json_value` parser
  - [x] Update `NextStoryContext` to include new fields
  - [x] Update `default.md` template to render verification info
  - [x] Fix all source files using `..Default::default()` spread
  - [x] Verify all 724 tests pass

- [x] Phase 1B: Interactive Planning (`afk plan`)
  - [x] Create `src/prompt/sop_planning.md` template
  - [x] Create `src/cli/commands/plan.rs` command
  - [x] Register `plan` subcommand in CLI
  - [x] Add tests
  - [x] Verify all 733 tests pass
