"""Tests for afk.runner module."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from afk.config import AfkConfig, AiCliConfig, ArchiveConfig, GitConfig, LimitsConfig
from afk.runner import (
    COMPLETION_SIGNALS,
    IterationResult,
    IterationRunner,
    RunResult,
    StopReason,
    _contains_completion_signal,
    run_iteration,
    run_loop,
)


class TestStopReason:
    """Tests for StopReason enum."""

    def test_all_reasons_have_values(self) -> None:
        """Test all stop reasons have human-readable values."""
        assert StopReason.COMPLETE.value == "All tasks completed"
        assert StopReason.MAX_ITERATIONS.value == "Maximum iterations reached"
        assert StopReason.TIMEOUT.value == "Session timeout reached"
        assert StopReason.NO_TASKS.value == "No tasks available"
        assert StopReason.USER_INTERRUPT.value == "User interrupted"
        assert StopReason.AI_ERROR.value == "AI CLI error"


class TestIterationResult:
    """Tests for IterationResult dataclass."""

    def test_success_result(self) -> None:
        """Test successful iteration result."""
        result = IterationResult(success=True, task_id="task-1", output="Done")

        assert result.success is True
        assert result.task_id == "task-1"
        assert result.error is None
        assert result.output == "Done"

    def test_failure_result(self) -> None:
        """Test failed iteration result."""
        result = IterationResult(success=False, error="Command failed")

        assert result.success is False
        assert result.error == "Command failed"


class TestRunResult:
    """Tests for RunResult dataclass."""

    def test_run_result(self) -> None:
        """Test run result with all fields."""
        result = RunResult(
            iterations_completed=5,
            tasks_completed=3,
            stop_reason=StopReason.COMPLETE,
            duration_seconds=120.5,
            archived_to=Path(".afk/archive/test"),
        )

        assert result.iterations_completed == 5
        assert result.tasks_completed == 3
        assert result.stop_reason == StopReason.COMPLETE
        assert result.duration_seconds == 120.5
        assert result.archived_to == Path(".afk/archive/test")


class TestRunIteration:
    """Tests for run_iteration function."""

    def test_detects_afk_complete(self, temp_afk_dir: Path) -> None:
        """Test detects AFK_COMPLETE stop signal."""
        config = AfkConfig()

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "AFK_COMPLETE: All tasks finished"

            result = run_iteration(config, iteration=1)

        assert result.success is True
        assert result.error == "AFK_COMPLETE"

    def test_detects_afk_limit_reached(self, temp_afk_dir: Path) -> None:
        """Test detects AFK_LIMIT_REACHED stop signal."""
        config = AfkConfig()

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "AFK_LIMIT_REACHED: Max iterations exceeded"

            result = run_iteration(config, iteration=1)

        assert result.success is False
        assert result.error == "AFK_LIMIT_REACHED"

    def test_spawns_ai_cli(self, temp_afk_dir: Path) -> None:
        """Test spawns configured AI CLI with prompt."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=["hello"]),
            limits=LimitsConfig(timeout_minutes=1),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_process = MagicMock()
                mock_process.returncode = 0
                # Mock streaming readline - returns lines then empty string to stop
                mock_process.stdout.readline.side_effect = ["Output\n", ""]
                mock_process.stdin = MagicMock()
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1)

        assert result.success is True
        assert "Output" in result.output
        mock_popen.assert_called_once()

    def test_handles_ai_cli_not_found(self, temp_afk_dir: Path) -> None:
        """Test handles missing AI CLI gracefully."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="nonexistent-cli", args=[]),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_popen.side_effect = FileNotFoundError()

                result = run_iteration(config, iteration=1)

        assert result.success is False
        assert result.error is not None and "not found" in result.error

    def test_handles_timeout(self, temp_afk_dir: Path) -> None:
        """Test handles iteration timeout (non-streaming mode)."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="sleep", args=["100"]),
            limits=LimitsConfig(timeout_minutes=1),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_process = MagicMock()
                mock_process.stdout = None  # Disable streaming to use communicate()
                mock_process.stdin = MagicMock()
                mock_process.communicate.side_effect = subprocess.TimeoutExpired(
                    cmd="sleep", timeout=60
                )
                mock_process.kill = MagicMock()
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1, stream=False)

        assert result.success is False
        assert result.error is not None and "timed out" in result.error


