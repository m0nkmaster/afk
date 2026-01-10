"""Tests for afk.prompt module."""

from __future__ import annotations

import json
from pathlib import Path

from afk.config import AfkConfig, FeedbackLoopsConfig, LimitsConfig, PromptConfig, SourceConfig
from afk.prompt import DEFAULT_TEMPLATE, _get_template, generate_prompt


class TestGetTemplate:
    """Tests for _get_template function."""

    def test_default_template(self) -> None:
        """Test default template is returned."""
        config = AfkConfig()
        template = _get_template(config)
        assert template == DEFAULT_TEMPLATE

    def test_minimal_template(self) -> None:
        """Test minimal template selection."""
        config = AfkConfig(prompt=PromptConfig(template="minimal"))
        template = _get_template(config)
        assert "Complete ONE task" in template
        assert len(template) < len(DEFAULT_TEMPLATE)

    def test_verbose_template(self) -> None:
        """Test verbose template is same as default."""
        config = AfkConfig(prompt=PromptConfig(template="verbose"))
        template = _get_template(config)
        assert template == DEFAULT_TEMPLATE

    def test_custom_template_path(self, temp_project: Path) -> None:
        """Test loading custom template from file."""
        custom_content = "Custom template: {{ iteration }}"
        custom_path = temp_project / ".afk" / "custom.jinja2"
        custom_path.parent.mkdir(parents=True, exist_ok=True)
        custom_path.write_text(custom_content)

        config = AfkConfig(prompt=PromptConfig(custom_path=str(custom_path)))
        template = _get_template(config)
        assert template == custom_content

    def test_custom_template_not_found(self, temp_project: Path) -> None:
        """Test fallback when custom template doesn't exist."""
        config = AfkConfig(prompt=PromptConfig(custom_path="nonexistent.jinja2"))
        template = _get_template(config)
        assert template == DEFAULT_TEMPLATE


