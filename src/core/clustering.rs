//! Seed-neighborhood clustering over memory embeddings.
//!
//! Groups memories by cosine similarity of their stored embeddings — the
//! prerequisite for dream synthesis (the first caller passes each cluster to
//! an LLM for summary). Pure algorithm; deliberately NOT connected-component:
//! if A~B and B~C but A!~C, the seed-based pass does not chain them into one
//! cluster. Tighter topical groups produce better summarization prompts than
//! chain-linked sprawls.
//!
//! Default threshold `0.75` matches Sentence-Transformers'
//! `community_detection` default for `all-MiniLM-L6-v2` (the model mengdie
//! runs) — "tight topical community", excludes related-but-not-same
//! (~0.65–0.70). Callers can pass a lower threshold for looser clustering.

use std::collections::HashSet;

use anyhow::Context;

use super::db::Db;
use super::embeddings::{blob_to_embedding, cosine_similarity};

pub const DEFAULT_THRESHOLD: f32 = 0.75;
pub const DEFAULT_MIN_SIZE: usize = 3;

/// Dimension of the embedding model mengdie runs today
/// (`all-MiniLM-L6-v2`, see `embeddings::Embedder::new`). Inlined here until
/// fastembed exposes a const; bump together with the model swap.
const EMBEDDING_DIM: i64 = 384;

/// A cluster of memories with a centroid for downstream labeling/ranking.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub memory_ids: Vec<String>,
    pub centroid: Vec<f32>,
}

/// Result of a clustering pass: kept clusters and memories that didn't reach
/// `min_size` (policy decision — skip / summarize / misc — belongs to the
/// caller, not here).
#[derive(Debug, Clone, PartialEq)]
pub struct ClusteringResult {
    pub clusters: Vec<Cluster>,
    pub residuals: Vec<String>,
}

/// Cluster memories loaded from `db` for a given project.
///
/// Filter matches `search_vector`: non-null embedding, dimension match, not
/// expired, project-scoped. Returns `Ok(ClusteringResult { [], [] })` when
/// nothing qualifies — never Err and never panics on that path.
pub fn cluster_memories(
    db: &Db,
    project_id: Option<&str>,
    threshold: f32,
    min_size: usize,
) -> anyhow::Result<ClusteringResult> {
    let pairs = load_embeddings(db, project_id)?;
    Ok(cluster_embeddings(&pairs, threshold, min_size))
}

fn load_embeddings(db: &Db, project_id: Option<&str>) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
    let conn = db.lock_conn()?;
    let now = chrono::Utc::now().to_rfc3339();

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match project_id {
        Some(pid) => (
            "SELECT id, embedding FROM memory_entries \
             WHERE embedding IS NOT NULL \
             AND embedding_dim = ?3 \
             AND (valid_until IS NULL OR valid_until > ?1) \
             AND project_id = ?2"
                .to_string(),
            vec![
                Box::new(now.clone()) as Box<dyn rusqlite::types::ToSql>,
                Box::new(pid.to_string()),
                Box::new(EMBEDDING_DIM),
            ],
        ),
        None => (
            "SELECT id, embedding FROM memory_entries \
             WHERE embedding IS NOT NULL \
             AND embedding_dim = ?2 \
             AND (valid_until IS NULL OR valid_until > ?1)"
                .to_string(),
            vec![
                Box::new(now.clone()) as Box<dyn rusqlite::types::ToSql>,
                Box::new(EMBEDDING_DIM),
            ],
        ),
    };

    let mut stmt = conn.prepare(&sql).context("prepare cluster load")?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let id: String = row.get(0)?;
        let blob: Vec<u8> = row.get(1)?;
        Ok((id, blob))
    })?;

    let mut pairs: Vec<(String, Vec<f32>)> = Vec::new();
    for row in rows {
        let (id, blob) = row?;
        match blob_to_embedding(&blob) {
            Ok(v) => pairs.push((id, v)),
            Err(e) => {
                tracing::warn!(id = %id, error = %e, "skipping entry with malformed embedding");
            }
        }
    }
    Ok(pairs)
}