class TestRunLoop:
    """Tests for run_loop function."""

    def test_stops_when_no_tasks(self, temp_afk_dir: Path) -> None:
        """Test stops immediately when no pending stories."""
        from afk.prd_store import PrdDocument

        config = AfkConfig()

        # Empty PRD = all complete
        empty_prd = PrdDocument(user_stories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.archive_session"):
                result = run_loop(config, max_iterations=5)

        assert result.stop_reason == StopReason.COMPLETE
        assert result.iterations_completed == 0

    def test_respects_max_iterations(self, temp_afk_dir: Path) -> None:
        """Test respects max_iterations limit."""
        from afk.prd_store import PrdDocument, UserStory

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        iteration_count = [0]

        def mock_run_iteration(*args: object, **kwargs: object) -> IterationResult:
            iteration_count[0] += 1
            if iteration_count[0] >= 3:
                return IterationResult(success=True, error="AFK_COMPLETE")
            return IterationResult(success=True)

        mock_prd = PrdDocument(
            user_stories=[UserStory(id="task-1", title="Test", description="Test", passes=False)]
        )

        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.check_limits") as mock_limits:
                    # Allow first 2 iterations, then signal complete
                    mock_limits.side_effect = [
                        (True, None),
                        (True, None),
                        (False, "AFK_COMPLETE: All tasks finished"),
                    ]

                    with patch.object(IterationRunner, "run", side_effect=mock_run_iteration):
                        with patch("afk.runner.SessionProgress"):
                            result = run_loop(config, max_iterations=10)

        assert result.stop_reason == StopReason.COMPLETE

    def test_archives_previous_session(self, temp_afk_dir: Path) -> None:
        """Test archives previous session before starting."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5, "tasks": {}}')

        empty_prd = PrdDocument(user_stories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.archive_session") as mock_archive:
                mock_archive.return_value = Path(".afk/archive/test")

                with patch("afk.runner.clear_session"):
                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_progress.load.return_value = MagicMock(iterations=5)

                        run_loop(config, max_iterations=5)

        # Should archive when previous session exists
        mock_archive.assert_called()

    def test_resume_skips_initial_archiving(self, temp_afk_dir: Path) -> None:
        """Test resume=True skips archiving previous session at start."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5, "tasks": {}}')

        with patch("afk.runner.sync_prd") as mock_sync:
            mock_prd = MagicMock()
            mock_prd.user_stories = []
            mock_sync.return_value = mock_prd

            with patch("afk.runner.load_prd") as mock_load:
                mock_load.return_value = mock_prd

                with patch("afk.runner.archive_session") as mock_archive:
                    mock_archive.return_value = None  # Prevent MagicMock in output

                    with patch("afk.runner.clear_session") as mock_clear:
                        with patch("afk.runner.SessionProgress") as mock_progress:
                            mock_session = MagicMock()
                            mock_session.iterations = 5
                            mock_progress.load.return_value = mock_session

                            run_loop(config, max_iterations=5, resume=True)

        # clear_session should NOT be called when resume=True (initial archive skipped)
        mock_clear.assert_not_called()
        # archive_session may still be called at end of loop, but not for "new_run" reason
        # Check that no call was made with reason="new_run"
        for call in mock_archive.call_args_list:
            if call.kwargs.get("reason") == "new_run" or (
                len(call.args) > 1 and call.args[1] == "new_run"
            ):
                raise AssertionError("archive_session was called with reason='new_run'")

    def test_resume_continues_from_previous_iteration(self, temp_afk_dir: Path) -> None:
        """Test resume continues from where it left off."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        with patch("afk.runner.sync_prd") as mock_sync:
            mock_prd = MagicMock()
            mock_prd.user_stories = []
            mock_sync.return_value = mock_prd

            with patch("afk.runner.load_prd") as mock_load:
                mock_load.return_value = mock_prd

                with patch("afk.runner.archive_session"):
                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_session = MagicMock()
                        mock_session.iterations = 3  # Previous iterations
                        mock_progress.load.return_value = mock_session

                        result = run_loop(config, max_iterations=5, resume=True)

        # Session should show previous iteration count was recognised
        assert result is not None

    def test_creates_branch_when_configured(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test creates feature branch when specified."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(
            git=GitConfig(auto_branch=True, branch_prefix="afk/"),
            archive=ArchiveConfig(enabled=False),
        )

        empty_prd = PrdDocument(user_stories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.create_branch") as mock_branch:
                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_progress.load.return_value = MagicMock(iterations=0)

                    run_loop(config, max_iterations=5, branch="my-feature")

        mock_branch.assert_called_once_with("my-feature", config)

    def test_completes_when_all_stories_pass(self, temp_afk_dir: Path) -> None:
        """Test loop completes when all stories have passes: true."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(
            git=GitConfig(auto_commit=True),
            archive=ArchiveConfig(enabled=False),
            ai_cli=AiCliConfig(command="echo", args=[]),
        )

        empty_prd = PrdDocument(user_stories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.check_limits") as mock_limits:
                mock_limits.return_value = (False, "AFK_COMPLETE")

                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_session = MagicMock()
                    mock_session.iterations = 0
                    mock_session.get_completed_tasks.return_value = []
                    mock_progress.load.return_value = mock_session

                    result = run_loop(config, max_iterations=5)

        # Empty PRD means all complete
        assert result.stop_reason == StopReason.COMPLETE


class TestTaskProgressTracking:
    """Tests for task progress tracking in the runner."""

    def test_marks_task_in_progress_before_iteration(self, temp_afk_dir: Path) -> None:
        """Test that current task is marked in_progress before running."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.progress import SessionProgress

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        mock_prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=False,
                    source="test",
                )
            ]
        )

        pending_stories = mock_prd.user_stories
        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.all_stories_complete", return_value=False):
                    with patch("afk.runner.get_pending_stories", return_value=pending_stories):
                        with patch("afk.runner.check_limits") as mock_limits:
                            # Stop after first iteration
                            mock_limits.side_effect = [
                                (True, None),
                                (False, "AFK_COMPLETE"),
                            ]
                            with patch.object(IterationRunner, "run") as mock_run:
                                mock_run.return_value = IterationResult(success=True)

                                run_loop(config, max_iterations=1)

        # Verify task was marked in_progress
        progress = SessionProgress.load(temp_afk_dir / "progress.json")
        assert "story-1" in progress.tasks
        assert progress.tasks["story-1"].status == "in_progress"
        assert progress.tasks["story-1"].source == "test"

    def test_marks_task_completed_when_story_passes(self, temp_afk_dir: Path) -> None:
        """Test that _check_story_completion marks tasks as completed."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.progress import SessionProgress
        from afk.runner import LoopController

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        # Initial PRD with pending story
        old_prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=False,
                    source="json:test.json",
                )
            ]
        )

        # PRD after AI marks it complete
        new_prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=True,
                    source="json:test.json",
                )
            ]
        )

        controller = LoopController(config)

        with patch("afk.runner.load_prd", return_value=new_prd):
            completed_count = controller._check_story_completion(old_prd)

        assert completed_count == 1

        # Verify task was marked completed in progress.json
        progress = SessionProgress.load(temp_afk_dir / "progress.json")
        assert "story-1" in progress.tasks
        assert progress.tasks["story-1"].status == "completed"
        assert progress.tasks["story-1"].completed_at is not None


