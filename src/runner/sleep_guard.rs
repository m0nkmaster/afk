//! Sleep prevention guard for long-running autonomous sessions.
//!
//! This module provides a RAII guard that prevents the system from sleeping
//! while the autonomous loop is running. It uses platform-specific tools:
//!
//! - **macOS**: Uses `caffeinate -i` to prevent idle sleep
//! - **Linux**: Uses `systemd-inhibit` to prevent sleep
//! - **Windows/Other**: No-op (not yet implemented)
//!
//! The guard automatically releases the sleep lock when dropped.

use std::process::Child;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::{Command, Stdio};

/// A guard that prevents system sleep while held.
///
/// When created, this spawns a background process that inhibits system sleep.
/// When dropped, the process is terminated and normal sleep behaviour resumes.
///
/// # Example
///
/// ```ignore
/// let _guard = SleepGuard::new();
/// // System will not sleep while guard is in scope
/// run_long_task();
/// // Guard dropped here, sleep resumes
/// ```
pub struct SleepGuard {
    /// The background process keeping the system awake.
    child: Option<Child>,
    /// Whether sleep prevention is active.
    active: bool,
}

impl SleepGuard {
    /// Create a new sleep guard.
    ///
    /// Spawns a platform-specific process to prevent system sleep.
    /// If the process fails to spawn, returns a guard that does nothing.
    pub fn new() -> Self {
        let (child, active) = spawn_sleep_inhibitor();
        Self { child, active }
    }

    /// Create a no-op guard that doesn't prevent sleep.
    ///
    /// Useful when sleep prevention is disabled in config.
    pub fn disabled() -> Self {
        Self {
            child: None,
            active: false,
        }
    }

    /// Check if sleep prevention is currently active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get a description of the sleep prevention method in use.
    #[must_use]
    pub fn method(&self) -> &'static str {
        if !self.active {
            return "none";
        }

        #[cfg(target_os = "macos")]
        {
            "caffeinate"
        }

        #[cfg(target_os = "linux")]
        {
            "systemd-inhibit"
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            "none"
        }
    }
}

impl Default for SleepGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SleepGuard {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // Attempt graceful termination first
            let _ = child.kill();
            // Wait briefly to ensure cleanup
            let _ = child.wait();
        }
    }
}

/// Spawn the platform-specific sleep inhibitor process.
///
/// Returns the child process handle and whether it was successful.
fn spawn_sleep_inhibitor() -> (Option<Child>, bool) {
    #[cfg(target_os = "macos")]
    {
        spawn_macos_caffeinate()
    }

    #[cfg(target_os = "linux")]
    {
        spawn_linux_inhibitor()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // No sleep prevention on unsupported platforms
        (None, false)
    }
}

/// Spawn caffeinate on macOS.
///
/// Uses `-i` flag to prevent idle sleep (system can still sleep on lid close).
/// Uses `-w` with our own PID so caffeinate exits if we crash.
#[cfg(target_os = "macos")]
fn spawn_macos_caffeinate() -> (Option<Child>, bool) {
    // -i: Prevent idle sleep
    // -w: Wait for process with given PID (our own, for automatic cleanup)
    let pid = std::process::id().to_string();

    match Command::new("caffeinate")
        .args(["-i", "-w", &pid])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => (Some(child), true),
        Err(_) => {
            // caffeinate not available (shouldn't happen on macOS)
            (None, false)
        }
    }
}

/// Spawn systemd-inhibit on Linux.
///
/// Inhibits idle and sleep while our process runs.
#[cfg(target_os = "linux")]
fn spawn_linux_inhibitor() -> (Option<Child>, bool) {
    // Try systemd-inhibit first (most common on modern Linux)
    if let Ok(child) = Command::new("systemd-inhibit")
        .args([
            "--what=idle:sleep",
            "--who=afk",
            "--why=Autonomous coding session in progress",
            "--mode=block",
            "sleep",
            "infinity",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        return (Some(child), true);
    }

    // Fallback: try gnome-session-inhibit
    if let Ok(child) = Command::new("gnome-session-inhibit")
        .args([
            "--inhibit",
            "idle:suspend",
            "--reason",
            "Autonomous coding session in progress",
            "sleep",
            "infinity",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        return (Some(child), true);
    }

    // No inhibitor available
    (None, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_guard_creation() {
        let guard = SleepGuard::new();
        // On supported platforms, should be active
        // On unsupported platforms, should be inactive
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            // May or may not be active depending on whether the tool is installed
            let _ = guard.is_active();
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(!guard.is_active());
        }
    }

    #[test]
    fn test_sleep_guard_disabled() {
        let guard = SleepGuard::disabled();
        assert!(!guard.is_active());
        assert_eq!(guard.method(), "none");
    }

    #[test]
    fn test_sleep_guard_drop_cleanup() {
        // Create and immediately drop - should not panic
        let guard = SleepGuard::new();
        drop(guard);
    }

    #[test]
    fn test_sleep_guard_method_reporting() {
        let guard = SleepGuard::disabled();
        assert_eq!(guard.method(), "none");

        // Active guard reports platform-specific method
        let active_guard = SleepGuard::new();
        if active_guard.is_active() {
            #[cfg(target_os = "macos")]
            assert_eq!(active_guard.method(), "caffeinate");

            #[cfg(target_os = "linux")]
            assert_eq!(active_guard.method(), "systemd-inhibit");
        }
    }

    #[test]
    fn test_sleep_guard_default() {
        let guard = SleepGuard::default();
        // Default should attempt to create an active guard
        let _ = guard.is_active();
    }
}
