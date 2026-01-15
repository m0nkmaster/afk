//! Performance benchmarks for afk.
//!
//! Run with: cargo bench
//!
//! These benchmarks measure key performance metrics:
//! - Startup time (parse args, load config)
//! - Config load/save
//! - PRD sync with various file sizes
//! - Prompt generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use tempfile::TempDir;

use afk::config::AfkConfig;
use afk::prd::{PrdDocument, UserStory};
use afk::progress::SessionProgress;
use afk::prompt::generate_prompt;
use afk::sources::json::load_json_tasks;

/// Helper to create a temp directory with config.
fn setup_config_env() -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();

    let config = r#"{
        "ai_cli": {
            "command": "echo",
            "args": ["test"]
        },
        "sources": [],
        "limits": {
            "max_iterations": 100,
            "max_task_failures": 5,
            "timeout_minutes": 60
        },
        "feedback_loops": {
            "lint": "cargo clippy",
            "test": "cargo test"
        }
    }"#;
    fs::write(afk_dir.join("config.json"), config).unwrap();

    (temp, afk_dir)
}

/// Helper to create PRD with N stories.
fn create_prd_with_stories(afk_dir: &std::path::Path, num_stories: usize) {
    let stories: Vec<serde_json::Value> = (0..num_stories)
        .map(|i| {
            serde_json::json!({
                "id": format!("task-{:04}", i),
                "title": format!("Task number {} with a moderately long title", i),
                "description": format!("Description for task {} with enough text to be realistic", i),
                "acceptanceCriteria": [
                    format!("Criterion 1 for task {}", i),
                    format!("Criterion 2 for task {}", i),
                    format!("Criterion 3 for task {}", i),
                ],
                "priority": (i % 3) + 1,
                "passes": i < num_stories / 2  // Half complete
            })
        })
        .collect();

    let prd = serde_json::json!({
        "projectName": "benchmark-project",
        "branch": "main",
        "userStories": stories
    });

    fs::write(
        afk_dir.join("tasks.json"),
        serde_json::to_string_pretty(&prd).unwrap(),
    )
    .unwrap();
}

/// Helper to create progress file.
fn create_progress(afk_dir: &std::path::Path) {
    let progress = r#"{
        "started_at": "2025-01-01T00:00:00",
        "iterations": 5,
        "tasks": {}
    }"#;
    fs::write(afk_dir.join("progress.json"), progress).unwrap();
}

// ============================================================================
// Config benchmarks
// ============================================================================

fn bench_config_load(c: &mut Criterion) {
    let (temp, afk_dir) = setup_config_env();
    let config_path = afk_dir.join("config.json");

    c.bench_function("config_load", |b| {
        b.iter(|| {
            let config = AfkConfig::load(black_box(Some(config_path.as_path()))).unwrap();
            black_box(config)
        })
    });

    drop(temp);
}

fn bench_config_save(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();
    let config_path = afk_dir.join("config.json");

    let config = AfkConfig::default();

    c.bench_function("config_save", |b| {
        b.iter(|| {
            config.save(black_box(Some(config_path.as_path()))).unwrap();
        })
    });

    drop(temp);
}

// ============================================================================
// PRD benchmarks
// ============================================================================

fn bench_prd_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("prd_load");

    for size in [10, 50, 100, 500].iter() {
        let (temp, afk_dir) = setup_config_env();
        create_prd_with_stories(&afk_dir, *size);
        let prd_path = afk_dir.join("tasks.json");

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let prd = PrdDocument::load(black_box(Some(prd_path.as_path()))).unwrap();
                black_box(prd)
            })
        });

        drop(temp);
    }

    group.finish();
}