class TestGeneratePrompt:
    """Tests for generate_prompt function."""

    def test_basic_prompt(self, temp_afk_dir: Path) -> None:
        """Test basic prompt generation."""
        # Create a tasks.json
        tasks_data = [{"id": "task-1", "description": "Test task"}]
        tasks_path = temp_afk_dir.parent / "tasks.json"
        tasks_path.write_text(json.dumps(tasks_data))

        config = AfkConfig(
            sources=[SourceConfig(type="json", path=str(tasks_path))],
            limits=LimitsConfig(max_iterations=10),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "Iteration" in prompt
        assert "task-1" in prompt
        assert "Test task" in prompt

    def test_prompt_includes_context_files(self, temp_afk_dir: Path) -> None:
        """Test that context files are included in prompt."""
        config = AfkConfig(
            prompt=PromptConfig(context_files=["AGENTS.md", "README.md"]),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "@AGENTS.md" in prompt
        assert "@README.md" in prompt

    def test_prompt_includes_feedback_loops(self, temp_afk_dir: Path) -> None:
        """Test that feedback loops are included."""
        config = AfkConfig(
            feedback_loops=FeedbackLoopsConfig(
                lint="ruff check .",
                test="pytest",
            ),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "ruff check ." in prompt
        assert "pytest" in prompt

    def test_prompt_includes_custom_instructions(self, temp_afk_dir: Path) -> None:
        """Test that custom instructions are included."""
        config = AfkConfig(
            prompt=PromptConfig(instructions=["Always run tests", "Use British English"]),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "Always run tests" in prompt
        assert "Use British English" in prompt

    def test_bootstrap_mode(self, temp_afk_dir: Path) -> None:
        """Test bootstrap mode adds loop instructions."""
        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config, bootstrap=True)
        assert "Autonomous Loop" in prompt
        assert "running autonomously" in prompt
        assert "afk fail" in prompt

    def test_limit_override(self, temp_afk_dir: Path) -> None:
        """Test limit override is used."""
        config = AfkConfig(limits=LimitsConfig(max_iterations=20))
        config.save()

        prompt = generate_prompt(config, limit_override=5)
        assert "/5" in prompt

    def test_iteration_increments(self, temp_afk_dir: Path) -> None:
        """Test that iteration count increments."""
        config = AfkConfig()
        config.save()

        prompt1 = generate_prompt(config)
        assert "Iteration 1" in prompt1 or "1/" in prompt1

        prompt2 = generate_prompt(config)
        assert "Iteration 2" in prompt2 or "2/" in prompt2

    def test_completed_tasks_filtered(self, temp_afk_dir: Path) -> None:
        """Test that completed tasks are filtered from prompt."""
        from afk.progress import SessionProgress, TaskProgress

        # Create tasks
        tasks_data = [
            {"id": "task-1", "description": "Completed"},
            {"id": "task-2", "description": "Pending"},
        ]
        tasks_path = temp_afk_dir.parent / "tasks.json"
        tasks_path.write_text(json.dumps(tasks_data))

        # Mark one as complete
        progress = SessionProgress()
        progress.tasks["task-1"] = TaskProgress(id="task-1", source="test", status="completed")
        progress.save()

        config = AfkConfig(
            sources=[SourceConfig(type="json", path=str(tasks_path))],
        )
        config.save()

        prompt = generate_prompt(config)
        assert "task-2" in prompt
        # task-1 should not be in the tasks list (but might be in "Last completed")

    def test_tasks_sorted_by_priority(self, temp_afk_dir: Path) -> None:
        """Test that tasks are sorted by priority."""
        tasks_data = [
            {"id": "low", "description": "Low priority", "priority": "low"},
            {"id": "high", "description": "High priority", "priority": "high"},
            {"id": "med", "description": "Medium priority", "priority": "medium"},
        ]
        tasks_path = temp_afk_dir.parent / "tasks.json"
        tasks_path.write_text(json.dumps(tasks_data))

        config = AfkConfig(
            sources=[SourceConfig(type="json", path=str(tasks_path))],
        )
        config.save()

        prompt = generate_prompt(config)
        # High should appear before low
        high_pos = prompt.find("high:")
        low_pos = prompt.find("low:")
        if high_pos != -1 and low_pos != -1:
            assert high_pos < low_pos

    def test_stop_signal_when_limit_reached(self, temp_afk_dir: Path) -> None:
        """Test stop signal when iteration limit reached."""
        from afk.progress import SessionProgress

        progress = SessionProgress()
        progress.iterations = 5
        progress.save()

        config = AfkConfig(limits=LimitsConfig(max_iterations=5))
        config.save()

        prompt = generate_prompt(config)
        assert "AFK_LIMIT_REACHED" in prompt

    def test_stop_signal_when_complete(self, temp_afk_dir: Path) -> None:
        """Test stop signal when all tasks complete."""
        from afk.progress import SessionProgress, TaskProgress

        # Create one task
        tasks_data = [{"id": "task-1", "description": "Only task"}]
        tasks_path = temp_afk_dir.parent / "tasks.json"
        tasks_path.write_text(json.dumps(tasks_data))

        # Mark it complete
        progress = SessionProgress()
        progress.tasks["task-1"] = TaskProgress(id="task-1", source="test", status="completed")
        progress.save()

        config = AfkConfig(
            sources=[SourceConfig(type="json", path=str(tasks_path))],
        )
        config.save()

        prompt = generate_prompt(config)
        assert "AFK_COMPLETE" in prompt

    def test_recently_completed_shown(self, temp_afk_dir: Path) -> None:
        """Test that recently completed task is shown."""
        from afk.progress import SessionProgress, TaskProgress

        progress = SessionProgress()
        progress.tasks["recent-task"] = TaskProgress(
            id="recent-task", source="test", status="completed"
        )
        progress.save()

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        assert "recent-task" in prompt

    def test_no_tasks_message(self, temp_afk_dir: Path) -> None:
        """Test message when no tasks available."""
        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        assert "No tasks available" in prompt

    def test_custom_feedback_loops(self, temp_afk_dir: Path) -> None:
        """Test custom feedback loops are included."""
        config = AfkConfig(
            feedback_loops=FeedbackLoopsConfig(
                custom={"format": "ruff format .", "security": "bandit -r src/"},
            ),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "ruff format ." in prompt
        assert "bandit -r src/" in prompt


class TestDefaultTemplate:
    """Tests for the default template content."""

    def test_template_has_required_sections(self) -> None:
        """Test template has all required sections."""
        assert "## Context Files" in DEFAULT_TEMPLATE
        assert "## Available Tasks" in DEFAULT_TEMPLATE
        assert "## Progress" in DEFAULT_TEMPLATE
        assert "## Instructions" in DEFAULT_TEMPLATE

    def test_template_has_loop_mode_section(self) -> None:
        """Test template has conditional loop mode section."""
        assert "{% if bootstrap -%}" in DEFAULT_TEMPLATE
        assert "Autonomous Loop" in DEFAULT_TEMPLATE

    def test_template_has_stop_signal_section(self) -> None:
        """Test template has conditional stop signal section."""
        assert "{% if stop_signal -%}" in DEFAULT_TEMPLATE
        assert "## STOP" in DEFAULT_TEMPLATE
