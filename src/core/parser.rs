use std::path::Path;

use anyhow::Context;

/// Valid source types for memory entries.
pub const VALID_SOURCE_TYPES: &[&str] = &["conclusion", "review", "plan", "retrospect"];

/// Valid knowledge types for memory entries.
pub const VALID_KNOWLEDGE_TYPES: &[&str] = &["decisional", "experiential", "factual"];

/// Validate source_type against known values. Returns "unknown" for unrecognized values.
pub fn validate_source_type(s: &str) -> &str {
    if VALID_SOURCE_TYPES.contains(&s) { s } else { "unknown" }
}

/// Validate knowledge_type against known values. Returns "factual" (safest default) for unrecognized values.
pub fn validate_knowledge_type(s: &str) -> &str {
    if VALID_KNOWLEDGE_TYPES.contains(&s) { s } else { "factual" }
}

/// Parsed AE output file.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub title: String,
    pub content: String,
    pub entities: Vec<String>,
    pub source_type: String,
    pub knowledge_type: String,
    pub source_file: String,
}

/// Parse an AE output file (conclusion.md, review.md, plan.md, retrospect.md).
/// Extracts YAML frontmatter for metadata and body for content.
pub fn parse_ae_file(path: &Path) -> anyhow::Result<ParsedDocument> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let (frontmatter, body) = split_frontmatter(&text);

    // Parse frontmatter YAML
    let fm: serde_yaml::Value = if let Some(fm_str) = frontmatter {
        serde_yaml::from_str(fm_str)
            .with_context(|| format!("invalid frontmatter in {}", path.display()))?
    } else {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    };

    // Extract title: frontmatter "title" field, or first # heading, or filename
    let title = fm
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| extract_first_heading(body))
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    // Extract entities from frontmatter "tags" field
    let entities = extract_tags(&fm);

    // Infer source_type from filename
    let source_type = infer_source_type(path);

    // Infer knowledge_type from source_type
    let knowledge_type = match source_type.as_str() {
        "conclusion" | "plan" => "decisional".to_string(),
        "review" | "retrospect" => "experiential".to_string(),
        _ => "factual".to_string(),
    };

    Ok(ParsedDocument {
        title,
        content: body.to_string(),
        entities,
        source_type,
        knowledge_type,
        source_file: path.to_string_lossy().to_string(),
    })
}

/// Split YAML frontmatter (between --- delimiters) from body.
/// Handles: trailing newline, no trailing newline, EOF after closing ---.
fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return (None, text);
    }

    // Skip the opening "---" and any trailing chars on that line
    let after_first = &trimmed[3..];
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    // Find the closing --- on its own line
    for (i, line) in after_first.lines().enumerate() {
        if line.trim() == "---" {
            // Calculate byte offset of this line
            let mut offset = 0;
            for (j, l) in after_first.lines().enumerate() {
                if j == i {
                    break;
                }
                offset += l.len() + 1; // +1 for \n
            }
            let fm = after_first[..offset].trim_end_matches('\n');
            let body_start = offset + line.len();
            let body = if body_start < after_first.len() {
                after_first[body_start..].trim_start_matches('\n')
            } else {
                ""
            };
            return (Some(fm), body);
        }
    }

    (None, text)
}

/// Extract tags from frontmatter YAML value.
fn extract_tags(fm: &serde_yaml::Value) -> Vec<String> {
    if let Some(tags) = fm.get("tags") {
        match tags {
            serde_yaml::Value::Sequence(seq) => seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            serde_yaml::Value::String(s) => s
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect(),
            _ => vec![],
        }
    } else {
        vec![]
    }
}

