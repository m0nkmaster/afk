# TODO App - Product Requirements

A simple command-line TODO manager for testing afk.

## Overview

This is a minimal TODO application with intentional gaps that need to be implemented. The AI should be able to complete these tasks and verify them with tests.

---

## User Stories

### 1. Implement TaskList.remove()

**Priority:** High

The `TaskList.remove()` method in `src/todo/models.py` is not implemented. It should:

- Remove a task from the list by its ID
- Return `True` if the task was found and removed
- Return `False` if no task with that ID exists

**Acceptance Criteria:**
- [ ] `test_remove_task` passes
- [ ] `test_remove_task_not_found` passes

---

### 2. Implement TaskList.list_pending()

**Priority:** High

The `TaskList.list_pending()` method in `src/todo/models.py` is not implemented. It should:

- Return a list of all tasks where `completed` is `False`
- Return an empty list if all tasks are complete

**Acceptance Criteria:**
- [ ] `test_list_pending` passes

---

### 3. Implement TaskList.list_by_priority()

**Priority:** Medium

The `TaskList.list_by_priority()` method in `src/todo/models.py` is not implemented. It should:

- Accept a `Priority` enum value
- Return all tasks matching that priority level
- Return an empty list if no tasks match

**Acceptance Criteria:**
- [ ] `test_list_by_priority` passes

---

### 4. Implement save_tasks()

**Priority:** High

The `save_tasks()` function in `src/todo/app.py` is not implemented. It should:

- Serialize the TaskList to JSON
- Write to the `tasks.json` file
- Include all task fields: id, title, description, completed, priority

**Acceptance Criteria:**
- [ ] Tasks persist after being added
- [ ] JSON file is valid and readable by `load_tasks()`

---

### 5. Implement cmd_done()

**Priority:** Medium

The `cmd_done()` function in `src/todo/app.py` is not implemented. It should:

- Find the task by ID
- Mark it as complete
- Save the updated task list
- Print a confirmation message
- Return 0 on success, 1 if task not found

**Acceptance Criteria:**
- [ ] Running `todo done task-1` marks task-1 as complete
- [ ] The change persists to tasks.json

---

## Notes

- All tests are in `tests/test_todo.py`
- Run tests with `pytest` from the sandbox directory
- The app uses a simple JSON file for storage
