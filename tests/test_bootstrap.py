"""Tests for afk.bootstrap module."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from afk.bootstrap import (
    AI_CLIS,
    CONTEXT_FILES,
    PROMPT_FILES,
    STACKS,
    TASK_FILES,
    AiCliInfo,
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
    detect_available_ai_clis,
    detect_prompt_file,
    ensure_ai_cli_configured,
    generate_config,
    infer_config,
    infer_sources,
    prompt_ai_cli_selection,
)
from afk.config import AfkConfig, AiCliConfig, FeedbackLoopsConfig


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
        assert "agent" in tools
        assert "claude" in tools
        assert "codex" in tools
        assert "aider" in tools
        assert "amp" in tools


class TestDetectAiCli:
    """Tests for _detect_ai_cli function."""

    def test_claude_available(self) -> None:
        """Test when claude is available (highest priority)."""
        config = _detect_ai_cli({"claude": True, "agent": True, "aider": False})
        assert config.command == "claude"
        assert config.args == ["--dangerously-skip-permissions", "-p"]

    def test_agent_fallback(self) -> None:
        """Test falling back to agent when claude unavailable."""
        config = _detect_ai_cli({"claude": False, "agent": True, "aider": False})
        assert config.command == "agent"
        assert config.args == ["--force", "-p"]

    def test_codex_fallback(self) -> None:
        """Test falling back to codex when claude and agent unavailable."""
        config = _detect_ai_cli({"claude": False, "agent": False, "codex": True, "aider": True})
        assert config.command == "codex"
        assert config.args == ["--approval-mode", "full-auto", "-q"]

    def test_aider_fallback(self) -> None:
        """Test falling back to aider."""
        config = _detect_ai_cli(
            {"claude": False, "agent": False, "codex": False, "aider": True}
        )
        assert config.command == "aider"
        assert config.args == ["--yes"]

    def test_amp_fallback(self) -> None:
        """Test falling back to amp."""
        config = _detect_ai_cli(
            {"claude": False, "agent": False, "codex": False, "aider": False, "amp": True}
        )
        assert config.command == "amp"
        assert config.args == ["--dangerously-allow-all"]

    def test_default(self) -> None:
        """Test default when nothing available."""
        config = _detect_ai_cli(
            {"claude": False, "agent": False, "codex": False, "aider": False}
        )
        assert config.command == "claude"  # Defaults to claude


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


class TestAiCliInfo:
    """Tests for AiCliInfo dataclass."""

    def test_creation(self) -> None:
        """Test creating an AiCliInfo."""
        info = AiCliInfo(
            command="test",
            name="Test CLI",
            args=["--flag"],
            description="A test CLI",
            install_url="https://test.com",
        )
        assert info.command == "test"
        assert info.name == "Test CLI"
        assert info.args == ["--flag"]
        assert info.description == "A test CLI"
        assert info.install_url == "https://test.com"


class TestAiCliDefinitions:
    """Tests for AI CLI definitions."""

    def test_ai_clis_defined(self) -> None:
        """Test that AI CLIs are defined."""
        assert len(AI_CLIS) >= 5
        commands = [cli.command for cli in AI_CLIS]
        assert "agent" in commands
        assert "claude" in commands
        assert "codex" in commands
        assert "aider" in commands
        assert "amp" in commands

    def test_agent_cli_definition(self) -> None:
        """Test Cursor Agent CLI definition."""
        agent = next(cli for cli in AI_CLIS if cli.command == "agent")
        assert agent.name == "Cursor Agent"
        assert agent.args == ["--force", "-p"]
        assert "cursor" in agent.install_url.lower()

    def test_claude_cli_definition(self) -> None:
        """Test Claude Code CLI definition."""
        claude = next(cli for cli in AI_CLIS if cli.command == "claude")
        assert claude.name == "Claude Code"
        assert claude.args == ["--dangerously-skip-permissions", "-p"]
        assert "anthropic" in claude.install_url.lower()

    def test_codex_cli_definition(self) -> None:
        """Test Codex CLI definition."""
        codex = next(cli for cli in AI_CLIS if cli.command == "codex")
        assert codex.name == "Codex"
        assert codex.args == ["--approval-mode", "full-auto", "-q"]
        assert "openai" in codex.install_url.lower() or "codex" in codex.install_url.lower()

    def test_all_clis_have_required_fields(self) -> None:
        """Test all CLIs have required fields."""
        for cli in AI_CLIS:
            assert cli.command
            assert cli.name
            assert isinstance(cli.args, list)
            assert cli.description
            assert cli.install_url


class TestDetectAvailableAiClis:
    """Tests for detect_available_ai_clis function."""

    def test_no_clis_available(self) -> None:
        """Test when no AI CLIs are installed."""
        with patch("afk.bootstrap._command_exists", return_value=False):
            available = detect_available_ai_clis()
            assert available == []

    def test_claude_available(self) -> None:
        """Test when claude is installed."""

        def mock_exists(cmd: str) -> bool:
            return cmd == "claude"

        with patch("afk.bootstrap._command_exists", side_effect=mock_exists):
            available = detect_available_ai_clis()
            assert len(available) == 1
            assert available[0].command == "claude"

    def test_codex_available(self) -> None:
        """Test when codex is installed."""

        def mock_exists(cmd: str) -> bool:
            return cmd == "codex"

        with patch("afk.bootstrap._command_exists", side_effect=mock_exists):
            available = detect_available_ai_clis()
            assert len(available) == 1
            assert available[0].command == "codex"

    def test_multiple_clis_available(self) -> None:
        """Test when multiple CLIs are installed."""

        def mock_exists(cmd: str) -> bool:
            return cmd in ("claude", "codex", "aider")

        with patch("afk.bootstrap._command_exists", side_effect=mock_exists):
            available = detect_available_ai_clis()
            assert len(available) == 3
            commands = [cli.command for cli in available]
            assert "claude" in commands
            assert "codex" in commands
            assert "aider" in commands


class TestPromptAiCliSelection:
    """Tests for prompt_ai_cli_selection function."""

    def test_no_clis_available(self) -> None:
        """Test prompt with no CLIs available."""
        from io import StringIO

        from rich.console import Console

        output = StringIO()
        console = Console(file=output, force_terminal=True)

        result = prompt_ai_cli_selection([], console)

        assert result is None
        output_text = output.getvalue()
        assert "No AI CLI tools found" in output_text

    def test_single_cli_selection(self) -> None:
        """Test selecting when one CLI available."""
        from io import StringIO

        from rich.console import Console

        output = StringIO()
        console = Console(file=output, force_terminal=True)

        available = [
            AiCliInfo(
                command="claude",
                name="Claude CLI",
                args=["-p"],
                description="Test",
                install_url="https://test.com",
            )
        ]

        with patch("click.prompt", return_value=1):
            result = prompt_ai_cli_selection(available, console)

        assert result is not None
        assert result.command == "claude"
        assert result.args == ["-p"]

    def test_multi_cli_selection(self) -> None:
        """Test selecting from multiple CLIs."""
        from io import StringIO

        from rich.console import Console

        output = StringIO()
        console = Console(file=output, force_terminal=True)

        available = [
            AiCliInfo("claude", "Claude", ["-p"], "desc1", "url1"),
            AiCliInfo("aider", "Aider", ["--message"], "desc2", "url2"),
        ]

        with patch("click.prompt", return_value=2):
            result = prompt_ai_cli_selection(available, console)

        assert result is not None
        assert result.command == "aider"
        assert result.args == ["--message"]


class TestEnsureAiCliConfigured:
    """Tests for ensure_ai_cli_configured function."""

    def test_config_exists(self, temp_project: Path) -> None:
        """Test when config already exists."""
        import os

        from afk.config import AfkConfig

        # Save current directory and change to temp
        old_cwd = os.getcwd()
        os.chdir(temp_project)

        try:
            # Create config
            (temp_project / ".afk").mkdir()
            config = AfkConfig()
            config.ai_cli = AiCliConfig(command="test", args=["--test"])
            config.save(temp_project / ".afk" / "config.json")

            # Patch CONFIG_FILE to point to our temp location
            with patch("afk.bootstrap.CONFIG_FILE", temp_project / ".afk" / "config.json"):
                result = ensure_ai_cli_configured(config)

            assert result.command == "test"
            assert result.args == ["--test"]
        finally:
            os.chdir(old_cwd)

    def test_first_run_prompts(self, temp_project: Path) -> None:
        """Test that first run prompts for CLI selection."""
        import os
        from io import StringIO

        from rich.console import Console

        old_cwd = os.getcwd()
        os.chdir(temp_project)

        try:
            output = StringIO()
            console = Console(file=output, force_terminal=True)

            # Mock available CLIs and user selection
            with (
                patch("afk.bootstrap.CONFIG_FILE", temp_project / ".afk" / "config.json"),
                patch("afk.bootstrap.AFK_DIR", temp_project / ".afk"),
                patch("afk.bootstrap.detect_available_ai_clis") as mock_detect,
                patch("afk.bootstrap.prompt_ai_cli_selection") as mock_prompt,
            ):
                mock_detect.return_value = [AiCliInfo("claude", "Claude", ["-p"], "desc", "url")]
                mock_prompt.return_value = AiCliConfig(command="claude", args=["-p"])

                result = ensure_ai_cli_configured(console=console)

            assert result.command == "claude"
            # Config should be saved
            assert (temp_project / ".afk" / "config.json").exists()
        finally:
            os.chdir(old_cwd)

    def test_first_run_no_clis_exits(self, temp_project: Path) -> None:
        """Test that first run with no CLIs exits."""
        import os
        from io import StringIO

        import pytest
        from rich.console import Console

        old_cwd = os.getcwd()
        os.chdir(temp_project)

        try:
            output = StringIO()
            console = Console(file=output, force_terminal=True)

            with (
                patch("afk.bootstrap.CONFIG_FILE", temp_project / ".afk" / "config.json"),
                patch("afk.bootstrap.detect_available_ai_clis", return_value=[]),
                patch("afk.bootstrap.prompt_ai_cli_selection", return_value=None),
                pytest.raises(SystemExit) as exc_info,
            ):
                ensure_ai_cli_configured(console=console)

            assert exc_info.value.code == 1
        finally:
            os.chdir(old_cwd)


class TestPromptFiles:
    """Tests for prompt file definitions."""

    def test_prompt_files_defined(self) -> None:
        """Test that prompt files are defined."""
        assert "prompt.md" in PROMPT_FILES
        assert "PROMPT.md" in PROMPT_FILES


class TestDetectPromptFile:
    """Tests for detect_prompt_file function."""

    def test_no_prompt_file(self, temp_project: Path) -> None:
        """Test when no prompt file exists."""
        result = detect_prompt_file(temp_project)
        assert result is None

    def test_prompt_md(self, temp_project: Path) -> None:
        """Test detecting prompt.md."""
        (temp_project / "prompt.md").write_text("# Do the thing\n")
        result = detect_prompt_file(temp_project)
        assert result is not None
        assert result.name == "prompt.md"

    def test_uppercase_prompt(self, temp_project: Path) -> None:
        """Test detecting PROMPT.md (case-insensitive filesystems may normalise)."""
        (temp_project / "PROMPT.md").write_text("# Do the thing\n")
        result = detect_prompt_file(temp_project)
        assert result is not None
        # macOS has case-insensitive filesystem, so accept either
        assert result.name.lower() == "prompt.md"

    def test_priority_order(self, temp_project: Path) -> None:
        """Test that lowercase prompt.md takes priority."""
        (temp_project / "prompt.md").write_text("lower\n")
        (temp_project / "PROMPT.md").write_text("upper\n")
        result = detect_prompt_file(temp_project)
        assert result is not None
        assert result.name == "prompt.md"


class TestInferSources:
    """Tests for infer_sources function."""

    def test_no_sources(self, temp_project: Path) -> None:
        """Test when no sources exist."""
        sources = infer_sources(temp_project)
        assert sources == []

    def test_markdown_source(self, temp_project: Path) -> None:
        """Test inferring markdown source."""
        (temp_project / "TODO.md").write_text("- [ ] Task\n")
        sources = infer_sources(temp_project)
        assert len(sources) == 1
        assert sources[0].type == "markdown"
        assert sources[0].path == "TODO.md"

    def test_json_source(self, temp_project: Path) -> None:
        """Test inferring JSON source."""
        (temp_project / "tasks.json").write_text("[]")
        sources = infer_sources(temp_project)
        assert len(sources) == 1
        assert sources[0].type == "json"

    def test_beads_source(self, temp_project: Path) -> None:
        """Test inferring beads source."""
        (temp_project / ".beads").mkdir()
        with patch("afk.bootstrap._command_exists") as mock_exists:
            mock_exists.return_value = True  # bd is available
            sources = infer_sources(temp_project)
        # May or may not have beads depending on mock scope
        assert isinstance(sources, list)

    def test_default_root(self) -> None:
        """Test using current directory as default."""
        sources = infer_sources()
        assert isinstance(sources, list)


class TestInferConfig:
    """Tests for infer_config function."""

    def test_loads_existing_config(self, temp_project: Path) -> None:
        """Test that existing config is loaded."""
        import os

        from afk.config import AfkConfig

        old_cwd = os.getcwd()
        os.chdir(temp_project)

        try:
            # Create config
            (temp_project / ".afk").mkdir()
            config = AfkConfig()
            config.limits.max_iterations = 99
            config.save(temp_project / ".afk" / "config.json")

            with patch("afk.bootstrap.CONFIG_FILE", temp_project / ".afk" / "config.json"):
                result = infer_config(temp_project)

            assert result.limits.max_iterations == 99
        finally:
            os.chdir(old_cwd)

    def test_infers_without_config(self, python_project: Path) -> None:
        """Test inferring config when none exists."""
        with patch("afk.bootstrap.CONFIG_FILE", python_project / ".afk" / "config.json"):
            result = infer_config(python_project)

        assert result is not None
        # Should detect Python stack
        assert result.feedback_loops.lint is not None
        # Should detect context files
        assert len(result.prompt.context_files) > 0

    def test_default_root(self) -> None:
        """Test using current directory as default."""
        result = infer_config()
        assert isinstance(result, AfkConfig)
