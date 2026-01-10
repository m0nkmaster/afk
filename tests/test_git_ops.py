"""Tests for afk.git_ops module."""

from __future__ import annotations

from pathlib import Path
from unittest.mock import MagicMock

from afk.config import AfkConfig, ArchiveConfig, GitConfig
from afk.git_ops import (
    archive_session,
    auto_commit,
    clear_session,
    create_branch,
    get_current_branch,
    get_staged_files,
    get_uncommitted_changes,
    is_git_repo,
    should_archive_on_branch_change,
)


class TestIsGitRepo:
    """Tests for is_git_repo function."""

    def test_is_git_repo_true(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns True when in a git repo."""
        mock_subprocess_run.return_value = MagicMock(returncode=0)

        assert is_git_repo() is True
        mock_subprocess_run.assert_called_once()

    def test_is_git_repo_false(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns False when not in a git repo."""
        mock_subprocess_run.return_value = MagicMock(returncode=128)

        assert is_git_repo() is False


class TestGetCurrentBranch:
    """Tests for get_current_branch function."""

    def test_returns_branch_name(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns branch name."""
        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="feature/my-branch\n")

        assert get_current_branch() == "feature/my-branch"

    def test_returns_none_on_error(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns None when git command fails."""
        mock_subprocess_run.return_value = MagicMock(returncode=1)

        assert get_current_branch() is None


class TestCreateBranch:
    """Tests for create_branch function."""

    def test_does_nothing_when_disabled(self, mock_subprocess_run: MagicMock) -> None:
        """Test no action when auto_branch is disabled."""
        config = AfkConfig(git=GitConfig(auto_branch=False))

        result = create_branch("my-feature", config)

        assert result is False
        mock_subprocess_run.assert_not_called()

    def test_creates_new_branch(self, mock_subprocess_run: MagicMock) -> None:
        """Test creates and checks out new branch."""
        config = AfkConfig(git=GitConfig(auto_branch=True, branch_prefix="afk/"))

        # Branch doesn't exist (rev-parse fails), then checkout -b succeeds
        mock_subprocess_run.side_effect = [
            MagicMock(returncode=128),  # rev-parse (branch doesn't exist)
            MagicMock(returncode=0),  # checkout -b
        ]

        result = create_branch("my-feature", config)

        assert result is True
        assert mock_subprocess_run.call_count == 2

    def test_checks_out_existing_branch(self, mock_subprocess_run: MagicMock) -> None:
        """Test checks out existing branch."""
        config = AfkConfig(git=GitConfig(auto_branch=True, branch_prefix="afk/"))

        mock_subprocess_run.side_effect = [
            MagicMock(returncode=0),  # rev-parse (branch exists)
            MagicMock(returncode=0),  # checkout
        ]

        result = create_branch("my-feature", config)

        assert result is True


class TestAutoCommit:
    """Tests for auto_commit function."""

    def test_does_nothing_when_disabled(self, mock_subprocess_run: MagicMock) -> None:
        """Test no action when auto_commit is disabled."""
        config = AfkConfig(git=GitConfig(auto_commit=False))

        result = auto_commit("task-1", "Done", config)

        assert result is False
        mock_subprocess_run.assert_not_called()

    def test_stages_and_commits(self, mock_subprocess_run: MagicMock) -> None:
        """Test stages all changes and commits."""
        config = AfkConfig(
            git=GitConfig(
                auto_commit=True,
                commit_message_template="afk: {task_id} - {message}",
            )
        )

        mock_subprocess_run.side_effect = [
            MagicMock(returncode=0),  # git add -A
            MagicMock(returncode=0, stdout="M file.py"),  # git status
            MagicMock(returncode=0),  # git commit
        ]

        result = auto_commit("task-1", "Implemented feature", config)

        assert result is True
        assert mock_subprocess_run.call_count == 3

        # Check commit message
        commit_call = mock_subprocess_run.call_args_list[2]
        assert "afk: task-1 - Implemented feature" in commit_call[0][0]

    def test_returns_true_when_nothing_to_commit(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns True when no changes to commit."""
        config = AfkConfig(git=GitConfig(auto_commit=True))

        mock_subprocess_run.side_effect = [
            MagicMock(returncode=0),  # git add -A
            MagicMock(returncode=0, stdout=""),  # git status (empty)
        ]

        result = auto_commit("task-1", "Done", config)

        assert result is True
        assert mock_subprocess_run.call_count == 2

    def test_truncates_long_messages(self, mock_subprocess_run: MagicMock) -> None:
        """Test message is truncated to 50 chars."""
        config = AfkConfig(
            git=GitConfig(
                auto_commit=True,
                commit_message_template="{task_id}: {message}",
            )
        )

        long_message = "A" * 100

        mock_subprocess_run.side_effect = [
            MagicMock(returncode=0),  # git add
            MagicMock(returncode=0, stdout="M file.py"),  # git status
            MagicMock(returncode=0),  # git commit
        ]

        auto_commit("task-1", long_message, config)

        commit_call = mock_subprocess_run.call_args_list[2]
        # Message should be truncated
        assert len(commit_call[0][0][3]) <= 60  # task-1: + 50 chars


class TestGetStagedFiles:
    """Tests for get_staged_files function."""

    def test_returns_file_list(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns list of staged files."""
        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="file1.py\nfile2.py\n")

        result = get_staged_files()

        assert result == ["file1.py", "file2.py"]

    def test_returns_empty_on_error(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns empty list on error."""
        mock_subprocess_run.return_value = MagicMock(returncode=1)

        result = get_staged_files()

        assert result == []


class TestGetUncommittedChanges:
    """Tests for get_uncommitted_changes function."""

    def test_returns_true_with_changes(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns True when there are changes."""
        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="M file.py\n")

        assert get_uncommitted_changes() is True

    def test_returns_false_when_clean(self, mock_subprocess_run: MagicMock) -> None:
        """Test returns False when working directory is clean."""
        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="")

        assert get_uncommitted_changes() is False


class TestArchiveSession:
    """Tests for archive_session function."""

    def test_does_nothing_when_disabled(self, temp_project: Path) -> None:
        """Test no action when archiving is disabled."""
        config = AfkConfig(archive=ArchiveConfig(enabled=False))

        result = archive_session(config)

        assert result is None

    def test_creates_archive_directory(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test creates timestamped archive directory."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True, directory=".afk/archive"))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5}')

        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="main\n")

        result = archive_session(config, reason="test")

        assert result is not None
        assert result.exists()
        assert "test" in result.name
        assert (result / "progress.json").exists()
        assert (result / "metadata.json").exists()

    def test_copies_progress_and_prompt(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test copies both progress and prompt files."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True, directory=".afk/archive"))

        # Create files
        (temp_afk_dir / "progress.json").write_text('{"iterations": 5}')
        (temp_afk_dir / "prompt.md").write_text("# Prompt")

        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="main\n")

        result = archive_session(config, reason="complete")

        assert (result / "progress.json").exists()
        assert (result / "prompt.md").exists()


class TestClearSession:
    """Tests for clear_session function."""

    def test_removes_progress_file(self, temp_afk_dir: Path) -> None:
        """Test removes progress.json."""
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5}')

        clear_session()

        assert not progress_file.exists()

    def test_handles_missing_file(self, temp_afk_dir: Path) -> None:
        """Test handles missing progress file gracefully."""
        clear_session()  # Should not raise


