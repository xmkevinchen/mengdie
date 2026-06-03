use std::path::Path;

use anyhow::Context;

use super::contradiction::{Conflict, ConflictScanOutcome};
use super::db::{Db, NewMemory};
use super::embeddings::{embedding_to_blob, Embed, EmbeddingContext};
use super::parser::{parse_ae_file, ParsedDocument};

/// Result of ingesting a document, including any detected contradictions.
pub struct IngestResult {
    pub entry_id: String,
    pub conflicts: Vec<Conflict>,
    /// F-016 / BL-056: `resolves` ids that matched no live row and were NOT
    /// superseded (typo / cross-project / already-tombstoned / self-ref).
    /// Always `[]` for plain inserts (no resolves).
    pub unmatched_resolves: Vec<String>,
    /// F-016 / BL-058: `true` when the pre-insert conflict scan failed or
    /// degraded (whole-call error, or a per-entity lookup skipped), so an empty
    /// `conflicts` list is NOT a reliable "no conflicts" signal.
    pub conflict_scan_degraded: bool,
}

/// Caller-supplied metadata for a content ingest. Fields match the subset
/// of `NewMemory` that surfaces (file-parser, MCP tool, future internal
/// callers) populate distinctly from the content + project_id + embedding.
///
/// `entities` is the **raw** comma-separated tag string from the caller;
/// the shared private prep helper lowercases it before storage so file-
/// ingest and MCP-ingest paths converge on the same case-normalized form
/// (fixes the F-002 plan-time entity-case asymmetry bug per discussion
/// 001 Topic 4).
///
/// Plan F-003 Step 5 / discussion 001 Topic 4 (two public entries +
/// shared private prep helper).
pub struct IngestMetadata {
    pub title: String,
    pub entities: String,
    pub source_file: String,
    pub source_type: String,
    pub knowledge_type: String,
    pub is_longterm: bool,
}

/// Shared private prep helper for the `ingest_text` family. Generates the
/// embedding (with metadata-in-chunk encoding), lowercases entities,
/// runs the contradiction check, and constructs the `NewMemory` ready to
/// hand to either `Db::insert_memory` (`ingest_text`) or
/// `Db::insert_memory_resolving` (`ingest_text_with_resolves`).
///
/// Both file-parsed (`ingest_document` → `ingest_text`) and MCP-direct
/// (`mcp_tools::ingest` → `ingest_text` / `ingest_text_with_resolves`)
/// paths converge here, eliminating the F-002-era divergence in
/// entity-case handling and embed-fail mode (entity-case lowercased once
/// here; embedding errors hard-propagate via `?` for both surfaces).
///
/// Returns `(NewMemory, Vec<Conflict>, conflict_scan_degraded)`. The caller
/// decides whether to invoke `insert_memory` or `insert_memory_resolving` next.
///
/// Thin wrapper over `prepare_memory_with_scan` passing the real
/// `Db::check_contradictions` as the scan.
fn prepare_memory(
    db: &Db,
    embedder: &mut dyn Embed,
    content: &str,
    metadata: &IngestMetadata,
    project_id: &str,
) -> anyhow::Result<(NewMemory, Vec<Conflict>, bool)> {
    prepare_memory_with_scan(
        embedder,
        content,
        metadata,
        project_id,
        |entities, embedding, knowledge_type, pid| {
            db.check_contradictions(entities, embedding, knowledge_type, pid)
        },
    )
}

