use rusqlite::params;

use super::db::Db;
use super::embeddings::{blob_to_embedding, cosine_similarity};

// -- Tunable thresholds --
// See BL-002-6: make configurable when users report false positives.

/// Cosine similarity threshold for evolution candidate detection (both decisional, same entities).
pub const EVOLUTION_SIMILARITY_THRESHOLD: f32 = 0.7;
/// Cosine similarity floor for recent conflict detection (reduces false positives on common tags).
pub const RECENT_CONFLICT_SIMILARITY_FLOOR: f32 = 0.4;
/// Time window (days) for recent conflict detection.
pub const RECENT_CONFLICT_WINDOW_DAYS: i64 = 30;

/// A detected conflict between a new memory and an existing one.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub existing_id: String,
    pub existing_title: String,
    pub reason: ConflictReason,
}

#[derive(Debug, Clone)]
pub enum ConflictReason {
    /// Same entities + high semantic similarity + both decisional → likely supersedes
    EvolutionCandidate { similarity: f32 },
    /// Same entities + created within 30 days → potential conflict
    RecentConflict { days_apart: i64 },
}

impl std::fmt::Display for ConflictReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictReason::EvolutionCandidate { similarity } => {
                write!(f, "evolution candidate (similarity: {similarity:.2})")
            }
            ConflictReason::RecentConflict { days_apart } => {
                write!(f, "recent conflict ({days_apart} days apart)")
            }
        }
    }
}

