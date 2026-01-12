//! Build script for afk.
//!
//! Generates version string with build timestamp in format:
//! MAJOR.MINOR.PATCH+YYYYMMDDHHmmss
//!
//! The timestamp updates whenever source files change.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Get the base version from CARGO_PKG_VERSION
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());

    // Generate timestamp in YYYYMMDDHHmmss format using the date command
    let timestamp = Command::new("date")
        .arg("+%Y%m%d%H%M%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "00000000000000".to_string());

    // Create full version string
    let full_version = format!("{}+{}", version, timestamp);

    // Write to OUT_DIR for the main crate to include
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version.rs");
    fs::write(
        &dest_path,
        format!(
            r#"/// Full version string with build timestamp.
pub const VERSION: &str = "{full_version}";
"#
        ),
    )
    .unwrap();

    // Rerun when source files change
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
