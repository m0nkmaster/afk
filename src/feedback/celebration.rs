//! Celebration displays for task and session completion.
//!
//! This module provides visual feedback when quality gates pass/fail,
//! tasks complete, and sessions finish.

use std::time::Duration;

use super::art::get_mascot;

/// Calculate visible length of a string, excluding ANSI escape codes.
pub fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

/// Display visual feedback when quality gates fail.
pub fn show_gates_failed(failed_gates: &[String], continuing: bool) {
    let mut msg = String::new();
    msg.push_str("\x1b[33;1m⚠\x1b[0m ");
    msg.push_str("\x1b[31;1mQuality gates failed:\x1b[0m ");
    msg.push_str(&format!("\x1b[31m{}\x1b[0m", failed_gates.join(", ")));

    if continuing {
        msg.push_str(" \x1b[2m│\x1b[0m ");
        msg.push_str("\x1b[33mContinuing...\x1b[0m");
    }

    println!("{}", msg);
}

/// Display visual feedback when quality gates pass.
pub fn show_gates_passed(gates: &[String]) {
    for gate in gates {
        println!(
            "  \x1b[32;1m✓\x1b[0m \x1b[32m{}\x1b[0m \x1b[2mpassed\x1b[0m",
            gate
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Display a celebration when a task is completed.
pub fn show_celebration(task_id: &str) {
    let celebration_art = get_mascot("celebration");

    println!();
    println!(
        "\x1b[32m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m"
    );
    println!(
        "\x1b[32m│\x1b[0m                          \x1b[32;1m🎉 Celebration 🎉\x1b[0m                               \x1b[32m│\x1b[0m"
    );
    println!(
        "\x1b[32m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
    );
    println!(
        "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
        "★ ".repeat(16),
        " ".repeat(45 - 32)
    );
    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

    for line in celebration_art.lines() {
        let padded = format!("\x1b[32m│\x1b[0m  \x1b[32;1m{}\x1b[0m", line);
        let padded_len = visible_len(&padded);
        let padding = 77_usize.saturating_sub(padded_len);
        println!("{}{}\x1b[32m│\x1b[0m", padded, " ".repeat(padding));
    }

    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
    let msg = format!(
        "  \x1b[32;1m✓ Task Complete!\x1b[0m \x1b[36;1m{}\x1b[0m",
        task_id
    );
    let msg_len = visible_len(&msg);
    let msg_padding = 77_usize.saturating_sub(msg_len);
    println!(
        "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
        msg,
        " ".repeat(msg_padding)
    );
    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
    println!(
        "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
        "★ ".repeat(16),
        " ".repeat(45 - 32)
    );
    println!(
        "\x1b[32m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m"
    );
    println!();

    std::thread::sleep(Duration::from_millis(500));
}

/// Display a full celebration when the session is complete.
pub fn show_session_complete(tasks_completed: u32, iterations: u32, duration_seconds: f64) {
    let celebration_art = get_mascot("celebration");
    let total_seconds = duration_seconds as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let duration_str = format!("{}m {}s", minutes, seconds);

    println!();
    println!(
        "\x1b[32m┌─────────────────────────────────────────────────────────────────────────────┐\x1b[0m"
    );
    println!(
        "\x1b[32m│\x1b[0m                        \x1b[32;1m🎉 Session Complete 🎉\x1b[0m                             \x1b[32m│\x1b[0m"
    );
    println!(
        "\x1b[32m├─────────────────────────────────────────────────────────────────────────────┤\x1b[0m"
    );
    println!(
        "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
        "★ ".repeat(20),
        " ".repeat(77 - 42)
    );
    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

    for line in celebration_art.lines() {
        let padded = format!("\x1b[32m│\x1b[0m  \x1b[32;1m{}\x1b[0m", line);
        let padded_len = visible_len(&padded);
        let padding = 77_usize.saturating_sub(padded_len);
        println!("{}{}\x1b[32m│\x1b[0m", padded, " ".repeat(padding));
    }

    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
    println!(
        "\x1b[32m│\x1b[0m  \x1b[32;1m✓ All Tasks Complete!\x1b[0m{}\x1b[32m│\x1b[0m",
        " ".repeat(77 - 24)
    );
    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));

    let stats1 = format!(
        "  \x1b[2mTasks completed:\x1b[0m \x1b[36;1m{}\x1b[0m",
        tasks_completed
    );
    let stats1_len = visible_len(&stats1);
    println!(
        "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
        stats1,
        " ".repeat(77 - stats1_len)
    );

    let stats2 = format!(
        "  \x1b[2mIterations:\x1b[0m \x1b[36;1m{}\x1b[0m",
        iterations
    );
    let stats2_len = visible_len(&stats2);
    println!(
        "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
        stats2,
        " ".repeat(77 - stats2_len)
    );

    let stats3 = format!(
        "  \x1b[2mTotal time:\x1b[0m \x1b[36;1m{}\x1b[0m",
        duration_str
    );
    let stats3_len = visible_len(&stats3);
    println!(
        "\x1b[32m│\x1b[0m{}{}\x1b[32m│\x1b[0m",
        stats3,
        " ".repeat(77 - stats3_len)
    );

    println!("\x1b[32m│\x1b[0m{}\x1b[32m│\x1b[0m", " ".repeat(77));
    println!(
        "\x1b[32m│\x1b[0m  \x1b[33;1m{}\x1b[0m{}\x1b[32m│\x1b[0m",
        "★ ".repeat(20),
        " ".repeat(77 - 42)
    );
    println!(
        "\x1b[32m└─────────────────────────────────────────────────────────────────────────────┘\x1b[0m"
    );
    println!();

    std::thread::sleep(Duration::from_millis(500));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_len_plain_text() {
        assert_eq!(visible_len("hello"), 5);
        assert_eq!(visible_len(""), 0);
        assert_eq!(visible_len("a b c"), 5);
    }

    #[test]
    fn test_visible_len_with_ansi_codes() {
        assert_eq!(visible_len("\x1b[31mred\x1b[0m"), 3);
        assert_eq!(visible_len("\x1b[1;32mbold green\x1b[0m"), 10);
        assert_eq!(visible_len("\x1b[36m◉\x1b[0m afk"), 5);
    }

    #[test]
    fn test_show_gates_passed_empty() {
        show_gates_passed(&[]);
        // Should not panic
    }

    #[test]
    fn test_show_gates_passed_with_gates() {
        show_gates_passed(&["lint".to_string(), "test".to_string()]);
        // Should not panic
    }

    #[test]
    fn test_show_gates_failed_continuing() {
        show_gates_failed(&["test".to_string()], true);
        // Should not panic
    }

    #[test]
    fn test_show_gates_failed_not_continuing() {
        show_gates_failed(&["lint".to_string(), "test".to_string()], false);
        // Should not panic
    }

    #[test]
    fn test_show_celebration() {
        // This will print to stdout and sleep briefly
        // We just verify it doesn't panic
        show_celebration("test-task-001");
    }

    #[test]
    fn test_show_session_complete() {
        // This will print to stdout and sleep briefly
        // We just verify it doesn't panic
        show_session_complete(5, 10, 120.5);
    }
}