class TestShouldArchiveOnBranchChange:
    """Tests for should_archive_on_branch_change function."""

    def test_returns_false_when_disabled(
        self, temp_project: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test returns False when on_branch_change is disabled."""
        config = AfkConfig(archive=ArchiveConfig(on_branch_change=False))

        result = should_archive_on_branch_change("new-branch", config)

        assert result is False

    def test_returns_false_when_no_progress(
        self, temp_project: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test returns False when no progress file exists."""
        config = AfkConfig(archive=ArchiveConfig(on_branch_change=True))

        result = should_archive_on_branch_change("new-branch", config)

        assert result is False

    def test_returns_true_on_branch_change(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test returns True when switching branches with progress."""
        config = AfkConfig(archive=ArchiveConfig(on_branch_change=True))

        # Create progress file
        (temp_afk_dir / "progress.json").write_text('{"iterations": 5}')

        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="current-branch\n")

        result = should_archive_on_branch_change("new-branch", config)

        assert result is True

    def test_returns_false_on_same_branch(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test returns False when staying on same branch."""
        config = AfkConfig(archive=ArchiveConfig(on_branch_change=True))

        (temp_afk_dir / "progress.json").write_text('{"iterations": 5}')

        mock_subprocess_run.return_value = MagicMock(returncode=0, stdout="same-branch\n")

        result = should_archive_on_branch_change("same-branch", config)

        assert result is False
