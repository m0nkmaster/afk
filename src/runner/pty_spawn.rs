//! PTY-based subprocess spawning with ANSI escape stripping.
//!
//! When compiled with `--features pty`, this module provides an alternative
//! to piped stdout: the AI CLI is spawned under a pseudo-terminal so it
//! believes it's talking to a real terminal. Raw PTY output is stripped of
//! ANSI escape sequences before being fed to the streaming loop.
//!
//! **Known limitation:** A PTY merges stdout and stderr into a single stream,
//! so separate stderr capture is not available in this mode.

use portable_pty::{native_pty_system, Child as PtyChild, CommandBuilder, MasterPty, PtySize};
use std::io::{self, BufRead, BufReader, Read};

/// A subprocess spawned under a PTY, providing an ANSI-stripped reader
/// and process lifecycle methods compatible with `stream_subprocess_output`.
pub struct PtyProcess {
    /// Buffered, ANSI-stripped reader from the PTY master.
    /// Wrapped in Option so it can be taken separately from the kill handle
    /// (needed to satisfy the borrow checker when passing to `stream_subprocess_output`).
    reader: Option<Box<dyn BufRead + Send>>,
    /// Handle to the child process for kill/wait.
    child: Box<dyn PtyChild + Send>,
    /// Keep the master PTY alive for the lifetime of the process.
    _master: Box<dyn MasterPty + Send>,
}

impl PtyProcess {
    /// Spawn a command under a PTY.
    ///
    /// `cmd_parts` is the command + arguments (e.g. `["claude", "--dangerously-skip-permissions"]`).
    /// `prompt` is appended as the final argument.
    ///
    /// Returns a `PtyProcess` whose `reader` yields ANSI-stripped, line-buffered output.
    pub fn spawn(cmd_parts: &[String], prompt: &str) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 50,
            cols: 220,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(&cmd_parts[0]);
        for arg in &cmd_parts[1..] {
            cmd.arg(arg);
        }
        cmd.arg(prompt);

        let child = pair.slave.spawn_command(cmd)?;
        // Close the slave end in the parent — the child owns it now.
        drop(pair.slave);

        let raw_reader = pair.master.try_clone_reader()?;
        let stripped = AnsiStrippingReader::new(raw_reader);
        let reader: Box<dyn BufRead + Send> = Box::new(BufReader::new(stripped));

        Ok(Self {
            reader: Some(reader),
            child,
            _master: pair.master,
        })
    }

    /// Take the reader out of this process handle.
    ///
    /// Call this before creating a kill closure so the borrow checker
    /// allows both the reader and kill closure to coexist.
    pub fn take_reader(&mut self) -> Box<dyn BufRead + Send> {
        self.reader.take().expect("reader already taken")
    }

    /// Kill the child process.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }

    /// Wait for the child process to exit, returning success/failure.
    pub fn wait(&mut self) -> bool {
        self.child
            .wait()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// ANSI escape stripping adapter
// ---------------------------------------------------------------------------

/// A `Read` adapter that strips ANSI escape sequences from the underlying reader.
///
/// Reads raw bytes in chunks, passes them through `strip_ansi_escapes::strip()`,
/// and serves the cleaned output. This sits between the PTY master and the
/// `BufReader` that feeds `reader.lines()`.
struct AnsiStrippingReader {
    inner: Box<dyn Read + Send>,
    buffer: Vec<u8>,
    pos: usize,
}

impl AnsiStrippingReader {
    fn new(inner: Box<dyn Read + Send>) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            pos: 0,
        }
    }
}

impl Read for AnsiStrippingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Serve from buffered stripped data first.
        if self.pos < self.buffer.len() {
            let n = std::cmp::min(buf.len(), self.buffer.len() - self.pos);
            buf[..n].copy_from_slice(&self.buffer[self.pos..self.pos + n]);
            self.pos += n;
            return Ok(n);
        }

        // Read a chunk of raw PTY output.
        let mut raw = [0u8; 4096];
        let n = self.inner.read(&mut raw)?;
        if n == 0 {
            return Ok(0); // EOF
        }

        // Strip ANSI escape sequences and buffer the result.
        self.buffer = strip_ansi_escapes::strip(&raw[..n]);
        self.pos = 0;

        let out_n = std::cmp::min(buf.len(), self.buffer.len());
        buf[..out_n].copy_from_slice(&self.buffer[..out_n]);
        self.pos = out_n;
        Ok(out_n)
    }
}

// Send is safe: inner reader is Send, buffer is owned.
unsafe impl Send for AnsiStrippingReader {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_ansi_stripping_reader_plain_text() {
        let input = b"hello world\n";
        let inner: Box<dyn Read + Send> = Box::new(Cursor::new(input.to_vec()));
        let mut reader = AnsiStrippingReader::new(inner);
        let mut buf = vec![0u8; 64];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello world\n");
    }

    #[test]
    fn test_ansi_stripping_reader_removes_escapes() {
        // "\x1b[31mred\x1b[0m" → "red"
        let input = b"\x1b[31mred\x1b[0m\n";
        let inner: Box<dyn Read + Send> = Box::new(Cursor::new(input.to_vec()));
        let mut reader = AnsiStrippingReader::new(inner);
        let mut buf = vec![0u8; 64];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"red\n");
    }

    #[test]
    fn test_ansi_stripping_reader_json_preserved() {
        let input = b"\x1b[0m{\"type\":\"assistant\",\"text\":\"hello\"}\n\x1b[0m";
        let inner: Box<dyn Read + Send> = Box::new(Cursor::new(input.to_vec()));
        let mut reader = BufReader::new(AnsiStrippingReader::new(inner));
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "{\"type\":\"assistant\",\"text\":\"hello\"}");
    }
}
