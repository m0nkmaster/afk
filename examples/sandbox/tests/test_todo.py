"""Tests for the TODO app models and commands."""

from todo.models import Priority, Task, TaskList


class TestTask:
    """Tests for the Task model."""

    def test_create_task(self) -> None:
        """Test creating a basic task."""
        task = Task(id="1", title="Test task")
        assert task.id == "1"
        assert task.title == "Test task"
        assert task.completed is False
        assert task.priority == Priority.MEDIUM

    def test_complete_task(self) -> None:
        """Test marking a task as complete."""
        task = Task(id="1", title="Test task")
        task.complete()
        assert task.completed is True

    def test_task_string_pending(self) -> None:
        """Test string representation of pending task."""
        task = Task(id="1", title="Buy milk")
        assert str(task) == "[○] Buy milk"

    def test_task_string_completed(self) -> None:
        """Test string representation of completed task."""
        task = Task(id="1", title="Buy milk", completed=True)
        assert str(task) == "[✓] Buy milk"


class TestTaskList:
    """Tests for the TaskList model."""

    def test_add_task(self) -> None:
        """Test adding a task to the list."""
        task_list = TaskList()
        task = Task(id="1", title="Test")
        task_list.add(task)
        assert len(task_list.tasks) == 1

    def test_get_task_found(self) -> None:
        """Test getting a task by ID."""
        task_list = TaskList()
        task = Task(id="abc", title="Test")
        task_list.add(task)
        found = task_list.get("abc")
        assert found is not None
        assert found.title == "Test"

    def test_get_task_not_found(self) -> None:
        """Test getting a non-existent task."""
        task_list = TaskList()
        found = task_list.get("missing")
        assert found is None

    def test_remove_task(self) -> None:
        """Test removing a task from the list."""
        task_list = TaskList()
        task = Task(id="1", title="Test")
        task_list.add(task)
        result = task_list.remove("1")
        assert result is True
        assert len(task_list.tasks) == 0

    def test_remove_task_not_found(self) -> None:
        """Test removing a non-existent task."""
        task_list = TaskList()
        result = task_list.remove("missing")
        assert result is False

    def test_list_pending(self) -> None:
        """Test listing pending tasks."""
        task_list = TaskList()
        task_list.add(Task(id="1", title="Pending", completed=False))
        task_list.add(Task(id="2", title="Done", completed=True))
        task_list.add(Task(id="3", title="Also pending", completed=False))

        pending = task_list.list_pending()
        assert len(pending) == 2
        assert all(not t.completed for t in pending)

    def test_list_by_priority(self) -> None:
        """Test filtering tasks by priority."""
        task_list = TaskList()
        task_list.add(Task(id="1", title="Low", priority=Priority.LOW))
        task_list.add(Task(id="2", title="High", priority=Priority.HIGH))
        task_list.add(Task(id="3", title="Also high", priority=Priority.HIGH))

        high_priority = task_list.list_by_priority(Priority.HIGH)
        assert len(high_priority) == 2
        assert all(t.priority == Priority.HIGH for t in high_priority)
