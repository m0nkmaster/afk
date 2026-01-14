# AFK CLI Commands Review

## Executive Summary

AFK currently has **17 top-level commands** (plus subcommands). This review consolidates to **12 primary commands** with clearer purpose and no redundancy.

---

## Decisions Made

### 1. Consolidate Run Commands ✅

**Before**: `go`, `run`, `start`, `resume`
**After**: Just `go`

`go` becomes the single entry point:
- First run with no `.afk/` → auto-detects project, **prompts user to confirm AI CLI selection**, creates config
- Existing config → uses it
- Always resumes previous session automatically
- `--init` → deletes existing config and re-runs setup (start fresh)

### 2. Top-Level `sync` as Alias ✅

`afk sync` is an alias for `afk tasks sync`. Convenient shorthand.

### 3. Merge `explain` into `status` ✅

`status` shows overview, `status -v` adds:
- Feedback loops configuration
- Pending stories list
- **Last 5 learnings** from progress.json

### 4. Keep `done`/`fail`/`reset` Flat ✅

No change to these commands.

**New commands**:
- `afk list [--limit N]` — list tasks with optional limit
- `afk task <id>` — show details of a specific task

### 5. `prd` subcommand → `import` ✅

Clearer name. No backwards compatibility needed.

### 6. Simplify Archive Commands ✅

`archive` (archives and clears) and `archive list` — simple and discoverable.

### 7. Rename `next` to `prompt` ✅

Clearer name. Update description to clarify it shows "the prompt that will be sent to the AI on the next iteration".

### 8. No Bare `afk` Default ✅

`afk` with no command shows help. Prevents accidental runs.

---

## Final Command Structure

### Primary Commands (12)

| Command | Purpose |
|---------|---------|
| `go` | Run the loop (handles all run scenarios) |
| `init` | Initialise/reconfigure project |
| `status` | Show current state |
| `list` | List tasks |
| `task` | Show task details |
| `prompt` | Preview what AI will see next |
| `verify` | Run quality gates |
| `done` | Mark task complete |
| `fail` | Mark task failed |
| `reset` | Reset stuck task |
| `update` | Self-update |
| `completions` | Shell completions |

### Subcommands (3 groups)

| Group | Commands | Purpose |
|-------|----------|---------|
| `source` | `add`, `list`, `remove` | Manage task sources |
| `prd` | `import` | Import requirements |
| `tasks` | `sync`, `show` | Manage tasks |
| `archive` | (default), `list` | Session history |

### Removed Commands

| Command | Replacement |
|---------|-------------|
| `run` | `go` |
| `start` | `go` (auto-inits if needed) |
| `resume` | `go` (always resumes automatically) |
| `sync` | `afk sync` (alias for `tasks sync`) |
| `explain` | `status -v` |
| `next` | `prompt` |

---

## Command Specifications

### `afk go`

The unified entry point for running the loop.

```bash
afk go                       # Run 10 iterations (default)
afk go 20                    # Run 20 iterations
afk go -u, --until-complete  # Run until all tasks complete
afk go -n, --dry-run         # Show what would happen
afk go -t, --timeout <mins>  # Set timeout in minutes
afk go TODO.md               # Use specific source file
afk go TODO.md 5             # Source file + iteration count
afk go --feedback <mode>     # tui | full | minimal | off
afk go --no-mascot           # Disable ASCII mascot
afk go --init                # Delete config and re-run setup
afk go --fresh               # Clear session progress and start fresh
```

#### First-Run Behaviour

When no `.afk/` directory exists:

1. Analyse project (detect type, package manager, etc.)
2. Detect available AI CLIs
3. **Prompt user to select/confirm AI CLI** (required)
4. Infer sources (TODO.md, beads, etc.)
5. Create `.afk/config.json`
6. Start running

```
Analysing project...
  Type: Rust
  Package manager: cargo

Available AI CLIs:
  1. claude
  2. cursor agent

Select AI CLI [1]: _

✓ Configuration saved to .afk/config.json
Starting loop...
```

#### `--init` Flag

Deletes existing `.afk/config.json` and re-runs the setup flow. Useful when:
- Switching AI CLI
- Project type changed
- Config got corrupted

```bash
afk go --init   # Wipe config, reconfigure, then run
```

#### Resume Behaviour

`afk go` always preserves `progress.json` and continues from where you left off:
- Keeps iteration count
- Keeps task status
- Keeps learnings

Use `--fresh` to start with a clean session:

```bash
afk go           # Continue from last session
afk go --fresh   # Clear progress and start fresh
```

---

### `afk status`

Show current state and configuration.

```bash
afk status        # Overview
afk status -v     # Verbose (includes learnings)
```

#### Default Output

```
=== afk status ===

Tasks: 3/10 complete (7 pending)
  Next: auth-001 - Implement login flow

Session: iteration 5, started 2h ago
Sources: beads, TODO.md
AI CLI: claude
```

#### Verbose Output (`-v`)

