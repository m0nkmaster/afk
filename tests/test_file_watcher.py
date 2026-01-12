"""Tests for file watcher module."""

from __future__ import annotations

import time
from pathlib import Path

from afk.file_watcher import FileChange, FileWatcher, _ChangeHandler


class TestFileChange:
    """Tests for FileChange dataclass."""

    def test_file_change_creation(self) -> None:
        """Test FileChange can be created with required fields."""
        change = FileChange(path="/path/to/file.py", change_type="created")
        assert change.path == "/path/to/file.py"
        assert change.change_type == "created"
        assert change.timestamp is not None

    def test_file_change_types(self) -> None:
        """Test FileChange accepts different change types."""
        for change_type in ["created", "modified", "deleted"]:
            change = FileChange(path="/file.py", change_type=change_type)
            assert change.change_type == change_type


class TestChangeHandler:
    """Tests for internal _ChangeHandler."""

    def test_handler_creation_default_patterns(self) -> None:
        """Test handler can be created without ignore patterns."""
        handler = _ChangeHandler()
        assert handler._ignore_patterns == []

    def test_handler_creation_with_patterns(self) -> None:
        """Test handler accepts ignore patterns."""
        patterns = [".git", "*.pyc"]
        handler = _ChangeHandler(patterns)
        assert handler._ignore_patterns == patterns

    def test_should_ignore_matching_pattern(self) -> None:
        """Test paths matching ignore patterns are ignored."""
        handler = _ChangeHandler(["*.pyc", ".git"])
        assert handler._should_ignore("/path/to/file.pyc") is True
        assert handler._should_ignore("/project/.git/config") is True

    def test_should_not_ignore_non_matching(self) -> None:
        """Test paths not matching patterns are not ignored."""
        handler = _ChangeHandler(["*.pyc", ".git"])
        assert handler._should_ignore("/path/to/file.py") is False
        assert handler._should_ignore("/project/src/main.py") is False

    def test_record_change_appends(self) -> None:
        """Test recording changes adds to the list."""
        handler = _ChangeHandler()
        handler._record_change("/file1.py", "created")
        handler._record_change("/file2.py", "modified")
        assert len(handler._changes) == 2

    def test_record_change_ignores_matching_patterns(self) -> None:
        """Test recording changes respects ignore patterns."""
        handler = _ChangeHandler(["*.pyc"])
        handler._record_change("/file.py", "created")
        handler._record_change("/file.pyc", "created")
        assert len(handler._changes) == 1
        assert handler._changes[0].path == "/file.py"

    def test_get_changes_returns_and_clears(self) -> None:
        """Test get_changes returns accumulated changes and clears buffer."""
        handler = _ChangeHandler()
        handler._record_change("/file1.py", "created")
        handler._record_change("/file2.py", "modified")

        changes = handler.get_changes()
        assert len(changes) == 2
        assert handler._changes == []

    def test_get_changes_returns_copy(self) -> None:
        """Test get_changes returns a copy, not the internal list."""
        handler = _ChangeHandler()
        handler._record_change("/file.py", "created")

        changes = handler.get_changes()
        changes.append(FileChange("/other.py", "modified"))

        # Internal list should still be empty after get_changes
        assert len(handler._changes) == 0


