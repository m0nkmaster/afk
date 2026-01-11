"""Tests for ASCII art spinners and mascots."""

from afk.art import SPINNERS, get_spinner_frame


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
