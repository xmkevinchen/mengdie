use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use second_brain::core::db::Db;
use second_brain::core::embeddings::Embedder;
use second_brain::core::ingest::ingest_file;
use second_brain::core::parser::is_ingestable;
use second_brain::core::project::infer_project_id;

#[derive(Parser)]
#[command(name = "second-brain", about = "AI-native Second Brain CLI")]
struct Cli {
    /// Database path (default: ~/.second-brain/db.sqlite)
    #[arg(long, global = true)]
    db_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Dreaming promotion pass
    Dream,

    /// Batch import AE discussion files
    Import {
        /// Directory to scan for conclusion.md, review.md, plan.md files
        #[arg(long)]
        dir: PathBuf,
    },

    /// Search memories (debugging)
    Search {
        /// Search query
        query: String,

        /// Search globally (all projects)
        #[arg(long)]
        global: bool,
    },

    /// Print observability metrics
    Stats,
}

fn main() -> anyhow::Result<()> {
    // Logging to stderr
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let db_path = cli.db_path.unwrap_or_else(Db::default_path);
    let db = Db::open(&db_path)
        .with_context(|| format!("failed to open database at {}", db_path.display()))?;

    match cli.command {
        Commands::Dream => cmd_dream(&db),
        Commands::Import { dir } => cmd_import(&db, &dir),
        Commands::Search { query, global } => cmd_search(&db, &query, global),
        Commands::Stats => cmd_stats(&db),
    }
}

fn cmd_dream(db: &Db) -> anyhow::Result<()> {
    // Run globally (all projects) — per-project scoping can be added via CLI flag later
    let result = db.run_dreaming(None)?;
    println!(
        "Dreaming complete: {} promoted out of {} eligible memories",
        result.promoted, result.total_eligible
    );
    Ok(())
}

fn cmd_import(db: &Db, dir: &PathBuf) -> anyhow::Result<()> {
    anyhow::ensure!(dir.exists(), "directory does not exist: {}", dir.display());

    eprintln!("Loading embedding model...");
    let mut embedder = Embedder::new().context("failed to initialize embedding model")?;
    eprintln!("Model loaded.");

    // Infer project_id from the import directory (not cwd), so importing
    // from an external archive gets the correct project_id.
    let project_id = infer_project_id(dir);

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = 0;

    // Walk directory recursively
    for entry in walkdir(dir)? {
        let path = entry;
        if !is_ingestable(&path) {
            continue;
        }

        match ingest_file(db, &mut embedder, &path, &project_id) {
            Ok(id) => {
                println!("  ✓ {} → {}", path.display(), id);
                imported += 1;
            }
            Err(e) => {
                if is_unique_violation(&e) {
                    eprintln!("  ⊘ {} (already imported)", path.display());
                    skipped += 1;
                } else {
                    eprintln!("  ✗ {}: {}", path.display(), e);
                    errors += 1;
                }
            }
        }
    }

    println!(
        "\nImport complete: {} imported, {} skipped (duplicates), {} errors",
        imported, skipped, errors
    );
    Ok(())
}

fn cmd_search(db: &Db, query: &str, global: bool) -> anyhow::Result<()> {
    eprintln!("Loading embedding model...");
    let mut embedder = Embedder::new().context("failed to initialize embedding model")?;
    eprintln!("Model loaded.");

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    let scope = if global { None } else { Some(project_id.as_str()) };

    let query_embedding = embedder.embed_text(query)?;
    let results = db.memory_search(query, &query_embedding, scope, 10)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    for (i, r) in results.iter().enumerate() {
        let snippet: String = r.entry.content.chars().take(100).collect();
        println!(
            "{}. [score: {:.4}] {} ({})",
            i + 1,
            r.score,
            r.entry.title,
            r.entry.knowledge_type,
        );
        println!(
            "   source: {} | entities: {} | recalled: {}x",
            r.entry.source_file, r.entry.entities, r.entry.recall_count,
        );
        println!("   {}", snippet.replace('\n', " "));
        println!();
    }

    Ok(())
}

fn cmd_stats(db: &Db) -> anyhow::Result<()> {
    let s = db.stats()?;
    println!("Second Brain Stats:");
    println!("  Total memories:    {}", s.total);
    println!("  Valid (active):    {}", s.valid);
    println!("  Long-term:         {}", s.longterm);
    println!("  Recalled (≥1x):    {}", s.recalled);

    let metrics = db.list_metrics()?;
    let get = |key: &str| -> i64 {
        metrics.iter().find(|(k, _)| k == key).map(|(_, v)| *v).unwrap_or(0)
    };

    let search_count = get("search_count");
    let search_nonempty = get("search_nonempty_count");
    let ingest_count = get("ingest_count");
    let conflict_count = get("conflict_count");

    if search_count > 0 {
        let injection_rate = (search_nonempty as f64 / search_count as f64) * 100.0;
        println!("  Context injection rate: {injection_rate:.1}% ({search_nonempty}/{search_count} non-empty)");
    }
    if ingest_count > 0 {
        let conflict_rate = (conflict_count as f64 / ingest_count as f64) * 100.0;
        println!("  Conflict detection rate: {conflict_rate:.1}% ({conflict_count}/{ingest_count} ingestions)");
    }

    Ok(())
}

/// Check if an error is a SQLite UNIQUE constraint violation.
fn is_unique_violation(err: &anyhow::Error) -> bool {
    // Walk the error chain looking for rusqlite constraint violation
    for cause in err.chain() {
        if let Some(rusqlite_err) = cause.downcast_ref::<rusqlite::Error>() {
            if let rusqlite::Error::SqliteFailure(ffi_err, _) = rusqlite_err {
                // SQLITE_CONSTRAINT = 19, extended code SQLITE_CONSTRAINT_UNIQUE = 2067
                if ffi_err.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                    return true;
                }
            }
        }
    }
    false
}

/// Simple recursive directory walk, collecting file paths.
fn walkdir(dir: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk_recursive(dir, &mut files)?;
    Ok(files)
}

fn walk_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}