class TestFileWatcher:
    """Tests for FileWatcher class."""

    def test_watcher_creation_defaults(self, tmp_path: Path) -> None:
        """Test FileWatcher can be created with defaults."""
        watcher = FileWatcher(tmp_path)
        assert watcher._root == tmp_path
        assert ".git" in watcher._ignore_patterns
        assert "__pycache__" in watcher._ignore_patterns
        assert watcher.is_running is False

    def test_watcher_creation_custom_patterns(self, tmp_path: Path) -> None:
        """Test FileWatcher accepts custom ignore patterns."""
        patterns = ["*.log", "build"]
        watcher = FileWatcher(tmp_path, patterns)
        assert watcher._ignore_patterns == patterns

    def test_watcher_accepts_string_path(self, tmp_path: Path) -> None:
        """Test FileWatcher accepts string path."""
        watcher = FileWatcher(str(tmp_path))
        assert watcher._root == tmp_path

    def test_watcher_start_stop(self, tmp_path: Path) -> None:
        """Test watcher can be started and stopped."""
        watcher = FileWatcher(tmp_path)
        assert watcher.is_running is False

        watcher.start()
        assert watcher.is_running is True

        watcher.stop()
        assert watcher.is_running is False

    def test_watcher_start_idempotent(self, tmp_path: Path) -> None:
        """Test calling start multiple times is safe."""
        watcher = FileWatcher(tmp_path)
        watcher.start()
        watcher.start()  # Should not raise
        assert watcher.is_running is True
        watcher.stop()

    def test_watcher_stop_idempotent(self, tmp_path: Path) -> None:
        """Test calling stop multiple times is safe."""
        watcher = FileWatcher(tmp_path)
        watcher.stop()  # Should not raise when not started
        watcher.start()
        watcher.stop()
        watcher.stop()  # Should not raise when already stopped
        assert watcher.is_running is False

    def test_watcher_get_changes_empty(self, tmp_path: Path) -> None:
        """Test get_changes returns empty list when no changes."""
        watcher = FileWatcher(tmp_path)
        watcher.start()
        changes = watcher.get_changes()
        assert changes == []
        watcher.stop()

    def test_watcher_detects_file_creation(self, tmp_path: Path) -> None:
        """Test watcher detects when files are created."""
        watcher = FileWatcher(tmp_path)
        watcher.start()

        # Give the observer time to start
        time.sleep(0.1)

        # Create a file
        test_file = tmp_path / "test_file.py"
        test_file.write_text("# test")

        # Give watchdog time to detect the change
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        # Should have at least a created event (may also have modified)
        assert len(changes) >= 1
        created_paths = [c.path for c in changes if c.change_type == "created"]
        assert str(test_file) in created_paths

    def test_watcher_detects_file_modification(self, tmp_path: Path) -> None:
        """Test watcher detects when files are modified."""
        # Create file before starting watcher
        test_file = tmp_path / "existing.py"
        test_file.write_text("# original")

        watcher = FileWatcher(tmp_path)
        watcher.start()
        time.sleep(0.1)

        # Modify the file
        test_file.write_text("# modified")
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        assert len(changes) >= 1
        modified_paths = [c.path for c in changes if c.change_type == "modified"]
        assert str(test_file) in modified_paths

    def test_watcher_detects_file_deletion(self, tmp_path: Path) -> None:
        """Test watcher detects when files are deleted."""
        # Create file before starting watcher
        test_file = tmp_path / "to_delete.py"
        test_file.write_text("# will be deleted")

        watcher = FileWatcher(tmp_path)
        watcher.start()
        time.sleep(0.1)

        # Delete the file
        test_file.unlink()
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        assert len(changes) >= 1
        deleted_paths = [c.path for c in changes if c.change_type == "deleted"]
        assert str(test_file) in deleted_paths

    def test_watcher_ignores_git_directory(self, tmp_path: Path) -> None:
        """Test watcher ignores changes in .git directory."""
        git_dir = tmp_path / ".git"
        git_dir.mkdir()

        watcher = FileWatcher(tmp_path)
        watcher.start()
        time.sleep(0.1)

        # Create a file in .git
        (git_dir / "config").write_text("# git config")
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        # Should have no changes (git dir is ignored)
        git_changes = [c for c in changes if ".git" in c.path]
        assert len(git_changes) == 0

    def test_watcher_ignores_pycache(self, tmp_path: Path) -> None:
        """Test watcher ignores __pycache__ directory."""
        cache_dir = tmp_path / "__pycache__"
        cache_dir.mkdir()

        watcher = FileWatcher(tmp_path)
        watcher.start()
        time.sleep(0.1)

        # Create a .pyc file
        (cache_dir / "module.cpython-311.pyc").write_bytes(b"bytecode")
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        # Should have no changes (pycache is ignored)
        cache_changes = [c for c in changes if "__pycache__" in c.path]
        assert len(cache_changes) == 0

    def test_watcher_custom_ignore_patterns(self, tmp_path: Path) -> None:
        """Test watcher respects custom ignore patterns."""
        watcher = FileWatcher(tmp_path, ["*.log"])
        watcher.start()
        time.sleep(0.1)

        # Create a log file (should be ignored)
        (tmp_path / "app.log").write_text("log entry")
        # Create a py file (should be detected)
        (tmp_path / "app.py").write_text("# code")
        time.sleep(0.5)

        changes = watcher.get_changes()
        watcher.stop()

        log_changes = [c for c in changes if c.path.endswith(".log")]
        py_changes = [c for c in changes if c.path.endswith(".py")]

        assert len(log_changes) == 0
        assert len(py_changes) >= 1

    def test_watcher_restart_after_stop(self, tmp_path: Path) -> None:
        """Test watcher can be stopped and restarted (new observer created)."""
        watcher = FileWatcher(tmp_path)

        # First start/stop cycle
        watcher.start()
        assert watcher.is_running is True
        time.sleep(0.1)
        (tmp_path / "file1.py").write_text("# first")
        time.sleep(0.5)
        changes1 = watcher.get_changes()
        watcher.stop()
        assert watcher.is_running is False

        # Second start/stop cycle - this should work without RuntimeError
        watcher.start()
        assert watcher.is_running is True
        time.sleep(0.1)
        (tmp_path / "file2.py").write_text("# second")
        time.sleep(0.5)
        changes2 = watcher.get_changes()
        watcher.stop()
        assert watcher.is_running is False

        # Both cycles should have detected changes
        assert len(changes1) >= 1
        assert len(changes2) >= 1
        assert any("file1.py" in c.path for c in changes1)
        assert any("file2.py" in c.path for c in changes2)
