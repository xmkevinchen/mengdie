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

        /// Dry-run the BL-008 decay pass: compute `decay_floor_breaches` and
        /// report the would-demote list WITHOUT clearing any `is_longterm`
        /// flags. Incompatible with `--synthesize --dry-run` (the two dry-run
        /// modes target different passes). See docs/operations/dreaming-decay.md
        /// for the operator procedure.
        #[arg(long)]
        decay_dry_run: bool,

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

    /// Read-only audit of a synthesis row + its source memories.
    /// Scaffolding for future Options 2/3 ship-gate data collection
    /// (plan 017, discussion 022) — operator eyeballs fidelity.
    SynthesisAudit {
        /// Synthesis memory id to inspect.
        id: String,
    },
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
            decay_dry_run,
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
                decay_dry_run,
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
        Commands::SynthesisAudit { id } => cmd_synthesis_audit(&db, &id),
    }
}

/// Format the structured-JSON event for machine consumers. Single-line
/// `{"event":"dreaming_pass",...}` on stderr — stable contract.
/// Emitted directly via `eprintln!` to bypass tracing's default formatter
/// (which otherwise wraps the JSON in a log line with timestamp prefix).
fn format_structured_json(
    result: &mengdie::core::dreaming::DreamingResult,
    decay_dry_run: bool,
) -> String {
    let v = serde_json::json!({
        "schema_version": 1,
        "event": "dreaming_pass",
        "promoted": result.promoted,
        "demoted": result.demoted,
        "decay_floor_breaches": result.decay_floor_breaches,
        "avg_effective_before": result.avg_effective_score_before,
        "avg_effective_after": result.avg_effective_score_after,
        "dry_run": decay_dry_run,
        "breaches": result.breached_ids,
    });
    v.to_string()
}

/// Format the single-line human-readable dreaming-pass summary. Extracted
/// for unit testing — callers include the AC5 regex contract.
fn format_dreaming_line(
    result: &mengdie::core::dreaming::DreamingResult,
    decay_dry_run: bool,
) -> String {
    let demote_phrase = if decay_dry_run {
        format!("{} would-demote (DRY RUN)", result.decay_floor_breaches)
    } else {
        format!("{} demoted", result.demoted)
    };
    format!(
        "Dreaming pass: {} promoted, {} ({} floor breaches, avg effective {:.3} → {:.3})",
        result.promoted,
        demote_phrase,
        result.decay_floor_breaches,
        result.avg_effective_score_before,
        result.avg_effective_score_after,
    )
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
    decay_dry_run: bool,
    project: Option<&str>,
) -> anyhow::Result<()> {
    use mengdie::core::dreaming::{run_synthesis_pass, DreamingConfig};
    use mengdie::core::llm::build_provider;

    // Flag-conflict guard (BL-008 AC5): the two dry-run modes target
    // different passes (`--dry-run` previews synthesis; `--decay-dry-run`
    // previews demotion). Combining them would be ambiguous — refuse.
    if decay_dry_run && dry_run {
        anyhow::bail!(
            "--decay-dry-run is not compatible with --synthesize --dry-run. \
             The two dry-run modes target different passes (synthesis vs decay). \
             Run them in separate invocations. See docs/operations/dreaming-decay.md."
        );
    }

    let config = DreamingConfig {
        min_recall,
        min_relevance,
        window_days,
    };
    // Dreaming pass: promotion + decay/demotion (BL-008). `write_demotions`
    // flipped by the `--decay-dry-run` flag.
    let write_demotions = !decay_dry_run;
    let result = db.run_dreaming_with_config(None, &config, None, write_demotions)?;

    // Human-readable line (AC5 loose regex tolerates whitespace + decimal variation).
    println!("{}", format_dreaming_line(&result, decay_dry_run));

    // Structured-JSON line for machine consumption (AC5 blocker from
    // codex). Emitted as a RAW JSON line on stderr via `eprintln!` — NOT
    // through `tracing` — because the default tracing formatter wraps
    // field values in a human-readable log line with ISO timestamps and
    // the `structured=` prefix, which breaks stderr parsers expecting a
    // bare `{"event":"dreaming_pass",...}` line. `scripts/verify-decay.sh`
    // (Step 5) greps for exactly that shape.
    let structured_line = format_structured_json(&result, decay_dry_run);
    eprintln!("{structured_line}");

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
    let pair_skip_pct = (syn.pair_clusters_skipped * 100)
        .checked_div(syn.pair_clusters_processed)
        .unwrap_or(0);
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
            "{:<8} {:<40} {:<12} {:<12} {:>6} {:<4} Source",
            "ID", "Title", "Knowledge", "Origin", "Recall", "LT"
        );
        println!("{}", "-".repeat(100));
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
                "{:<8} {:<40} {:<12} {:<12} {:>6} {:<4} {}",
                short_id, title, e.knowledge_type, e.source_type, e.recall_count, lt, source_short
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
        println!("{}", format_search_result(r, i + 1));
        println!();
    }

    Ok(())
}

