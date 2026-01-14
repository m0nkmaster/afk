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
- `--continue` → resumes previous session
- `--init` → deletes existing config and re-runs setup (start fresh)

### 2. Remove Top-Level `sync` ✅

Use `afk prd sync` instead. No top-level alias needed.

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

### 5. Rename `prd parse` to `prd import` ✅

Clearer name. No backwards compatibility needed.

### 6. Keep Archive Subcommands As-Is ✅

`archive create/list/clear` — explicit and discoverable.

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
| `prd` | `import`, `sync`, `show` | Manage PRD/tasks |
| `archive` | `create`, `list`, `clear` | Session history |

### Removed Commands

| Command | Replacement |
|---------|-------------|
| `run` | `go` |
| `start` | `go` (auto-inits if needed) |
| `resume` | `go --continue` |
| `sync` | `prd sync` |
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

#### `--continue` Flag

Preserves `progress.json` and continues from where you left off:
- Keeps iteration count
- Keeps task status
- Keeps learnings

```bash
afk go --continue      # Continue with 10 more iterations
afk go --continue 20   # Continue with 20 more iterations
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

### `afk prd import`

Import a requirements document and convert to structured JSON tasks.

```bash
afk prd import requirements.md           # Parse and create .afk/prd.json
afk prd import PRD.md -o tasks.json      # Custom output path
afk prd import spec.md --copy            # Copy prompt to clipboard (manual mode)
afk prd import spec.md --stdout          # Print prompt to stdout (manual mode)
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
| `prd sync/show` | Sync and display PRD |
| `archive create/list/clear` | Manage archives |

---

## Implementation Checklist

### Remove Commands
- [ ] Delete `run` command
- [ ] Delete `start` command
- [ ] Delete `resume` command
- [ ] Delete `sync` command (top-level)
- [ ] Delete `explain` command
- [ ] Delete `next` command

### Modify Commands
- [ ] `go`: Add `--continue` flag (from resume)
- [ ] `go`: Add `--init` flag (delete config + reconfigure)
- [ ] `go`: Ensure first-run prompts for AI CLI confirmation
- [ ] `status`: Add `-v` flag with explain behaviour
- [ ] `status -v`: Include last 5 learnings

### Add Commands
- [ ] `list`: New command with `--limit`, `--pending`, `--complete` flags
- [ ] `task <id>`: New command to show task details
- [ ] `prompt`: Rename from `next`

### Rename Commands
- [ ] `next` → `prompt`
- [ ] `prd parse` → `prd import`

### Update Documentation
- [ ] Update README.md
- [ ] Update docs/user-guide.md
- [ ] Update AGENTS.md command examples
- [ ] Update shell completion generation

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
├── update          # Self-update
├── completions     # Shell completions
├── source
│   ├── add
│   ├── list
│   └── remove
├── prd
│   ├── import      # (was parse)
│   ├── sync
│   └── show
└── archive
    ├── create
    ├── list
    └── clear
```

**Total: 12 primary commands + 9 subcommands = 21 commands**
(Down from 17 primary + 9 subcommands = 26 commands)
