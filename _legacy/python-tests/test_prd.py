"""Tests for afk.prd module."""

from __future__ import annotations

from pathlib import Path

import pytest

from afk.config import AfkConfig
from afk.prd import generate_prd_prompt, load_prd_file
from afk.prd_store import PrdDocument, UserStory, load_prd, save_prd, sync_prd


class TestGeneratePrdPrompt:
    """Tests for generate_prd_prompt function."""

    def test_basic_prompt_generation(self) -> None:
        """Test basic prompt generation with simple content."""
        prd_content = "Users should be able to log in."
        prompt = generate_prd_prompt(prd_content)

        assert "Users should be able to log in." in prompt
        assert ".afk/prd.json" in prompt
        assert "tasks" in prompt
        assert "passes" in prompt

    def test_custom_output_path(self) -> None:
        """Test prompt with custom output path."""
        prompt = generate_prd_prompt("Test content", output_path="custom/path.json")

        assert "custom/path.json" in prompt
        assert ".afk/prd.json" not in prompt

    def test_prompt_contains_format_instructions(self) -> None:
        """Test that prompt contains all required format instructions."""
        prompt = generate_prd_prompt("My feature requirements")

        # Check for required JSON fields
        assert "id" in prompt
        assert "category" in prompt
        assert "description" in prompt
        assert "priority" in prompt
        assert "steps" in prompt
        assert "passes" in prompt

    def test_prompt_contains_category_options(self) -> None:
        """Test that prompt includes category options."""
        prompt = generate_prd_prompt("Feature spec")

        assert "functional" in prompt
        assert "non-functional" in prompt
        assert "technical" in prompt
        assert "ux" in prompt
        assert "security" in prompt

    def test_prompt_contains_priority_guidance(self) -> None:
        """Test that prompt includes priority guidance."""
        prompt = generate_prd_prompt("Requirements")

        assert "Priority 1" in prompt
        assert "Priority 5" in prompt

    def test_prompt_contains_guidelines(self) -> None:
        """Test that prompt includes implementation guidelines."""
        prompt = generate_prd_prompt("Spec")

        assert "comprehensive" in prompt.lower()
        assert "atomic" in prompt.lower()
        assert "testable" in prompt.lower()

    def test_multiline_prd_content(self) -> None:
        """Test with multiline PRD content."""
        prd_content = """# My App

## Features
- User authentication
- Dashboard

## Requirements
Users must be able to log in.
"""
        prompt = generate_prd_prompt(prd_content)

        assert "# My App" in prompt
        assert "User authentication" in prompt
        assert "Dashboard" in prompt

    def test_special_characters_in_prd(self) -> None:
        """Test that special characters are preserved."""
        prd_content = "Feature with <brackets> and 'quotes' and \"double quotes\""
        prompt = generate_prd_prompt(prd_content)

        assert "<brackets>" in prompt
        assert "'quotes'" in prompt


class TestLoadPrdFile:
    """Tests for load_prd_file function."""

    def test_load_existing_file(self, temp_project: Path) -> None:
        """Test loading an existing PRD file."""
        prd_path = temp_project / "requirements.md"
        prd_path.write_text("# Requirements\n\nBuild a thing.")

        content = load_prd_file(str(prd_path))

        assert "# Requirements" in content
        assert "Build a thing." in content

    def test_load_nonexistent_file(self, temp_project: Path) -> None:
        """Test loading a nonexistent file raises error."""
        with pytest.raises(FileNotFoundError, match="PRD file not found"):
            load_prd_file("nonexistent.md")

    def test_load_with_path_object(self, temp_project: Path) -> None:
        """Test loading with Path object."""
        prd_path = temp_project / "prd.md"
        prd_path.write_text("Content here")

        content = load_prd_file(prd_path)

        assert content == "Content here"

    def test_load_empty_file(self, temp_project: Path) -> None:
        """Test loading an empty file."""
        prd_path = temp_project / "empty.md"
        prd_path.write_text("")

        content = load_prd_file(str(prd_path))

        assert content == ""

    def test_load_unicode_content(self, temp_project: Path) -> None:
        """Test loading file with unicode content."""
        prd_path = temp_project / "unicode.md"
        prd_path.write_text("Café résumé naïve 日本語")

        content = load_prd_file(str(prd_path))

        assert "Café" in content
        assert "日本語" in content


