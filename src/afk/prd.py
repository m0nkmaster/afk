"""PRD parsing prompt generation for afk."""

from __future__ import annotations

from pathlib import Path

from jinja2 import BaseLoader, Environment

PRD_PARSE_TEMPLATE = """\
# Parse PRD into Structured Feature List

You are an AI assistant tasked with converting a product requirements document
into a structured JSON feature list.

## Input PRD

```
{{ prd_content }}
```

## Output Format

Create a JSON file at `{{ output_path }}` with the following structure:

```json
{
  "tasks": [
    {
      "id": "kebab-case-feature-id",
      "category": "functional|non-functional|technical|ux|security",
      "description": "Clear, actionable description of the feature",
      "priority": 1,
      "steps": [
        "Step 1: Navigate to...",
        "Step 2: Perform action...",
        "Step 3: Verify result..."
      ],
      "passes": false
    }
  ]
}
```

## Field Definitions

- **id**: Unique kebab-case identifier (e.g., `user-auth-login`, `api-rate-limiting`)
- **category**: One of:
  - `functional` - Core user-facing features
  - `non-functional` - Performance, scalability, reliability
  - `technical` - Infrastructure, architecture, tooling
  - `ux` - User experience, design, accessibility
  - `security` - Authentication, authorisation, data protection
- **description**: Single sentence describing what the feature does (not how)
- **priority**: Integer 1-5 (1 = highest priority, implement first)
- **steps**: Array of verification steps to confirm the feature works end-to-end
- **passes**: Always `false` initially (will be marked `true` when verified)

## Guidelines

1. **Be comprehensive**: Extract ALL features, requirements, and acceptance criteria
2. **Be atomic**: Each task should be a single, implementable unit of work
3. **Be testable**: Every task must have clear verification steps
4. **Prioritise wisely**:
   - Priority 1: Core architecture, dependencies, blocking features
   - Priority 2: Essential user-facing functionality
   - Priority 3: Standard features and integrations
   - Priority 4: Nice-to-have features, polish
   - Priority 5: Future considerations, stretch goals
5. **Order by dependency**: If feature B requires feature A, A should have higher priority
6. **Include edge cases**: Error handling, validation, and edge cases as separate tasks

## Output

Write the complete JSON to `{{ output_path }}` and confirm the number of tasks extracted.
"""


def generate_prd_prompt(
    prd_content: str,
    output_path: str = ".afk/prd.json",
) -> str:
    """Generate a prompt for AI to parse a PRD into structured JSON.

    Args:
        prd_content: The raw PRD content (markdown, text, etc.)
        output_path: Target path for the generated JSON file

    Returns:
        The generated prompt string
    """
    env = Environment(loader=BaseLoader())
    template = env.from_string(PRD_PARSE_TEMPLATE)

    return template.render(
        prd_content=prd_content,
        output_path=output_path,
    )


def load_prd_file(path: str | Path) -> str:
    """Load PRD content from a file.

    Args:
        path: Path to the PRD file

    Returns:
        The file content as a string

    Raises:
        FileNotFoundError: If the file doesn't exist
    """
    file_path = Path(path)
    if not file_path.exists():
        raise FileNotFoundError(f"PRD file not found: {path}")

    return file_path.read_text()
