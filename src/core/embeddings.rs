use anyhow::Context;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wrapper around fastembed for generating text embeddings.
pub struct Embedder {
    model: TextEmbedding,
    dim: usize,
}

/// Metadata to prepend to content before embedding (from qmd learnings).
pub struct EmbeddingContext {
    pub knowledge_type: String,
    pub entities: String,
    pub project_id: String,
    pub title: String,
}

impl Embedder {
    /// Initialize the embedding model. Downloads ~90MB on first run.
    pub fn new() -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(
            // false: progress output to stderr corrupts MCP stdio transport
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
        .context("failed to initialize embedding model")?;

        Ok(Self { model, dim: 384 })
    }

    /// Generate an embedding for raw text.
    pub fn embed_text(&mut self, text: &str) -> anyhow::Result<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![text.to_string()], None)
            .context("embedding generation failed")?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no embedding returned"))
    }

    /// Generate an embedding with metadata-in-chunk encoding.
    /// Prepends structured metadata to content before embedding,
    /// improving retrieval quality without changing the query path.
    pub fn embed_with_context(
        &mut self,
        content: &str,
        ctx: &EmbeddingContext,
    ) -> anyhow::Result<Vec<f32>> {
        let enriched = format!(
            "[{}] [entities: {}] [project: {}]\nTitle: {}\n---\n{}",
            ctx.knowledge_type, ctx.entities, ctx.project_id, ctx.title, content
        );
        self.embed_text(&enriched)
    }

    /// Expected embedding dimension.
    pub fn dimension(&self) -> usize {
        self.dim
    }
}

/// Serialize a Vec<f32> to IEEE 754 little-endian bytes for BLOB storage.
pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Deserialize IEEE 754 little-endian BLOB back to Vec<f32>.
/// Returns error if blob length is not divisible by 4 (corrupted data).
pub fn blob_to_embedding(blob: &[u8]) -> anyhow::Result<Vec<f32>> {
    anyhow::ensure!(
        blob.len() % 4 == 0,
        "invalid embedding blob: length {} not divisible by 4",
        blob.len()
    );
    Ok(blob
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

/// Validate that an embedding contains only finite values (no NaN/Inf).
pub fn validate_embedding(embedding: &[f32]) -> anyhow::Result<()> {
    for (i, &v) in embedding.iter().enumerate() {
        anyhow::ensure!(
            v.is_finite(),
            "embedding contains non-finite value at index {i}: {v}"
        );
    }
    Ok(())
}

/// Cosine similarity between two vectors. Returns 0.0 for zero-length or mismatched vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_roundtrip() {
        let original = vec![1.0_f32, -0.5, 0.0, 3.14159];
        let blob = embedding_to_blob(&original);
        assert_eq!(blob.len(), 16); // 4 floats * 4 bytes
        let restored = blob_to_embedding(&blob).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_blob_malformed_length() {
        let bad_blob = vec![0u8, 1, 2]; // 3 bytes, not divisible by 4
        assert!(blob_to_embedding(&bad_blob).is_err());
    }

    #[test]
    fn test_validate_embedding_finite() {
        assert!(validate_embedding(&[1.0, 2.0, 3.0]).is_ok());
    }

    #[test]
    fn test_validate_embedding_nan() {
        assert!(validate_embedding(&[1.0, f32::NAN, 3.0]).is_err());
    }

    #[test]
    fn test_validate_embedding_inf() {
        assert!(validate_embedding(&[f32::INFINITY, 0.0]).is_err());
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_mismatched_length() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