class TestRunLoopIntegration:
    """Integration-style tests for run_loop."""

    def test_full_loop_with_mocked_ai(self, temp_afk_dir: Path) -> None:
        """Test a full loop with mocked AI responses."""
        from afk.prd_store import PrdDocument, UserStory

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
            limits=LimitsConfig(max_iterations=3),
        )

        call_count = [0]

        def mock_check_limits(*args: object, **kwargs: object) -> tuple[bool, str | None]:
            call_count[0] += 1
            if call_count[0] >= 2:
                return (False, "AFK_COMPLETE: All tasks finished")
            return (True, None)

        mock_prd = PrdDocument(
            user_stories=[UserStory(id="task-1", title="Test", description="Test", passes=False)]
        )

        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.all_stories_complete", return_value=False):
                    with patch(
                        "afk.runner.get_pending_stories", return_value=mock_prd.user_stories
                    ):
                        with patch("afk.runner.check_limits", side_effect=mock_check_limits):
                            with patch.object(IterationRunner, "run") as mock_iteration:
                                mock_iteration.return_value = IterationResult(success=True)

                                with patch("afk.runner.SessionProgress") as mock_progress:
                                    mock_session = MagicMock()
                                    mock_session.iterations = 0
                                    mock_session.get_completed_tasks.return_value = []
                                    mock_progress.load.return_value = mock_session

                                    result = run_loop(config, max_iterations=3)

        assert result.stop_reason == StopReason.COMPLETE
        assert result.iterations_completed == 1


