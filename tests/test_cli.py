"""Tests for afk.cli module."""

from __future__ import annotations

import json
from pathlib import Path
from unittest.mock import patch

import pytest
from click.testing import CliRunner

from afk.cli import main


@pytest.fixture
def cli_runner() -> CliRunner:
    """Create a CLI runner."""
    return CliRunner()


@pytest.fixture
def initialized_project(temp_project: Path) -> Path:
    """Create an initialized project with .afk directory."""
    afk_dir = temp_project / ".afk"
    afk_dir.mkdir()

    config = {
        "sources": [{"type": "json", "path": "tasks.json"}],
        "limits": {"max_iterations": 10},
        "output": {"default": "stdout"},
    }
    (afk_dir / "config.json").write_text(json.dumps(config))

    # Create tasks file
    tasks = [{"id": "task-1", "description": "Test task"}]
    (temp_project / "tasks.json").write_text(json.dumps(tasks))

    # Create prd.json (Ralph pattern - AI reads this directly)
    prd = {
        "project": "test",
        "branchName": "main",
        "description": "Test project",
        "userStories": [
            {
                "id": "task-1",
                "title": "Test task",
                "description": "Test task description",
                "acceptanceCriteria": ["Complete the task"],
                "priority": 1,
                "passes": False,
                "source": "json:tasks.json",
                "notes": "",
            }
        ],
        "lastSynced": "2025-01-10T10:00:00",
    }
    (afk_dir / "prd.json").write_text(json.dumps(prd))

    return temp_project


class TestMainGroup:
    """Tests for main CLI group."""

    def test_version(self, cli_runner: CliRunner) -> None:
        """Test --version option."""
        result = cli_runner.invoke(main, ["--version"])
        assert result.exit_code == 0
        assert "afk" in result.output
        assert "0.1.0" in result.output

    def test_help(self, cli_runner: CliRunner) -> None:
        """Test --help option."""
        result = cli_runner.invoke(main, ["--help"])
        assert result.exit_code == 0
        assert "Autonomous AI coding loops" in result.output


