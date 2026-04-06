use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use mengdie::core::db::Db;
use mengdie::core::embeddings::Embedder;
use mengdie::core::ingest::ingest_file;
use mengdie::core::parser::is_ingestable;
use mengdie::core::project::infer_project_id;

#[derive(Parser)]
#[command(name = "mengdie", about = "Mengdie — AI-native knowledge memory CLI")]
struct Cli {
    /// Database path (default: ~/.mengdie/db.sqlite)
    #[arg(long, global = true)]
    db_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Dreaming promotion pass
    Dream {
        /// Minimum recall count for promotion (default: 3)
        #[arg(long, default_value = "3")]
        min_recall: i64,

        /// Minimum average relevance for promotion (default: 0.65)
        #[arg(long, default_value = "0.65")]
        min_relevance: f64,

        /// Recency window in days — last_recalled must be within this window (default: 14)
        #[arg(long, default_value = "14")]
        window_days: i64,
    },

    /// Batch import AE discussion files
    Import {
        /// Directory to scan for conclusion.md, review.md, plan.md files
        #[arg(long)]
        dir: PathBuf,

        /// Preview what would be imported without writing to the database
        #[arg(long)]
        dry_run: bool,
    },

    /// List all memories in the database
    List {
        /// Show memories from all projects (default: current project only)
        #[arg(long)]
        global: bool,

        /// Output format: table (default) or json
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Search memories (debugging)
    Search {
        /// Search query
        query: String,

        /// Search globally (all projects)
        #[arg(long)]
        global: bool,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum score threshold (results below this are filtered out)
        #[arg(long)]
        min_score: Option<f64>,
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
        Commands::Dream { min_recall, min_relevance, window_days } => cmd_dream(&db, min_recall, min_relevance, window_days),
        Commands::Import { dir, dry_run } => cmd_import(&db, &dir, dry_run),
        Commands::List { global, format } => cmd_list(&db, global, &format),
        Commands::Search { query, global, limit, min_score } => cmd_search(&db, &query, global, limit, min_score),
        Commands::Stats => cmd_stats(&db),
    }
}

fn cmd_dream(db: &Db, min_recall: i64, min_relevance: f64, window_days: i64) -> anyhow::Result<()> {
    use mengdie::core::dreaming::DreamingConfig;

    let config = DreamingConfig { min_recall, min_relevance, window_days };
    // Run globally (all projects) — per-project scoping can be added via CLI flag later
    let result = db.run_dreaming_with_config(None, &config)?;
    println!(
        "Dreaming complete: {} promoted out of {} eligible memories (thresholds: recall≥{}, relevance≥{:.2}, window={}d)",
        result.promoted, result.total_eligible, min_recall, min_relevance, window_days
    );
    Ok(())
}

fn cmd_import(db: &Db, dir: &PathBuf, dry_run: bool) -> anyhow::Result<()> {
    anyhow::ensure!(dir.exists(), "directory does not exist: {}", dir.display());

    let project_id = infer_project_id(dir);

    // Collect ingestable files first
    let files: Vec<_> = walkdir(dir)?
        .into_iter()
        .filter(|p| is_ingestable(p))
        .collect();

    if dry_run {
        println!("Dry run — no changes will be made.\n");
        println!("Project ID: {}", project_id);
        println!("Files to import: {}\n", files.len());
        for path in &files {
            println!("  + {}", path.display());
        }
        if files.is_empty() {
            println!("  (no ingestable files found)");
        }
        return Ok(());
    }

    eprintln!("Loading embedding model...");
    let mut embedder = Embedder::new().context("failed to initialize embedding model")?;
    eprintln!("Model loaded.");

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = 0;
    let mut conflicts_found = 0;

    for path in &files {
        match ingest_file(db, &mut embedder, path, &project_id) {
            Ok(result) => {
                println!("  + {} -> {}", path.display(), result.entry_id);
                for conflict in &result.conflicts {
                    println!("    ! conflict: \"{}\" -- {}", conflict.existing_title, conflict.reason);
                    conflicts_found += 1;
                }
                imported += 1;
            }
            Err(e) => {
                if is_unique_violation(&e) {
                    eprintln!("  = {} (already imported)", path.display());
                    skipped += 1;
                } else {
                    eprintln!("  x {}: {}", path.display(), e);
                    errors += 1;
                }
            }
        }
    }

    println!(
        "\nImport complete: {} imported, {} skipped (duplicates), {} errors, {} conflicts detected",
        imported, skipped, errors, conflicts_found
    );
    Ok(())
}

fn cmd_list(db: &Db, global: bool, format: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    let scope = if global { None } else { Some(project_id.as_str()) };

    let entries = db.list_memories(scope)?;

    if entries.is_empty() {
        println!("No memories found.");
        return Ok(());
    }

    if format == "json" {
        let items: Vec<serde_json::Value> = entries.iter().map(|e| {
            serde_json::json!({
                "id": e.id,
                "project_id": e.project_id,
                "title": e.title,
                "knowledge_type": e.knowledge_type,
                "source_file": e.source_file,
                "entities": e.entities,
                "recall_count": e.recall_count,
                "is_longterm": e.is_longterm,
                "valid_from": e.valid_from,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        // Table format
        println!("{:<8} {:<40} {:<12} {:>6} {:<4} {}",
            "ID", "Title", "Type", "Recall", "LT", "Source");
        println!("{}", "-".repeat(90));
        for e in &entries {
            let short_id = if e.id.len() > 8 { &e.id[..8] } else { &e.id };
            let title = if e.title.len() > 40 { format!("{}...", &e.title[..37]) } else { e.title.clone() };
            let lt = if e.is_longterm { "Y" } else { "N" };
            let source = if e.source_file.is_empty() { "-" } else { &e.source_file };
            let source_short = if source.len() > 30 { &source[source.len()-30..] } else { source };
            println!("{:<8} {:<40} {:<12} {:>6} {:<4} {}",
                short_id, title, e.knowledge_type, e.recall_count, lt, source_short);
        }
        println!("\n{} memories total", entries.len());
    }
    Ok(())
}

fn cmd_search(db: &Db, query: &str, global: bool, limit: usize, min_score: Option<f64>) -> anyhow::Result<()> {
    eprintln!("Loading embedding model...");
    let mut embedder = Embedder::new().context("failed to initialize embedding model")?;
    eprintln!("Model loaded.");

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    let scope = if global { None } else { Some(project_id.as_str()) };

    let query_embedding = embedder.embed_text(query)?;
    let results: Vec<_> = db.memory_search(query, &query_embedding, scope, limit)?
        .into_iter()
        .filter(|r| min_score.is_none_or(|ms| r.score >= ms))
        .collect();

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
    println!("Mengdie Stats:");
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
    } else {
        println!("  Context injection rate: no searches yet");
    }
    if ingest_count > 0 {
        let conflict_rate = (conflict_count as f64 / ingest_count as f64) * 100.0;
        println!("  Conflict detection rate: {conflict_rate:.1}% ({conflict_count}/{ingest_count} ingestions)");
    } else {
        println!("  Conflict detection rate: no ingestions yet");
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
    for entry in walkdir::WalkDir::new(dir).follow_links(false) {
        match entry {
            Ok(e) if e.file_type().is_file() => files.push(e.into_path()),
            Ok(_) => {} // directories, symlinks — skip
            Err(e) => {
                tracing::warn!(error = %e, "walkdir error, skipping entry");
            }
        }
    }
    Ok(files)
}
