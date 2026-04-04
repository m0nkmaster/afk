//! PTY Validation Spike
//!
//! Spawns the claude CLI under a PTY, strips ANSI escape sequences, and
//! checks whether each output line is valid JSON. This is the primary
//! validation artifact for Phase 6.
//!
//! Run:
//!   cargo run --example pty_spike --features pty
//!
//! Outputs:
//!   pty_raw.log      — raw bytes from PTY master
//!   pty_stripped.log  — after ANSI stripping
//!   Console report   — line counts and parse results

#[cfg(not(feature = "pty"))]
fn main() {
    eprintln!("This example requires the `pty` feature:");
    eprintln!("  cargo run --example pty_spike --features pty");
    std::process::exit(1);
}

#[cfg(feature = "pty")]
fn main() {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::{BufRead, BufReader, Read, Write};
    use vte::{Parser, Perform};

    /// Collects printable output while discarding ANSI control sequences.
    /// Same approach as the production `AnsiStrippingReader` in `pty_spawn.rs`.
    #[derive(Default)]
    struct StripAnsi {
        out: Vec<u8>,
    }

    impl Perform for StripAnsi {
        fn print(&mut self, c: char) {
            let mut buf = [0u8; 4];
            self.out
                .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
        fn execute(&mut self, byte: u8) {
            if matches!(byte, b'\n' | b'\r' | b'\t') {
                self.out.push(byte);
            }
        }
        fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
        fn put(&mut self, _: u8) {}
        fn unhook(&mut self) {}
        fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
        fn csi_dispatch(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
        fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
    }

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Say hello in one sentence.".to_string());

    println!("=== PTY Validation Spike ===");
    println!("Prompt: {prompt}");
    println!();

    // Open a PTY pair
    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 50,
        cols: 220,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Failed to open PTY: {e}");
            std::process::exit(1);
        }
    };

    // Build the claude command
    let mut cmd = CommandBuilder::new("claude");
    cmd.arg("--dangerously-skip-permissions");
    cmd.arg("-p");
    cmd.arg(&prompt);
    cmd.arg("--output-format");
    cmd.arg("stream-json");

    println!(
        "Spawning: claude --dangerously-skip-permissions -p \"{}\" --output-format stream-json",
        prompt
    );
    println!();

    // Spawn on the slave side
    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn command: {e}");
            std::process::exit(1);
        }
    };
    drop(pair.slave);

    // Read raw output from the master
    let mut raw_reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to clone PTY reader: {e}");
            std::process::exit(1);
        }
    };

    let mut raw_bytes = Vec::new();
    let read_result = raw_reader.read_to_end(&mut raw_bytes);
    match read_result {
        Ok(n) => println!("Read {n} raw bytes from PTY"),
        Err(e) => {
            // PTY read may return an error when the child exits — that's normal.
            println!(
                "PTY read ended with: {e} ({} bytes captured)",
                raw_bytes.len()
            );
        }
    }

    // Save raw output
    if let Ok(mut f) = std::fs::File::create("pty_raw.log") {
        let _ = f.write_all(&raw_bytes);
        println!("Saved raw output to pty_raw.log");
    }

    // Strip ANSI using vte (same stateful parser as production pty_spawn.rs)
    let mut parser = Parser::new();
    let mut performer = StripAnsi::default();
    parser.advance(&mut performer, &raw_bytes);
    let stripped_bytes = performer.out;
    if let Ok(mut f) = std::fs::File::create("pty_stripped.log") {
        let _ = f.write_all(&stripped_bytes);
        println!("Saved stripped output to pty_stripped.log");
    }

    // Parse lines and attempt JSON validation
    let stripped_text = String::from_utf8_lossy(&stripped_bytes);
    let reader = BufReader::new(stripped_text.as_bytes());

    let mut total_lines = 0;
    let mut empty_lines = 0;
    let mut json_ok = 0;
    let mut json_fail = 0;
    let mut plain_text = 0;
    let mut failures: Vec<(usize, String)> = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        total_lines += 1;

        if line.trim().is_empty() {
            empty_lines += 1;
            continue;
        }

        let trimmed = line.trim();
        let looks_like_json = trimmed.starts_with('{') && trimmed.ends_with('}');

        if looks_like_json {
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(_) => json_ok += 1,
                Err(_) => {
                    json_fail += 1;
                    if failures.len() < 10 {
                        failures.push((i + 1, line.clone()));
                    }
                }
            }
        } else {
            plain_text += 1;
        }
    }

    // Wait for child
    let exit_ok = child.wait().map(|s| s.success()).unwrap_or(false);

    // Report
    println!();
    println!("=== Results ===");
    println!("Child exited successfully: {exit_ok}");
    println!("Total lines:    {total_lines}");
    println!("  Empty:        {empty_lines}");
    println!("  Valid JSON:   {json_ok}");
    println!("  Invalid JSON: {json_fail}");
    println!("  Plain text:   {plain_text}");
    println!();

    if failures.is_empty() && json_ok > 0 {
        println!("PASS: All JSON-shaped lines parsed successfully.");
    } else if json_fail > 0 {
        println!("FAIL: {json_fail} JSON-shaped lines failed to parse.");
        println!();
        println!("Sample failures:");
        for (line_num, content) in &failures {
            println!("  Line {line_num}: {}", &content[..content.len().min(120)]);
        }
    } else if json_ok == 0 {
        println!("INCONCLUSIVE: No JSON-shaped lines found in output.");
        println!("The CLI may not be producing stream-json output under PTY.");
    }
}
