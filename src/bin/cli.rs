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
        /// Minimum recall count for promotion
        #[arg(long, default_value_t = mengdie::core::dreaming::DEFAULT_MIN_RECALL)]
        min_recall: i64,

        /// Minimum average relevance for promotion
        #[arg(long, default_value_t = mengdie::core::dreaming::DEFAULT_MIN_RELEVANCE)]
        min_relevance: f64,

        /// Recency window in days — last_recalled must be within this window
        #[arg(long, default_value_t = mengdie::core::dreaming::DEFAULT_WINDOW_DAYS)]
        window_days: i64,

        /// Run LLM synthesis after promotion (opt-in: makes network calls + writes synthesis rows).
        #[arg(long)]
        synthesize: bool,

        /// Cluster threshold override. Default tracks clustering::DEFAULT_THRESHOLD;
        /// see docs/backlog/BL-clustering-validation.md for why 0.75.
        #[arg(long, default_value_t = mengdie::core::clustering::DEFAULT_THRESHOLD)]
        threshold: f32,

        /// Minimum cluster size for synthesis.
        #[arg(long, default_value_t = mengdie::core::clustering::DEFAULT_MIN_SIZE)]
        min_cluster_size: usize,

        /// Maximum cluster size — oversized clusters are truncated to this many members
        /// before prompt building (bounds the LLM token budget).
        #[arg(long, default_value_t = 20)]
        max_cluster_size: usize,

        /// Show synthesis prompts without making LLM calls or writing rows.
        /// Requires `--synthesize` (review feedback: previously `--dry-run`
        /// silently triggered the synthesis path even without `--synthesize`,
        /// which surprised users expecting a promotion-pass preview).
        #[arg(long)]
        dry_run: bool,

        /// Project scope for synthesis (default: all projects).
        #[arg(long)]
        project: Option<String>,
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

    /// Rename a project_id in the database
    Rename {
        /// Source project_id to rename from (or --list to show all projects)
        from: Option<String>,

        /// Target project_id to rename to
        to: Option<String>,

        /// List all project_ids with memory counts
        #[arg(long)]
        list: bool,

        /// Preview what would happen without writing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Print observability metrics
    Stats,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
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
        Commands::Dream {
            min_recall,
            min_relevance,
            window_days,
            synthesize,
            threshold,
            min_cluster_size,
            max_cluster_size,
            dry_run,
            project,
        } => {
            cmd_dream(
                &db,
                min_recall,
                min_relevance,
                window_days,
                synthesize,
                threshold,
                min_cluster_size,
                max_cluster_size,
                dry_run,
                project.as_deref(),
            )
            .await
        }
        Commands::Import { dir, dry_run } => cmd_import(&db, &dir, dry_run),
        Commands::Rename {
            from,
            to,
            list,
            dry_run,
            yes,
        } => cmd_rename(&db, from, to, list, dry_run, yes),
        Commands::List { global, format } => cmd_list(&db, global, &format),
        Commands::Search {
            query,
            global,
            limit,
            min_score,
        } => cmd_search(&db, &query, global, limit, min_score),
        Commands::Stats => cmd_stats(&db),
    }
}

