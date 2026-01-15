//! Inline spinner for showing progress during long-running operations.
//!
//! The spinner runs in a background thread and updates the terminal with
//! an animated spinner and message.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::art::get_spinner_frame;

/// A simple inline spinner for showing progress during long-running operations.
///
/// The spinner runs in a background thread and updates the terminal with
/// an animated spinner and message. Call `stop()` to clear it.
///
/// # Example
///
/// ```ignore
/// let spinner = Spinner::start("Loading...");
/// // ... do work ...
/// spinner.stop();
/// ```
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Start a new spinner with the given message.
    ///
    /// The spinner will animate in the terminal until `stop()` is called.
    pub fn start(message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let message = message.to_string();

        let handle = thread::spawn(move || {
            let mut frame = 0usize;
            while running_clone.load(Ordering::Relaxed) {
                let spinner_char = get_spinner_frame("dots", frame);
                print!(
                    "\r\x1b[36m{}\x1b[0m \x1b[2m{}\x1b[0m",
                    spinner_char, message
                );
                let _ = io::stdout().flush();

                frame = frame.wrapping_add(1);
                thread::sleep(Duration::from_millis(80));
            }
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the spinner and clear the line.
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Clear the spinner line
        print!("\r\x1b[2K");
        let _ = io::stdout().flush();
    }

    /// Stop the spinner and replace with a success message.
    pub fn stop_with_success(mut self, message: &str) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Clear and print success
        println!("\r\x1b[2K\x1b[32m✓\x1b[0m {}", message);
    }

    /// Stop the spinner and replace with an error message.
    pub fn stop_with_error(mut self, message: &str) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Clear and print error
        println!("\r\x1b[2K\x1b[31m✗\x1b[0m {}", message);
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // Ensure spinner is stopped if dropped without explicit stop
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        print!("\r\x1b[2K");
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_start_and_stop() {
        // Start spinner and immediately stop it
        let spinner = Spinner::start("Test message");
        // Brief pause to let at least one frame render
        thread::sleep(Duration::from_millis(100));
        spinner.stop();
        // Should not panic
    }

    #[test]
    fn test_spinner_stop_with_success() {
        let spinner = Spinner::start("Loading...");
        thread::sleep(Duration::from_millis(50));
        spinner.stop_with_success("Completed!");
        // Should not panic
    }

    #[test]
    fn test_spinner_stop_with_error() {
        let spinner = Spinner::start("Loading...");
        thread::sleep(Duration::from_millis(50));
        spinner.stop_with_error("Failed!");
        // Should not panic
    }

    #[test]
    fn test_spinner_drop_cleanup() {
        // Create spinner in a scope and let it drop
        {
            let _spinner = Spinner::start("Will be dropped");
            thread::sleep(Duration::from_millis(50));
            // Spinner drops here, should clean up gracefully
        }
        // Should not panic
    }
}
