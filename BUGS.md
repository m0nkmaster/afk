# Bugs

## GitHub Source Handling

1. Should only allow one GitHub source - adding one should overwrite existing
2. If added without a repo, infer from the current folder's git remote
3. Close GitHub issues when tasks are marked complete (like beads sync does)

## Status Command

1. `afk status` should show current task as well as next task
2. Task counts under "Session" don't match "Tasks" section (Session shows 0 pending when Tasks shows 4 pending)

## Init Command

1. Don't allow `afk init` to run inside a .afk folder - show helpful error message