/// F-016 / BL-058: injectable-scan core of `prepare_memory`. The `scan` closure
/// is the contradiction check; production passes `Db::check_contradictions`,
/// tests pass a closure that can force an `Err` or a degraded
/// `ConflictScanOutcome` — the scan-error path is otherwise untestable without
/// also breaking the insert (which would prevent verifying "scan degraded but
/// ingest still succeeds").
///
/// Degraded has two triggers: (a) the scan returns `Ok` with `degraded: true`
/// (a per-entity lookup was skipped); (b) the scan returns `Err` (whole-call
/// failure) — caught here, logged, and mapped to `degraded = true` with empty
/// conflicts so the ingest still proceeds best-effort.
fn prepare_memory_with_scan<F>(
    embedder: &mut dyn Embed,
    content: &str,
    metadata: &IngestMetadata,
    project_id: &str,
    scan: F,
) -> anyhow::Result<(NewMemory, Vec<Conflict>, bool)>
where
    F: FnOnce(&[String], Option<&[f32]>, &str, &str) -> anyhow::Result<ConflictScanOutcome>,
{
    let ctx = EmbeddingContext {
        knowledge_type: metadata.knowledge_type.clone(),
        entities: metadata.entities.clone(),
        project_id: project_id.to_string(),
        title: metadata.title.clone(),
    };
    let embedding = embedder
        .embed_with_context(content, &ctx)
        .context("embedding generation failed during ingestion")?;
    let dim = embedding.len() as i64;

    // Normalize entities to lowercase. Both file-ingest and MCP-ingest
    // paths converge here — fixes the F-002-plan-time entity-case
    // asymmetry where MCP path stored raw-case entities while the
    // file-ingest path lowercased them, breaking `check_contradictions`
    // matching across surfaces.
    let normalized_entities: Vec<String> = metadata
        .entities
        .split(',')
        .map(|e| e.trim().to_lowercase())
        .filter(|e| !e.is_empty())
        .collect();

    // Check contradictions BEFORE insert (so we don't match against ourselves).
    // Whole-call Err is downgraded to a degraded flag (not a hard ingest
    // failure): conflict detection is advisory relative to the primary write.
    let (conflicts, conflict_scan_degraded) = match scan(
        &normalized_entities,
        Some(&embedding),
        &metadata.knowledge_type,
        project_id,
    ) {
        Ok(outcome) => (outcome.conflicts, outcome.degraded),
        Err(e) => {
            tracing::warn!(error = %e, "contradiction check failed during ingestion");
            (vec![], true)
        }
    };

    let mem = NewMemory {
        project_id: project_id.to_string(),
        source_file: metadata.source_file.clone(),
        source_type: metadata.source_type.clone(),
        knowledge_type: metadata.knowledge_type.clone(),
        title: metadata.title.clone(),
        content: content.to_string(),
        entities: normalized_entities.join(","),
        embedding: Some(embedding_to_blob(&embedding)),
        embedding_dim: Some(dim),
        is_longterm: metadata.is_longterm,
    };

    Ok((mem, conflicts, conflict_scan_degraded))
}

// FUTURE-CALLER: Future internal callers (e.g., dreaming-time auto-ingest)
// that need atomic resolve+insert MUST use `ingest_text_with_resolves`,
// NOT this function — calling `ingest_text` from a context with
// resolve targets will silently drop the supersession relationships
// (no atomicity guarantee, no `superseded_by` linkage).
//
/// Ingest inline content as a new memory.
///
/// Routes through the shared `prepare_memory` helper (embed + lowercase
/// entities + contradiction check), then `Db::insert_memory` for plain
/// content-hash dedup insert. Returns the inserted entry's id + any
/// detected conflicts.
///
/// # Future Internal Callers
///
/// **If your caller needs atomic resolve+insert** (the MCP `resolves`
/// parameter contract — atomically insert a new memory and invalidate
/// the listed old memory ids in one transaction), use
/// `ingest_text_with_resolves` instead. Calling `ingest_text` from a
/// context that has resolve targets will silently drop those resolves —
/// no `superseded_by` linkage, no atomicity guarantee.
///
/// Plan F-003 Step 5 / discussion 001 Topic 4.
pub fn ingest_text(
    db: &Db,
    embedder: &mut dyn Embed,
    content: &str,
    metadata: IngestMetadata,
    project_id: &str,
) -> anyhow::Result<IngestResult> {
    let (mem, conflicts, conflict_scan_degraded) =
        prepare_memory(db, embedder, content, &metadata, project_id)?;
    // F-007 dual-write happens atomically inside db::insert_memory_inner
    // (under the same connection lock as the memory_entries INSERT) —
    // no separate materialization needed here.
    let entry_id = db.insert_memory(mem)?;
    Ok(IngestResult {
        entry_id,
        conflicts,
        unmatched_resolves: vec![],
        conflict_scan_degraded,
    })
}

