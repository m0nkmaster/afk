"""CLI interface for the TODO app."""

import argparse
import json
import sys
from pathlib import Path

from todo.models import Priority, Task, TaskList

# Default storage file
TASKS_FILE = Path("tasks.json")


def load_tasks() -> TaskList:
    """Load tasks from the JSON file."""
    task_list = TaskList()
    if TASKS_FILE.exists():
        data = json.loads(TASKS_FILE.read_text())
        for item in data.get("tasks", []):
            task = Task(
                id=item["id"],
                title=item["title"],
                description=item.get("description", ""),
                completed=item.get("completed", False),
                priority=Priority(item.get("priority", 2)),
            )
            task_list.add(task)
    return task_list


def save_tasks(task_list: TaskList) -> None:
    """Save tasks to the JSON file."""
    # TODO: Implement this function
    pass


def cmd_add(args: argparse.Namespace) -> int:
    """Add a new task."""
    task_list = load_tasks()

    # Generate a simple ID
    task_id = f"task-{len(task_list.tasks) + 1}"

    task = Task(
        id=task_id,
        title=args.title,
        description=args.description or "",
        priority=Priority[args.priority.upper()],
    )
    task_list.add(task)
    save_tasks(task_list)

    print(f"Added: {task}")
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    """List all tasks."""
    task_list = load_tasks()

    if not task_list.tasks:
        print("No tasks found.")
        return 0

    for task in task_list.tasks:
        print(task)

    return 0


def cmd_done(args: argparse.Namespace) -> int:
    """Mark a task as complete."""
    # TODO: Implement this command
    pass


def cmd_remove(args: argparse.Namespace) -> int:
    """Remove a task."""
    # TODO: Implement this command
    pass


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(description="Simple TODO manager")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Add command
    add_parser = subparsers.add_parser("add", help="Add a new task")
    add_parser.add_argument("title", help="Task title")
    add_parser.add_argument("-d", "--description", help="Task description")
    add_parser.add_argument(
        "-p", "--priority", choices=["low", "medium", "high"], default="medium"
    )
    add_parser.set_defaults(func=cmd_add)

    # List command
    list_parser = subparsers.add_parser("list", help="List all tasks")
    list_parser.set_defaults(func=cmd_list)

    # Done command
    done_parser = subparsers.add_parser("done", help="Mark a task as complete")
    done_parser.add_argument("task_id", help="Task ID to complete")
    done_parser.set_defaults(func=cmd_done)

    # Remove command
    remove_parser = subparsers.add_parser("remove", help="Remove a task")
    remove_parser.add_argument("task_id", help="Task ID to remove")
    remove_parser.set_defaults(func=cmd_remove)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