impl Db {
    /// Check for contradictions between a new memory and existing ones.
    /// Returns conflicts found (caller decides what to do with them).
    pub fn check_contradictions(
        &self,
        new_entities: &[String],
        new_embedding: Option<&[f32]>,
        new_knowledge_type: &str,
        project_id: &str,
    ) -> anyhow::Result<Vec<Conflict>> {
        if new_entities.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.lock_conn()?;
        let now = chrono::Utc::now();

        // Find existing memories with overlapping entities in the same project
        // that are still valid (not invalidated)
        let mut stmt = conn.prepare(
            "SELECT id, title, entities, knowledge_type, embedding, created_at
             FROM memory_entries
             WHERE project_id = ?1
             AND valid_until IS NULL
             AND entities != ''",
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,          // id
                row.get::<_, String>(1)?,          // title
                row.get::<_, String>(2)?,          // entities
                row.get::<_, String>(3)?,          // knowledge_type
                row.get::<_, Option<Vec<u8>>>(4)?, // embedding
                row.get::<_, String>(5)?,          // created_at
            ))
        })?;

        let mut conflicts = Vec::new();

        for row in rows {
            let (id, title, entities_str, knowledge_type, embedding_blob, created_at) = row?;

            // Check entity overlap: ≥1 shared tag
            let existing_entities: Vec<&str> = entities_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            let has_overlap = new_entities
                .iter()
                .any(|e| existing_entities.contains(&e.as_str()));

            if !has_overlap {
                continue;
            }

            // Check 1: Evolution candidate
            // Entity overlap + high semantic similarity + both decisional
            if new_knowledge_type == "decisional" && knowledge_type == "decisional" {
                if let (Some(new_emb), Some(existing_blob)) = (new_embedding, &embedding_blob) {
                    if let Ok(existing_emb) = blob_to_embedding(existing_blob) {
                        let sim = cosine_similarity(new_emb, &existing_emb);
                        if sim > EVOLUTION_SIMILARITY_THRESHOLD {
                            conflicts.push(Conflict {
                                existing_id: id.clone(),
                                existing_title: title.clone(),
                                reason: ConflictReason::EvolutionCandidate { similarity: sim },
                            });
                            continue; // Don't double-flag
                        }
                    }
                }
            }

            // Check 2: Recent conflict
            // Entity overlap + created within 30 days + minimum semantic similarity
            // (without similarity floor, common tags like "auth" would always trigger)
            if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&created_at) {
                let days_apart = (now - created.with_timezone(&chrono::Utc)).num_days().abs();
                if days_apart < RECENT_CONFLICT_WINDOW_DAYS {
                    let has_similarity = match (new_embedding, &embedding_blob) {
                        (Some(new_emb), Some(blob)) => blob_to_embedding(blob)
                            .map(|existing_emb| {
                                cosine_similarity(new_emb, &existing_emb)
                                    > RECENT_CONFLICT_SIMILARITY_FLOOR
                            })
                            .unwrap_or(false),
                        _ => false, // No embeddings → can't verify similarity, skip flagging
                    };
                    if has_similarity {
                        conflicts.push(Conflict {
                            existing_id: id,
                            existing_title: title,
                            reason: ConflictReason::RecentConflict { days_apart },
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;
    use crate::core::embeddings::embedding_to_blob;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn insert_mem(
        db: &Db,
        project: &str,
        title: &str,
        entities: &str,
        knowledge_type: &str,
        embedding: &[f32],
    ) -> String {
        db.insert_memory(NewMemory {
            project_id: project.to_string(),
            source_file: format!("test-{}.md", uuid::Uuid::new_v4()),
            source_type: "conclusion".to_string(),
            knowledge_type: knowledge_type.to_string(),
            title: title.to_string(),
            content: format!("Content about {title}"),
            entities: entities.to_string(),
            embedding: Some(embedding_to_blob(embedding)),
            embedding_dim: Some(embedding.len() as i64),
        })
        .unwrap()
    }

    #[test]
    fn test_no_conflicts_empty_entities() {
        let db = test_db();
        insert_mem(
            &db,
            "proj",
            "Old Decision",
            "auth,jwt",
            "decisional",
            &[1.0, 0.0, 0.0],
        );

        let conflicts = db
            .check_contradictions(&[], Some(&[1.0, 0.0, 0.0]), "decisional", "proj")
            .unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_no_conflicts_no_overlap() {
        let db = test_db();
        insert_mem(
            &db,
            "proj",
            "Old Decision",
            "database,postgresql",
            "decisional",
            &[0.0, 1.0, 0.0],
        );

        let conflicts = db
            .check_contradictions(
                &["auth".to_string(), "jwt".to_string()],
                Some(&[1.0, 0.0, 0.0]),
                "decisional",
                "proj",
            )
            .unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_evolution_candidate() {
        let db = test_db();
        // Similar embedding + same entities + both decisional
        insert_mem(
            &db,
            "proj",
            "Old Auth Decision",
            "auth,jwt",
            "decisional",
            &[0.9, 0.1, 0.0],
        );

        let conflicts = db
            .check_contradictions(
                &["auth".to_string()],
                Some(&[0.85, 0.15, 0.0]), // very similar
                "decisional",
                "proj",
            )
            .unwrap();
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            conflicts[0].reason,
            ConflictReason::EvolutionCandidate { .. }
        ));
    }

    #[test]
    fn test_recent_conflict() {
        let db = test_db();
        // Same entities, created recently, somewhat similar (above 0.4 floor)
        insert_mem(
            &db,
            "proj",
            "Auth Review",
            "auth",
            "experiential",
            &[0.7, 0.5, 0.1],
        );

        let conflicts = db
            .check_contradictions(
                &["auth".to_string()],
                Some(&[0.6, 0.6, 0.2]), // similar enough (cosine > 0.4)
                "experiential",
                "proj",
            )
            .unwrap();
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            conflicts[0].reason,
            ConflictReason::RecentConflict { .. }
        ));
    }

    #[test]
    fn test_no_recent_conflict_low_similarity() {
        let db = test_db();
        // Same entity but orthogonal embeddings — below 0.4 similarity floor
        insert_mem(
            &db,
            "proj",
            "Auth Setup Guide",
            "auth",
            "factual",
            &[1.0, 0.0, 0.0],
        );

        let conflicts = db
            .check_contradictions(
                &["auth".to_string()],
                Some(&[0.0, 0.0, 1.0]), // orthogonal — cosine ≈ 0
                "experiential",
                "proj",
            )
            .unwrap();
        assert!(
            conflicts.is_empty(),
            "orthogonal embeddings should not trigger RecentConflict"
        );
    }

    #[test]
    fn test_no_conflict_with_invalidated() {
        let db = test_db();
        let id = insert_mem(
            &db,
            "proj",
            "Old Auth",
            "auth",
            "decisional",
            &[0.9, 0.1, 0.0],
        );
        db.invalidate_memory(&id, None, None).unwrap();

        let conflicts = db
            .check_contradictions(
                &["auth".to_string()],
                Some(&[0.85, 0.15, 0.0]),
                "decisional",
                "proj",
            )
            .unwrap();
        assert!(
            conflicts.is_empty(),
            "invalidated memories should not trigger conflicts"
        );
    }

    #[test]
    fn test_cross_project_no_conflict() {
        let db = test_db();
        insert_mem(
            &db,
            "proj-a",
            "Auth Decision",
            "auth",
            "decisional",
            &[0.9, 0.1, 0.0],
        );

        let conflicts = db
            .check_contradictions(
                &["auth".to_string()],
                Some(&[0.85, 0.15, 0.0]),
                "decisional",
                "proj-b", // different project
            )
            .unwrap();
        assert!(conflicts.is_empty());
    }
}
