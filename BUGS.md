# Bugs

This file previously tracked known issues. All entries have been resolved:

- GitHub source handling (single-source replacement, remote inference, issue
  closing on completion) — fixed; see `src/cli/commands/source.rs` and
  `src/runner/controller.rs::sync_completed_tasks`
- `afk status` task counts / current task display — fixed; session progress is
  merged over PRD state
- `afk init` inside a `.afk` folder — rejected with a helpful error
- Slow iteration overhead — the runner now drains subprocess pipes
  concurrently, enforces stall timeouts, and retries/skips failing tasks
  instead of aborting the session

New issues are tracked with **beads** (`bd`). Run `bd ready` to see open work.