/// Ingest inline content + atomically invalidate `resolves` predecessor
/// memory ids in one transaction.
///
/// Routes through the shared `prepare_memory` helper (embed + lowercase
/// entities + contradiction check), then `Db::insert_memory_resolving`
/// for the atomic insert+invalidate. Atomicity is enforced by the Db
/// layer (single TX over the INSERT + N UPDATE statements at
/// `db.rs:200-256`); this function is a thin shape adapter.
///
/// External MCP callers (Claude Code via the `memory_ingest` tool) rely
/// on this contract per discussion 001 Topic 5 (the `resolves` feature
/// contract is locked for v0.0.1 — F-003 carries it through unchanged).
///
/// Plan F-003 Step 5 / discussion 001 Topic 4 + Topic 5.
pub fn ingest_text_with_resolves(
    db: &Db,
    embedder: &mut dyn Embed,
    content: &str,
    metadata: IngestMetadata,
    project_id: &str,
    resolves: &[String],
) -> anyhow::Result<IngestResult> {
    let (mem, conflicts, conflict_scan_degraded) =
        prepare_memory(db, embedder, content, &metadata, project_id)?;
    // F-007 dual-write happens atomically inside
    // db::insert_memory_resolving (under the same transaction as the
    // INSERT + supersession UPDATEs).
    let outcome = db.insert_memory_resolving(mem, resolves)?;
    Ok(IngestResult {
        entry_id: outcome.entry_id,
        conflicts,
        unmatched_resolves: outcome.unmatched,
        conflict_scan_degraded,
    })
}

/// Ingest a parsed document into the database with embedding.
///
/// Post-F-003 Step 5: thin file-parsing wrapper around `ingest_text`.
/// File-ingest paths construct `IngestMetadata` from `ParsedDocument`
/// fields, then delegate. The shared private `prepare_memory` helper
/// owns the embed + entity-case + contradiction-check logic — file
/// path and MCP-direct path converge there.
pub fn ingest_document(
    db: &Db,
    embedder: &mut dyn Embed,
    doc: &ParsedDocument,
    project_id: &str,
) -> anyhow::Result<IngestResult> {
    let metadata = IngestMetadata {
        title: doc.title.clone(),
        entities: doc.entities.join(","),
        source_file: doc.source_file.clone(),
        source_type: doc.source_type.clone(),
        knowledge_type: doc.knowledge_type.clone(),
        is_longterm: false,
    };
    ingest_text(db, embedder, &doc.content, metadata, project_id)
}