class TestInitCommand:
    """Tests for init command."""

    def test_init_dry_run(self, cli_runner: CliRunner, python_project: Path) -> None:
        """Test init with --dry-run."""
        result = cli_runner.invoke(main, ["init", "--dry-run"])
        assert result.exit_code == 0
        assert "Dry run" in result.output
        assert not (python_project / ".afk").exists()

    def test_init_creates_config(self, cli_runner: CliRunner, python_project: Path) -> None:
        """Test init creates configuration."""
        result = cli_runner.invoke(main, ["init", "--yes"])
        assert result.exit_code == 0
        assert (python_project / ".afk" / "config.json").exists()

    def test_init_existing_config(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test init with existing config shows warning."""
        result = cli_runner.invoke(main, ["init"])
        assert result.exit_code == 0
        assert "already initialized" in result.output

    def test_init_force(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test init --force overwrites existing config."""
        result = cli_runner.invoke(main, ["init", "--force", "--yes"])
        assert result.exit_code == 0
        assert "Configuration saved" in result.output

    def test_init_shows_analysis(self, cli_runner: CliRunner, python_project: Path) -> None:
        """Test init shows project analysis."""
        result = cli_runner.invoke(main, ["init", "--dry-run"])
        assert "Stack" in result.output
        assert "Python" in result.output

    def test_init_cancelled(self, cli_runner: CliRunner, python_project: Path) -> None:
        """Test init can be cancelled."""
        result = cli_runner.invoke(main, ["init"], input="n\n")
        assert result.exit_code == 0
        assert "Cancelled" in result.output
        assert not (python_project / ".afk").exists()


class TestStatusCommand:
    """Tests for status command."""

    def test_status_not_initialized(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test status when not initialized."""
        result = cli_runner.invoke(main, ["status"])
        assert result.exit_code == 0
        assert "not initialized" in result.output

    def test_status_shows_sources(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test status shows configured sources."""
        result = cli_runner.invoke(main, ["status"])
        assert result.exit_code == 0
        assert "Task Sources" in result.output
        assert "json" in result.output

    def test_status_shows_limits(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test status shows limits."""
        result = cli_runner.invoke(main, ["status"])
        assert result.exit_code == 0
        assert "Limits" in result.output
        assert "10" in result.output  # max_iterations


class TestSourceCommands:
    """Tests for source subcommands."""

    def test_source_list_empty(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test source list with no sources."""
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text('{"sources": []}')

        result = cli_runner.invoke(main, ["source", "list"])
        assert result.exit_code == 0
        assert "No sources configured" in result.output

    def test_source_list(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test source list shows sources."""
        result = cli_runner.invoke(main, ["source", "list"])
        assert result.exit_code == 0
        assert "json" in result.output

    def test_source_add_beads(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test adding beads source."""
        result = cli_runner.invoke(main, ["source", "add", "beads"])
        assert result.exit_code == 0
        assert "Added source" in result.output

        # Verify it was saved
        config_path = initialized_project / ".afk" / "config.json"
        config = json.loads(config_path.read_text())
        assert any(s["type"] == "beads" for s in config["sources"])

    def test_source_add_with_path(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test adding source with path."""
        (initialized_project / "TODO.md").write_text("- [ ] Task\n")
        result = cli_runner.invoke(main, ["source", "add", "markdown", "TODO.md"])
        assert result.exit_code == 0
        assert "Added source" in result.output
        assert "TODO.md" in result.output

    def test_source_add_file_not_found(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test adding source with non-existent file."""
        result = cli_runner.invoke(main, ["source", "add", "json", "nonexistent.json"])
        assert result.exit_code == 0
        assert "File not found" in result.output

    def test_source_remove(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test removing a source."""
        result = cli_runner.invoke(main, ["source", "remove", "1"])
        assert result.exit_code == 0
        assert "Removed source" in result.output

        # Verify it was removed
        config_path = initialized_project / ".afk" / "config.json"
        config = json.loads(config_path.read_text())
        assert len(config["sources"]) == 0

    def test_source_remove_invalid_index(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test removing source with invalid index."""
        result = cli_runner.invoke(main, ["source", "remove", "99"])
        assert result.exit_code == 0
        assert "Invalid index" in result.output


class TestNextCommand:
    """Tests for next command."""

    def test_next_not_initialized(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test next when not initialized."""
        result = cli_runner.invoke(main, ["next"])
        assert result.exit_code == 0
        assert "not initialized" in result.output

    def test_next_no_sources(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test next with no sources configured."""
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text('{"sources": []}')

        result = cli_runner.invoke(main, ["next"])
        assert result.exit_code == 0
        assert "No sources configured" in result.output

    def test_next_stdout(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test next with stdout output."""
        result = cli_runner.invoke(main, ["next", "--stdout"])
        assert result.exit_code == 0
        assert "task-1" in result.output
        assert "prd.json" in result.output

    def test_next_file(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test next with file output."""
        result = cli_runner.invoke(main, ["next", "--file"])
        assert result.exit_code == 0
        assert "Prompt written to" in result.output

    def test_next_clipboard(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test next with clipboard output."""
        with patch("pyperclip.copy"):
            result = cli_runner.invoke(main, ["next", "--copy"])
            assert result.exit_code == 0
            # Should show success message or fallback to stdout

    def test_next_bootstrap(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test next with --bootstrap flag."""
        result = cli_runner.invoke(main, ["next", "--stdout", "--bootstrap"])
        assert result.exit_code == 0
        assert "Autonomous Loop" in result.output

    def test_next_limit_override(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test next with --limit override."""
        result = cli_runner.invoke(main, ["next", "--stdout", "--limit", "5"])
        assert result.exit_code == 0
        assert "/5" in result.output


class TestDoneCommand:
    """Tests for done command."""

    def test_done_marks_complete(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test done marks task as complete."""
        result = cli_runner.invoke(main, ["done", "task-1"])
        assert result.exit_code == 0
        assert "Task completed" in result.output

    def test_done_with_message(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test done with custom message."""
        result = cli_runner.invoke(main, ["done", "task-1", "-m", "All done!"])
        assert result.exit_code == 0
        assert "Task completed" in result.output

    def test_done_new_task(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test done for task not in progress file."""
        result = cli_runner.invoke(main, ["done", "new-task"])
        assert result.exit_code == 0
        assert "Task completed" in result.output


class TestFailCommand:
    """Tests for fail command."""

    def test_fail_marks_failed(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test fail marks task as failed."""
        result = cli_runner.invoke(main, ["fail", "task-1"])
        assert result.exit_code == 0
        assert "Task failed" in result.output
        assert "task-1" in result.output

    def test_fail_with_message(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test fail with custom message."""
        result = cli_runner.invoke(main, ["fail", "task-1", "-m", "Build error"])
        assert result.exit_code == 0
        assert "Task failed" in result.output

    def test_fail_shows_failure_count(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test fail shows failure count on repeated failures."""
        cli_runner.invoke(main, ["fail", "task-1"])
        result = cli_runner.invoke(main, ["fail", "task-1"])
        assert result.exit_code == 0
        assert "2" in result.output  # Second failure


class TestRunCommand:
    """Tests for run command."""

    def test_run_starts_loop(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test run starts the loop."""
        # Mock sync_prd to avoid subprocess calls
        from afk.prd_store import PrdDocument, UserStory

        mock_prd = PrdDocument(
            project="test",
            userStories=[UserStory(id="task-1", title="Test", description="Test", passes=False)],
        )
        with patch("afk.runner.sync_prd", return_value=mock_prd):
            result = cli_runner.invoke(main, ["run"])
        assert result.exit_code == 0
        assert "Starting afk loop" in result.output or "pending" in result.output

    def test_run_with_iterations(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test run with custom iteration count."""
        from afk.prd_store import PrdDocument, UserStory

        mock_prd = PrdDocument(
            project="test",
            userStories=[UserStory(id="task-1", title="Test", description="Test", passes=False)],
        )
        with patch("afk.runner.sync_prd", return_value=mock_prd):
            result = cli_runner.invoke(main, ["run", "10"])
        assert result.exit_code == 0

    def test_run_not_initialized(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test run fails when not initialized."""
        result = cli_runner.invoke(main, ["run"])
        assert "not initialized" in result.output

    def test_run_no_sources(self, cli_runner: CliRunner, temp_afk_dir: Path) -> None:
        """Test run fails when no sources configured."""
        # Create empty config
        from afk.config import AfkConfig

        AfkConfig().save()

        result = cli_runner.invoke(main, ["run"])
        assert "No sources configured" in result.output


class TestResumeCommand:
    """Tests for resume command."""

    def test_resume_continues_session(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test resume continues from existing session."""
        from afk.prd_store import PrdDocument, UserStory

        # Create progress file with some iterations
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text(
            '{"iterations": 3, "tasks": {}, "started_at": "2026-01-10T10:00:00"}'
        )

        mock_prd = PrdDocument(
            project="test",
            userStories=[
                UserStory(id="task-1", title="Test", description="Test", passes=False)
            ],
        )
        with patch("afk.runner.sync_prd", return_value=mock_prd):
            result = cli_runner.invoke(main, ["resume"])
        assert result.exit_code == 0
        assert "Resuming session" in result.output or "Session Complete" in result.output

    def test_resume_not_initialized(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test resume fails when not initialized."""
        result = cli_runner.invoke(main, ["resume"])
        assert "not initialized" in result.output

    def test_resume_no_sources(self, cli_runner: CliRunner, temp_afk_dir: Path) -> None:
        """Test resume fails when no sources configured."""
        from afk.config import AfkConfig

        AfkConfig().save()

        result = cli_runner.invoke(main, ["resume"])
        assert "No sources configured" in result.output

    def test_resume_no_session_starts_fresh(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test resume works even without existing session (starts fresh)."""
        result = cli_runner.invoke(main, ["resume"])
        assert result.exit_code == 0
        # Should either warn about no session or just start
        assert "No session to resume" in result.output or "Session Complete" in result.output

    def test_resume_with_iterations(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test resume with custom iteration count."""
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text(
            '{"iterations": 2, "tasks": {}, "started_at": "2026-01-10T10:00:00"}'
        )

        result = cli_runner.invoke(main, ["resume", "20"])
        assert result.exit_code == 0
        assert "Max iterations: 20" in result.output

    def test_run_with_continue_flag(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test run --continue skips archiving."""
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text(
            '{"iterations": 2, "tasks": {}, "started_at": "2026-01-10T10:00:00"}'
        )

        result = cli_runner.invoke(main, ["run", "--continue"])
        assert result.exit_code == 0
        assert "Resuming session" in result.output or "Session Complete" in result.output


class TestPrdCommands:
    """Tests for prd subcommands."""

    def test_prd_help(self, cli_runner: CliRunner) -> None:
        """Test prd --help."""
        result = cli_runner.invoke(main, ["prd", "--help"])
        assert result.exit_code == 0
        assert "product requirements" in result.output.lower()

    def test_prd_parse_help(self, cli_runner: CliRunner) -> None:
        """Test prd parse --help."""
        result = cli_runner.invoke(main, ["prd", "parse", "--help"])
        assert result.exit_code == 0
        assert "INPUT_FILE" in result.output
        assert "--output" in result.output

    def test_prd_parse_stdout(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test prd parse with stdout output."""
        prd_file = initialized_project / "requirements.md"
        prd_file.write_text("# My App\n\nUsers can log in.")

        result = cli_runner.invoke(main, ["prd", "parse", str(prd_file), "--stdout"])
        assert result.exit_code == 0
        assert "Users can log in" in result.output
        assert "tasks" in result.output
        assert "passes" in result.output

    def test_prd_parse_custom_output(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test prd parse with custom output path."""
        prd_file = initialized_project / "prd.md"
        prd_file.write_text("Build a thing.")

        result = cli_runner.invoke(
            main, ["prd", "parse", str(prd_file), "-o", "custom.json", "--stdout"]
        )
        assert result.exit_code == 0
        assert "custom.json" in result.output

    def test_prd_parse_shows_next_step(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test prd parse shows next step instructions."""
        prd_file = initialized_project / "spec.md"
        prd_file.write_text("Requirements here.")

        result = cli_runner.invoke(main, ["prd", "parse", str(prd_file), "--stdout"])
        assert result.exit_code == 0
        assert "afk source add json" in result.output

    def test_prd_parse_file_not_found(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test prd parse with non-existent file."""
        result = cli_runner.invoke(main, ["prd", "parse", "nonexistent.md"])
        # Click's Path(exists=True) will catch this
        assert result.exit_code != 0

    def test_prd_parse_clipboard(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test prd parse with clipboard output."""
        prd_file = initialized_project / "prd.md"
        prd_file.write_text("Feature requirements.")

        with patch("pyperclip.copy"):
            result = cli_runner.invoke(main, ["prd", "parse", str(prd_file), "--copy"])
            assert result.exit_code == 0

    def test_prd_parse_file_output(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test prd parse with file output."""
        prd_file = initialized_project / "prd.md"
        prd_file.write_text("Requirements.")

        result = cli_runner.invoke(main, ["prd", "parse", str(prd_file), "--file"])
        assert result.exit_code == 0
        assert "Prompt written to" in result.output

    def test_prd_parse_preserves_multiline(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test prd parse preserves multiline PRD content."""
        prd_content = """# Application Requirements

## Authentication
- Users can sign up
- Users can log in
- Password reset via email

## Dashboard
- Show user stats
- Recent activity feed
"""
        prd_file = initialized_project / "full-prd.md"
        prd_file.write_text(prd_content)

        result = cli_runner.invoke(main, ["prd", "parse", str(prd_file), "--stdout"])
        assert result.exit_code == 0
        assert "Authentication" in result.output
        assert "Dashboard" in result.output
        assert "Password reset" in result.output


class TestArchiveCommands:
    """Tests for archive subcommands."""

    def test_archive_help(self, cli_runner: CliRunner) -> None:
        """Test archive --help."""
        result = cli_runner.invoke(main, ["archive", "--help"])
        assert result.exit_code == 0
        assert "archive" in result.output.lower()

    def test_archive_create(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test archive create."""
        # Create progress file
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text('{"iterations": 3, "tasks": {}}')

        result = cli_runner.invoke(main, ["archive", "create"])
        assert result.exit_code == 0
        assert "archived" in result.output.lower()

    def test_archive_create_with_reason(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test archive create with custom reason."""
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text('{"iterations": 3, "tasks": {}}')

        result = cli_runner.invoke(main, ["archive", "create", "-r", "testing"])
        assert result.exit_code == 0
        assert "testing" in result.output

    def test_archive_list_empty(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test archive list with no archives."""
        result = cli_runner.invoke(main, ["archive", "list"])
        assert result.exit_code == 0
        assert "No archives found" in result.output

    def test_archive_list_with_archives(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test archive list with existing archives."""
        # Create some archives
        archive_dir = initialized_project / ".afk" / "archive"
        archive_dir.mkdir(parents=True)

        test_archive = archive_dir / "2026-01-10_12-00-00_main_test"
        test_archive.mkdir()
        (test_archive / "metadata.json").write_text(
            '{"archived_at": "2026-01-10T12:00:00", "reason": "test"}'
        )

        result = cli_runner.invoke(main, ["archive", "list"])
        assert result.exit_code == 0
        assert "2026-01-10" in result.output
        assert "test" in result.output

    def test_archive_clear(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test archive clear."""
        progress_file = initialized_project / ".afk" / "progress.json"
        progress_file.write_text('{"iterations": 3, "tasks": {}}')

        result = cli_runner.invoke(main, ["archive", "clear", "-y"])
        assert result.exit_code == 0
        assert "cleared" in result.output.lower()
        assert not progress_file.exists()

    def test_archive_clear_no_session(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test archive clear with no active session."""
        result = cli_runner.invoke(main, ["archive", "clear"])
        assert result.exit_code == 0
        assert "No active session" in result.output


class TestLearnCommand:
    """Tests for learn command."""

    def test_learn_records_learning(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test learn records a learning."""
        result = cli_runner.invoke(main, ["learn", "This is a discovery"])
        assert result.exit_code == 0
        assert "Learning recorded" in result.output

        # Verify it was saved
        learnings_file = initialized_project / ".afk" / "learnings.txt"
        assert learnings_file.exists()
        assert "This is a discovery" in learnings_file.read_text()

    def test_learn_with_task_id(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test learn with associated task ID."""
        result = cli_runner.invoke(main, ["learn", "Gotcha found", "--task", "auth-login"])
        assert result.exit_code == 0

        learnings_file = initialized_project / ".afk" / "learnings.txt"
        content = learnings_file.read_text()
        assert "[auth-login]" in content
        assert "Gotcha found" in content


class TestResetCommand:
    """Tests for reset command."""

    def test_reset_clears_failures(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test reset clears failure count and status."""
        # First, fail a task a few times
        cli_runner.invoke(main, ["fail", "task-1"])
        cli_runner.invoke(main, ["fail", "task-1"])

        result = cli_runner.invoke(main, ["reset", "task-1"])
        assert result.exit_code == 0
        assert "Task reset" in result.output
        assert "2 failures" in result.output

    def test_reset_task_not_found(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test reset with non-existent task."""
        result = cli_runner.invoke(main, ["reset", "nonexistent-task"])
        assert result.exit_code == 0
        assert "Task not found" in result.output


class TestExplainCommand:
    """Tests for explain command."""

    def test_explain_shows_state(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test explain shows session state."""
        result = cli_runner.invoke(main, ["explain"])
        assert result.exit_code == 0
        assert "Session State" in result.output
        assert "Iterations" in result.output
        assert "Tasks" in result.output

    def test_explain_not_initialized(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test explain when not initialized."""
        result = cli_runner.invoke(main, ["explain"])
        assert result.exit_code == 0
        assert "not initialized" in result.output

    def test_explain_shows_next_task(
        self, cli_runner: CliRunner, initialized_project: Path
    ) -> None:
        """Test explain shows next task to work on."""
        result = cli_runner.invoke(main, ["explain"])
        assert result.exit_code == 0
        assert "Next task" in result.output
        assert "task-1" in result.output

    def test_explain_verbose(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test explain with --verbose flag."""
        # Add a learning first
        cli_runner.invoke(main, ["learn", "Test learning"])

        result = cli_runner.invoke(main, ["explain", "-v"])
        assert result.exit_code == 0
        assert "Learnings" in result.output


class TestStartCommand:
    """Tests for start command."""

    def test_start_inits_if_needed(self, cli_runner: CliRunner, python_project: Path) -> None:
        """Test start initializes project if needed."""
        result = cli_runner.invoke(main, ["start", "-y"])
        assert result.exit_code == 0
        assert "Initializing" in result.output or "Configuration saved" in result.output
        assert (python_project / ".afk" / "config.json").exists()

    def test_start_warns_no_sources(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test start warns when no sources configured."""
        # Create minimal afk setup
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text('{"sources": []}')

        result = cli_runner.invoke(main, ["start"])
        assert result.exit_code == 0
        assert "No task sources configured" in result.output

    def test_start_runs_loop(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test start runs the loop when sources exist."""
        from afk.prd_store import PrdDocument, UserStory

        mock_prd = PrdDocument(
            project="test",
            userStories=[
                UserStory(id="task-1", title="Test", description="Test", passes=False)
            ],
        )
        with patch("afk.runner.sync_prd", return_value=mock_prd):
            result = cli_runner.invoke(main, ["start", "-y"])
        assert result.exit_code == 0


class TestGoCommand:
    """Tests for afk go command (zero-config quick start)."""

    def test_go_help(self, cli_runner: CliRunner) -> None:
        """Test go command has help."""
        result = cli_runner.invoke(main, ["go", "--help"])
        assert result.exit_code == 0
        assert "Quick start with zero config" in result.output

    def test_go_no_sources(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go fails gracefully with no sources."""
        # Need to have AI CLI configured to get past first-run
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text(
            '{"ai_cli": {"command": "echo", "args": []}}'
        )
        result = cli_runner.invoke(main, ["go"])
        assert result.exit_code == 1
        assert "No task sources found" in result.output

    def test_go_dry_run(self, cli_runner: CliRunner, initialized_project: Path) -> None:
        """Test go --dry-run shows what would run."""
        result = cli_runner.invoke(main, ["go", "--dry-run"])
        assert result.exit_code == 0
        assert "Dry Run" in result.output
        assert "Iterations" in result.output
        assert "Sources" in result.output

    def test_go_with_iterations(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go with iteration count."""
        (temp_project / "TODO.md").write_text("- [ ] Task 1\n")
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text(
            '{"sources": [{"type": "markdown", "path": "TODO.md"}], '
            '"ai_cli": {"command": "echo", "args": []}}'
        )
        result = cli_runner.invoke(main, ["go", "5", "--dry-run"])
        assert result.exit_code == 0
        assert "Iterations" in result.output

    def test_go_with_source_file(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go with explicit source file."""
        (temp_project / "my-tasks.md").write_text("- [ ] Task 1\n")
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text(
            '{"ai_cli": {"command": "echo", "args": []}}'
        )
        result = cli_runner.invoke(main, ["go", "my-tasks.md", "3", "--dry-run"])
        assert result.exit_code == 0
        assert "markdown" in result.output

    def test_go_source_not_found(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go with non-existent source file."""
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text(
            '{"ai_cli": {"command": "echo", "args": []}}'
        )
        result = cli_runner.invoke(main, ["go", "nonexistent.md"])
        assert result.exit_code == 1
        assert "Source file not found" in result.output

    def test_go_infers_sources(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go auto-detects sources when none configured."""
        (temp_project / "TODO.md").write_text("- [ ] Task 1\n")
        (temp_project / ".afk").mkdir()
        (temp_project / ".afk" / "config.json").write_text(
            '{"ai_cli": {"command": "echo", "args": []}}'
        )
        result = cli_runner.invoke(main, ["go", "--dry-run"])
        assert result.exit_code == 0
        assert "markdown" in result.output

    def test_go_uses_existing_prd(self, cli_runner: CliRunner, temp_project: Path) -> None:
        """Test go uses existing .afk/prd.json when no sources configured.

        This is the case where user runs afk prd parse first or places a
        prd.json manually - we should use it directly without overwriting.
        """
        import json

        (temp_project / ".afk").mkdir()
        # Create a PRD with stories but NO sources configured
        prd_data = {
            "project": "test",
            "userStories": [
                {
                    "id": "task-1",
                    "title": "Test task",
                    "description": "A test task",
                    "acceptanceCriteria": ["It works"],
                    "priority": 1,
                    "passes": False,
                }
            ],
        }
        (temp_project / ".afk" / "prd.json").write_text(json.dumps(prd_data))
        # Config with NO sources
        (temp_project / ".afk" / "config.json").write_text(
            '{"sources": [], "ai_cli": {"command": "echo", "args": []}}'
        )

        result = cli_runner.invoke(main, ["go", "--dry-run"])
        # Should NOT fail with "no sources" - should use the PRD
        assert "No task sources found" not in result.output
        # Should show dry run output
        assert result.exit_code == 0
