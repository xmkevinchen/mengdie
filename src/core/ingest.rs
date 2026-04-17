use std::path::Path;

use anyhow::Context;

use super::contradiction::Conflict;
use super::db::{Db, NewMemory};
use super::embeddings::{embedding_to_blob, Embedder, EmbeddingContext};
use super::parser::{parse_ae_file, ParsedDocument};

/// Result of ingesting a document, including any detected contradictions.
pub struct IngestResult {
    pub entry_id: String,
    pub conflicts: Vec<Conflict>,
}

/// Ingest a parsed document into the database with embedding.
pub fn ingest_document(
    db: &Db,
    embedder: &mut Embedder,
    doc: &ParsedDocument,
    project_id: &str,
) -> anyhow::Result<IngestResult> {
    // Generate embedding with metadata-in-chunk encoding
    let ctx = EmbeddingContext {
        knowledge_type: doc.knowledge_type.clone(),
        entities: doc.entities.join(","),
        project_id: project_id.to_string(),
        title: doc.title.clone(),
    };
    let embedding = embedder
        .embed_with_context(&doc.content, &ctx)
        .context("embedding generation failed during ingestion")?;
    let dim = embedding.len() as i64;

    // Normalize entities to lowercase for consistent matching
    let normalized_entities: Vec<String> = doc.entities.iter().map(|e| e.to_lowercase()).collect();

    // Check contradictions BEFORE insert (so we don't match against ourselves)
    let conflicts = db
        .check_contradictions(
            &normalized_entities,
            Some(&embedding),
            &doc.knowledge_type,
            project_id,
        )
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "contradiction check failed during ingestion");
            vec![]
        });

    let mem = NewMemory {
        project_id: project_id.to_string(),
        source_file: doc.source_file.clone(),
        source_type: doc.source_type.clone(),
        knowledge_type: doc.knowledge_type.clone(),
        title: doc.title.clone(),
        content: doc.content.clone(),
        entities: normalized_entities.join(","),
        embedding: Some(embedding_to_blob(&embedding)),
        embedding_dim: Some(dim),
    };

    let entry_id = db.insert_memory(mem)?;
    Ok(IngestResult {
        entry_id,
        conflicts,
    })
}

/// Parse and ingest a file from disk.
pub fn ingest_file(
    db: &Db,
    embedder: &mut Embedder,
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

    // Note: these tests require the fastembed model (~90MB download on first run).
    // They are ignored by default and run with `cargo test -- --ignored`.

    #[test]
    fn test_ingest_file_e2e() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "---\ntitle: Test Decision\ntags: [test, ingest]\n---\n# Test\n\nThis is a test decision."
        )
        .unwrap();

        let db = Db::open_in_memory().unwrap();
        let mut embedder = Embedder::new().unwrap();

        let result = ingest_file(&db, &mut embedder, &path, "test-project").unwrap();
        let entry = db.get_memory(&result.entry_id).unwrap().unwrap();
        assert_eq!(entry.title, "Test Decision");
        assert_eq!(entry.entities, "test,ingest");
        assert_eq!(entry.knowledge_type, "decisional");
        assert!(entry.embedding.is_some());
        assert_eq!(entry.embedding_dim, Some(384));
    }
}
