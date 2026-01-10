"""Tests for afk.prd module."""

from __future__ import annotations

from pathlib import Path

import pytest

from afk.prd import generate_prd_prompt, load_prd_file


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