class TestSyncPrd:
    """Tests for sync_prd function in prd_store module."""

    def test_sync_uses_existing_prd_when_no_sources(self, temp_project: Path) -> None:
        """Test that sync_prd returns existing PRD when no sources configured.

        This is the key behaviour: if .afk/prd.json exists with stories and
        no sources are configured, use the PRD directly without overwriting.
        """
        # Create PRD with stories
        (temp_project / ".afk").mkdir(parents=True)
        prd = PrdDocument(
            project="test",
            user_stories=[
                UserStory(
                    id="task-1",
                    title="Test task",
                    description="A test task",
                    acceptance_criteria=["It works"],
                    priority=1,
                )
            ],
        )
        save_prd(prd)

        # Config with no sources
        config = AfkConfig(sources=[])

        result = sync_prd(config)

        # Should return the existing PRD, not an empty one
        assert len(result.user_stories) == 1
        assert result.user_stories[0].id == "task-1"
        assert result.project == "test"

    def test_sync_does_not_overwrite_existing_prd(self, temp_project: Path) -> None:
        """Test that sync_prd doesn't overwrite PRD when no sources configured."""
        # Create PRD with stories
        (temp_project / ".afk").mkdir(parents=True)
        original_prd = PrdDocument(
            project="my-project",
            description="Original description",
            user_stories=[
                UserStory(
                    id="original-task",
                    title="Original",
                    description="Original task",
                    priority=1,
                )
            ],
        )
        save_prd(original_prd)

        # Sync with no sources
        config = AfkConfig(sources=[])
        sync_prd(config)

        # Reload and check it wasn't modified
        reloaded = load_prd()
        assert len(reloaded.user_stories) == 1
        assert reloaded.user_stories[0].id == "original-task"
        assert reloaded.project == "my-project"

    def test_sync_overwrites_when_sources_configured(self, temp_project: Path) -> None:
        """Test that sync_prd does sync when sources are configured."""
        from afk.config import SourceConfig

        # Create PRD with stories
        (temp_project / ".afk").mkdir(parents=True)
        original_prd = PrdDocument(
            project="old-project",
            user_stories=[
                UserStory(
                    id="old-task",
                    title="Old",
                    description="Old task",
                    priority=1,
                )
            ],
        )
        save_prd(original_prd)

        # Create a TODO.md source
        (temp_project / "TODO.md").write_text("- [ ] New task from markdown\n")

        # Sync with markdown source
        config = AfkConfig(sources=[SourceConfig(type="markdown", path="TODO.md")])
        result = sync_prd(config)

        # Should have synced from markdown, not the old PRD
        assert len(result.user_stories) >= 1
        # The old task should be gone (replaced by sync)
        task_ids = [s.id for s in result.user_stories]
        assert "old-task" not in task_ids

    def test_sync_preserves_prd_when_sources_return_empty(self, temp_project: Path) -> None:
        """Test that sync_prd doesn't wipe PRD when sources return nothing.

        This protects against the case where user has a PRD but sources
        (e.g., beads) are empty - we shouldn't lose their work.
        """
        from afk.config import SourceConfig

        # Create PRD with stories
        (temp_project / ".afk").mkdir(parents=True)
        original_prd = PrdDocument(
            project="my-project",
            user_stories=[
                UserStory(
                    id="important-task",
                    title="Important",
                    description="Important task",
                    priority=1,
                )
            ],
        )
        save_prd(original_prd)

        # Create an empty TODO.md source (no tasks)
        (temp_project / "TODO.md").write_text("# Nothing here\n\nNo tasks.\n")

        # Sync with empty source
        config = AfkConfig(sources=[SourceConfig(type="markdown", path="TODO.md")])
        result = sync_prd(config)

        # Should preserve the existing PRD, not wipe it
        assert len(result.user_stories) == 1
        assert result.user_stories[0].id == "important-task"
        assert result.project == "my-project"


class TestMarkStoryComplete:
    """Tests for mark_story_complete function."""

    def test_marks_story_complete(self, temp_project: Path) -> None:
        """Test marking a story as complete."""
        from afk.prd_store import mark_story_complete

        (temp_project / ".afk").mkdir(parents=True)
        prd = PrdDocument(
            user_stories=[
                UserStory(id="task-1", title="Task", description="Do thing", passes=False)
            ]
        )
        save_prd(prd)

        result = mark_story_complete("task-1")

        assert result is True
        reloaded = load_prd()
        assert reloaded.user_stories[0].passes is True

    def test_returns_false_for_unknown_story(self, temp_project: Path) -> None:
        """Test returns False when story not found."""
        from afk.prd_store import mark_story_complete

        (temp_project / ".afk").mkdir(parents=True)
        prd = PrdDocument(user_stories=[])
        save_prd(prd)

        result = mark_story_complete("nonexistent")

        assert result is False

    def test_closes_beads_issue_when_source_is_beads(self, temp_project: Path) -> None:
        """Test that beads issues are closed when story is from beads."""
        from unittest.mock import patch

        from afk.prd_store import mark_story_complete

        (temp_project / ".afk").mkdir(parents=True)
        prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="beads-123",
                    title="Beads Task",
                    description="From beads",
                    source="beads",
                    passes=False,
                )
            ]
        )
        save_prd(prd)

        with patch("afk.sources.beads.close_beads_issue") as mock_close:
            mock_close.return_value = True
            result = mark_story_complete("beads-123")

            assert result is True
            mock_close.assert_called_once_with("beads-123")

    def test_does_not_close_beads_for_other_sources(self, temp_project: Path) -> None:
        """Test that non-beads sources don't trigger beads close."""
        from unittest.mock import patch

        from afk.prd_store import mark_story_complete

        (temp_project / ".afk").mkdir(parents=True)
        prd = PrdDocument(
            user_stories=[
                UserStory(
                    id="json-task",
                    title="JSON Task",
                    description="From JSON",
                    source="json:prd.json",
                    passes=False,
                )
            ]
        )
        save_prd(prd)

        with patch("afk.sources.beads.close_beads_issue") as mock_close:
            result = mark_story_complete("json-task")

            assert result is True
            mock_close.assert_not_called()
