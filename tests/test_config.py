"""Tests for afk.config module."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from afk.config import (
    AFK_DIR,
    CONFIG_FILE,
    AfkConfig,
    AiCliConfig,
    FeedbackLoopsConfig,
    LimitsConfig,
    OutputConfig,
    PromptConfig,
    SourceConfig,
)


class TestSourceConfig:
    """Tests for SourceConfig model."""

    def test_beads_source(self) -> None:
        """Test beads source configuration."""
        source = SourceConfig(type="beads")
        assert source.type == "beads"
        assert source.path is None
        assert source.repo is None
        assert source.labels == []

    def test_json_source_with_path(self) -> None:
        """Test JSON source with path."""
        source = SourceConfig(type="json", path="tasks.json")
        assert source.type == "json"
        assert source.path == "tasks.json"

    def test_github_source_with_options(self) -> None:
        """Test GitHub source with repo and labels."""
        source = SourceConfig(
            type="github",
            repo="owner/repo",
            labels=["bug", "enhancement"],
        )
        assert source.type == "github"
        assert source.repo == "owner/repo"
        assert source.labels == ["bug", "enhancement"]

    def test_invalid_source_type(self) -> None:
        """Test that invalid source types are rejected."""
        with pytest.raises(Exception):
            SourceConfig(type="invalid")  # type: ignore[arg-type]


class TestFeedbackLoopsConfig:
    """Tests for FeedbackLoopsConfig model."""

    def test_defaults(self) -> None:
        """Test default values."""
        config = FeedbackLoopsConfig()
        assert config.types is None
        assert config.lint is None
        assert config.test is None
        assert config.build is None
        assert config.custom == {}

    def test_all_fields(self) -> None:
        """Test all fields populated."""
        config = FeedbackLoopsConfig(
            types="mypy .",
            lint="ruff check .",
            test="pytest",
            build="pip wheel .",
            custom={"format": "ruff format ."},
        )
        assert config.types == "mypy ."
        assert config.lint == "ruff check ."
        assert config.test == "pytest"
        assert config.build == "pip wheel ."
        assert config.custom == {"format": "ruff format ."}


class TestLimitsConfig:
    """Tests for LimitsConfig model."""

    def test_defaults(self) -> None:
        """Test default values."""
        config = LimitsConfig()
        assert config.max_iterations == 20
        assert config.max_task_failures == 3
        assert config.timeout_minutes == 120

    def test_custom_values(self) -> None:
        """Test custom limit values."""
        config = LimitsConfig(
            max_iterations=5,
            max_task_failures=1,
            timeout_minutes=30,
        )
        assert config.max_iterations == 5
        assert config.max_task_failures == 1
        assert config.timeout_minutes == 30


class TestOutputConfig:
    """Tests for OutputConfig model."""

    def test_defaults(self) -> None:
        """Test default values."""
        config = OutputConfig()
        assert config.default == "stdout"
        assert config.file_path == ".afk/prompt.md"

    def test_clipboard_default(self) -> None:
        """Test clipboard as default."""
        config = OutputConfig(default="clipboard")
        assert config.default == "clipboard"


class TestAiCliConfig:
    """Tests for AiCliConfig model."""

    def test_defaults(self) -> None:
        """Test default values."""
        config = AiCliConfig()
        assert config.command == "claude"
        assert config.args == ["-p"]

    def test_aider_config(self) -> None:
        """Test aider configuration."""
        config = AiCliConfig(command="aider", args=["--message"])
        assert config.command == "aider"
        assert config.args == ["--message"]


class TestPromptConfig:
    """Tests for PromptConfig model."""

    def test_defaults(self) -> None:
        """Test default values."""
        config = PromptConfig()
        assert config.template == "default"
        assert config.custom_path is None
        assert config.context_files == []
        assert config.instructions == []

    def test_custom_config(self) -> None:
        """Test custom prompt configuration."""
        config = PromptConfig(
            template="minimal",
            custom_path=".afk/prompt.jinja2",
            context_files=["AGENTS.md", "README.md"],
            instructions=["Always run tests", "Use British English"],
        )
        assert config.template == "minimal"
        assert config.custom_path == ".afk/prompt.jinja2"
        assert config.context_files == ["AGENTS.md", "README.md"]
        assert config.instructions == ["Always run tests", "Use British English"]


class TestAfkConfig:
    """Tests for AfkConfig model."""

    def test_defaults(self) -> None:
        """Test default configuration."""
        config = AfkConfig()
        assert config.sources == []
        assert isinstance(config.feedback_loops, FeedbackLoopsConfig)
        assert isinstance(config.limits, LimitsConfig)
        assert isinstance(config.output, OutputConfig)
        assert isinstance(config.ai_cli, AiCliConfig)
        assert isinstance(config.prompt, PromptConfig)

    def test_load_missing_file(self, temp_project: Path) -> None:
        """Test loading returns defaults when file doesn't exist."""
        config = AfkConfig.load(temp_project / ".afk/config.json")
        assert config.sources == []
        assert config.limits.max_iterations == 20

    def test_load_existing_file(self, temp_afk_dir: Path, sample_config_data: dict) -> None:
        """Test loading from existing file."""
        config_path = temp_afk_dir / "config.json"
        with open(config_path, "w") as f:
            json.dump(sample_config_data, f)

        config = AfkConfig.load(config_path)
        assert len(config.sources) == 2
        assert config.sources[0].type == "beads"
        assert config.sources[1].type == "json"
        assert config.limits.max_iterations == 10

    def test_load_default_path(self, temp_afk_dir: Path, sample_config_data: dict) -> None:
        """Test loading from default .afk/config.json path."""
        config_path = temp_afk_dir / "config.json"
        with open(config_path, "w") as f:
            json.dump(sample_config_data, f)

        config = AfkConfig.load()
        assert len(config.sources) == 2

    def test_save_creates_directory(self, temp_project: Path) -> None:
        """Test save creates parent directory if needed."""
        config = AfkConfig(
            sources=[SourceConfig(type="beads")],
        )
        config_path = temp_project / ".afk" / "config.json"
        config.save(config_path)

        assert config_path.exists()
        with open(config_path) as f:
            data = json.load(f)
        assert data["sources"][0]["type"] == "beads"

    def test_save_default_path(self, temp_project: Path) -> None:
        """Test save to default path."""
        config = AfkConfig(
            sources=[SourceConfig(type="markdown", path="TODO.md")],
        )
        config.save()

        config_path = Path(".afk/config.json")
        assert config_path.exists()

    def test_round_trip(self, temp_afk_dir: Path) -> None:
        """Test save and load round-trip preserves data."""
        original = AfkConfig(
            sources=[
                SourceConfig(type="github", repo="owner/repo", labels=["bug"]),
            ],
            feedback_loops=FeedbackLoopsConfig(lint="ruff check ."),
            limits=LimitsConfig(max_iterations=15),
        )

        config_path = temp_afk_dir / "config.json"
        original.save(config_path)
        loaded = AfkConfig.load(config_path)

        assert loaded.sources[0].type == "github"
        assert loaded.sources[0].repo == "owner/repo"
        assert loaded.sources[0].labels == ["bug"]
        assert loaded.feedback_loops.lint == "ruff check ."
        assert loaded.limits.max_iterations == 15


class TestModulePaths:
    """Tests for module-level path constants."""

    def test_afk_dir(self) -> None:
        """Test AFK_DIR constant."""
        assert AFK_DIR == Path(".afk")

    def test_config_file(self) -> None:
        """Test CONFIG_FILE constant."""
        assert CONFIG_FILE == Path(".afk/config.json")