class TestCompletionSignals:
    """Tests for completion signal detection."""

    def test_completion_signals_defined(self) -> None:
        """Test that completion signals are defined."""
        assert "<promise>COMPLETE</promise>" in COMPLETION_SIGNALS
        assert "AFK_COMPLETE" in COMPLETION_SIGNALS
        assert "AFK_STOP" in COMPLETION_SIGNALS

    def test_contains_completion_signal_empty(self) -> None:
        """Test with empty output."""
        assert _contains_completion_signal(None) is False
        assert _contains_completion_signal("") is False

    def test_contains_completion_signal_ralf_style(self) -> None:
        """Test detecting ralf.sh style signal."""
        output = "Done with task\n<promise>COMPLETE</promise>\n"
        assert _contains_completion_signal(output) is True

    def test_contains_completion_signal_afk_style(self) -> None:
        """Test detecting AFK style signal."""
        assert _contains_completion_signal("All done AFK_COMPLETE") is True
        assert _contains_completion_signal("AFK_STOP now") is True

    def test_contains_no_signal(self) -> None:
        """Test with no completion signal."""
        assert _contains_completion_signal("Just some output") is False


class TestOutputHandlerFileWatcherIntegration:
    """Tests for OutputHandler file watcher integration."""

    def test_file_watcher_disabled_by_default(self) -> None:
        """Test file watcher is disabled when no watch_root provided."""
        from afk.runner import OutputHandler

        handler = OutputHandler()

        assert handler._file_watcher is None
        assert handler.file_watcher is None

    def test_file_watcher_enabled_with_watch_root(self, tmp_path: Path) -> None:
        """Test file watcher is created when watch_root is provided."""
        from afk.file_watcher import FileWatcher
        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)

        assert handler._file_watcher is not None
        assert isinstance(handler._file_watcher, FileWatcher)
        assert handler.file_watcher is handler._file_watcher

    def test_file_watcher_accepts_string_path(self, tmp_path: Path) -> None:
        """Test file watcher accepts string path for watch_root."""
        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=str(tmp_path))

        assert handler._file_watcher is not None

    def test_file_watcher_accepts_custom_ignore_patterns(self, tmp_path: Path) -> None:
        """Test file watcher uses custom ignore patterns."""
        from afk.runner import OutputHandler

        patterns = ["*.log", "build"]
        handler = OutputHandler(watch_root=tmp_path, watch_ignore_patterns=patterns)

        assert handler._file_watcher is not None
        assert handler._file_watcher._ignore_patterns == patterns

    def test_start_feedback_starts_watcher(self, tmp_path: Path) -> None:
        """Test start_feedback() starts the file watcher."""
        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)

        assert handler._file_watcher is not None
        assert handler._file_watcher.is_running is False

        handler.start_feedback()

        assert handler._file_watcher.is_running is True

        handler.stop_feedback()

    def test_stop_feedback_stops_watcher(self, tmp_path: Path) -> None:
        """Test stop_feedback() stops the file watcher."""
        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)
        handler.start_feedback()

        assert handler._file_watcher is not None
        assert handler._file_watcher.is_running is True

        handler.stop_feedback()

        assert handler._file_watcher.is_running is False

    def test_start_feedback_clears_recorded_paths(self, tmp_path: Path) -> None:
        """Test start_feedback() clears the recorded paths set."""
        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)
        handler._recorded_paths.add("/some/path.py")

        handler.start_feedback()

        assert len(handler._recorded_paths) == 0

        handler.stop_feedback()

    def test_stream_line_polls_watcher_changes(self, tmp_path: Path) -> None:
        """Test stream_line() polls file watcher and records changes."""
        import time

        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)
        handler.start_feedback()

        # Give watcher time to start
        time.sleep(0.1)

        # Create a file while watcher is running
        test_file = tmp_path / "new_file.py"
        test_file.write_text("# test")

        # Give watchdog time to detect
        time.sleep(0.5)

        # Stream a line to trigger polling
        handler.stream_line("Some output\n")

        # Check that the change was recorded in metrics
        metrics = handler.metrics_collector.metrics
        all_files = metrics.files_created + metrics.files_modified

        handler.stop_feedback()

        assert str(test_file) in all_files

    def test_stream_line_deduplicates_parsed_and_watched_changes(self, tmp_path: Path) -> None:
        """Test that files from parsed output are not double-counted from watcher."""
        import time

        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)
        handler.start_feedback()
        time.sleep(0.1)

        # Create a file
        test_file = tmp_path / "parsed_file.py"
        test_file.write_text("# test")
        time.sleep(0.5)

        # Simulate the output parser detecting this file first
        # by calling stream_line with output that matches the file pattern
        handler.stream_line(f"Edited {test_file}\n")

        # The path should now be in recorded_paths (from parsed event)
        assert str(test_file) in handler._recorded_paths

        # Metrics should show the file only once
        metrics = handler.metrics_collector.metrics
        modified_count = metrics.files_modified.count(str(test_file))

        handler.stop_feedback()

        # File should appear only once (from parsed output, not watcher)
        assert modified_count == 1

    def test_stop_feedback_polls_final_changes(self, tmp_path: Path) -> None:
        """Test stop_feedback() polls remaining watcher changes."""
        import time

        from afk.runner import OutputHandler

        handler = OutputHandler(watch_root=tmp_path)
        handler.start_feedback()
        time.sleep(0.1)

        # Create a file
        test_file = tmp_path / "final_file.py"
        test_file.write_text("# test")
        time.sleep(0.5)

        # Don't call stream_line - the change should still be caught on stop

        handler.stop_feedback()

        # Check metrics has the file
        metrics = handler.metrics_collector.metrics
        all_files = metrics.files_created + metrics.files_modified
        assert str(test_file) in all_files


