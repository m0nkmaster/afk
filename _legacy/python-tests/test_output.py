"""Tests for afk.output module."""

from __future__ import annotations

from pathlib import Path
from unittest.mock import patch

from afk.config import AfkConfig, OutputConfig
from afk.output import (
    _copy_to_clipboard,
    _print_to_stdout,
    _write_to_file,
    output_prompt,
)


class TestOutputPrompt:
    """Tests for output_prompt function."""

    def test_clipboard_mode(self) -> None:
        """Test clipboard output mode."""
        with patch("afk.output._copy_to_clipboard") as mock_copy:
            config = AfkConfig()
            output_prompt("test prompt", mode="clipboard", config=config)
            mock_copy.assert_called_once_with("test prompt")

    def test_file_mode(self) -> None:
        """Test file output mode."""
        with patch("afk.output._write_to_file") as mock_write:
            config = AfkConfig(output=OutputConfig(file_path=".afk/out.md"))
            output_prompt("test prompt", mode="file", config=config)
            mock_write.assert_called_once_with("test prompt", ".afk/out.md")

    def test_stdout_mode(self) -> None:
        """Test stdout output mode."""
        with patch("afk.output._print_to_stdout") as mock_print:
            config = AfkConfig()
            output_prompt("test prompt", mode="stdout", config=config)
            mock_print.assert_called_once_with("test prompt")


class TestCopyToClipboard:
    """Tests for _copy_to_clipboard function."""

    def test_successful_copy(self) -> None:
        """Test successful copy to clipboard."""
        with patch("pyperclip.copy") as mock_copy:
            with patch("afk.output.console.print"):
                _copy_to_clipboard("test content")
                mock_copy.assert_called_once_with("test content")

    def test_fallback_on_error(self) -> None:
        """Test fallback to stdout on clipboard error."""
        with patch("pyperclip.copy") as mock_copy:
            mock_copy.side_effect = Exception("Clipboard error")
            with patch("afk.output._print_to_stdout") as mock_stdout:
                with patch("afk.output.console.print"):
                    _copy_to_clipboard("test content")
                    mock_stdout.assert_called_once_with("test content")


class TestWriteToFile:
    """Tests for _write_to_file function."""

    def test_writes_content(self, temp_project: Path) -> None:
        """Test content is written to file."""
        file_path = temp_project / "output.md"
        with patch("afk.output.console.print"):
            _write_to_file("test content", str(file_path))

        assert file_path.exists()
        assert file_path.read_text() == "test content"

    def test_creates_parent_directory(self, temp_project: Path) -> None:
        """Test parent directory is created if needed."""
        file_path = temp_project / "nested" / "dir" / "output.md"
        with patch("afk.output.console.print"):
            _write_to_file("test content", str(file_path))

        assert file_path.exists()

    def test_overwrites_existing_file(self, temp_project: Path) -> None:
        """Test existing file is overwritten."""
        file_path = temp_project / "output.md"
        file_path.write_text("old content")

        with patch("afk.output.console.print"):
            _write_to_file("new content", str(file_path))

        assert file_path.read_text() == "new content"


class TestPrintToStdout:
    """Tests for _print_to_stdout function."""

    def test_prints_content(self) -> None:
        """Test content is printed to console."""
        with patch("afk.output.console.print") as mock_print:
            _print_to_stdout("test content")
            mock_print.assert_called_once_with("test content")
