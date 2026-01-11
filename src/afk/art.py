"""ASCII art module with animated spinner definitions.

Provides spinner frame sequences and helper functions for terminal animations.
"""

from __future__ import annotations

# Spinner frame sequences for different animation styles
SPINNERS: dict[str, list[str]] = {
    "dots": ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    "arrows": ["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
    "bounce": ["⠁", "⠂", "⠄", "⠂"],
    "pulse": ["◯", "◎", "●", "◎"],
}


def get_spinner_frame(name: str, index: int) -> str:
    """Get a spinner frame by name and index.

    The index wraps around the frame sequence, so you can increment
    indefinitely and the frames will cycle.

    Args:
        name: Name of the spinner (dots, arrows, bounce, pulse).
        index: Frame index, will wrap around frame count.

    Returns:
        The spinner frame character(s) at the given index.
        Falls back to 'dots' spinner if name is unknown.
    """
    frames = SPINNERS.get(name, SPINNERS["dots"])
    return frames[index % len(frames)]
