# AI-Assisted Task Planning

You are helping decompose a requirements document into structured, executable tasks.

---

## Input Requirements

{{ prd_content }}

---

## Your Job

1. **Analyse** the requirements carefully
2. **Decompose** into right-sized tasks (each completable in one AI context window)
3. **Order** by dependency (foundations first)
4. **Add verification** to each task where possible
5. **Output** structured JSON to `{{ output_path }}`

---

## Output Format

Write the output to `{{ output_path }}`:

```json
{
  "project": "[Project Name]",
  "branchName": "afk/[feature-name-kebab-case]",
  "description": "[Feature description from requirements]",
  "userStories": [
    {
      "id": "US-001",
      "title": "[Story title]",
      "description": "As a [user], I want [feature] so that [benefit]",
      "acceptanceCriteria": [
        "Criterion 1",
        "Criterion 2",
        "Code compiles/builds successfully"
      ],
      "priority": 1,
      "passes": false,
      "notes": "",
      "verifyCommand": "[optional shell command to verify this task]",
      "doneCriteria": [
        "Observable outcome 1 that proves this task is truly done",
        "Observable outcome 2"
      ]
    }
  ]
}
```

---

## Task Sizing (CRITICAL)

**Each story must be completable in ONE iteration (one context window).**

Each iteration is a fresh agent instance with no memory of previous work. If a story is too big, the LLM runs out of context before finishing and produces broken code.

**Rule of thumb:** If you cannot describe the change in 2-3 sentences, it is too big.

### Right-sized stories:

- Add a new CLI flag with validation
- Implement a single function or module
- Add error handling to an existing component
- Write tests for a specific feature
- Add a configuration option

### Too big (split these):

- "Build the entire feature" → Split into: data structures, core logic, interface, tests
- "Add networking support" → Split into: connection handling, protocol, error handling
- "Refactor the codebase" → Split into one story per module or pattern

---

## Story Ordering: Dependencies First

Stories execute in priority order. Earlier stories must not depend on later ones.

**Correct order:**

1. Data structures and types
2. Core logic that uses those types
3. Interface layer (CLI, API, UI, etc.)
4. Integration and polish

**Wrong order:**

1. Interface (depends on core logic that doesn't exist yet)
2. Core logic

---

## Acceptance Criteria Guidelines

Each criterion must be something the agent can CHECK, not something vague.

### Good criteria (verifiable):

- "Add `--verbose` flag that enables debug output"
- "Function returns error when input is empty"
- "Config file is parsed correctly"
- "Code compiles/builds successfully"
- "Tests pass"

### Bad criteria (vague):

- "Works correctly"
- "User can do X easily"
- "Good performance"
- "Handles edge cases"

### Always include as final criterion:

Choose the appropriate build check for your project:

- **Compiled languages** (Rust, Go, C++): `"Code compiles/builds successfully"`
- **TypeScript**: `"Typecheck passes (tsc --noEmit)"`
- **Python with types**: `"Type check passes (mypy/pyright)"`
- **Other**: Whatever build/check command the project uses

For stories with testable logic, also include:

```
"Tests pass"
```

---

## Per-Task Verification (NEW)

For each task, consider adding:

### `verifyCommand`

A shell command the agent can run to verify this specific task is done. Examples:

- `"cargo test --lib auth"` — run a subset of tests
- `"python -m pytest tests/test_api.py"` — run specific test file
- `"curl -s http://localhost:3000/health | jq .status"` — check an endpoint
- `"grep -r 'TODO' src/new_module/"` — verify no TODOs left

If no specific verification command makes sense, omit the field entirely.

### `doneCriteria`

Observable outcomes that prove the task is truly done (beyond acceptance criteria). These are for the agent to self-check. Examples:

- `"New endpoint returns 200 with valid auth token"`
- `"Error messages include the invalid field name"`
- `"Config file is backwards-compatible with existing format"`

---

## Verification by Output Type

For stories that change user-facing output, include specific verification:

- **Web UI**: `"Navigate to the page and visually verify changes in browser"`
- **CLI output**: `"Run command and verify output format matches expected"`
- **API endpoints**: `"Test endpoint and verify response structure"`
- **File output**: `"Verify generated file contents are correct"`

---

## Field Definitions

| Field                | Description                                                              |
| -------------------- | ------------------------------------------------------------------------ |
| `id`                 | Unique identifier (US-001, US-002, etc.)                                 |
| `title`              | Brief, action-oriented title                                             |
| `description`        | User story format: As a [user], I want [feature] so that [benefit]       |
| `acceptanceCriteria` | Array of verifiable, specific criteria                                   |
| `priority`           | Execution order (1 = first). Based on dependencies, then document order. |
| `passes`             | Always `false` initially                                                 |
| `notes`              | Empty string initially; used for agent notes during execution            |
| `verifyCommand`      | Optional shell command to verify this specific task                      |
| `doneCriteria`       | Optional array of observable outcomes proving the task is done           |

---

## Checklist Before Saving

Before writing the output file, verify:

- [ ] Each story is completable in one iteration (2-3 sentences to describe)
- [ ] Stories are ordered by dependency (foundations first)
- [ ] Every story has a build/compile check as criterion
- [ ] Acceptance criteria are verifiable (not vague)
- [ ] No story depends on a later story
- [ ] IDs are sequential (US-001, US-002, etc.)
- [ ] All stories have `passes: false` and empty `notes`
- [ ] `verifyCommand` added where a specific test or check is possible
- [ ] `doneCriteria` added for tasks with observable outcomes
