"""Tests for ASCII art spinners and mascots."""

from afk.art import MASCOT_STATES, SPINNERS, get_mascot, get_spinner_frame


class TestSpinners:
    """Tests for spinner definitions and frame cycling."""

    def test_spinners_dict_contains_required_spinners(self) -> None:
        """SPINNERS dict should contain dots, arrows, bounce, pulse."""
        required_spinners = ["dots", "arrows", "bounce", "pulse"]
        for name in required_spinners:
            assert name in SPINNERS, f"Missing spinner: {name}"

    def test_spinners_have_frames(self) -> None:
        """Each spinner should have at least 2 frames."""
        for name, frames in SPINNERS.items():
            assert len(frames) >= 2, f"Spinner {name} has fewer than 2 frames"

    def test_spinner_frames_are_strings(self) -> None:
        """All spinner frames should be strings."""
        for name, frames in SPINNERS.items():
            for i, frame in enumerate(frames):
                assert isinstance(frame, str), f"Frame {i} of {name} is not a string"


class TestGetSpinnerFrame:
    """Tests for get_spinner_frame helper function."""

    def test_get_spinner_frame_returns_valid_frame(self) -> None:
        """get_spinner_frame should return a frame from the spinner."""
        frame = get_spinner_frame("dots", 0)
        assert frame in SPINNERS["dots"]

    def test_get_spinner_frame_cycles_correctly(self) -> None:
        """Index should wrap around when exceeding frame count."""
        frames = SPINNERS["dots"]
        num_frames = len(frames)

        # Test that wrapping works
        for i in range(num_frames * 2):
            frame = get_spinner_frame("dots", i)
            expected = frames[i % num_frames]
            assert frame == expected, f"Frame at index {i} should be {expected}"

    def test_get_spinner_frame_handles_negative_index(self) -> None:
        """Negative indices should also wrap correctly."""
        # Python's modulo handles negatives, so -1 should give last frame
        frame = get_spinner_frame("dots", -1)
        assert frame == SPINNERS["dots"][-1]

    def test_get_spinner_frame_unknown_spinner_returns_default(self) -> None:
        """Unknown spinner name should return a default frame."""
        frame = get_spinner_frame("nonexistent", 0)
        # Should fall back to dots spinner
        assert frame in SPINNERS["dots"]

    def test_get_spinner_frame_all_spinners_work(self) -> None:
        """All defined spinners should work with get_spinner_frame."""
        for name in SPINNERS:
            frame = get_spinner_frame(name, 0)
            assert frame == SPINNERS[name][0]
            # Also test cycling
            frame = get_spinner_frame(name, len(SPINNERS[name]))
            assert frame == SPINNERS[name][0]  # Should wrap


class TestMascotStates:
    """Tests for mascot state definitions."""

    def test_mascot_states_contains_required_states(self) -> None:
        """MASCOT_STATES should contain working, success, error, waiting."""
        required_states = ["working", "success", "error", "waiting", "celebration"]
        for state in required_states:
            assert state in MASCOT_STATES, f"Missing mascot state: {state}"

    def test_mascot_states_are_non_empty_strings(self) -> None:
        """Each mascot state should be a non-empty string."""
        for state, art in MASCOT_STATES.items():
            assert isinstance(art, str), f"Mascot {state} is not a string"
            assert len(art) > 0, f"Mascot {state} is empty"

    def test_mascot_states_contain_expected_characters(self) -> None:
        """Mascot art should contain expected face/body characters."""
        # Working should have neutral face
        assert "o_o" in MASCOT_STATES["working"]

        # Success should have happy face
        assert "^o^" in MASCOT_STATES["success"]

        # Error should have X eyes
        assert "x_x" in MASCOT_STATES["error"]

        # Waiting should have dot eyes
        assert "._." in MASCOT_STATES["waiting"]

        # Celebration should have happy face and stars
        assert "^o^" in MASCOT_STATES["celebration"]
        assert "*" in MASCOT_STATES["celebration"]


class TestGetMascot:
    """Tests for get_mascot helper function."""

    def test_get_mascot_returns_art_for_each_state(self) -> None:
        """get_mascot should return art for all defined states."""
        for state in MASCOT_STATES:
            art = get_mascot(state)
            assert art == MASCOT_STATES[state]

    def test_get_mascot_unknown_state_returns_default(self) -> None:
        """Unknown state should return working mascot as default."""
        art = get_mascot("nonexistent")
        assert art == MASCOT_STATES["working"]

    def test_get_mascot_art_is_multiline(self) -> None:
        """Mascot art should span multiple lines for terminal display."""
        for state in MASCOT_STATES:
            art = get_mascot(state)
            lines = art.split("\n")
            assert len(lines) >= 2, f"Mascot {state} should have multiple lines"

    def test_get_mascot_art_renders_in_terminal(self) -> None:
        """Mascot art should contain only printable ASCII characters."""
        import string

        # Allow printable ASCII, backslash, newlines, and common symbols
        allowed = set(string.printable)

        for state in MASCOT_STATES:
            art = get_mascot(state)
            for char in art:
                assert char in allowed, (
                    f"Mascot {state} contains non-printable char: {repr(char)}"
                )
