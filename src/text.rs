//! Character-safe string truncation helpers.
//!
//! Display code should never slice strings by byte index (`&s[..n]`): that
//! panics when `n` falls inside a multi-byte UTF-8 character. These helpers
//! truncate by *characters* instead, so emoji and CJK text are handled safely.

/// Take at most `max_chars` characters from the start of `s`.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Take at most `max_chars` characters from the end of `s`.
pub fn tail_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        s.chars().skip(count - max_chars).collect()
    }
}

/// Truncate `s` to at most `max_chars` characters, appending "..." if truncated.
///
/// The returned string never exceeds `max_chars` characters, so callers can
/// rely on it for fixed-width layout.
pub fn ellipsize(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", s.chars().take(keep).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_chars_short() {
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_chars_exact() {
        assert_eq!(truncate_chars("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_chars_long() {
        assert_eq!(truncate_chars("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_chars_multibyte() {
        // Emoji are multi-byte; byte slicing would panic here.
        assert_eq!(truncate_chars("a🦀b🦀c", 3), "a🦀b");
        assert_eq!(truncate_chars("日本語のテキストです", 4), "日本語の");
    }

    #[test]
    fn test_tail_chars_short() {
        assert_eq!(tail_chars("file.rs", 40), "file.rs");
    }

    #[test]
    fn test_tail_chars_long() {
        assert_eq!(tail_chars("abcdef", 3), "def");
    }

    #[test]
    fn test_tail_chars_multibyte() {
        assert_eq!(tail_chars("x🦀y🦀z", 2), "🦀z");
    }

    #[test]
    fn test_ellipsize_short() {
        assert_eq!(ellipsize("short", 10), "short");
    }

    #[test]
    fn test_ellipsize_exact() {
        assert_eq!(ellipsize("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn test_ellipsize_long() {
        assert_eq!(ellipsize("hello world", 8), "hello...");
        assert_eq!(ellipsize("hello world", 8).chars().count(), 8);
    }

    #[test]
    fn test_ellipsize_multibyte_no_panic() {
        // 200+ chars with emoji — previously &text[..197] could panic.
        let text = "🦀".repeat(300);
        let out = ellipsize(&text, 200);
        assert_eq!(out.chars().count(), 200);
        assert!(out.ends_with("..."));
    }

    #[test]
    fn test_ellipsize_tiny_max() {
        assert_eq!(ellipsize("hello", 2), "...");
    }
}