/// Extract first markdown heading (# Title) from body.
fn extract_first_heading(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

/// Infer source_type from filename.
fn infer_source_type(path: &Path) -> String {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    if filename.contains("conclusion") {
        "conclusion".to_string()
    } else if filename.contains("review") {
        "review".to_string()
    } else if filename.contains("plan") {
        "plan".to_string()
    } else if filename.contains("retrospect") {
        "retrospect".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Check if a path matches AE output patterns we should ingest.
/// Uses strict matching to avoid swap files (.swp), temp files, etc.
pub fn is_ingestable(path: &Path) -> bool {
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    // Must be a .md file (not .md.swp, .md~, etc.)
    if !filename.ends_with(".md") || filename.contains(".swp") || filename.ends_with("~") {
        return false;
    }

    // Exact match or known patterns
    filename == "conclusion.md"
        || filename.starts_with("review") || filename.ends_with("-review.md") || filename == "code-review.md"
        || filename.starts_with("plan") || filename.ends_with("-plan.md")
        || filename.starts_with("retrospect") || filename.ends_with("-retrospect.md")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_split_frontmatter() {
        let text = "---\ntitle: Test\ntags: [a, b]\n---\n# Body\nContent here";
        let (fm, body) = split_frontmatter(text);
        assert!(fm.is_some());
        assert!(fm.unwrap().contains("title: Test"));
        assert!(body.starts_with("# Body"));
    }

    #[test]
    fn test_split_frontmatter_no_frontmatter() {
        let text = "# Just a heading\nSome content";
        let (fm, body) = split_frontmatter(text);
        assert!(fm.is_none());
        assert_eq!(body, text);
    }

    #[test]
    fn test_extract_tags_sequence() {
        let yaml: serde_yaml::Value =
            serde_yaml::from_str("tags: [auth, middleware, jwt]").unwrap();
        let tags = extract_tags(&yaml);
        assert_eq!(tags, vec!["auth", "middleware", "jwt"]);
    }

    #[test]
    fn test_extract_tags_string() {
        let yaml: serde_yaml::Value =
            serde_yaml::from_str("tags: \"auth, middleware\"").unwrap();
        let tags = extract_tags(&yaml);
        assert_eq!(tags, vec!["auth", "middleware"]);
    }

    #[test]
    fn test_extract_tags_missing() {
        let yaml: serde_yaml::Value = serde_yaml::from_str("title: Test").unwrap();
        let tags = extract_tags(&yaml);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_infer_source_type() {
        assert_eq!(infer_source_type(Path::new("conclusion.md")), "conclusion");
        assert_eq!(infer_source_type(Path::new("code-review.md")), "review");
        assert_eq!(infer_source_type(Path::new("001-mvp-plan.md")), "plan");
        assert_eq!(infer_source_type(Path::new("retrospect-q1.md")), "retrospect");
        assert_eq!(infer_source_type(Path::new("random.md")), "unknown");
    }

    #[test]
    fn test_is_ingestable() {
        assert!(is_ingestable(Path::new("conclusion.md")));
        assert!(is_ingestable(Path::new("code-review.md")));
        assert!(is_ingestable(Path::new("plan-001.md")));
        assert!(is_ingestable(Path::new("retrospect-q1.md")));
        assert!(!is_ingestable(Path::new("round-01.md")));
        assert!(!is_ingestable(Path::new("index.md")));
        assert!(!is_ingestable(Path::new("summary.md")));
        // Swap files / temp files excluded
        assert!(!is_ingestable(Path::new("conclusion.md.swp")));
        assert!(!is_ingestable(Path::new("conclusion.md~")));
    }

    #[test]
    fn test_split_frontmatter_eof_no_newline() {
        let text = "---\ntitle: Test\n---";
        let (fm, body) = split_frontmatter(text);
        assert!(fm.is_some());
        assert!(fm.unwrap().contains("title: Test"));
        assert!(body.is_empty());
    }

    #[test]
    fn test_parse_ae_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "---\ntitle: Auth Decision\ntags: [auth, jwt]\n---\n# Auth Decision\n\nUse JWT tokens."
        )
        .unwrap();

        let doc = parse_ae_file(&path).unwrap();
        assert_eq!(doc.title, "Auth Decision");
        assert_eq!(doc.entities, vec!["auth", "jwt"]);
        assert_eq!(doc.source_type, "conclusion");
        assert_eq!(doc.knowledge_type, "decisional");
        assert!(doc.content.contains("Use JWT tokens"));
    }

    #[test]
    fn test_parse_ae_file_no_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conclusion.md");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Simple Decision\n\nJust content.").unwrap();

        let doc = parse_ae_file(&path).unwrap();
        assert_eq!(doc.title, "Simple Decision");
        assert!(doc.entities.is_empty());
    }
}
