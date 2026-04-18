use serde::Deserialize;

use crate::core::db::MemoryEntry;

const SYSTEM_PROMPT: &str = "You are consolidating related engineering memories. Output ONLY a JSON object with keys title, content, entities. title ≤ 80 chars. content 3–6 sentences, self-contained, cites the underlying decisions without naming file paths. entities is an array of 2–6 compound tags (lowercase, hyphen-separated, no spaces). No markdown, no prose outside the JSON.";

const CONTENT_CHAR_LIMIT: usize = 4000;
const TITLE_HARD_CAP: usize = 200;

pub struct SynthesisInput<'a> {
    pub cluster_memories: &'a [MemoryEntry],
    pub cluster_centroid: &'a [f32],
    pub project_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesisDraft {
    pub title: String,
    pub content: String,
    pub entities: String,
    pub source_memory_ids: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SynthesisError {
    #[error("no JSON object found in LLM response")]
    NoJsonObject,
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },
    #[error("title is empty")]
    EmptyTitle,
    #[error("content is empty")]
    EmptyContent,
}

pub fn build_synthesis_prompt(input: &SynthesisInput) -> (String, String) {
    debug_assert!(
        !input.cluster_memories.is_empty(),
        "build_synthesis_prompt called with empty cluster"
    );
    let _ = input.cluster_centroid;

    let n = input.cluster_memories.len();
    let mut user = String::new();
    user.push_str("Project: ");
    user.push_str(input.project_id);
    user.push_str("\n\nMemories in this cluster (");
    user.push_str(&n.to_string());
    user.push_str("):\n\n");

    for (i, mem) in input.cluster_memories.iter().enumerate() {
        user.push_str("--- MEMORY ");
        user.push_str(&(i + 1).to_string());
        user.push_str(" ---\n");
        user.push_str("Title: ");
        user.push_str(&mem.title);
        user.push('\n');
        user.push_str("Entities: ");
        user.push_str(&mem.entities);
        user.push('\n');

        let char_count = mem.content.chars().count();
        if char_count > CONTENT_CHAR_LIMIT {
            let truncated: String = mem.content.chars().take(CONTENT_CHAR_LIMIT).collect();
            user.push_str(&truncated);
            user.push_str(" … [truncated]");
        } else {
            user.push_str(&mem.content);
        }
        user.push_str("\n\n");
    }

    user.push_str("Write the synthesis JSON now.");

    (SYSTEM_PROMPT.to_string(), user)
}

pub fn parse_synthesis_response(
    raw: &str,
    source_ids: &[String],
) -> Result<SynthesisDraft, SynthesisError> {
    let json_slice = extract_first_json_object(raw).ok_or(SynthesisError::NoJsonObject)?;

    #[derive(Deserialize)]
    struct RawJson {
        title: Option<String>,
        content: Option<String>,
        entities: Option<Vec<String>>,
    }

    let parsed: RawJson = serde_json::from_str(json_slice)?;

    let title = parsed
        .title
        .ok_or(SynthesisError::MissingField { field: "title" })?;
    let content = parsed
        .content
        .ok_or(SynthesisError::MissingField { field: "content" })?;
    let entities = parsed.entities.unwrap_or_default();

    if title.is_empty() {
        return Err(SynthesisError::EmptyTitle);
    }
    if content.is_empty() {
        return Err(SynthesisError::EmptyContent);
    }

    let title = if title.chars().count() > TITLE_HARD_CAP {
        title.chars().take(TITLE_HARD_CAP).collect()
    } else {
        title
    };

    Ok(SynthesisDraft {
        title,
        content,
        entities: entities.join(","),
        source_memory_ids: source_ids.to_vec(),
    })
}

