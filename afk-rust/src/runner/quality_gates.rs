//! Quality gate runner.
//!
//! This module provides functionality to run quality gates (lint, test, types, etc.)
//! and report pass/fail status.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::config::FeedbackLoopsConfig;

/// Result of a single quality gate.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Name of the gate.
    pub name: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Combined stdout and stderr output.
    pub output: String,
    /// Duration in seconds.
    pub duration_seconds: f64,
}

/// Result of running all quality gates.
#[derive(Debug)]
pub struct QualityGateResult {
    /// Whether all gates passed.
    pub all_passed: bool,
    /// List of gate results.
    pub gates: Vec<GateResult>,
    /// Names of failed gates.
    pub failed_gates: Vec<String>,
}

impl QualityGateResult {
    /// Create a new empty result.
    pub fn new() -> Self {
        Self {
            all_passed: true,
            gates: Vec::new(),
            failed_gates: Vec::new(),
        }
    }

    /// Add a gate result.
    pub fn add_gate(&mut self, result: GateResult) {
        if !result.passed {
            self.all_passed = false;
            self.failed_gates.push(result.name.clone());
        }
        self.gates.push(result);
    }
}

impl Default for QualityGateResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Default timeout for gates in seconds (5 minutes).
#[allow(dead_code)]
const DEFAULT_GATE_TIMEOUT_SECS: u64 = 300;

/// Run all configured quality gates.
///
/// # Arguments
///
/// * `feedback_loops` - The configured feedback loops from config
/// * `verbose` - Whether to print verbose output
///
/// # Returns
///
/// QualityGateResult with pass/fail status for each gate.
pub fn run_quality_gates(feedback_loops: &FeedbackLoopsConfig, verbose: bool) -> QualityGateResult {
    let mut result = QualityGateResult::new();

    // Collect gates to run
    let mut gates: Vec<(String, String)> = Vec::new();

    if let Some(ref cmd) = feedback_loops.types {
        gates.push(("types".to_string(), cmd.clone()));
    }
    if let Some(ref cmd) = feedback_loops.lint {
        gates.push(("lint".to_string(), cmd.clone()));
    }
    if let Some(ref cmd) = feedback_loops.test {
        gates.push(("test".to_string(), cmd.clone()));
    }
    if let Some(ref cmd) = feedback_loops.build {
        gates.push(("build".to_string(), cmd.clone()));
    }

    // Add custom gates
    for (name, cmd) in &feedback_loops.custom {
        gates.push((name.clone(), cmd.clone()));
    }

    if gates.is_empty() {
        println!("\x1b[2mNo quality gates configured.\x1b[0m");
        return result;
    }

    println!();
    println!("\x1b[1mRunning quality gates...\x1b[0m");
    println!();

    for (name, cmd) in gates {
        let gate_result = run_single_gate(&name, &cmd, verbose);
        
        let status = if gate_result.passed {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[31m✗\x1b[0m"
        };

        println!(
            "  {} {} ({:.1}s)",
            status,
            name,
            gate_result.duration_seconds
        );

        if verbose && !gate_result.output.is_empty() {
            for line in gate_result.output.lines() {
                println!("      {line}");
            }
        }

        result.add_gate(gate_result);
    }

    println!();

    if result.all_passed {
        println!("\x1b[32m✓ All gates passed\x1b[0m");
    } else {
        println!("\x1b[31m✗ Some gates failed: {}\x1b[0m", result.failed_gates.join(", "));
    }

    result
}

/// Run a single quality gate.
fn run_single_gate(name: &str, cmd: &str, _verbose: bool) -> GateResult {
    let start = std::time::Instant::now();

    // Parse command - use shell for complex commands
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let shell_arg = if cfg!(windows) { "/C" } else { "-c" };

    let process = Command::new(shell)
        .args([shell_arg, cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut process = match process {
        Ok(p) => p,
        Err(e) => {
            return GateResult {
                name: name.to_string(),
                passed: false,
                output: format!("Failed to run command: {e}"),
                duration_seconds: start.elapsed().as_secs_f64(),
            };
        }
    };

    // Collect output
    let mut output = String::new();

    if let Some(stdout) = process.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');
        }
    }

    if let Some(stderr) = process.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');
        }
    }

    // Wait for process with timeout
    let result = process.wait();

    let passed = match result {
        Ok(status) => status.success(),
        Err(_) => false,
    };

    GateResult {
        name: name.to_string(),
        passed,
        output,
        duration_seconds: start.elapsed().as_secs_f64(),
    }
}

/// Check if any gates are configured.
pub fn has_configured_gates(feedback_loops: &FeedbackLoopsConfig) -> bool {
    feedback_loops.types.is_some()
        || feedback_loops.lint.is_some()
        || feedback_loops.test.is_some()
        || feedback_loops.build.is_some()
        || !feedback_loops.custom.is_empty()
}

