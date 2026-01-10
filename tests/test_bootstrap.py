"""Tests for afk.bootstrap module."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from afk.bootstrap import (
    CONTEXT_FILES,
    STACKS,
    TASK_FILES,
    BootstrapResult,
    ProjectStack,
    _command_exists,
    _detect_ai_cli,
    _detect_context_files,
    _detect_sources,
    _detect_stack,
    _detect_tools,
    _is_github_repo,
    analyse_project,
    generate_config,
)
from afk.config import FeedbackLoopsConfig


class TestProjectStack:
    """Tests for ProjectStack dataclass."""

    def test_creation(self) -> None:
        """Test creating a ProjectStack."""
        loops = FeedbackLoopsConfig(lint="ruff check .")
        stack = ProjectStack(
            name="Python",
            config_file="pyproject.toml",
            feedback_loops=loops,
        )
        assert stack.name == "Python"
        assert stack.config_file == "pyproject.toml"
        assert stack.feedback_loops.lint == "ruff check ."


class TestBootstrapResult:
    """Tests for BootstrapResult dataclass."""

    def test_defaults(self) -> None:
        """Test default values."""
        result = BootstrapResult()
        assert result.stack is None
        assert result.sources == []
        assert result.context_files == []
        assert result.available_tools == {}
        assert result.warnings == []


class TestStackDefinitions:
    """Tests for stack definitions."""

    def test_stacks_defined(self) -> None:
        """Test that common stacks are defined."""
        assert "pyproject.toml" in STACKS
        assert "package.json" in STACKS
        assert "Cargo.toml" in STACKS
        assert "go.mod" in STACKS

    def test_python_stack_feedback_loops(self) -> None:
        """Test Python stack has expected feedback loops."""
        name, loops = STACKS["pyproject.toml"]
        assert name == "Python"
        assert loops.lint == "ruff check ."
        assert loops.types == "mypy ."
        assert loops.test == "pytest"


class TestContextFiles:
    """Tests for context file definitions."""

    def test_context_files_defined(self) -> None:
        """Test that common context files are defined."""
        assert "AGENTS.md" in CONTEXT_FILES
        assert "README.md" in CONTEXT_FILES
        assert "CONTRIBUTING.md" in CONTEXT_FILES


class TestTaskFiles:
    """Tests for task file definitions."""

    def test_task_files_defined(self) -> None:
        """Test that task files are defined."""
        assert "TODO.md" in TASK_FILES
        assert TASK_FILES["TODO.md"] == "markdown"
        assert "prd.json" in TASK_FILES
        assert TASK_FILES["prd.json"] == "json"


class TestCommandExists:
    """Tests for _command_exists function."""

    def test_existing_command(self) -> None:
        """Test with a command that exists."""
        # Python should always exist in test environment
        assert _command_exists("python") is True

    def test_nonexistent_command(self) -> None:
        """Test with a command that doesn't exist."""
        assert _command_exists("definitely-not-a-real-command") is False


class TestDetectStack:
    """Tests for _detect_stack function."""

    def test_python_project(self, python_project: Path) -> None:
        """Test detecting Python project."""
        stack = _detect_stack(python_project)
        assert stack is not None
        assert stack.name == "Python"
        assert stack.config_file == "pyproject.toml"

    def test_node_project(self, node_project: Path) -> None:
        """Test detecting Node.js project."""
        stack = _detect_stack(node_project)
        assert stack is not None
        assert stack.name == "Node.js"

    def test_no_stack(self, temp_project: Path) -> None:
        """Test when no stack is detected."""
        stack = _detect_stack(temp_project)
        assert stack is None

    def test_rust_project(self, temp_project: Path) -> None:
        """Test detecting Rust project."""
        (temp_project / "Cargo.toml").write_text('[package]\nname = "test"\n')
        stack = _detect_stack(temp_project)
        assert stack is not None
        assert stack.name == "Rust"

    def test_go_project(self, temp_project: Path) -> None:
        """Test detecting Go project."""
        (temp_project / "go.mod").write_text("module example.com/test\n")
        stack = _detect_stack(temp_project)
        assert stack is not None
        assert stack.name == "Go"


class TestDetectSources:
    """Tests for _detect_sources function."""

    def test_no_sources(self, temp_project: Path) -> None:
        """Test when no sources are found."""
        sources = _detect_sources(temp_project, {})
        assert sources == []

    def test_beads_source(self, temp_project: Path) -> None:
        """Test detecting beads source."""
        (temp_project / ".beads").mkdir()
        sources = _detect_sources(temp_project, {"bd": True})
        assert len(sources) == 1
        assert sources[0].type == "beads"

    def test_beads_source_requires_bd(self, temp_project: Path) -> None:
        """Test beads source requires bd command."""
        (temp_project / ".beads").mkdir()
        sources = _detect_sources(temp_project, {"bd": False})
        assert len(sources) == 0

    def test_markdown_source(self, temp_project: Path) -> None:
        """Test detecting markdown task file."""
        (temp_project / "TODO.md").write_text("- [ ] Task\n")
        sources = _detect_sources(temp_project, {})
        assert len(sources) == 1
        assert sources[0].type == "markdown"
        assert sources[0].path == "TODO.md"

    def test_json_source(self, temp_project: Path) -> None:
        """Test detecting JSON task file."""
        (temp_project / "tasks.json").write_text("[]")
        sources = _detect_sources(temp_project, {})
        assert len(sources) == 1
        assert sources[0].type == "json"
        assert sources[0].path == "tasks.json"

    def test_prd_glob(self, temp_project: Path) -> None:
        """Test detecting *.prd.json files."""
        (temp_project / "feature.prd.json").write_text("[]")
        sources = _detect_sources(temp_project, {})
        assert len(sources) == 1
        assert sources[0].type == "json"
        assert sources[0].path == "feature.prd.json"

    def test_multiple_sources(self, temp_project: Path) -> None:
        """Test detecting multiple sources."""
        (temp_project / ".beads").mkdir()
        (temp_project / "TODO.md").write_text("- [ ] Task\n")
        (temp_project / "prd.json").write_text("[]")
        sources = _detect_sources(temp_project, {"bd": True})
        assert len(sources) == 3