/// Format a single `SearchResultItem` for `mengdie search` output.
/// Extracted for testability (plan 017 Step 4, AC5). Returns a 3-line
/// string (header, metadata, snippet) joined by `\n`. Caller appends a
/// blank line between results.
///
/// Plan 017 Step 4: the metadata line includes `type: {source_type}`
/// (the provenance-axis field from discussion 022 Option 4 reinterpretation)
/// alongside the existing `source:` (file path), `entities`, and
/// `recalled: Nx`. `type:` is deliberately named distinct from `source:`
/// (which is the file path) to avoid operator confusion.
pub(crate) fn format_search_result(
    r: &mengdie::core::search::SearchResult,
    index: usize,
) -> String {
    let snippet: String = r.entry.content.chars().take(100).collect();
    format!(
        "{index}. [score: {:.4}] {} ({})\n   type: {} | source: {} | entities: {} | recalled: {}x\n   {}",
        r.score,
        r.entry.title,
        r.entry.knowledge_type,
        r.entry.source_type,
        r.entry.source_file,
        r.entry.entities,
        r.entry.recall_count,
        snippet.replace('\n', " "),
    )
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

/// Audit a synthesis row + its linked source memories (plan 017 Step 3).
///
/// Read-only. Prints the synthesis's title + full content, then each
/// linked source memory with source_type + first 200 chars of content.
/// Intended as scaffolding for future Options 2/3 ship-gate data
/// collection (discussion 022). Does NOT embed any fidelity judgment —
/// the operator reads both and decides.
fn cmd_synthesis_audit(db: &Db, id: &str) -> anyhow::Result<()> {
    let (synthesis, sources) = db.get_synthesis_with_sources(id)?;

    println!("=== Synthesis ===");
    println!("ID:      {}", synthesis.id);
    println!("Title:   {}", synthesis.title);
    println!(
        "Project: {} | Entities: {} | Recalled: {}x | Long-term: {}",
        synthesis.project_id, synthesis.entities, synthesis.recall_count, synthesis.is_longterm
    );
    println!("Content:");
    for line in synthesis.content.lines() {
        println!("  {line}");
    }

    let n = sources.len();
    println!();
    println!("Sources ({n}):");
    for (i, src) in sources.iter().enumerate() {
        println!();
        println!("--- Source {}/{} ---", i + 1, n);
        println!("ID:     {}", src.id);
        println!("Type:   {}", src.source_type);
        println!("Title:  {}", src.title);
        let preview: String = src.content.chars().take(200).collect();
        let truncated = src.content.chars().count() > 200;
        println!(
            "Preview: {}{}",
            preview.replace('\n', " "),
            if truncated { " […]" } else { "" }
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mengdie::core::db::MemoryEntry;
    use mengdie::core::dreaming::DreamingResult;
    use mengdie::core::search::SearchResult;
    use regex::Regex;

    fn sample_search_result(source_type: &str, knowledge_type: &str) -> SearchResult {
        SearchResult {
            entry: MemoryEntry {
                id: "abc123def".to_string(),
                project_id: "proj".to_string(),
                source_file: "docs/plans/017.md".to_string(),
                source_type: source_type.to_string(),
                knowledge_type: knowledge_type.to_string(),
                title: "Sample title".to_string(),
                content: "Content that is longer than 100 chars to exercise the 100-char \
                          preview cap in format_search_result for the snippet field."
                    .to_string(),
                entities: "x,y".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: None,
                superseded_by: None,
                recall_count: 3,
                avg_relevance: 0.7,
                last_recalled: None,
                embedding: None,
                embedding_dim: None,
                is_longterm: false,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            score: 0.4321,
        }
    }

    #[test]
    fn format_search_result_includes_source_type_synthesis() {
        let r = sample_search_result("synthesis", "factual");
        let s = format_search_result(&r, 1);
        assert!(
            s.contains("type: synthesis"),
            "expected source_type 'synthesis' surfaced in metadata line, got:\n{s}"
        );
    }

    #[test]
    fn format_search_result_includes_source_type_conclusion() {
        let r = sample_search_result("conclusion", "decisional");
        let s = format_search_result(&r, 1);
        assert!(
            s.contains("type: conclusion"),
            "expected source_type 'conclusion' surfaced in metadata line, got:\n{s}"
        );
    }

    #[test]
    fn format_search_result_distinguishes_type_from_source() {
        // `type:` (source_type enum) must NOT be confused with `source:` (file path).
        // Both appear on the metadata line; plan 017 Step 4 picked distinct labels
        // to avoid operator confusion.
        let r = sample_search_result("review", "experiential");
        let s = format_search_result(&r, 1);
        assert!(s.contains("type: review"));
        assert!(s.contains("source: docs/plans/017.md"));
    }

    #[test]
    fn format_search_result_snippet_is_capped_and_single_line() {
        let r = sample_search_result("conclusion", "decisional");
        let s = format_search_result(&r, 1);
        // The snippet uses `chars().take(100)` + newline replacement; verify
        // no newlines leaked into the third line of output and the preview
        // stays bounded.
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines.len(), 3, "expected 3 lines, got:\n{s}");
        // Third line ("   <snippet>"): 3-space indent + up to 100 chars.
        assert!(lines[2].starts_with("   "));
        assert!(lines[2].len() <= 3 + 100);
    }

    fn sample_result(demoted: usize, breaches: usize, before: f64, after: f64) -> DreamingResult {
        DreamingResult {
            promoted: 1,
            candidates_not_promoted: 0,
            total_eligible: 10,
            demoted,
            avg_effective_score_before: before,
            avg_effective_score_after: after,
            decay_floor_breaches: breaches,
            breached_ids: vec![],
        }
    }

    #[test]
    fn format_dreaming_line_live_matches_ac5_regex() {
        // AC5 regex: Dreaming pass:\s+\d+\s+promoted,\s+\d+\s+demoted\s+\(...
        let line = format_dreaming_line(&sample_result(2, 2, 0.421, 0.500), false);
        let re = Regex::new(
            r"^Dreaming pass:\s+\d+\s+promoted,\s+\d+\s+demoted\s+\(\d+\s+floor breaches,\s+avg effective\s+\d+\.\d+\s+→\s+\d+\.\d+\)$",
        )
        .unwrap();
        assert!(re.is_match(&line), "live line didn't match: {line}");
    }

    #[test]
    fn format_dreaming_line_dry_run_matches_ac5_regex() {
        let line = format_dreaming_line(&sample_result(0, 3, 0.421, 0.421), true);
        let re = Regex::new(
            r"^Dreaming pass:\s+\d+\s+promoted,\s+\d+\s+would-demote\s+\(DRY RUN\)\s+\(\d+\s+floor breaches,\s+avg effective\s+\d+\.\d+\s+→\s+\d+\.\d+\)$",
        )
        .unwrap();
        assert!(re.is_match(&line), "dry-run line didn't match: {line}");
    }

    #[test]
    fn format_dreaming_line_dry_run_uses_breach_count_not_demoted() {
        // In dry-run, `demoted = 0` but `decay_floor_breaches` reflects what WOULD demote.
        // The human phrase must surface the breach count (`would-demote`), not the
        // always-zero `demoted`.
        let line = format_dreaming_line(&sample_result(0, 7, 0.421, 0.421), true);
        assert!(line.contains("7 would-demote"));
        assert!(!line.contains("0 would-demote"));
    }

    #[test]
    fn format_structured_json_parses_with_all_required_fields() {
        let mut r = sample_result(2, 3, 0.421, 0.500);
        r.breached_ids = vec!["BL-X".to_string(), "BL-Y".to_string()];
        let line = format_structured_json(&r, true);
        // Must parse as valid JSON
        let v: serde_json::Value = serde_json::from_str(&line).expect("valid JSON");
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["event"], "dreaming_pass");
        assert_eq!(v["promoted"], 1);
        assert_eq!(v["demoted"], 2);
        assert_eq!(v["decay_floor_breaches"], 3);
        assert_eq!(v["dry_run"], true);
        assert!(v["avg_effective_before"].is_number());
        assert!(v["avg_effective_after"].is_number());
        assert_eq!(v["breaches"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn format_structured_json_emits_schema_version_1() {
        // Plan 015: schema_version locks the dreaming_pass contract. Any bump
        // requires coordinated update of docs/schemas/dreaming_pass.json and
        // scripts/verify-decay.sh field whitelist.
        let r = sample_result(0, 0, 0.0, 0.0);
        let line = format_structured_json(&r, false);
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            v["schema_version"], 1,
            "schema_version must be 1 (plan 015)"
        );
    }

    #[test]
    fn format_structured_json_breaches_array_length_matches_decay_floor_breaches() {
        // Invariant: JSON `breaches.len() == decay_floor_breaches`.
        let mut r = sample_result(2, 2, 0.4, 0.5);
        r.breached_ids = vec!["a".into(), "b".into()];
        let line = format_structured_json(&r, false);
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            v["breaches"].as_array().unwrap().len() as u64,
            v["decay_floor_breaches"].as_u64().unwrap()
        );
    }

    #[test]
    fn format_dreaming_line_shape_stable_on_edge_counts() {
        // Zeros and large numbers must still round-trip through the regex.
        let zero = format_dreaming_line(&sample_result(0, 0, 0.0, 0.0), false);
        let huge = format_dreaming_line(&sample_result(9999, 9999, 0.999, 0.999), false);
        let re = Regex::new(
            r"^Dreaming pass:\s+\d+\s+promoted,\s+\d+\s+demoted\s+\(\d+\s+floor breaches,\s+avg effective\s+\d+\.\d+\s+→\s+\d+\.\d+\)$",
        )
        .unwrap();
        assert!(re.is_match(&zero));
        assert!(re.is_match(&huge));
    }
}