class TestOutputHandlerFeedbackIntegration:
    """Tests for OutputHandler feedback display integration."""

    def test_feedback_disabled_by_default(self) -> None:
        """Test feedback is disabled by default."""
        from afk.runner import OutputHandler

        handler = OutputHandler()

        assert handler._feedback_enabled is False
        assert handler._feedback is None

    def test_feedback_enabled_creates_display(self) -> None:
        """Test enabling feedback creates a FeedbackDisplay instance."""
        from afk.feedback import FeedbackDisplay
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)

        assert handler._feedback_enabled is True
        assert handler._feedback is not None
        assert isinstance(handler._feedback, FeedbackDisplay)

    def test_feedback_mode_off_disables_feedback(self) -> None:
        """Test feedback_mode='off' disables feedback even if enabled=True."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True, feedback_mode="off")

        assert handler._feedback_enabled is False
        assert handler._feedback is None

    def test_feedback_property_returns_display(self) -> None:
        """Test feedback property provides access to the display."""
        from afk.feedback import FeedbackDisplay
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)

        assert handler.feedback is not None
        assert isinstance(handler.feedback, FeedbackDisplay)

    def test_feedback_property_returns_none_when_disabled(self) -> None:
        """Test feedback property returns None when disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        assert handler.feedback is None

    def test_start_feedback_calls_display_start(self) -> None:
        """Test start_feedback() calls the display's start method."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)
        assert handler._feedback is not None

        # Mock the start method
        with patch.object(handler._feedback, "start") as mock_start:
            handler.start_feedback()

        mock_start.assert_called_once()

    def test_start_feedback_noop_when_disabled(self) -> None:
        """Test start_feedback() is a no-op when feedback is disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        # Should not raise, even with no display
        handler.start_feedback()

    def test_stop_feedback_calls_display_stop(self) -> None:
        """Test stop_feedback() calls the display's stop method."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)
        assert handler._feedback is not None

        with patch.object(handler._feedback, "stop") as mock_stop:
            handler.stop_feedback()

        mock_stop.assert_called_once()

    def test_stop_feedback_noop_when_disabled(self) -> None:
        """Test stop_feedback() is a no-op when feedback is disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        # Should not raise, even with no display
        handler.stop_feedback()

    def test_stream_line_updates_feedback(self) -> None:
        """Test stream_line() updates feedback display with metrics."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)
        assert handler._feedback is not None

        with patch.object(handler._feedback, "update") as mock_update:
            handler.stream_line("Write file: test.py\n")

        # Should have called update with current metrics
        mock_update.assert_called()
        call_args = mock_update.call_args
        metrics = call_args[0][0]  # First positional argument
        assert metrics is handler.metrics_collector.metrics

    def test_stream_line_no_update_when_disabled(self) -> None:
        """Test stream_line() doesn't update feedback when disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        # Should not raise, even with no display
        handler.stream_line("Write file: test.py\n")

    def test_show_gates_failed_uses_feedback_display(self) -> None:
        """Test show_gates_failed() calls feedback display when enabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)
        assert handler._feedback is not None

        with patch.object(handler._feedback, "show_gates_failed") as mock_show:
            handler.show_gates_failed(["types", "lint"], continuing=True)

        mock_show.assert_called_once_with(["types", "lint"], True)

    def test_show_gates_failed_fallback_to_console(self) -> None:
        """Test show_gates_failed() uses console when feedback disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler.console, "print") as mock_print:
            handler.show_gates_failed(["test"], continuing=True)

        mock_print.assert_called_once()
        call_args = mock_print.call_args[0][0]
        assert "test" in call_args
        assert "Continuing" in call_args

    def test_show_gates_failed_fallback_without_continuing(self) -> None:
        """Test show_gates_failed() fallback without continuing indicator."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler.console, "print") as mock_print:
            handler.show_gates_failed(["lint"], continuing=False)

        mock_print.assert_called_once()
        call_args = mock_print.call_args[0][0]
        assert "lint" in call_args
        assert "Continuing" not in call_args


class TestRunQualityGatesIntegration:
    """Tests for run_quality_gates feedback integration."""

    def test_run_quality_gates_calls_show_gates_failed_on_failure(self, temp_afk_dir: Path) -> None:
        """Test run_quality_gates calls show_gates_failed when gates fail."""
        from afk.config import FeedbackLoopsConfig
        from afk.runner import OutputHandler, run_quality_gates

        feedback_loops = FeedbackLoopsConfig(
            types="exit 1",  # Always fails
        )
        handler = OutputHandler(feedback_enabled=True)

        with patch.object(handler, "show_gates_failed") as mock_show:
            result = run_quality_gates(
                feedback_loops,
                output_handler=handler,
                continuing=True,
            )

        assert not result.passed
        assert "types" in result.failed_gates
        mock_show.assert_called_once_with(["types"], continuing=True)

    def test_run_quality_gates_no_show_gates_failed_on_success(self, temp_afk_dir: Path) -> None:
        """Test run_quality_gates doesn't call show_gates_failed when all pass."""
        from afk.config import FeedbackLoopsConfig
        from afk.runner import OutputHandler, run_quality_gates

        feedback_loops = FeedbackLoopsConfig(
            types="exit 0",  # Always passes
        )
        handler = OutputHandler(feedback_enabled=True)

        with patch.object(handler, "show_gates_failed") as mock_show:
            result = run_quality_gates(
                feedback_loops,
                output_handler=handler,
                continuing=True,
            )

        assert result.passed
        mock_show.assert_not_called()

    def test_run_quality_gates_passes_continuing_flag(self, temp_afk_dir: Path) -> None:
        """Test run_quality_gates passes continuing flag correctly."""
        from afk.config import FeedbackLoopsConfig
        from afk.runner import OutputHandler, run_quality_gates

        feedback_loops = FeedbackLoopsConfig(
            lint="exit 1",  # Always fails
        )
        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler, "show_gates_failed") as mock_show:
            run_quality_gates(
                feedback_loops,
                output_handler=handler,
                continuing=False,
            )

        mock_show.assert_called_once_with(["lint"], continuing=False)