class TestDetectContextFiles:
    """Tests for _detect_context_files function."""

    def test_no_context_files(self, temp_project: Path) -> None:
        """Test when no context files exist."""
        files = _detect_context_files(temp_project)
        assert files == []

    def test_agents_md(self, temp_project: Path) -> None:
        """Test detecting AGENTS.md."""
        (temp_project / "AGENTS.md").write_text("# Agents\n")
        files = _detect_context_files(temp_project)
        assert "AGENTS.md" in files

    def test_multiple_context_files(self, python_project: Path) -> None:
        """Test detecting multiple context files."""
        files = _detect_context_files(python_project)
        assert "README.md" in files
        assert "AGENTS.md" in files


class TestDetectTools:
    """Tests for _detect_tools function."""

    def test_detects_tools(self) -> None:
        """Test that tool detection returns dict."""
        tools = _detect_tools()
        assert isinstance(tools, dict)
        assert "bd" in tools
        assert "gh" in tools
        assert "claude" in tools
        assert "cursor" in tools
        assert "aider" in tools


class TestDetectAiCli:
    """Tests for _detect_ai_cli function."""

    def test_claude_available(self) -> None:
        """Test when claude is available."""
        config = _detect_ai_cli({"claude": True, "aider": False})
        assert config.command == "claude"
        assert config.args == ["-p"]

    def test_aider_fallback(self) -> None:
        """Test falling back to aider."""
        config = _detect_ai_cli({"claude": False, "aider": True})
        assert config.command == "aider"
        assert config.args == ["--message"]

    def test_default(self) -> None:
        """Test default when nothing available."""
        config = _detect_ai_cli({"claude": False, "aider": False})
        assert config.command == "claude"  # Still defaults to claude


class TestIsGithubRepo:
    """Tests for _is_github_repo function."""

    def test_not_git_repo(self, temp_project: Path) -> None:
        """Test when not a git repo."""
        assert _is_github_repo(temp_project) is False

    def test_github_repo(self, temp_project: Path) -> None:
        """Test when it's a GitHub repo."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0, stdout="git@github.com:owner/repo.git\n"
            )
            assert _is_github_repo(temp_project) is True

    def test_non_github_repo(self, temp_project: Path) -> None:
        """Test when it's a non-GitHub repo."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0, stdout="git@gitlab.com:owner/repo.git\n"
            )
            assert _is_github_repo(temp_project) is False

    def test_git_error(self, temp_project: Path) -> None:
        """Test when git command fails."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.SubprocessError()
            assert _is_github_repo(temp_project) is False


class TestAnalyseProject:
    """Tests for analyse_project function."""

    def test_python_project(self, python_project: Path) -> None:
        """Test analysing Python project."""
        result = analyse_project(python_project)
        assert result.stack is not None
        assert result.stack.name == "Python"
        assert "README.md" in result.context_files
        assert "AGENTS.md" in result.context_files

    def test_empty_project(self, temp_project: Path) -> None:
        """Test analysing empty project."""
        result = analyse_project(temp_project)
        assert result.stack is None
        assert len(result.warnings) >= 2  # No stack, no sources, maybe no context

    def test_default_root(self) -> None:
        """Test using current directory as default."""
        result = analyse_project()
        assert isinstance(result, BootstrapResult)

    def test_warns_on_missing_sources(self, python_project: Path) -> None:
        """Test warning when no sources detected."""
        result = analyse_project(python_project)
        assert any("No task sources" in w for w in result.warnings)


class TestGenerateConfig:
    """Tests for generate_config function."""

    def test_from_python_project(self, python_project: Path) -> None:
        """Test generating config from Python project analysis."""
        result = analyse_project(python_project)
        config = generate_config(result)

        assert config.feedback_loops.lint == "ruff check ."
        assert config.feedback_loops.test == "pytest"
        assert config.output.default == "clipboard"
        assert "AGENTS.md" in config.prompt.context_files

    def test_no_stack(self, temp_project: Path) -> None:
        """Test generating config when no stack detected."""
        result = analyse_project(temp_project)
        config = generate_config(result)

        assert config.feedback_loops.lint is None
        assert config.feedback_loops.test is None

    def test_sources_preserved(self, temp_project: Path) -> None:
        """Test that detected sources are preserved."""
        (temp_project / "TODO.md").write_text("- [ ] Task\n")
        result = analyse_project(temp_project)
        config = generate_config(result)

        assert len(config.sources) == 1
        assert config.sources[0].type == "markdown"
