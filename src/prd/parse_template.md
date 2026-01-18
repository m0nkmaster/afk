# Parse PRD into Structured User Stories

Convert the PRD below into a structured JSON task list for autonomous execution.

---

## Input PRD

{{ prd_content }}

---

## Output Format

Write the output to `{{ output_path }}`:

```json
{
  "project": "[Project Name]",
  "branchName": "afk/[feature-name-kebab-case]",
  "description": "[Feature description from PRD title/intro]",
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
      "notes": ""
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
```
"Code compiles/builds successfully"
```

For stories with testable logic, also include:
```
"Tests pass"
```

For stories that change user-facing output (UI, CLI, etc.), verify the output works as expected.

---

## Field Definitions

| Field | Description |
|-------|-------------|
| `id` | Unique identifier (US-001, US-002, etc.) |
| `title` | Brief, action-oriented title |
| `description` | User story format: As a [user], I want [feature] so that [benefit] |
| `acceptanceCriteria` | Array of verifiable, specific criteria |
| `priority` | Execution order (1 = first). Based on dependencies, then document order. |
| `passes` | Always `false` initially |
| `notes` | Empty string initially; used for agent notes during execution |

---

## Example

**Input PRD:**
```markdown
# CLI Progress Indicator

Add a progress bar to the file processor CLI.

## Requirements
- Show progress during long operations
- Support both spinner and percentage modes
- Allow disabling via --quiet flag
```

**Output:**
```json
{
  "project": "FileProcessor",
  "branchName": "afk/progress-indicator",
  "description": "CLI Progress Indicator - Visual feedback during file processing",
  "userStories": [
    {
      "id": "US-001",
      "title": "Add progress bar module",
      "description": "As a developer, I need a reusable progress bar component.",
      "acceptanceCriteria": [
        "Create progress module with ProgressBar struct",
        "Support setting total count and incrementing",
        "Render percentage and bar to stderr",
        "Code compiles/builds successfully"
      ],
      "priority": 1,
      "passes": false,
      "notes": ""
    },
    {
      "id": "US-002",
      "title": "Add spinner mode",
      "description": "As a user, I want a spinner when total count is unknown.",
      "acceptanceCriteria": [
        "Add Spinner variant to progress module",
        "Animate spinner characters on update",
        "Code compiles/builds successfully"
      ],
      "priority": 2,
      "passes": false,
      "notes": ""
    },
    {
      "id": "US-003",
      "title": "Integrate progress into file processor",
      "description": "As a user, I want to see progress during processing.",
      "acceptanceCriteria": [
        "Show progress bar during batch processing",
        "Show spinner during single-file processing",
        "Code compiles/builds successfully"
      ],
      "priority": 3,
      "passes": false,
      "notes": ""
    },
    {
      "id": "US-004",
      "title": "Add --quiet flag",
      "description": "As a user, I want to disable progress output for scripts.",
      "acceptanceCriteria": [
        "Add --quiet / -q flag to CLI",
        "Flag suppresses all progress output",
        "Other output (results, errors) still shown",
        "Code compiles/builds successfully"
      ],
      "priority": 4,
      "passes": false,
      "notes": ""
    }
  ]
}
```

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