/// Get list of configured gate names.
pub fn get_configured_gate_names(feedback_loops: &FeedbackLoopsConfig) -> Vec<String> {
    let mut names = Vec::new();

    if feedback_loops.types.is_some() {
        names.push("types".to_string());
    }
    if feedback_loops.lint.is_some() {
        names.push("lint".to_string());
    }
    if feedback_loops.test.is_some() {
        names.push("test".to_string());
    }
    if feedback_loops.build.is_some() {
        names.push("build".to_string());
    }

    for name in feedback_loops.custom.keys() {
        names.push(name.clone());
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_quality_gate_result_new() {
        let result = QualityGateResult::new();
        assert!(result.all_passed);
        assert!(result.gates.is_empty());
        assert!(result.failed_gates.is_empty());
    }

    #[test]
    fn test_quality_gate_result_add_passing_gate() {
        let mut result = QualityGateResult::new();
        result.add_gate(GateResult {
            name: "test".to_string(),
            passed: true,
            output: String::new(),
            duration_seconds: 1.0,
        });

        assert!(result.all_passed);
        assert_eq!(result.gates.len(), 1);
        assert!(result.failed_gates.is_empty());
    }

    #[test]
    fn test_quality_gate_result_add_failing_gate() {
        let mut result = QualityGateResult::new();
        result.add_gate(GateResult {
            name: "lint".to_string(),
            passed: false,
            output: "Error: lint failed".to_string(),
            duration_seconds: 0.5,
        });

        assert!(!result.all_passed);
        assert_eq!(result.gates.len(), 1);
        assert_eq!(result.failed_gates, vec!["lint"]);
    }

    #[test]
    fn test_quality_gate_result_mixed_gates() {
        let mut result = QualityGateResult::new();

        result.add_gate(GateResult {
            name: "lint".to_string(),
            passed: true,
            output: String::new(),
            duration_seconds: 0.5,
        });

        result.add_gate(GateResult {
            name: "test".to_string(),
            passed: false,
            output: "1 test failed".to_string(),
            duration_seconds: 2.0,
        });

        result.add_gate(GateResult {
            name: "build".to_string(),
            passed: true,
            output: String::new(),
            duration_seconds: 1.0,
        });

        assert!(!result.all_passed);
        assert_eq!(result.gates.len(), 3);
        assert_eq!(result.failed_gates, vec!["test"]);
    }

    #[test]
    fn test_has_configured_gates_empty() {
        let config = FeedbackLoopsConfig::default();
        assert!(!has_configured_gates(&config));
    }

    #[test]
    fn test_has_configured_gates_with_lint() {
        let config = FeedbackLoopsConfig {
            lint: Some("cargo clippy".to_string()),
            ..Default::default()
        };
        assert!(has_configured_gates(&config));
    }

    #[test]
    fn test_has_configured_gates_with_custom() {
        let mut custom = HashMap::new();
        custom.insert("format".to_string(), "cargo fmt --check".to_string());

        let config = FeedbackLoopsConfig {
            custom,
            ..Default::default()
        };
        assert!(has_configured_gates(&config));
    }

    #[test]
    fn test_get_configured_gate_names() {
        let mut custom = HashMap::new();
        custom.insert("format".to_string(), "cargo fmt --check".to_string());

        let config = FeedbackLoopsConfig {
            lint: Some("cargo clippy".to_string()),
            test: Some("cargo test".to_string()),
            custom,
            ..Default::default()
        };

        let names = get_configured_gate_names(&config);
        assert!(names.contains(&"lint".to_string()));
        assert!(names.contains(&"test".to_string()));
        assert!(names.contains(&"format".to_string()));
        assert!(!names.contains(&"types".to_string()));
    }

    #[test]
    fn test_run_single_gate_success() {
        // This test runs an actual command - 'true' always succeeds
        let result = run_single_gate("test", "true", false);
        assert!(result.passed);
        assert_eq!(result.name, "test");
    }

    #[test]
    fn test_run_single_gate_failure() {
        // This test runs an actual command - 'false' always fails
        let result = run_single_gate("test", "false", false);
        assert!(!result.passed);
        assert_eq!(result.name, "test");
    }

    #[test]
    fn test_run_single_gate_with_output() {
        let result = run_single_gate("echo", "echo hello", false);
        assert!(result.passed);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn test_run_quality_gates_no_gates() {
        let config = FeedbackLoopsConfig::default();
        let result = run_quality_gates(&config, false);
        assert!(result.all_passed);
        assert!(result.gates.is_empty());
    }

    #[test]
    fn test_run_quality_gates_with_passing_gates() {
        let config = FeedbackLoopsConfig {
            lint: Some("true".to_string()),
            test: Some("true".to_string()),
            ..Default::default()
        };

        let result = run_quality_gates(&config, false);
        assert!(result.all_passed);
        assert_eq!(result.gates.len(), 2);
    }

    #[test]
    fn test_run_quality_gates_with_failing_gate() {
        let config = FeedbackLoopsConfig {
            lint: Some("true".to_string()),
            test: Some("false".to_string()),
            ..Default::default()
        };

        let result = run_quality_gates(&config, false);
        assert!(!result.all_passed);
        assert_eq!(result.failed_gates, vec!["test"]);
    }
}
