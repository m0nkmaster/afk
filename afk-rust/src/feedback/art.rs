//! ASCII art module with animated spinner definitions.
//!
//! Provides spinner frame sequences, mascot states, and helper functions
//! for terminal animations.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Spinner frame sequences for different animation styles.
pub static SPINNERS: LazyLock<HashMap<&'static str, Vec<&'static str>>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "dots",
        vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    );
    m.insert("arrows", vec!["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"]);
    m.insert("bounce", vec!["⠁", "⠂", "⠄", "⠂"]);
    m.insert("pulse", vec!["◯", "◎", "●", "◎"]);
    m
});

/// ASCII mascot character definitions for different states.
pub static MASCOT_STATES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "working",
        r#"    ( o_o)
    /|   |\
     |   |
    / \  / \"#,
    );
    m.insert(
        "success",
        r#"    \(^o^)/
     |   |
     |   |
    / \ / \"#,
    );
    m.insert(
        "error",
        r#"    (x_x)
    /|   |\
     |   |
    _|   |_"#,
    );
    m.insert(
        "waiting",
        r#"    (._.)
    /|   |\
     |   |
    / \ / \"#,
    );
    m.insert(
        "celebration",
        r#"  * \(^o^)/ *
 *   |   |   *
      |   |
     / \ / \"#,
    );
    m
});

/// Get a spinner frame by name and index.
///
/// The index wraps around the frame sequence, so you can increment
/// indefinitely and the frames will cycle.
///
/// # Arguments
///
/// * `name` - Name of the spinner (dots, arrows, bounce, pulse).
/// * `index` - Frame index, will wrap around frame count.
///
/// # Returns
///
/// The spinner frame character(s) at the given index.
/// Falls back to 'dots' spinner if name is unknown.
pub fn get_spinner_frame(name: &str, index: usize) -> &'static str {
    let frames = SPINNERS.get(name).or_else(|| SPINNERS.get("dots")).unwrap();
    frames[index % frames.len()]
}

/// Get mascot ASCII art for a given state.
///
/// # Arguments
///
/// * `state` - The mascot state (working, success, error, waiting, celebration).
///
/// # Returns
///
/// The ASCII art string for the given state.
/// Falls back to 'working' state if state is unknown.
pub fn get_mascot(state: &str) -> &'static str {
    MASCOT_STATES
        .get(state)
        .or_else(|| MASCOT_STATES.get("working"))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinners_exist() {
        assert!(SPINNERS.contains_key("dots"));
        assert!(SPINNERS.contains_key("arrows"));
        assert!(SPINNERS.contains_key("bounce"));
        assert!(SPINNERS.contains_key("pulse"));
    }

    #[test]
    fn test_spinner_frame_counts() {
        assert_eq!(SPINNERS.get("dots").unwrap().len(), 10);
        assert_eq!(SPINNERS.get("arrows").unwrap().len(), 8);
        assert_eq!(SPINNERS.get("bounce").unwrap().len(), 4);
        assert_eq!(SPINNERS.get("pulse").unwrap().len(), 4);
    }

    #[test]
    fn test_get_spinner_frame_dots() {
        assert_eq!(get_spinner_frame("dots", 0), "⠋");
        assert_eq!(get_spinner_frame("dots", 1), "⠙");
        assert_eq!(get_spinner_frame("dots", 9), "⠏");
    }

    #[test]
    fn test_get_spinner_frame_wraps() {
        // Index 10 should wrap to 0 for dots (10 frames)
        assert_eq!(get_spinner_frame("dots", 10), "⠋");
        assert_eq!(get_spinner_frame("dots", 11), "⠙");
        assert_eq!(get_spinner_frame("dots", 100), "⠋"); // 100 % 10 = 0
    }

    #[test]
    fn test_get_spinner_frame_arrows() {
        assert_eq!(get_spinner_frame("arrows", 0), "←");
        assert_eq!(get_spinner_frame("arrows", 4), "→");
    }

    #[test]
    fn test_get_spinner_frame_unknown_fallback() {
        // Unknown spinner should fall back to dots
        assert_eq!(get_spinner_frame("unknown", 0), "⠋");
        assert_eq!(get_spinner_frame("nonexistent", 1), "⠙");
    }

    #[test]
    fn test_mascot_states_exist() {
        assert!(MASCOT_STATES.contains_key("working"));
        assert!(MASCOT_STATES.contains_key("success"));
        assert!(MASCOT_STATES.contains_key("error"));
        assert!(MASCOT_STATES.contains_key("waiting"));
        assert!(MASCOT_STATES.contains_key("celebration"));
    }

    #[test]
    fn test_get_mascot_working() {
        let mascot = get_mascot("working");
        assert!(mascot.contains("o_o"));
    }

    #[test]
    fn test_get_mascot_success() {
        let mascot = get_mascot("success");
        assert!(mascot.contains("^o^"));
    }

    #[test]
    fn test_get_mascot_error() {
        let mascot = get_mascot("error");
        assert!(mascot.contains("x_x"));
    }

    #[test]
    fn test_get_mascot_waiting() {
        let mascot = get_mascot("waiting");
        assert!(mascot.contains("._.")); // Face: (._.)
    }

    #[test]
    fn test_get_mascot_celebration() {
        let mascot = get_mascot("celebration");
        assert!(mascot.contains("^o^"));
        assert!(mascot.contains("*"));
    }

    #[test]
    fn test_get_mascot_unknown_fallback() {
        // Unknown state should fall back to working
        let mascot = get_mascot("unknown");
        assert!(mascot.contains("o_o"));
    }

    #[test]
    fn test_mascot_multiline() {
        // All mascots should be multiline
        for (_, mascot) in MASCOT_STATES.iter() {
            assert!(mascot.contains('\n'), "Mascot should be multiline");
        }
    }
}