/// Parse and ingest a file from disk.
pub fn ingest_file(
    db: &Db,
    embedder: &mut dyn Embed,
    path: &Path,
    project_id: &str,
) -> anyhow::Result<IngestResult> {
    let doc = parse_ae_file(path)?;
    ingest_document(db, embedder, &doc, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Test double for `Embed` that returns a deterministic 384-dim
    /// non-zero vector derived from the content's bytes. Lets ingestion
    /// pipeline tests run without loading fastembed's ORT library (which
    /// requires AVX2 at init — see discussion 020).
    ///
    /// Uses non-zero values so downstream cosine similarity paths
    /// (contradiction detection, vector search) exercise real math
    /// rather than short-circuiting on zero-norm guards — a zero vector
    /// would silently disable half the ingestion logic under test.
    struct MockEmbedder;

    impl Embed for MockEmbedder {
        fn embed_with_context(
            &mut self,
            content: &str,
            _ctx: &EmbeddingContext,
        ) -> anyhow::Result<Vec<f32>> {
            // Deterministic content-derived embedding: cycle content bytes
            // across 384 dimensions, normalize to unit-length-ish range.
            // Collisions between different contents are OK for tests that
            // only check presence + dim; tests that care about distinct
            // vectors use this shape to get reproducible pairwise diffs.
            let bytes = content.as_bytes();
            if bytes.is_empty() {
                return Ok(vec![0.5_f32; 384]);
            }
            let vec: Vec<f32> = (0..384)
                .map(|i| {
                    let b = bytes[i % bytes.len()] as f32;
                    (b / 255.0) - 0.5
                })
                .collect();
            Ok(vec)
        }
    }

    #[test]
    fn test_ingest_file_pipeline_smoke() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "---\ntitle: Test Decision\ntags: [test, ingest]\n---\n# Test\n\nThis is a test decision."
        )
        .unwrap();

        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        let result = ingest_file(&db, &mut embedder, &path, "test-project").unwrap();
        let entry = db.get_memory(&result.entry_id).unwrap().unwrap();
        assert_eq!(entry.title, "Test Decision");
        assert_eq!(entry.entities, "test,ingest");
        assert_eq!(entry.knowledge_type, "decisional");
        assert!(entry.embedding.is_some());
        assert_eq!(entry.embedding_dim, Some(384));
    }

    // ---- F-003 Step 7 ingest_text + atomicity tests ----

    fn test_metadata(title: &str, entities: &str, knowledge_type: &str) -> IngestMetadata {
        IngestMetadata {
            title: title.to_string(),
            entities: entities.to_string(),
            source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: knowledge_type.to_string(),
            is_longterm: false,
        }
    }

    /// AC11: ingest_text inserts a memory and lowercases entities (verifies
    /// the F-002-plan-time entity-case asymmetry is fixed at the shared
    /// prep helper boundary — both file-ingest and MCP-ingest paths
    /// converge on lowercased entities).
    #[test]
    fn test_ingest_text_inserts_memory_and_lowercases_entities() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;
        let metadata = test_metadata("AUTH decision", "AUTH,Middleware,JWT", "decisional");

        let result = ingest_text(
            &db,
            &mut embedder,
            "JWT authentication content",
            metadata,
            "proj",
        )
        .unwrap();
        let entry = db.get_memory(&result.entry_id).unwrap().unwrap();
        assert_eq!(
            entry.entities, "auth,middleware,jwt",
            "entities must be lowercased + comma-joined"
        );
        assert_eq!(entry.title, "AUTH decision");
        assert_eq!(entry.knowledge_type, "decisional");
        assert!(entry.embedding.is_some());
    }

    /// AC11: ingest_text_with_resolves atomically inserts the new memory AND
    /// invalidates the listed predecessor ids in one transaction.
    #[test]
    fn test_ingest_text_with_resolves_invalidates_atomically() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        // Seed two predecessor memories.
        let metadata1 = test_metadata("old auth v1", "auth", "decisional");
        let id1 = ingest_text(&db, &mut embedder, "v1 content", metadata1, "proj")
            .unwrap()
            .entry_id;
        let metadata2 = test_metadata("old auth v2", "auth", "decisional");
        let id2 = ingest_text(&db, &mut embedder, "v2 content", metadata2, "proj")
            .unwrap()
            .entry_id;

        // Insert a new memory that supersedes both.
        let metadata3 = test_metadata("new auth", "auth", "decisional");
        let id3 = ingest_text_with_resolves(
            &db,
            &mut embedder,
            "v3 content",
            metadata3,
            "proj",
            &[id1.clone(), id2.clone()],
        )
        .unwrap()
        .entry_id;

        // Predecessors are invalidated; new entry is live.
        let old1 = db.get_memory(&id1).unwrap().unwrap();
        let old2 = db.get_memory(&id2).unwrap().unwrap();
        let new = db.get_memory(&id3).unwrap().unwrap();
        assert!(
            old1.valid_until.is_some(),
            "predecessor 1 must have valid_until set"
        );
        assert!(
            old2.valid_until.is_some(),
            "predecessor 2 must have valid_until set"
        );
        assert_eq!(
            old1.superseded_by.as_deref(),
            Some(id3.as_str()),
            "predecessor 1's superseded_by must point to new id"
        );
        assert_eq!(
            old2.superseded_by.as_deref(),
            Some(id3.as_str()),
            "predecessor 2's superseded_by must point to new id"
        );
        assert!(new.valid_until.is_none(), "new entry must be live");
    }

    /// F-016 / BL-056: a resolve id that matches no live row in this project
    /// is reported in `unmatched_resolves` (not silently dropped), while the
    /// new memory still commits and the matching ids are superseded.
    #[test]
    fn test_resolves_reports_unmatched_ids() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        // One live predecessor in this project.
        let id1 = ingest_text(
            &db,
            &mut embedder,
            "v1 content",
            test_metadata("old", "auth", "decisional"),
            "proj",
        )
        .unwrap()
        .entry_id;

        // A predecessor that lives in ANOTHER project (filtered out by the
        // AND project_id guard → must be reported unmatched, not superseded).
        let cross = ingest_text(
            &db,
            &mut embedder,
            "cross-project content",
            test_metadata("cross", "auth", "decisional"),
            "other-proj",
        )
        .unwrap()
        .entry_id;

        let result = ingest_text_with_resolves(
            &db,
            &mut embedder,
            "v2 content",
            test_metadata("new", "auth", "decisional"),
            "proj",
            &[
                id1.clone(),
                cross.clone(),
                "nonexistent-typo-id".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(
            result.unmatched_resolves,
            vec![cross.clone(), "nonexistent-typo-id".to_string()],
            "cross-project id + typo id must both report unmatched, in order"
        );
        assert!(
            db.get_memory(&id1).unwrap().unwrap().valid_until.is_some(),
            "the live in-project predecessor must be superseded"
        );
        assert!(
            db.get_memory(&cross)
                .unwrap()
                .unwrap()
                .valid_until
                .is_none(),
            "the cross-project memory must be untouched"
        );
        assert!(
            db.get_memory(&result.entry_id)
                .unwrap()
                .unwrap()
                .valid_until
                .is_none(),
            "the new memory must be live (insert always commits)"
        );
    }

    /// F-016 / BL-056: all-matching resolves report no unmatched; duplicate
    /// ids are processed once.
    #[test]
    fn test_resolves_all_match_and_dedup() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        let id1 = ingest_text(
            &db,
            &mut embedder,
            "v1",
            test_metadata("old", "auth", "decisional"),
            "proj",
        )
        .unwrap()
        .entry_id;

        // Pass the same live id twice — dedup means it is applied once and
        // reported unmatched zero times.
        let result = ingest_text_with_resolves(
            &db,
            &mut embedder,
            "v2",
            test_metadata("new", "auth", "decisional"),
            "proj",
            &[id1.clone(), id1.clone()],
        )
        .unwrap();

        assert!(
            result.unmatched_resolves.is_empty(),
            "a live id passed twice must not report as unmatched"
        );
        assert!(db.get_memory(&id1).unwrap().unwrap().valid_until.is_some());
    }

    /// F-016 / BL-058: `prepare_memory_with_scan` reports `degraded` from
    /// three scan outcomes — clean (false), per-entity skip (Ok degraded:true),
    /// and whole-call error (Err → true, ingest still proceeds with no
    /// conflicts). This is the injectable seam that makes the scan-error path
    /// testable without breaking the insert.
    #[test]
    fn test_prepare_scan_degraded_flag() {
        let mut embedder = MockEmbedder;
        let md = test_metadata("t", "auth", "decisional");

        let (_, _, degraded) =
            prepare_memory_with_scan(&mut embedder, "c", &md, "proj", |_, _, _, _| {
                Ok(ConflictScanOutcome {
                    conflicts: vec![],
                    degraded: false,
                })
            })
            .unwrap();
        assert!(!degraded, "clean scan → not degraded");

        let (_, _, degraded) =
            prepare_memory_with_scan(&mut embedder, "c", &md, "proj", |_, _, _, _| {
                Ok(ConflictScanOutcome {
                    conflicts: vec![],
                    degraded: true,
                })
            })
            .unwrap();
        assert!(degraded, "Ok(degraded:true) → degraded");

        let (_, conflicts, degraded) =
            prepare_memory_with_scan(&mut embedder, "c", &md, "proj", |_, _, _, _| {
                Err(anyhow::anyhow!("scan boom"))
            })
            .unwrap();
        assert!(degraded, "whole-call Err → degraded");
        assert!(conflicts.is_empty(), "degraded scan yields no conflicts");
    }

    /// F-016 / BL-058: a normal successful ingest reports `conflict_scan_degraded
    /// == false` end-to-end through `IngestResult`.
    #[test]
    fn test_scan_not_degraded_on_healthy_ingest() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;
        let result = ingest_text(
            &db,
            &mut embedder,
            "content",
            test_metadata("t", "auth", "decisional"),
            "proj",
        )
        .unwrap();
        assert!(!result.conflict_scan_degraded);
    }

    /// F-016 (codex Axis 3): `unmatched_resolves` is always computed even when
    /// the conflict scan degraded. Composes the two production units that
    /// `ingest_text_with_resolves` glues together (forced-degraded scan via the
    /// seam + `insert_memory_resolving`).
    #[test]
    fn test_degraded_and_unmatched_compose() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        let id1 = ingest_text(
            &db,
            &mut embedder,
            "v1",
            test_metadata("old", "auth", "decisional"),
            "proj",
        )
        .unwrap()
        .entry_id;

        let (mem, _conflicts, degraded) = prepare_memory_with_scan(
            &mut embedder,
            "v2",
            &test_metadata("new", "auth", "decisional"),
            "proj",
            |_, _, _, _| Err(anyhow::anyhow!("scan boom")),
        )
        .unwrap();
        assert!(degraded, "forced scan error marks degraded");

        let outcome = db
            .insert_memory_resolving(mem, &[id1.clone(), "missing".to_string()])
            .unwrap();
        assert_eq!(
            outcome.unmatched,
            vec!["missing".to_string()],
            "unmatched is still computed when the scan degraded"
        );
        assert!(
            db.get_memory(&id1).unwrap().unwrap().valid_until.is_some(),
            "the live resolve id is still superseded"
        );
        assert!(
            db.get_memory(&outcome.entry_id)
                .unwrap()
                .unwrap()
                .valid_until
                .is_none(),
            "the new memory is live"
        );
    }

    /// F-016 (codex Axis 2 #4): re-ingesting byte-identical content via the
    /// resolving path makes the content-hash upsert return the EXISTING id; if
    /// `resolves` lists that same id, the `id <> ?2` guard must prevent the loop
    /// from tombstoning the memory this call just returned (data-corruption fix).
    #[test]
    fn test_self_supersession_guard() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        let id_x = ingest_text(
            &db,
            &mut embedder,
            "same content",
            test_metadata("x", "auth", "decisional"),
            "proj",
        )
        .unwrap()
        .entry_id;

        // Identical content → content-hash upsert returns id_x; resolves lists
        // id_x itself.
        let result = ingest_text_with_resolves(
            &db,
            &mut embedder,
            "same content",
            test_metadata("x2", "auth", "decisional"),
            "proj",
            std::slice::from_ref(&id_x),
        )
        .unwrap();

        assert_eq!(
            result.entry_id, id_x,
            "content-hash upsert returns the existing id"
        );
        assert!(
            db.get_memory(&id_x).unwrap().unwrap().valid_until.is_none(),
            "the returned memory must NOT be tombstoned by self-supersession"
        );
        assert_eq!(
            result.unmatched_resolves,
            vec![id_x.clone()],
            "the self-referential resolve id reports as unmatched/not-applied"
        );
    }

    /// AC11: empty resolves slice is a valid input (just inserts without
    /// any invalidation — semantically equivalent to ingest_text but goes
    /// through the resolves path; useful for callers that build the
    /// resolves Vec dynamically and may pass empty).
    #[test]
    fn test_ingest_text_with_resolves_empty_resolves_is_plain_insert() {
        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;
        let metadata = test_metadata("alone", "tag", "decisional");

        let result = ingest_text_with_resolves(
            &db,
            &mut embedder,
            "isolated content",
            metadata,
            "proj",
            &[],
        )
        .unwrap();
        let entry = db.get_memory(&result.entry_id).unwrap().unwrap();
        assert!(entry.valid_until.is_none(), "lone insert must be live");
    }

    /// AC11: file-ingest and MCP-ingest converge on lowercased entities post-
    /// F-003 (regression test for the F-002-plan-time entity-case
    /// asymmetry). Verifies the ingest_document → ingest_text wrapper path
    /// produces the same observable case-normalized output as the direct
    /// ingest_text path.
    #[test]
    fn test_ingest_document_and_ingest_text_produce_same_entity_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "---\ntitle: Cross-surface test\ntags: [AUTH, Middleware]\n---\n# Test\n\nfile-ingest content."
        )
        .unwrap();

        let db = Db::open_in_memory().unwrap();
        let mut embedder = MockEmbedder;

        // file-ingest path
        let file_result = ingest_file(&db, &mut embedder, &path, "proj").unwrap();
        let file_entry = db.get_memory(&file_result.entry_id).unwrap().unwrap();

        // direct ingest_text path
        let metadata = test_metadata("Cross-surface test 2", "AUTH,Middleware", "decisional");
        let direct_result = ingest_text(
            &db,
            &mut embedder,
            "direct-ingest content",
            metadata,
            "proj",
        )
        .unwrap();
        let direct_entry = db.get_memory(&direct_result.entry_id).unwrap().unwrap();

        assert_eq!(
            file_entry.entities, "auth,middleware",
            "file-ingest path must lowercase entities"
        );
        assert_eq!(
            direct_entry.entities, "auth,middleware",
            "direct ingest_text path must lowercase entities"
        );
        assert_eq!(
            file_entry.entities, direct_entry.entities,
            "both paths must produce the same case-normalized entity string"
        );
    }
}
