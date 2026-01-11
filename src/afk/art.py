"""ASCII art module with animated spinner definitions.

Provides spinner frame sequences, mascot states, and helper functions
for terminal animations.
"""

from __future__ import annotations

# Spinner frame sequences for different animation styles
SPINNERS: dict[str, list[str]] = {
    "dots": ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    "arrows": ["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
    "bounce": ["⠁", "⠂", "⠄", "⠂"],
    "pulse": ["◯", "◎", "●", "◎"],
}

# ASCII mascot character definitions for different states
MASCOT_STATES: dict[str, str] = {
    "working": r"""
    ( o_o)
    /|   |\
     |   |
    / \  / \
    """.strip(),
    "success": r"""
    \(^o^)/
     |   |
     |   |
    / \ / \
    """.strip(),
    "error": r"""
    (x_x)
    /|   |\
     |   |
    _|   |_
    """.strip(),
    "waiting": r"""
    (._.)
    /|   |\
     |   |
    / \ / \
    """.strip(),
    "celebration": r"""
  * \(^o^)/ *
 *   |   |   *
      |   |
     / \ / \
    """.strip(),
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


def get_mascot(state: str) -> str:
    """Get the mascot ASCII art for a given state.

    Args:
        state: The mascot state (working, success, error, waiting, celebration).

    Returns:
        The ASCII art string for the given state.
        Falls back to 'working' state if state is unknown.
    """
    return MASCOT_STATES.get(state, MASCOT_STATES["working"])