fn extract_first_json_object(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth: i32 = 0;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_SYSTEM_PROMPT: &str = "You are consolidating related engineering memories. Output ONLY a JSON object with keys title, content, entities. title ≤ 80 chars. content 3–6 sentences, self-contained, cites the underlying decisions without naming file paths. entities is an array of 2–6 compound tags (lowercase, hyphen-separated, no spaces). No markdown, no prose outside the JSON.";

    fn mk_memory(id: &str, title: &str, entities: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            project_id: "proj-x".to_string(),
            source_file: format!("{id}.md"),
            source_type: "conclusion".to_string(),
            knowledge_type: "decision".to_string(),
            title: title.to_string(),
            content: content.to_string(),
            entities: entities.to_string(),
            valid_from: "2026-01-01".to_string(),
            valid_until: None,
            superseded_by: None,
            recall_count: 0,
            avg_relevance: 0.0,
            last_recalled: None,
            embedding: None,
            embedding_dim: None,
            is_longterm: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn prompt_builder_three_memories_no_truncation() {
        let mems = vec![
            mk_memory("a", "Title-A", "tag-a1,tag-a2", "Short body A."),
            mk_memory("b", "Title-B", "tag-b1", "Short body B."),
            mk_memory("c", "Title-C", "tag-c1,tag-c2", "Short body C."),
        ];
        let input = SynthesisInput {
            cluster_memories: &mems,
            cluster_centroid: &[0.1, 0.2],
            project_id: "demo-project",
        };
        let (sys, user) = build_synthesis_prompt(&input);
        assert_eq!(sys, EXPECTED_SYSTEM_PROMPT);
        assert!(user.contains("Title-A"));
        assert!(user.contains("Title-B"));
        assert!(user.contains("Title-C"));
        assert!(user.contains("tag-a1,tag-a2"));
        assert!(user.contains("tag-b1"));
        assert!(user.contains("demo-project"));
        assert!(user.contains("Memories in this cluster (3)"));
        assert!(user.contains("--- MEMORY 1 ---"));
        assert!(user.contains("--- MEMORY 3 ---"));
        assert!(!user.contains("[truncated]"));
        assert!(user.ends_with("Write the synthesis JSON now."));
    }

    #[test]
    fn prompt_builder_content_exactly_4000_chars_no_marker() {
        let content: String = "a".repeat(4000);
        let mems = vec![mk_memory("a", "T", "e1", &content)];
        let input = SynthesisInput {
            cluster_memories: &mems,
            cluster_centroid: &[],
            project_id: "p",
        };
        let (_sys, user) = build_synthesis_prompt(&input);
        assert!(!user.contains("[truncated]"));
    }

    #[test]
    fn prompt_builder_content_4001_chars_appends_marker() {
        let content: String = "a".repeat(4001);
        let mems = vec![mk_memory("a", "T", "e1", &content)];
        let input = SynthesisInput {
            cluster_memories: &mems,
            cluster_centroid: &[],
            project_id: "p",
        };
        let (_sys, user) = build_synthesis_prompt(&input);
        assert!(user.contains(" … [truncated]"));
    }

    #[test]
    fn system_prompt_matches_expected() {
        let mems = vec![mk_memory("a", "T", "e", "c")];
        let input = SynthesisInput {
            cluster_memories: &mems,
            cluster_centroid: &[],
            project_id: "p",
        };
        let (sys, _user) = build_synthesis_prompt(&input);
        assert_eq!(sys, EXPECTED_SYSTEM_PROMPT);
    }

    #[test]
    fn parser_happy_path() {
        let raw = r#"{"title":"X","content":"Y.","entities":["a","b"]}"#;
        let ids = vec!["m1".to_string(), "m2".to_string()];
        let draft = parse_synthesis_response(raw, &ids).unwrap();
        assert_eq!(draft.title, "X");
        assert_eq!(draft.content, "Y.");
        assert_eq!(draft.entities, "a,b");
        assert_eq!(draft.source_memory_ids, ids);
    }

    #[test]
    fn parser_tolerates_preamble() {
        let raw = "Sure! Here:\n\n{\"title\":\"X\",\"content\":\"Y.\",\"entities\":[\"a\"]}";
        let draft = parse_synthesis_response(raw, &[]).unwrap();
        assert_eq!(draft.title, "X");
        assert_eq!(draft.entities, "a");
    }

    #[test]
    fn parser_tolerates_postamble() {
        let raw = "{\"title\":\"X\",\"content\":\"Y.\",\"entities\":[\"a\"]}\n\nHope that helps!";
        let draft = parse_synthesis_response(raw, &[]).unwrap();
        assert_eq!(draft.title, "X");
    }

    #[test]
    fn parser_inner_braces_in_content() {
        let raw = r#"{"title":"X","content":"use Arc<Mutex<{}>>","entities":[]}"#;
        let draft = parse_synthesis_response(raw, &[]).unwrap();
        assert_eq!(draft.title, "X");
        assert_eq!(draft.content, "use Arc<Mutex<{}>>");
        assert_eq!(draft.entities, "");
    }

    #[test]
    fn parser_missing_title() {
        let raw = r#"{"content":"Y","entities":[]}"#;
        let err = parse_synthesis_response(raw, &[]).unwrap_err();
        match err {
            SynthesisError::MissingField { field } => assert_eq!(field, "title"),
            other => panic!("expected MissingField{{title}}, got {other:?}"),
        }
    }

    #[test]
    fn parser_empty_title() {
        let raw = r#"{"title":"","content":"Y","entities":[]}"#;
        let err = parse_synthesis_response(raw, &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::EmptyTitle));
    }

    #[test]
    fn parser_empty_content() {
        let raw = r#"{"title":"X","content":"","entities":[]}"#;
        let err = parse_synthesis_response(raw, &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::EmptyContent));
    }

    #[test]
    fn parser_not_json_at_all() {
        let err = parse_synthesis_response("I refuse.", &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::NoJsonObject));
    }

    #[test]
    fn parser_malformed_json() {
        let err = parse_synthesis_response("{title: X}", &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::InvalidJson(_)));
    }

    #[test]
    fn parser_entities_as_objects_rejected() {
        let raw = r#"{"title":"X","content":"Y.","entities":[{"tag":"x"}]}"#;
        let err = parse_synthesis_response(raw, &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::InvalidJson(_)));
    }
}