/// Pure seed-neighborhood clustering — the testable seam. No DB access.
///
/// Determinism derives from sorting the input slice by `memory_id`; the
/// `assigned` HashSet is used only for membership lookup (its iteration order
/// is never observed).
pub fn cluster_embeddings(
    pairs: &[(String, Vec<f32>)],
    threshold: f32,
    min_size: usize,
) -> ClusteringResult {
    let mut sorted: Vec<&(String, Vec<f32>)> = pairs.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut assigned: HashSet<&str> = HashSet::new();
    let mut groups: Vec<Vec<&(String, Vec<f32>)>> = Vec::new();

    for seed in &sorted {
        if assigned.contains(seed.0.as_str()) {
            continue;
        }
        let mut members: Vec<&(String, Vec<f32>)> = Vec::new();
        members.push(seed);
        assigned.insert(seed.0.as_str());

        for other in &sorted {
            if other.0 == seed.0 {
                continue;
            }
            if assigned.contains(other.0.as_str()) {
                continue;
            }
            // Mismatched-dimension pairs cannot participate in the same cluster —
            // cosine_similarity returns 0.0 for mismatched lengths anyway, but we
            // guard here explicitly so centroid() never sees mixed dimensions.
            if other.1.len() != seed.1.len() {
                continue;
            }
            if cosine_similarity(&seed.1, &other.1) >= threshold {
                members.push(other);
                assigned.insert(other.0.as_str());
            }
        }
        groups.push(members);
    }

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut residuals: Vec<String> = Vec::new();

    for group in groups {
        if group.len() >= min_size {
            let embeddings: Vec<&[f32]> = group.iter().map(|(_, e)| e.as_slice()).collect();
            let centroid = centroid(&embeddings);
            let memory_ids = group.iter().map(|(id, _)| id.clone()).collect();
            clusters.push(Cluster {
                memory_ids,
                centroid,
            });
        } else {
            for (id, _) in group {
                residuals.push(id.clone());
            }
        }
    }

    ClusteringResult {
        clusters,
        residuals,
    }
}

