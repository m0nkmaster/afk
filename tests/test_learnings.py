"""Tests for learnings module."""

from pathlib import Path

from afk.learnings import (
    append_learning,
    clear_learnings,
    get_recent_learnings,
    load_learnings,
)


class TestLoadLearnings:
    """Tests for load_learnings function."""

    def test_returns_empty_string_when_file_missing(self, tmp_path: Path) -> None:
        """Should return empty string when learnings file doesn't exist."""
        result = load_learnings(tmp_path / "missing.txt")
        assert result == ""

    def test_returns_file_content(self, tmp_path: Path) -> None:
        """Should return file content when file exists."""
        learnings_file = tmp_path / "learnings.txt"
        learnings_file.write_text("## 2024-01-01\n\nSome learning")

        result = load_learnings(learnings_file)

        assert "Some learning" in result


class TestAppendLearning:
    """Tests for append_learning function."""

    def test_creates_file_if_missing(self, tmp_path: Path) -> None:
        """Should create file if it doesn't exist."""
        learnings_file = tmp_path / ".afk" / "learnings.txt"

        append_learning("New discovery", path=learnings_file)

        assert learnings_file.exists()
        assert "New discovery" in learnings_file.read_text()

    def test_appends_to_existing_file(self, tmp_path: Path) -> None:
        """Should append to existing content."""
        learnings_file = tmp_path / "learnings.txt"
        learnings_file.write_text("## Existing\n\nOld content")

        append_learning("New content", path=learnings_file)

        content = learnings_file.read_text()
        assert "Old content" in content
        assert "New content" in content

    def test_includes_timestamp(self, tmp_path: Path) -> None:
        """Should include timestamp in entry."""
        learnings_file = tmp_path / "learnings.txt"

        append_learning("Discovery", path=learnings_file)

        content = learnings_file.read_text()
        assert "## " in content  # Timestamp header

    def test_includes_task_id_when_provided(self, tmp_path: Path) -> None:
        """Should include task ID when provided."""
        learnings_file = tmp_path / "learnings.txt"

        append_learning("Discovery", task_id="auth-login", path=learnings_file)

        content = learnings_file.read_text()
        assert "[auth-login]" in content


class TestClearLearnings:
    """Tests for clear_learnings function."""

    def test_removes_file(self, tmp_path: Path) -> None:
        """Should remove the learnings file."""
        learnings_file = tmp_path / "learnings.txt"
        learnings_file.write_text("content")

        clear_learnings(learnings_file)

        assert not learnings_file.exists()

    def test_handles_missing_file(self, tmp_path: Path) -> None:
        """Should not error when file doesn't exist."""
        clear_learnings(tmp_path / "missing.txt")  # Should not raise


class TestGetRecentLearnings:
    """Tests for get_recent_learnings function."""

    def test_returns_full_content_when_short(self, tmp_path: Path) -> None:
        """Should return full content when under max_chars."""
        learnings_file = tmp_path / "learnings.txt"
        learnings_file.write_text("Short content")

        result = get_recent_learnings(max_chars=1000, path=learnings_file)

        assert result == "Short content"

    def test_truncates_from_start(self, tmp_path: Path) -> None:
        """Should truncate old content and keep recent."""
        learnings_file = tmp_path / "learnings.txt"
        content = "## Old entry\n\nOld content\n\n## Recent entry\n\nRecent content"
        learnings_file.write_text(content)

        result = get_recent_learnings(max_chars=50, path=learnings_file)

        assert "Recent content" in result
        assert "[...truncated...]" in result

    def test_finds_complete_entry_start(self, tmp_path: Path) -> None:
        """Should start at a complete entry (## header)."""
        learnings_file = tmp_path / "learnings.txt"
        content = "## Entry 1\n\nContent 1\n\n## Entry 2\n\nContent 2"
        learnings_file.write_text(content)

        result = get_recent_learnings(max_chars=30, path=learnings_file)

        # Should start at an entry boundary
        assert result.strip().startswith("## ") or result.startswith("[...truncated...]")