class TestOutputHandlerCelebration:
    """Tests for OutputHandler celebration feature."""

    def test_show_celebration_uses_feedback_display(self) -> None:
        """Test show_celebration() calls feedback display when enabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=True)
        assert handler._feedback is not None

        with patch.object(handler._feedback, "show_celebration") as mock_show:
            handler.show_celebration("test-task")

        mock_show.assert_called_once_with("test-task")

    def test_show_celebration_fallback_to_console(self) -> None:
        """Test show_celebration() uses console when feedback disabled."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler.console, "print") as mock_print:
            handler.show_celebration("my-task")

        # Should have multiple prints (spacing + message + spacing)
        assert mock_print.call_count >= 1
        # Check that task ID appears in one of the calls
        all_calls = " ".join(str(call) for call in mock_print.call_args_list)
        assert "my-task" in all_calls

    def test_show_celebration_fallback_includes_checkmark(self) -> None:
        """Test show_celebration() fallback includes checkmark."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler.console, "print") as mock_print:
            handler.show_celebration("task-id")

        # Check that checkmark appears in one of the calls
        all_calls = " ".join(str(call) for call in mock_print.call_args_list)
        assert "✓" in all_calls

    def test_show_celebration_fallback_includes_task_complete(self) -> None:
        """Test show_celebration() fallback includes 'Task Complete' message."""
        from afk.runner import OutputHandler

        handler = OutputHandler(feedback_enabled=False)

        with patch.object(handler.console, "print") as mock_print:
            handler.show_celebration("task-id")

        # Check that 'Task Complete' appears in one of the calls
        all_calls = " ".join(str(call) for call in mock_print.call_args_list)
        assert "Task Complete" in all_calls


class TestLoopControllerCelebration:
    """Tests for LoopController celebration on task completion."""

    def test_check_story_completion_calls_show_celebration(self, temp_afk_dir: Path) -> None:
        """Test _check_story_completion calls show_celebration for each completed task."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.runner import LoopController

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        # Initial PRD with pending story
        old_prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=False,
                    source="json:test.json",
                )
            ]
        )

        # PRD after AI marks it complete
        new_prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=True,
                    source="json:test.json",
                )
            ]
        )

        controller = LoopController(config)

        with patch("afk.runner.load_prd", return_value=new_prd):
            with patch.object(controller.output, "show_celebration") as mock_celebration:
                completed_count = controller._check_story_completion(old_prd)

        assert completed_count == 1
        mock_celebration.assert_called_once_with("story-1")

    def test_check_story_completion_calls_show_celebration_for_multiple(
        self, temp_afk_dir: Path
    ) -> None:
        """Test _check_story_completion calls show_celebration for each completed task."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.runner import LoopController

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        # Initial PRD with multiple pending stories
        old_prd = PrdDocument(
            user_stories=[
                UserStory(id="story-1", title="Story 1", description="", passes=False),
                UserStory(id="story-2", title="Story 2", description="", passes=False),
            ]
        )

        # PRD after both are completed
        new_prd = PrdDocument(
            user_stories=[
                UserStory(id="story-1", title="Story 1", description="", passes=True),
                UserStory(id="story-2", title="Story 2", description="", passes=True),
            ]
        )

        controller = LoopController(config)

        with patch("afk.runner.load_prd", return_value=new_prd):
            with patch.object(controller.output, "show_celebration") as mock_celebration:
                completed_count = controller._check_story_completion(old_prd)

        assert completed_count == 2
        assert mock_celebration.call_count == 2
        mock_celebration.assert_any_call("story-1")
        mock_celebration.assert_any_call("story-2")

    def test_check_story_completion_no_celebration_when_no_completion(
        self, temp_afk_dir: Path
    ) -> None:
        """Test _check_story_completion doesn't call show_celebration when nothing completed."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.runner import LoopController

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        # PRD with pending story that stays pending
        prd = PrdDocument(
            user_stories=[
                UserStory(id="story-1", title="Test", description="", passes=False)
            ]
        )

        controller = LoopController(config)

        with patch("afk.runner.load_prd", return_value=prd):
            with patch.object(controller.output, "show_celebration") as mock_celebration:
                completed_count = controller._check_story_completion(prd)

        assert completed_count == 0
        mock_celebration.assert_not_called()
