# Roadmap

Planned features and improvements for afk. Feedback and suggestions welcome.

## Planned

### Improved Status Page

Enhance `afk status` with richer information display including a full task list, progress visualisation, and session statistics. Currently status shows basic session info; the goal is to make it a comprehensive dashboard for understanding project state at a glance.

### Ollama Support

Add Ollama as a supported AI CLI backend, enabling fully local AI coding loops without requiring cloud API keys. This would allow developers to run autonomous coding sessions using local models on their own hardware.

### AI Token Usage Metrics

Track and report token consumption across iterations and sessions. Display cumulative usage, per-task costs, and provide insights into which tasks consume the most tokens. Useful for cost management and understanding AI efficiency.

### Estimated Completion

Provide time and iteration estimates for completing remaining tasks based on historical performance data. Use metrics from past sessions to predict when the current task list might be finished, helping with planning and expectations.

### Feedback Mode Polish

Clean up the non-TUI feedback modes (`--feedback full`, `minimal`, `off`). The TUI dashboard has received the most attention; the alternative modes need refinement for consistent styling, better progress indication, and a more polished user experience when running without the full terminal UI.

### Prevent Sleep

Ability to stop the computer powering down if left for long periods of time. Utilise `caffeinate` on macOS and similar on other platforms.

### OpenSpec Source Adapter

Add [OpenSpec](https://github.com/Fission-AI/OpenSpec) as a task source, enabling afk to consume tasks from spec-driven development workflows. OpenSpec provides structured change proposals with formal requirements and scenarios—combining this with afk's autonomous execution creates a powerful workflow: use OpenSpec for planning and specification, then afk for implementation.

The adapter would:
- Read tasks from `openspec/changes/<change-id>/tasks.md`
- Inject spec context from `openspec/changes/<change-id>/specs/` into prompts
- Optionally include the proposal and design docs for richer context
- Mark tasks complete in the OpenSpec format (`- [x]`)