```
=== afk status ===

Tasks: 3/10 complete (7 pending)
  Next: auth-001 - Implement login flow

Session: iteration 5, started 2h ago
Sources: beads, TODO.md
AI CLI: claude --dangerously-skip-permissions -p

Feedback Loops:
  types: cargo check
  lint: cargo clippy
  test: cargo test

Pending Stories:
  - auth-001 (P1) Implement login flow
  - auth-002 (P1) Add session handling
  - auth-003 (P2) Password reset flow
  - ui-001 (P2) Dashboard layout
  - ui-002 (P3) Settings page

Recent Learnings:
  1. [auth-001] Use bcrypt for password hashing, not SHA256
  2. [auth-001] Session tokens stored in httpOnly cookies
  3. [db-002] Migration files must be idempotent
  4. [ui-001] Use Tailwind's container class for consistent margins
  5. [ui-001] Dark mode toggle in header, not settings
```

---

### `afk list`

List tasks from the current PRD.

```bash
afk list              # List all tasks (default limit)
afk list --limit 10   # Show only 10 tasks
afk list -l 5         # Short form
afk list --pending    # Show only pending tasks
afk list --complete   # Show only completed tasks
```

#### Output

```
ID          STATUS      PRIORITY  TITLE
auth-001    pending     1         Implement login flow
auth-002    pending     1         Add session handling
auth-003    pending     2         Password reset flow
db-001      complete    1         Create users table
db-002      complete    1         Add sessions table

5 tasks (3 pending, 2 complete)
```

---

### `afk task <id>`

Show details of a specific task.

```bash
afk task auth-001
```

#### Output

```
=== auth-001 ===

Title: Implement login flow
Status: in_progress
Priority: 1

Description:
  Implement user authentication with email and password.
  Handle validation, error messages, and successful redirect.

Acceptance Criteria:
  ✗ User can enter email and password
  ✗ Invalid credentials show error message
  ✗ Successful login redirects to dashboard
  ✗ Form validates email format

Learnings:
  - Use bcrypt for password hashing, not SHA256
  - Session tokens stored in httpOnly cookies

Attempts: 2
Last attempt: 10 minutes ago
```

---

### `afk prompt`

Preview the prompt that will be sent to the AI on the next iteration.

```bash
afk prompt              # Print to stdout
afk prompt -c, --copy   # Copy to clipboard
afk prompt -f, --file   # Write to file
afk prompt -b, --bootstrap  # Include afk command instructions
afk prompt -l, --limit 20   # Override max iterations in prompt
```

---

### `afk init`

Initialise or reconfigure the project. Rarely needed directly — `go` handles this automatically.

```bash
afk init              # Interactive setup
afk init -n, --dry-run   # Show what would be configured
afk init -f, --force     # Overwrite existing config
afk init -y, --yes       # Accept defaults without prompting
```

---

### `afk import`

Import a requirements document and convert to structured JSON tasks.

```bash
afk import requirements.md           # Parse and create .afk/tasks.json
afk import PRD.md -o tasks.json      # Custom output path
afk import spec.md --copy            # Copy prompt to clipboard (manual mode)
afk import spec.md --stdout          # Print prompt to stdout (manual mode)
```

By default, runs the AI CLI to perform the conversion. Use `--copy` or `--stdout` to get the prompt for manual use.

---

## Commands Unchanged

These commands remain as currently implemented:

| Command | Purpose |
|---------|---------|
| `verify [-v]` | Run quality gates |
| `done <id> [-m msg]` | Mark task complete |
| `fail <id> [-m msg]` | Mark task failed |
| `reset <id>` | Reset stuck task |
| `update [--beta] [--check]` | Self-update |
| `completions <shell>` | Generate shell completions |
| `source add/list/remove` | Manage sources |
| `sync` | Sync from all sources |
| `tasks sync/show` | Sync and display tasks |
| `archive [list]` | Manage archives |

---

## Implementation Checklist

### Remove Commands
- [x] Delete `run` command
- [x] Delete `start` command
- [x] Delete `resume` command
- [x] Keep `sync` as alias for `tasks sync`
- [x] Delete `explain` command
- [x] Delete `next` command

### Modify Commands
- [x] `go`: Always resumes; use `--fresh` to start clean
- [x] `go`: Add `--init` flag (delete config + reconfigure)
- [x] `go`: Ensure first-run prompts for AI CLI confirmation
- [x] `status`: Add `-v` flag with explain behaviour
- [x] `status -v`: Include last 5 learnings

### Add Commands
- [x] `list`: New command with `--limit`, `--pending`, `--complete` flags
- [x] `task <id>`: New command to show task details
- [x] `prompt`: Renamed from `next`

### Rename Commands
- [x] `next` → `prompt`
- [x] `prd` subcommand → `import` top-level command

### Update Documentation
- [x] Update README.md
- [x] Update docs/user-guide.md
- [x] Update AGENTS.md command examples
- [x] Update shell completion generation

---

## Final Command Tree

```
afk
├── go              # Run the loop (all scenarios)
├── init            # Initialise/reconfigure
├── status          # Show state (-v for verbose + learnings)
├── list            # List tasks (--limit, --pending, --complete)
├── task            # Show task details
├── prompt          # Preview AI prompt
├── verify          # Run quality gates
├── done            # Mark complete
├── fail            # Mark failed
├── reset           # Reset task
├── sync            # Sync from sources (alias for tasks sync)
├── update          # Self-update
├── completions     # Shell completions
├── source
│   ├── add
│   ├── list
│   └── remove
├── prd
│   └── import      # Import requirements doc
├── tasks
│   ├── sync
│   └── show
└── archive
    └── list
```

**Total: 13 primary commands + 6 subcommands = 19 commands**
