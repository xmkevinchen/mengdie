//! Memory Lint — deterministic DB health checks (F-008).
//!
//! Three read-only checks surface silent failure modes that other
//! systems can't easily detect at runtime:
//!
//! 1. **Orphan GC** — dangling FK-style references. The project runs
//!    with `PRAGMA foreign_keys = OFF` (BL-015), so the DB itself
//!    doesn't enforce referential integrity; orphan rows accumulate
//!    on rename / delete.
//!
//! 2. **Unresolved contradictions** — half-resolved supersession state
//!    (valid_until without superseded_by, or vice versa), circular
//!    supersession (size-2 cycles), and high-entity-overlap pairs that
//!    ingest-time contradiction.rs missed.
//!
//! 3. **Embedding drift** — facts where the embedding is missing or
//!    has wrong dimension. Subcheck 3c targets the BL-022 surface:
//!    synthesis rows with embedding=NULL (predicted 5 hits on the
//!    operator's dogfood DB).
//!
//! All checks are pure SQL — no LLM calls. Operator runs via
//! `mengdie lint` CLI or the `memory_lint` MCP tool; LLM consumers
//! can poll periodically to detect drift.

use rmcp::schemars;
use rusqlite::params;
use serde::Serialize;

use super::db::Db;

/// Cap on sample IDs returned per finding category — keeps the report
/// terse for LLM consumption + table-format CLI rendering while still
/// giving operators concrete IDs to investigate.
const SAMPLE_CAP: usize = 5;

/// Threshold for "high entity overlap" Jaccard score in Check 2d.
const ENTITY_OVERLAP_THRESHOLD: f64 = 0.7;

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct LintReport {
    pub generated_at: String, // RFC3339
    pub orphan_gc: OrphanCheck,
    pub unresolved_contradictions: ContradictionCheck,
    pub embedding_drift: EmbeddingDriftCheck,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema, Default)]
pub struct OrphanCheck {
    /// memory_entries.superseded_by points to a non-existent id.
    pub superseded_by_dangling_count: i64,
    pub superseded_by_dangling: Vec<String>,
    /// memory_synthesis_links rows with missing source or synthesis fact.
    pub synthesis_links_orphan_count: i64,
    pub synthesis_links_orphan: Vec<String>, // formatted "fact_id→synthesis_id"
    /// audit_returned_facts rows with missing fact_id or audit_id.
    pub audit_facts_orphan_count: i64,
    pub audit_facts_orphan: Vec<String>, // formatted "fact_id (audit=N)"
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema, Default)]
pub struct ContradictionCheck {
    /// valid_until set but superseded_by NULL.
    pub half_v_only_count: i64,
    pub half_v_only: Vec<String>,
    /// superseded_by set but valid_until NULL.
    pub half_s_only_count: i64,
    pub half_s_only: Vec<String>,
    /// Size-2 cycle: A.superseded_by=B AND B.superseded_by=A.
    pub circular_count: i64,
    pub circular: Vec<(String, String)>,
    /// Active facts (valid_until NULL) with ≥0.7 Jaccard entity overlap
    /// that have no supersession link between them. Surfaces likely
    /// candidate contradictions missed by ingest-time check.
    pub entity_overlap_unsuperseded_count: i64,
    pub entity_overlap_unsuperseded: Vec<EntityOverlapPair>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct EntityOverlapPair {
    pub fact_a: String,
    pub fact_b: String,
    pub overlap: f64,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema, Default)]
pub struct EmbeddingDriftCheck {
    /// embedding IS NULL AND source_type != 'synthesis'. Non-synthesis
    /// facts should always have embeddings.
    pub embedding_null_non_synthesis_count: i64,
    pub embedding_null_non_synthesis: Vec<String>,
    /// embedding_dim != 384 (the project-wide canonical dim).
    pub embedding_dim_mismatch_count: i64,
    pub embedding_dim_mismatch: Vec<String>,
    /// 3c: synthesis rows with embedding=NULL — BL-022's surface.
    /// On the operator's dogfood DB at v0.0.2 ship time, expected 5
    /// hits (per CLAUDE.md project state).
    pub synthesis_null_embedding_count: i64,
    pub synthesis_null_embedding: Vec<String>,
}

