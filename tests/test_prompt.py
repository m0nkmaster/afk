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
        assert "prd.json" in template
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
        """Test basic prompt generation with PRD."""
        from afk.prd_store import PrdDocument, UserStory, save_prd

        # Create a prd.json with stories
        prd = PrdDocument(
            project="Test",
            userStories=[
                UserStory(
                    id="task-1",
                    title="Test task",
                    description="Test task description",
                    acceptanceCriteria=["AC 1"],
                )
            ],
        )
        save_prd(prd)

        config = AfkConfig(
            sources=[SourceConfig(type="json", path="tasks.json")],
            limits=LimitsConfig(max_iterations=10),
        )
        config.save()

        prompt = generate_prompt(config)
        assert "Iteration" in prompt or "1/" in prompt
        # With Ralph pattern, prompt tells AI to read prd.json
        assert "prd.json" in prompt
        # Next story shown in progress
        assert "task-1" in prompt

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
        from afk.prd_store import PrdDocument, UserStory, save_prd

        # Need pending stories to not get AFK_COMPLETE
        prd = PrdDocument(
            userStories=[
                UserStory(id="task-1", title="Test", description="Test", passes=False)
            ]
        )
        save_prd(prd)

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config, bootstrap=True)
        assert "Autonomous Loop" in prompt
        assert "running autonomously" in prompt

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
        assert "1/" in prompt1

        prompt2 = generate_prompt(config)
        assert "2/" in prompt2

    def test_completed_stories_count(self, temp_afk_dir: Path) -> None:
        """Test that completed stories are counted correctly."""
        from afk.prd_store import PrdDocument, UserStory, save_prd

        # Create PRD with one complete and one pending
        prd = PrdDocument(
            userStories=[
                UserStory(id="task-1", title="Done", description="Done", passes=True),
                UserStory(id="task-2", title="Pending", description="Pending", passes=False),
            ]
        )
        save_prd(prd)

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        # Should show 1/2 completed
        assert "Completed: 1/2" in prompt
        # Next story should be task-2
        assert "task-2" in prompt

    def test_stories_sorted_by_priority(self, temp_afk_dir: Path) -> None:
        """Test that next story is highest priority pending."""
        from afk.prd_store import PrdDocument, UserStory, save_prd

        prd = PrdDocument(
            userStories=[
                UserStory(id="low", title="Low", description="Low", priority=4, passes=False),
                UserStory(id="high", title="High", description="High", priority=1, passes=False),
            ]
        )
        save_prd(prd)

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        # High priority (priority=1) should be shown as next
        assert "high" in prompt

    def test_stop_signal_when_limit_reached(self, temp_afk_dir: Path) -> None:
        """Test stop signal when iteration limit reached."""
        from afk.prd_store import PrdDocument, UserStory, save_prd
        from afk.progress import SessionProgress

        # Need pending stories
        prd = PrdDocument(
            userStories=[
                UserStory(id="task-1", title="Test", description="Test", passes=False)
            ]
        )
        save_prd(prd)

        progress = SessionProgress()
        progress.iterations = 5
        progress.save()

        config = AfkConfig(limits=LimitsConfig(max_iterations=5))
        config.save()

        prompt = generate_prompt(config)
        assert "AFK_LIMIT_REACHED" in prompt

    def test_stop_signal_when_complete(self, temp_afk_dir: Path) -> None:
        """Test stop signal when all stories complete."""
        from afk.prd_store import PrdDocument, UserStory, save_prd

        # All stories passed
        prd = PrdDocument(
            userStories=[
                UserStory(id="task-1", title="Done", description="Done", passes=True)
            ]
        )
        save_prd(prd)

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        assert "AFK_COMPLETE" in prompt

    def test_no_stories_shows_complete(self, temp_afk_dir: Path) -> None:
        """Test that no stories results in AFK_COMPLETE."""
        from afk.prd_store import PrdDocument, save_prd

        # Empty PRD
        prd = PrdDocument()
        save_prd(prd)

        config = AfkConfig()
        config.save()

        prompt = generate_prompt(config)
        assert "AFK_COMPLETE" in prompt

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
        assert "## Your Task" in DEFAULT_TEMPLATE
        assert "## Progress" in DEFAULT_TEMPLATE
        assert "prd.json" in DEFAULT_TEMPLATE

    def test_template_has_loop_mode_section(self) -> None:
        """Test template has conditional loop mode section."""
        assert "{% if bootstrap -%}" in DEFAULT_TEMPLATE
        assert "Autonomous Loop" in DEFAULT_TEMPLATE

    def test_template_has_stop_signal_section(self) -> None:
        """Test template has conditional stop signal section."""
        assert "{% if stop_signal -%}" in DEFAULT_TEMPLATE
        assert "## STOP" in DEFAULT_TEMPLATE

    def test_template_follows_ralph_pattern(self) -> None:
        """Test template instructs AI to read prd.json directly."""
        assert "Read the PRD at `.afk/prd.json`" in DEFAULT_TEMPLATE
        assert "`passes: true`" in DEFAULT_TEMPLATE
        assert "<promise>COMPLETE</promise>" in DEFAULT_TEMPLATE