fn bench_prd_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("prd_save");

    for size in [10, 50, 100, 500].iter() {
        let temp = TempDir::new().unwrap();
        let afk_dir = temp.path().join(".afk");
        fs::create_dir_all(&afk_dir).unwrap();
        let prd_path = afk_dir.join("tasks.json");

        // Create PRD in memory
        let stories: Vec<UserStory> = (0..*size)
            .map(|i| UserStory {
                id: format!("task-{:04}", i),
                title: format!("Task number {}", i),
                description: format!("Description for task {}", i),
                acceptance_criteria: vec![
                    format!("Criterion 1 for task {}", i),
                    format!("Criterion 2 for task {}", i),
                ],
                priority: (i % 3) + 1,
                passes: i < size / 2,
                source: "benchmark".to_string(),
                notes: String::new(),
            })
            .collect();

        let prd = PrdDocument {
            project: "benchmark".to_string(),
            branch_name: "main".to_string(),
            description: String::new(),
            user_stories: stories,
            last_synced: String::new(),
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                prd.save(black_box(Some(prd_path.as_path()))).unwrap();
            })
        });

        drop(temp);
    }

    group.finish();
}

// ============================================================================
// JSON source benchmarks
// ============================================================================

fn bench_json_source_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_source_load");

    for size in [10, 50, 100, 500].iter() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("tasks.json");

        // Create JSON tasks file
        let tasks: Vec<serde_json::Value> = (0..*size)
            .map(|i| {
                serde_json::json!({
                    "id": format!("task-{:04}", i),
                    "title": format!("Task {}", i),
                    "description": format!("Description {}", i),
                    "acceptanceCriteria": ["Done"],
                    "priority": 1,
                    "passes": false
                })
            })
            .collect();

        let json = serde_json::json!({ "tasks": tasks });
        fs::write(&json_path, serde_json::to_string(&json).unwrap()).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let tasks = load_json_tasks(black_box(Some(json_path.to_str().unwrap())));
                black_box(tasks)
            })
        });

        drop(temp);
    }

    group.finish();
}

// ============================================================================
// Prompt generation benchmarks
// ============================================================================

fn bench_prompt_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("prompt_generation");

    for size in [10, 50, 100].iter() {
        let (temp, afk_dir) = setup_config_env();
        create_prd_with_stories(&afk_dir, *size);
        create_progress(&afk_dir);

        // Need to cd to temp for generate_prompt to find files
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let config = AfkConfig::load(None).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result = generate_prompt(black_box(&config), false, None).unwrap();
                black_box(result)
            })
        });

        std::env::set_current_dir(original_dir).unwrap();
        drop(temp);
    }

    group.finish();
}

// ============================================================================
// Progress benchmarks
// ============================================================================

fn bench_progress_load(c: &mut Criterion) {
    let (temp, afk_dir) = setup_config_env();
    create_progress(&afk_dir);
    let progress_path = afk_dir.join("progress.json");

    c.bench_function("progress_load", |b| {
        b.iter(|| {
            let progress = SessionProgress::load(black_box(Some(progress_path.as_path()))).unwrap();
            black_box(progress)
        })
    });

    drop(temp);
}

fn bench_progress_save(c: &mut Criterion) {
    let temp = TempDir::new().unwrap();
    let afk_dir = temp.path().join(".afk");
    fs::create_dir_all(&afk_dir).unwrap();
    let progress_path = afk_dir.join("progress.json");

    let progress = SessionProgress::default();

    c.bench_function("progress_save", |b| {
        b.iter(|| {
            progress
                .save(black_box(Some(progress_path.as_path())))
                .unwrap();
        })
    });

    drop(temp);
}

// ============================================================================
// Startup benchmark (CLI parsing)
// ============================================================================

fn bench_cli_parsing(c: &mut Criterion) {
    use afk::cli::Cli;
    use clap::Parser;

    c.bench_function("cli_parse_go", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from(black_box(["afk", "go", "10"])).unwrap();
            black_box(cli)
        })
    });

    c.bench_function("cli_parse_run", |b| {
        b.iter(|| {
            let cli =
                Cli::try_parse_from(black_box(["afk", "run", "10", "-u", "-t", "60"])).unwrap();
            black_box(cli)
        })
    });

    c.bench_function("cli_parse_help", |b| {
        b.iter(|| {
            let result = Cli::try_parse_from(black_box(["afk", "--help"]));
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_config_load,
    bench_config_save,
    bench_prd_load,
    bench_prd_save,
    bench_json_source_load,
    bench_prompt_generation,
    bench_progress_load,
    bench_progress_save,
    bench_cli_parsing,
);

criterion_main!(benches);