impl Db {
    /// Run all 3 Memory Lint checks (F-008). Read-only — no INSERT/
    /// UPDATE/DELETE. Optionally project-scoped (None = global).
    /// Idempotent: running on unchanged DB produces byte-identical
    /// output modulo `generated_at`.
    pub fn run_lint(&self, project_id: Option<&str>) -> anyhow::Result<LintReport> {
        let now = chrono::Utc::now().to_rfc3339();
        let orphan_gc = self.lint_check_1_orphans(project_id)?;
        let unresolved_contradictions = self.lint_check_2_contradictions(project_id)?;
        let embedding_drift = self.lint_check_3_embedding(project_id)?;
        Ok(LintReport {
            generated_at: now,
            orphan_gc,
            unresolved_contradictions,
            embedding_drift,
        })
    }

    fn lint_check_1_orphans(&self, project_id: Option<&str>) -> anyhow::Result<OrphanCheck> {
        let conn = self.lock_conn()?;

        // 1a: orphan superseded_by — scoped to project_id when provided.
        let (count_a, samples_a): (i64, Vec<String>) = {
            let sql_count = match project_id {
                Some(_) => {
                    "SELECT COUNT(*) FROM memory_entries
                            WHERE superseded_by IS NOT NULL
                              AND superseded_by NOT IN (SELECT id FROM memory_entries)
                              AND project_id = ?1"
                }
                None => {
                    "SELECT COUNT(*) FROM memory_entries
                         WHERE superseded_by IS NOT NULL
                           AND superseded_by NOT IN (SELECT id FROM memory_entries)"
                }
            };
            let c: i64 = match project_id {
                Some(pid) => conn.query_row(sql_count, params![pid], |r| r.get(0))?,
                None => conn.query_row(sql_count, [], |r| r.get(0))?,
            };
            let sql_samples = match project_id {
                Some(_) => {
                    "SELECT id FROM memory_entries
                            WHERE superseded_by IS NOT NULL
                              AND superseded_by NOT IN (SELECT id FROM memory_entries)
                              AND project_id = ?1
                            ORDER BY id LIMIT 5"
                }
                None => {
                    "SELECT id FROM memory_entries
                         WHERE superseded_by IS NOT NULL
                           AND superseded_by NOT IN (SELECT id FROM memory_entries)
                         ORDER BY id LIMIT 5"
                }
            };
            let mut stmt = conn.prepare(sql_samples)?;
            let samples: Vec<String> = match project_id {
                Some(pid) => {
                    let mapped = stmt.query_map(params![pid], |r| r.get::<_, String>(0))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
                None => {
                    let mapped = stmt.query_map([], |r| r.get::<_, String>(0))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
            };
            (c, samples)
        };

        // 1b: orphan memory_synthesis_links. Global scope — synthesis_links
        // table doesn't carry project_id directly; orphans surface globally.
        let (count_b, samples_b): (i64, Vec<String>) = {
            let c: i64 = conn.query_row(
                "SELECT COUNT(*) FROM memory_synthesis_links
                 WHERE source_memory_id NOT IN (SELECT id FROM memory_entries)
                    OR synthesis_memory_id NOT IN (SELECT id FROM memory_entries)",
                [],
                |r| r.get(0),
            )?;
            let mut stmt = conn.prepare(
                "SELECT source_memory_id, synthesis_memory_id FROM memory_synthesis_links
                 WHERE source_memory_id NOT IN (SELECT id FROM memory_entries)
                    OR synthesis_memory_id NOT IN (SELECT id FROM memory_entries)
                 ORDER BY source_memory_id LIMIT 5",
            )?;
            let mapped = stmt.query_map([], |r| {
                Ok(format!(
                    "{}→{}",
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?
                ))
            })?;
            let samples = mapped.collect::<rusqlite::Result<Vec<_>>>()?;
            (c, samples)
        };

        // 1c: orphan audit_returned_facts — global (audit table not scoped).
        let (count_c, samples_c): (i64, Vec<String>) = {
            let c: i64 = conn.query_row(
                "SELECT COUNT(*) FROM audit_returned_facts
                 WHERE fact_id NOT IN (SELECT id FROM memory_entries)
                    OR audit_id NOT IN (SELECT id FROM memory_search_audit)",
                [],
                |r| r.get(0),
            )?;
            let mut stmt = conn.prepare(
                "SELECT fact_id, audit_id FROM audit_returned_facts
                 WHERE fact_id NOT IN (SELECT id FROM memory_entries)
                    OR audit_id NOT IN (SELECT id FROM memory_search_audit)
                 ORDER BY fact_id LIMIT 5",
            )?;
            let mapped = stmt.query_map([], |r| {
                Ok(format!(
                    "{} (audit={})",
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?
                ))
            })?;
            let samples = mapped.collect::<rusqlite::Result<Vec<_>>>()?;
            (c, samples)
        };

        Ok(OrphanCheck {
            superseded_by_dangling_count: count_a,
            superseded_by_dangling: samples_a.into_iter().take(SAMPLE_CAP).collect(),
            synthesis_links_orphan_count: count_b,
            synthesis_links_orphan: samples_b.into_iter().take(SAMPLE_CAP).collect(),
            audit_facts_orphan_count: count_c,
            audit_facts_orphan: samples_c.into_iter().take(SAMPLE_CAP).collect(),
        })
    }

    fn lint_check_2_contradictions(
        &self,
        project_id: Option<&str>,
    ) -> anyhow::Result<ContradictionCheck> {
        let conn = self.lock_conn()?;

        // 2a: valid_until set but superseded_by NULL.
        let (count_v, samples_v) = self.lint_query_ids(
            &conn,
            project_id,
            "WHERE valid_until IS NOT NULL AND superseded_by IS NULL",
        )?;

        // 2b: superseded_by set but valid_until NULL.
        let (count_s, samples_s) = self.lint_query_ids(
            &conn,
            project_id,
            "WHERE superseded_by IS NOT NULL AND valid_until IS NULL",
        )?;

        // 2c: size-2 supersession cycle.
        let (count_c, samples_c): (i64, Vec<(String, String)>) = {
            let sql = match project_id {
                Some(_) => {
                    "SELECT a.id, b.id FROM memory_entries a
                            JOIN memory_entries b ON a.superseded_by = b.id
                            WHERE b.superseded_by = a.id AND a.project_id = ?1
                            ORDER BY a.id LIMIT 50"
                }
                None => {
                    "SELECT a.id, b.id FROM memory_entries a
                         JOIN memory_entries b ON a.superseded_by = b.id
                         WHERE b.superseded_by = a.id
                         ORDER BY a.id LIMIT 50"
                }
            };
            let mut stmt = conn.prepare(sql)?;
            let rows: Vec<(String, String)> = match project_id {
                Some(pid) => {
                    let mapped = stmt.query_map(params![pid], |r| Ok((r.get(0)?, r.get(1)?)))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
                None => {
                    let mapped = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
            };
            (rows.len() as i64, rows)
        };

        // 2d: entity overlap > 0.7 Jaccard among unsuperseded active
        // facts (valid_until NULL). Uses F-007's fact_entity index via
        // entities_of_fact / facts_with_entity helpers.
        //
        // Strategy: iterate active facts (capped at 100 per run for
        // performance — operator can re-run); for each, find candidate
        // partners via shared entities, compute Jaccard, flag pairs
        // exceeding threshold that lack supersession link in either
        // direction.
        let overlap_pairs = self.lint_check_2d_entity_overlap(&conn, project_id)?;

        Ok(ContradictionCheck {
            half_v_only_count: count_v,
            half_v_only: samples_v,
            half_s_only_count: count_s,
            half_s_only: samples_s,
            circular_count: count_c,
            circular: samples_c.into_iter().take(SAMPLE_CAP).collect(),
            entity_overlap_unsuperseded_count: overlap_pairs.len() as i64,
            entity_overlap_unsuperseded: overlap_pairs.into_iter().take(SAMPLE_CAP).collect(),
        })
    }

    /// Helper for 2a/2b — query memory_entries with a WHERE-clause
    /// fragment, scoped to project_id when provided. Returns (count,
    /// sample-of-5 ids).
    fn lint_query_ids(
        &self,
        conn: &rusqlite::Connection,
        project_id: Option<&str>,
        where_extra: &str,
    ) -> anyhow::Result<(i64, Vec<String>)> {
        let scope_clause = match project_id {
            Some(_) => " AND project_id = ?1",
            None => "",
        };
        let sql_count = format!("SELECT COUNT(*) FROM memory_entries {where_extra}{scope_clause}");
        let sql_samples = format!(
            "SELECT id FROM memory_entries {where_extra}{scope_clause} ORDER BY id LIMIT 5"
        );

        let count: i64 = match project_id {
            Some(pid) => conn.query_row(&sql_count, params![pid], |r| r.get(0))?,
            None => conn.query_row(&sql_count, [], |r| r.get(0))?,
        };
        let mut stmt = conn.prepare(&sql_samples)?;
        let samples: Vec<String> = match project_id {
            Some(pid) => {
                let mapped = stmt.query_map(params![pid], |r| r.get::<_, String>(0))?;
                mapped.collect::<rusqlite::Result<Vec<_>>>()?
            }
            None => {
                let mapped = stmt.query_map([], |r| r.get::<_, String>(0))?;
                mapped.collect::<rusqlite::Result<Vec<_>>>()?
            }
        };
        Ok((count, samples))
    }

    /// Check 2d: entity-overlap Jaccard > 0.7 among unsuperseded facts
    /// without an existing supersession link. Uses F-007's normalized
    /// entities + fact_entity tables.
    fn lint_check_2d_entity_overlap(
        &self,
        conn: &rusqlite::Connection,
        project_id: Option<&str>,
    ) -> anyhow::Result<Vec<EntityOverlapPair>> {
        // Pull a bounded set of active fact IDs to scan; full corpus
        // scan is bounded by entity-tag density not row count, but a
        // hard 100-fact cap per run keeps operator-tooling latency
        // predictable. Re-run lint to scan more.
        let active_ids: Vec<String> = {
            let sql = match project_id {
                Some(_) => {
                    "SELECT id FROM memory_entries
                            WHERE valid_until IS NULL AND project_id = ?1
                            ORDER BY id LIMIT 100"
                }
                None => {
                    "SELECT id FROM memory_entries
                         WHERE valid_until IS NULL
                         ORDER BY id LIMIT 100"
                }
            };
            let mut stmt = conn.prepare(sql)?;
            match project_id {
                Some(pid) => {
                    let mapped = stmt.query_map(params![pid], |r| r.get::<_, String>(0))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
                None => {
                    let mapped = stmt.query_map([], |r| r.get::<_, String>(0))?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()?
                }
            }
        };

        if active_ids.len() < 2 {
            return Ok(vec![]);
        }

        // Cache: fact_id → set of entity names. Single pass through
        // fact_entity + entities for the active set.
        let mut by_fact: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        {
            let placeholders: String = (1..=active_ids.len())
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "SELECT fe.fact_id, e.name
                 FROM fact_entity fe
                 JOIN entities e ON e.id = fe.entity_id
                 WHERE fe.fact_id IN ({placeholders})"
            );
            let mut stmt = conn.prepare(&sql)?;
            let params_owned: Vec<&dyn rusqlite::ToSql> = active_ids
                .iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect();
            let mapped = stmt.query_map(params_owned.as_slice(), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for row in mapped {
                let (fid, name) = row?;
                by_fact.entry(fid).or_default().insert(name);
            }
        }

        // Existing supersession edges (to skip pairs already linked).
        let superseded: std::collections::HashSet<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, superseded_by FROM memory_entries
                 WHERE superseded_by IS NOT NULL",
            )?;
            let mapped =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            let pairs = mapped.collect::<rusqlite::Result<Vec<_>>>()?;
            let mut set = std::collections::HashSet::new();
            for (a, b) in pairs {
                set.insert((a.clone(), b.clone()));
                set.insert((b, a));
            }
            set
        };

        // O(N²) pair scan over active facts. Bounded by the 100-fact
        // cap above. Compute Jaccard from cached entity sets.
        let mut pairs: Vec<EntityOverlapPair> = Vec::new();
        for i in 0..active_ids.len() {
            for j in (i + 1)..active_ids.len() {
                let a = &active_ids[i];
                let b = &active_ids[j];
                if superseded.contains(&(a.clone(), b.clone())) {
                    continue;
                }
                let set_a = match by_fact.get(a) {
                    Some(s) if !s.is_empty() => s,
                    _ => continue,
                };
                let set_b = match by_fact.get(b) {
                    Some(s) if !s.is_empty() => s,
                    _ => continue,
                };
                let intersection = set_a.intersection(set_b).count() as f64;
                let union = set_a.union(set_b).count() as f64;
                if union == 0.0 {
                    continue;
                }
                let overlap = intersection / union;
                if overlap > ENTITY_OVERLAP_THRESHOLD {
                    pairs.push(EntityOverlapPair {
                        fact_a: a.clone(),
                        fact_b: b.clone(),
                        overlap,
                    });
                }
            }
        }
        // Deterministic ordering for idempotent output.
        pairs.sort_by(|p, q| {
            (p.fact_a.clone(), p.fact_b.clone()).cmp(&(q.fact_a.clone(), q.fact_b.clone()))
        });
        Ok(pairs)
    }

    fn lint_check_3_embedding(
        &self,
        project_id: Option<&str>,
    ) -> anyhow::Result<EmbeddingDriftCheck> {
        let conn = self.lock_conn()?;

        // 3a: embedding NULL on non-synthesis row.
        let (count_a, samples_a) = self.lint_query_ids(
            &conn,
            project_id,
            "WHERE embedding IS NULL AND source_type != 'synthesis'",
        )?;

        // 3b: embedding_dim != 384.
        let (count_b, samples_b) = self.lint_query_ids(
            &conn,
            project_id,
            "WHERE embedding_dim IS NOT NULL AND embedding_dim != 384",
        )?;

        // 3c: synthesis rows with embedding NULL — BL-022's surface.
        let (count_c, samples_c) = self.lint_query_ids(
            &conn,
            project_id,
            "WHERE source_type = 'synthesis' AND embedding IS NULL",
        )?;

        Ok(EmbeddingDriftCheck {
            embedding_null_non_synthesis_count: count_a,
            embedding_null_non_synthesis: samples_a,
            embedding_dim_mismatch_count: count_b,
            embedding_dim_mismatch: samples_b,
            synthesis_null_embedding_count: count_c,
            synthesis_null_embedding: samples_c,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::NewMemory;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn make_mem(project_id: &str, title: &str) -> NewMemory {
        let uid = uuid::Uuid::new_v4();
        NewMemory {
            project_id: project_id.to_string(),
            source_file: format!("test-{uid}.md"),
            source_type: "conclusion".to_string(),
            knowledge_type: "decisional".to_string(),
            title: title.to_string(),
            content: format!("body {uid}"),
            entities: "".to_string(),
            embedding: Some(vec![0u8; 1536]),
            embedding_dim: Some(384),
            is_longterm: false,
        }
    }

    #[test]
    fn test_lint_empty_db_all_zero() {
        let db = test_db();
        let report = db.run_lint(None).unwrap();
        assert_eq!(report.orphan_gc.superseded_by_dangling_count, 0);
        assert_eq!(report.orphan_gc.synthesis_links_orphan_count, 0);
        assert_eq!(report.unresolved_contradictions.half_v_only_count, 0);
        assert_eq!(report.unresolved_contradictions.half_s_only_count, 0);
        assert_eq!(report.unresolved_contradictions.circular_count, 0);
        assert_eq!(report.embedding_drift.embedding_null_non_synthesis_count, 0);
        assert_eq!(report.embedding_drift.synthesis_null_embedding_count, 0);
    }

    #[test]
    fn test_lint_detects_orphan_superseded_by() {
        let db = test_db();
        let id = db.insert_memory(make_mem("p", "T")).unwrap();
        // Set superseded_by to a non-existent id.
        db.invalidate_memory(&id, Some("ghost-id-deadbeef"), None)
            .unwrap();

        let report = db.run_lint(None).unwrap();
        assert_eq!(report.orphan_gc.superseded_by_dangling_count, 1);
        assert_eq!(report.orphan_gc.superseded_by_dangling[0], id);
    }

    #[test]
    fn test_lint_detects_half_v_only() {
        let db = test_db();
        let id = db.insert_memory(make_mem("p", "T")).unwrap();
        // Set valid_until but leave superseded_by NULL.
        db.invalidate_memory(&id, None, Some("expired")).unwrap();

        let report = db.run_lint(None).unwrap();
        assert_eq!(report.unresolved_contradictions.half_v_only_count, 1);
        assert_eq!(report.unresolved_contradictions.half_v_only[0], id);
    }

    #[test]
    fn test_lint_detects_synthesis_null_embedding() {
        let db = test_db();
        // Insert a synthesis row directly with NULL embedding (BL-022's surface).
        let mut m = make_mem("p", "Syn");
        m.source_type = "synthesis".to_string();
        m.knowledge_type = "factual".to_string();
        m.embedding = None;
        m.embedding_dim = None;
        let id = db.insert_memory(m).unwrap();

        let report = db.run_lint(None).unwrap();
        assert_eq!(report.embedding_drift.synthesis_null_embedding_count, 1);
        assert_eq!(report.embedding_drift.synthesis_null_embedding[0], id);
        // And NOT in 3a (which filters source_type != synthesis).
        assert_eq!(report.embedding_drift.embedding_null_non_synthesis_count, 0);
    }

    #[test]
    fn test_lint_is_read_only() {
        let db = test_db();
        let id = db.insert_memory(make_mem("p", "T")).unwrap();
        let before = db.get_memory(&id).unwrap().unwrap();
        let _ = db.run_lint(None).unwrap();
        let after = db.get_memory(&id).unwrap().unwrap();
        // Read-only invariant: recall_count + avg_relevance + last_recalled untouched.
        assert_eq!(after.recall_count, before.recall_count);
        assert_eq!(after.avg_relevance, before.avg_relevance);
        assert_eq!(after.last_recalled, before.last_recalled);
    }

    #[test]
    fn test_lint_idempotent() {
        let db = test_db();
        db.insert_memory(make_mem("p", "T")).unwrap();
        let r1 = db.run_lint(None).unwrap();
        let r2 = db.run_lint(None).unwrap();
        // generated_at differs but findings should match exactly.
        assert_eq!(
            r1.orphan_gc.superseded_by_dangling_count,
            r2.orphan_gc.superseded_by_dangling_count
        );
        assert_eq!(
            r1.unresolved_contradictions.half_v_only_count,
            r2.unresolved_contradictions.half_v_only_count
        );
        assert_eq!(
            r1.embedding_drift.synthesis_null_embedding_count,
            r2.embedding_drift.synthesis_null_embedding_count
        );
    }

    #[test]
    fn test_lint_entity_overlap_detects_high_jaccard() {
        let db = test_db();
        // Two facts with 4/4 entity match (Jaccard = 1.0) and no
        // supersession link → should appear in 2d.
        let mut m1 = make_mem("p", "A");
        m1.entities = "auth,jwt,redis,session".to_string();
        let id_a = db.insert_memory(m1).unwrap();
        let mut m2 = make_mem("p", "B");
        m2.entities = "auth,jwt,redis,session".to_string();
        let id_b = db.insert_memory(m2).unwrap();

        let report = db.run_lint(Some("p")).unwrap();
        assert_eq!(
            report
                .unresolved_contradictions
                .entity_overlap_unsuperseded_count,
            1
        );
        let pair = &report.unresolved_contradictions.entity_overlap_unsuperseded[0];
        assert!(
            (pair.fact_a == id_a && pair.fact_b == id_b)
                || (pair.fact_a == id_b && pair.fact_b == id_a)
        );
        assert!((pair.overlap - 1.0).abs() < 1e-9);
    }
}