#[allow(clippy::too_many_arguments)]
async fn cmd_dream(
    db: &Db,
    min_recall: i64,
    min_relevance: f64,
    window_days: i64,
    synthesize: bool,
    threshold: f32,
    min_cluster_size: usize,
    max_cluster_size: usize,
    dry_run: bool,
    project: Option<&str>,
) -> anyhow::Result<()> {
    use mengdie::core::dreaming::{run_synthesis_pass, DreamingConfig};
    use mengdie::core::llm::build_provider;

    let config = DreamingConfig {
        min_recall,
        min_relevance,
        window_days,
    };
    // Promotion pass (unchanged semantics: all projects).
    let result = db.run_dreaming_with_config(None, &config)?;
    println!(
        "Dreaming complete: {} promoted out of {} eligible memories (thresholds: recall≥{}, relevance≥{:.2}, window={}d)",
        result.promoted, result.total_eligible, min_recall, min_relevance, window_days
    );

    // Review feedback: `--dry-run` alone previously silently ran the synthesis
    // path. New contract: `--dry-run` requires explicit `--synthesize` to make
    // the operator's intent unambiguous. Plain `mengdie dream` still only
    // runs the promotion pass (no LLM calls, no writes).
    if dry_run && !synthesize {
        anyhow::bail!(
            "--dry-run requires --synthesize (dry-run is a preview of the synthesis pass, \
             not the promotion pass). Re-run with both flags if you want to inspect prompts."
        );
    }
    if !synthesize {
        return Ok(());
    }

    let cfg = mengdie::core::config::MengdieConfig::load_from_process_env()?;
    let provider = build_provider(&cfg.llm)?;
    let syn = run_synthesis_pass(
        db,
        project,
        provider.as_ref(),
        threshold,
        min_cluster_size,
        max_cluster_size,
        dry_run,
    )
    .await?;

    // Pair-cluster skip percentage: numerator is the pair-cluster subset of
    // LLM skips (NOT total LLM skips across all cluster sizes). The prior
    // impl displayed total-skips / pair-count, which on the 2026-04-19 run
    // produced "11/11 pair-clusters = 100%" when the true rate was 3/11.
    // Plan 012 / BL-synthesis-cli-skip-metric fix. Format string shape
    // stays "{S_all} LLM-skipped ({S_pair}/{P} pair-clusters = {X}%)" —
    // if a downstream parser depends on this shape, update this comment.
    let pair_skip_pct = if syn.pair_clusters_processed == 0 {
        0
    } else {
        (syn.pair_clusters_skipped * 100) / syn.pair_clusters_processed
    };
    println!(
        "Synthesis: {} syntheses created from {} clusters \
         ({} residuals skipped, {} LLM-skipped ({}/{} pair-clusters = {}%), \
         {} LLM-call errors, {} parse errors, {} memories truncated)",
        syn.syntheses_created,
        syn.clusters_processed,
        syn.residuals_skipped,
        syn.syntheses_llm_skipped,
        syn.pair_clusters_skipped,
        syn.pair_clusters_processed,
        pair_skip_pct,
        syn.llm_call_errors,
        syn.parse_errors,
        syn.memories_truncated
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
                    println!(
                        "    ! conflict: \"{}\" -- {}",
                        conflict.existing_title, conflict.reason
                    );
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

fn cmd_rename(
    db: &Db,
    from: Option<String>,
    to: Option<String>,
    list: bool,
    dry_run: bool,
    yes: bool,
) -> anyhow::Result<()> {
    if list {
        let projects = db.list_projects()?;
        if projects.is_empty() {
            println!("No projects found.");
        } else {
            println!("{:<40} {:>6}", "Project ID", "Count");
            println!("{}", "-".repeat(48));
            for (id, count) in &projects {
                println!("{:<40} {:>6}", id, count);
            }
        }
        return Ok(());
    }

    let from = from.ok_or_else(|| {
        anyhow::anyhow!("missing <from> argument (use --list to see project_ids)")
    })?;
    let to = to.ok_or_else(|| anyhow::anyhow!("missing <to> argument"))?;

    if from == to {
        anyhow::bail!("source and target project_id are the same: '{}'", from);
    }

    if dry_run {
        let (would_rename, would_merge) = db.rename_project_dry_run(&from, &to)?;
        if would_rename == 0 && would_merge == 0 {
            println!("No memories found under '{}'.", from);
        } else {
            println!(
                "Dry run: would rename {} memories from '{}' to '{}'",
                would_rename, from, to
            );
            if would_merge > 0 {
                println!(
                    "         would merge {} duplicates (same content already under '{}')",
                    would_merge, to
                );
            }
        }
        return Ok(());
    }

    // Count first for confirmation
    let (would_rename, would_merge) = db.rename_project_dry_run(&from, &to)?;
    if would_rename == 0 && would_merge == 0 {
        println!("No memories found under '{}'.", from);
        return Ok(());
    }

    if !yes {
        if would_merge > 0 {
            eprint!(
                "Rename {} memories from '{}' to '{}' ({} duplicates will be merged)? [y/N] ",
                would_rename + would_merge,
                from,
                to,
                would_merge
            );
        } else {
            eprint!(
                "Rename {} memories from '{}' to '{}'? [y/N] ",
                would_rename, from, to
            );
        }
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let (renamed, merged) = db.rename_project(&from, &to)?;
    println!("Renamed {} memories from '{}' to '{}'.", renamed, from, to);
    if merged > 0 {
        println!(
            "Merged {} duplicates (deleted from '{}', kept under '{}').",
            merged, from, to
        );
    }
    eprintln!("\nNote: restart mengdie-mcp if it's running — cached project_id is now stale.");

    Ok(())
}

fn cmd_list(db: &Db, global: bool, format: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    let scope = if global {
        None
    } else {
        Some(project_id.as_str())
    };

    let entries = db.list_memories(scope)?;

    if entries.is_empty() {
        println!("No memories found.");
        return Ok(());
    }

    if format == "json" {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
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
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        // Table format
        println!(
            "{:<8} {:<40} {:<12} {:>6} {:<4} Source",
            "ID", "Title", "Type", "Recall", "LT"
        );
        println!("{}", "-".repeat(90));
        for e in &entries {
            let short_id = if e.id.len() > 8 { &e.id[..8] } else { &e.id };
            let title = if e.title.len() > 40 {
                format!("{}...", &e.title[..37])
            } else {
                e.title.clone()
            };
            let lt = if e.is_longterm { "Y" } else { "N" };
            let source = if e.source_file.is_empty() {
                "-"
            } else {
                &e.source_file
            };
            let source_short = if source.len() > 30 {
                &source[source.len() - 30..]
            } else {
                source
            };
            println!(
                "{:<8} {:<40} {:<12} {:>6} {:<4} {}",
                short_id, title, e.knowledge_type, e.recall_count, lt, source_short
            );
        }
        println!("\n{} memories total", entries.len());
    }
    Ok(())
}

fn cmd_search(
    db: &Db,
    query: &str,
    global: bool,
    limit: usize,
    min_score: Option<f64>,
) -> anyhow::Result<()> {
    eprintln!("Loading embedding model...");
    let mut embedder = Embedder::new().context("failed to initialize embedding model")?;
    eprintln!("Model loaded.");

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_id = infer_project_id(&cwd);
    let scope = if global {
        None
    } else {
        Some(project_id.as_str())
    };

    let query_embedding = embedder.embed_text(query)?;
    let results: Vec<_> = db
        .memory_search(query, &query_embedding, scope, limit)?
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
        metrics
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0)
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
        if let Some(rusqlite::Error::SqliteFailure(ffi_err, _)) =
            cause.downcast_ref::<rusqlite::Error>()
        {
            // SQLITE_CONSTRAINT = 19, extended code SQLITE_CONSTRAINT_UNIQUE = 2067
            if ffi_err.code == rusqlite::ffi::ErrorCode::ConstraintViolation {
                return true;
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