/// Element-wise mean of a non-empty list of equal-length embeddings.
fn centroid(embeddings: &[&[f32]]) -> Vec<f32> {
    debug_assert!(
        !embeddings.is_empty(),
        "centroid called on empty embedding slice"
    );
    if embeddings.is_empty() {
        return Vec::new();
    }
    let dim = embeddings[0].len();
    debug_assert!(
        embeddings.iter().all(|e| e.len() == dim),
        "centroid called with mixed-dimension embeddings"
    );
    let mut out = vec![0.0_f32; dim];
    for emb in embeddings {
        for (i, v) in emb.iter().enumerate().take(dim) {
            out[i] += *v;
        }
    }
    let n = embeddings.len() as f32;
    for v in &mut out {
        *v /= n;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(id: &str, emb: Vec<f32>) -> (String, Vec<f32>) {
        (id.to_string(), emb)
    }

    #[test]
    fn test_empty_input_returns_empty_result() {
        let result = cluster_embeddings(&[], 0.75, 3);
        assert_eq!(result.clusters, vec![]);
        assert_eq!(result.residuals, Vec::<String>::new());
    }

    #[test]
    fn test_three_similar_two_orthogonal_min_three() {
        let pairs = vec![
            pair("a", vec![1.0, 0.0, 0.0]),
            pair("b", vec![0.99, 0.01, 0.0]),
            pair("c", vec![0.98, 0.0, 0.02]),
            pair("d", vec![0.0, 1.0, 0.0]),
            pair("e", vec![0.0, 0.0, 1.0]),
        ];
        let result = cluster_embeddings(&pairs, 0.75, 3);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(
            result.clusters[0].memory_ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        let mut residuals = result.residuals.clone();
        residuals.sort();
        assert_eq!(residuals, vec!["d".to_string(), "e".to_string()]);
    }

    #[test]
    fn test_two_pairs_below_min_size_all_residual() {
        let pairs = vec![
            pair("a", vec![1.0, 0.0, 0.0]),
            pair("b", vec![0.99, 0.01, 0.0]),
            pair("c", vec![0.0, 1.0, 0.0]),
            pair("d", vec![0.0, 0.99, 0.01]),
        ];
        let result = cluster_embeddings(&pairs, 0.9, 3);
        assert_eq!(result.clusters.len(), 0);
        let mut residuals = result.residuals.clone();
        residuals.sort();
        assert_eq!(
            residuals,
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ]
        );
    }

    #[test]
    fn test_five_near_identical_strict_threshold() {
        let pairs = vec![
            pair("a", vec![1.0, 0.0]),
            pair("b", vec![1.0, 0.0]),
            pair("c", vec![1.0, 0.0]),
            pair("d", vec![1.0, 0.0]),
            pair("e", vec![1.0, 0.0]),
        ];
        let result = cluster_embeddings(&pairs, 0.99, 3);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].memory_ids.len(), 5);
        assert!(result.residuals.is_empty());
    }

    #[test]
    fn test_unreachable_threshold_all_residual() {
        let pairs = vec![
            pair("a", vec![1.0, 0.0]),
            pair("b", vec![1.0, 0.0]),
            pair("c", vec![1.0, 0.0]),
            pair("d", vec![1.0, 0.0]),
            pair("e", vec![1.0, 0.0]),
        ];
        let result = cluster_embeddings(&pairs, 1.5, 2);
        assert_eq!(result.clusters.len(), 0);
        assert_eq!(result.residuals.len(), 5);
    }

    #[test]
    fn test_determinism_identical_back_to_back() {
        let pairs = vec![
            pair("beta", vec![1.0, 0.0, 0.0]),
            pair("alpha", vec![0.98, 0.02, 0.0]),
            pair("gamma", vec![0.97, 0.0, 0.03]),
            pair("delta", vec![0.0, 1.0, 0.0]),
            pair("epsilon", vec![0.0, 0.99, 0.01]),
        ];
        let r1 = cluster_embeddings(&pairs, 0.75, 2);
        let r2 = cluster_embeddings(&pairs, 0.75, 2);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_seed_selection_lowest_id_first() {
        let pairs = vec![
            pair("c", vec![1.0, 0.0, 0.0]),
            pair("b", vec![1.0, 0.0, 0.0]),
            pair("a", vec![1.0, 0.0, 0.0]),
        ];
        let result = cluster_embeddings(&pairs, 0.9, 3);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(
            result.clusters[0].memory_ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_centroid_identical_embeddings() {
        let e1 = [1.0_f32, 0.0, 0.0];
        let e2 = [1.0_f32, 0.0, 0.0];
        let e3 = [1.0_f32, 0.0, 0.0];
        let c = centroid(&[&e1, &e2, &e3]);
        assert!((c[0] - 1.0).abs() < 1e-6);
        assert!(c[1].abs() < 1e-6);
        assert!(c[2].abs() < 1e-6);
    }

    #[test]
    fn test_centroid_two_embeddings_mean() {
        let e1 = [2.0_f32, 0.0];
        let e2 = [0.0_f32, 2.0];
        let c = centroid(&[&e1, &e2]);
        assert!((c[0] - 1.0).abs() < 1e-6);
        assert!((c[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_mixed_dimension_inputs_are_skipped_not_panicking() {
        // Mixed-dim pairs are safe: the explicit length guard inside
        // cluster_embeddings prevents centroid() from ever seeing mismatched
        // dimensions, even though in production all embeddings come from the
        // same fastembed model. No panic, no silent wrong centroid.
        let pairs = vec![
            pair("a", vec![1.0, 0.0, 0.0]),
            pair("b", vec![1.0, 0.0, 0.0]),
            pair("c", vec![1.0, 0.0, 0.0]),
            pair("d", vec![1.0, 0.0, 0.0, 0.0]), // different dim
        ];
        let result = cluster_embeddings(&pairs, 0.9, 3);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(
            result.clusters[0].memory_ids,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(result.residuals, vec!["d".to_string()]);
    }

    #[test]
    fn test_n200_synthetic_completes() {
        // Local-dev sanity check that O(N²) cosine is not pathologically slow at N=200.
        // Plan AC2 measures <500ms on dev hardware; the assert ceiling here is 5_000ms
        // to stay green under CI load, debug builds, and slow sanitizer runs (per
        // Codex review). Record the measured ms in the commit message.
        let mut pairs: Vec<(String, Vec<f32>)> = Vec::with_capacity(200);
        for i in 0..100 {
            pairs.push(pair(&format!("cluster-{i:04}"), vec![1.0, 0.0, 0.0]));
        }
        for i in 0..100 {
            let angle = (i as f32) * 0.03;
            pairs.push(pair(
                &format!("random-{i:04}"),
                vec![angle.cos(), angle.sin(), 0.0],
            ));
        }
        let start = std::time::Instant::now();
        let result = cluster_embeddings(&pairs, 0.75, 3);
        let elapsed = start.elapsed();
        eprintln!("test_n200_synthetic_completes: {}ms", elapsed.as_millis());
        assert!(
            elapsed.as_millis() < 5_000,
            "clustering N=200 took {}ms, far above any reasonable bound",
            elapsed.as_millis()
        );
        assert!(
            !result.clusters.is_empty(),
            "expected at least one cluster from 100 identical embeddings"
        );
    }
}
