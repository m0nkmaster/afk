# Decompose Task into Parallel User Stories

Break the following task description into small, independent user stories suitable for parallel execution by multiple AI agents.

---

## Task Description

{{ task_description }}

---

## Output Format

Write the output to `{{ output_path }}`:

```json
{
  "project": "[Project Name from context]",
  "branchName": "afk/[feature-name-kebab-case]",
  "description": "[Summary of what we're building]",
  "userStories": [
    {
      "id": "[kebab-case-id]",
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

## Critical Rules

### 1. Independence (MOST IMPORTANT)

**Stories will be worked on IN PARALLEL by separate agents who cannot see each other's work.**

Each story must be completable WITHOUT any changes from the other stories existing yet. If story B needs code from story A, they cannot run in parallel.

Strategies for independence:
- Each story works on **different files** where possible
- If stories must touch the same file, one creates it and the others are ordered after it (higher priority number)
- Shared interfaces/types should be their own story at priority 1

### 2. Small Size

Each story must be completable in one context window (one AI iteration).

**Rule of thumb:** If you cannot describe the change in 2-3 sentences, it is too big.

Right-sized:
- Add a single component/module
- Add a config option with validation
- Write tests for a specific feature
- Add error handling to an existing component

Too big (split these):
- "Build the entire feature"
- "Add authentication"
- "Refactor the codebase"

### 3. Dependency Ordering via Priority

Priority 1 runs first. Higher numbers run after lower numbers complete. Use this to express dependencies:

- Priority 1: Shared types, interfaces, data structures (foundations)
- Priority 2: Core logic that uses the foundations (can run in parallel with each other)
- Priority 3: Integration, UI, CLI layers
- Priority 4: Tests, polish, documentation

Stories at the **same priority level** will run in parallel. Only put stories at the same level if they are truly independent.

### 4. Verifiable Acceptance Criteria

Each criterion must be something the agent can CHECK:

Good: "Function returns error when input is empty"
Bad: "Handles edge cases properly"

Always include a build check as the final criterion.

---

## Example

**Input:** "Build a settings page with dark mode toggle and notification preferences"

**Output:**
```json
{
  "project": "MyApp",
  "branchName": "afk/settings-page",
  "description": "Settings page with dark mode and notification preferences",
  "userStories": [
    {
      "id": "settings-types",
      "title": "Add settings data types and storage",
      "description": "As a developer, I need settings types and persistence so other components can use them.",
      "acceptanceCriteria": [
        "Create Settings interface with darkMode and notifications fields",
        "Add loadSettings/saveSettings functions using localStorage",
        "Export types from a shared module",
        "Code compiles successfully"
      ],
      "priority": 1,
      "passes": false,
      "notes": ""
    },
    {
      "id": "settings-page",
      "title": "Create settings page layout and routing",
      "description": "As a user, I want to navigate to a settings page.",
      "acceptanceCriteria": [
        "Add /settings route",
        "Create SettingsPage component with section layout",
        "Add navigation link to settings",
        "Code compiles successfully"
      ],
      "priority": 2,
      "passes": false,
      "notes": ""
    },
    {
      "id": "dark-mode",
      "title": "Add dark mode toggle",
      "description": "As a user, I want to toggle dark mode in settings.",
      "acceptanceCriteria": [
        "Add toggle switch component for dark mode",
        "Toggle updates settings in localStorage",
        "Dark mode class applied to document root",
        "Code compiles successfully"
      ],
      "priority": 2,
      "passes": false,
      "notes": ""
    },
    {
      "id": "notifications",
      "title": "Add notification preferences panel",
      "description": "As a user, I want to configure my notification preferences.",
      "acceptanceCriteria": [
        "Add notification preference checkboxes (email, push, in-app)",
        "Preferences saved to localStorage via settings module",
        "Code compiles successfully"
      ],
      "priority": 2,
      "passes": false,
      "notes": ""
    }
  ]
}
```

Note how priority-2 stories are independent and can run in parallel, while priority-1 creates the shared foundation first.

---

## Checklist Before Saving

- [ ] Stories at the same priority level are truly independent (different files, no shared mutations)
- [ ] Each story is completable in one iteration (2-3 sentences to describe)
- [ ] Foundations (types, interfaces) are at priority 1
- [ ] Every story has a build/compile check as final criterion
- [ ] Acceptance criteria are specific and verifiable
- [ ] All stories have `passes: false` and empty `notes`
- [ ] IDs are kebab-case and descriptive
