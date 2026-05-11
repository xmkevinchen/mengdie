use serde::Deserialize;

use crate::core::db::MemoryEntry;

const SYSTEM_PROMPT: &str = "You are consolidating related engineering memories. Output JSON matching one of two shapes. The synthesis shape has keys title, content, entities: title ≤ 80 chars; content 3–6 sentences, self-contained, cites the underlying decisions without naming file paths; entities is an array of 2–6 compound tags (lowercase, hyphen-separated, no spaces). The skip shape has keys skip (must be true) and reason. Only skip if the cluster demonstrates fundamental semantic incoherence (state specifically what prevents synthesis). Otherwise you MUST synthesize even a minimal common thread. No markdown, no prose outside the JSON. Do not invent a consolidation when none exists; do not abuse the skip shape as a shortcut.";

/// JSON Schema for token-decode-constrained structured output via
/// claude-CLI's `--json-schema` flag (plan 019 / BL-027 Path B).
///
/// The schema lives in `resources/synthesis-output-schema.json` so it
/// stays editor-highlightable, jq-able, and free of Rust string-escape
/// noise. `include_str!` embeds the file contents into the binary at
/// compile time — zero runtime cost, no I/O.
///
/// Shape is a **flat object** with `skip:bool` as the only required field
/// — NOT the `oneOf [synthesis-shape, skip-shape]` design originally
/// planned. Step 4 production probe found Anthropic API rejects
/// `oneOf`/`allOf`/`anyOf` at the top level of tool `input_schema`
/// ("API Error: 400 ... does not support oneOf, allOf, or anyOf at the
/// top level"); the schema was flattened with `required: ["skip"]` so
/// the model still must produce a discriminator, but all synthesis
/// fields (title/content/entities) and `reason` are schema-optional.
/// `parse_synthesis_response`'s runtime field-presence validation
/// (`MissingField`/`EmptyTitle`/`EmptyContent`) covers the semantic
/// shape that `oneOf` would have enforced structurally. `reason.minLength:
/// 20` raises the cost of lazy-skip decisions (codex-proxy plan-review
/// finding); the anti-laziness lever now leans more heavily on the
/// prompt (see `SYSTEM_PROMPT` above) than on the schema.
///
/// See `docs/spikes/019-rate-limit-measurement.md` "Schema-shape
/// post-mortem" for the incident write-up. The schema lives in
/// `resources/synthesis-output-schema.json` so it stays editor-
/// highlightable, jq-able, and free of Rust string-escape noise;
/// `include_str!` embeds the file at compile time — zero runtime cost,
/// no I/O.
///
/// Consumed by `dreaming.rs` (`run_synthesis_pass`) — passed to
/// `LlmProvider::complete_structured` so the model's output is
/// token-decode-constrained to the shape described above.
pub(crate) const SYNTHESIS_OUTPUT_SCHEMA: &str =
    include_str!("../../resources/synthesis-output-schema.json");

pub const CONTENT_CHAR_LIMIT: usize = 4000;
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

/// Result of parsing an LLM response. BL-residuals-reduction (plan 011):
/// the LLM is instructed to return `{"skip": true, "reason": "..."}` when a
/// cluster lacks a meaningful common thread (topically-adjacent pairs,
/// shared-vocabulary-but-distinct-intent pairs). The orchestration pass
/// counts `Skipped` as a separate outcome from successful synthesis or
/// parse errors, so the skip rate can be inspected per run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesisOutcome {
    Synthesized(SynthesisDraft),
    Skipped { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SynthesisError {
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
) -> Result<SynthesisOutcome, SynthesisError> {
    // Plan 019 Step 3: brace-depth scanner deleted. The LLM response
    // arrives via `LlmProvider::complete_structured`, which extracts the
    // wrapper's `.structured_output` field and re-serializes it. The
    // bytes handed to this parser are guaranteed to be a single JSON
    // object — no preamble/postamble tolerance needed.
    #[derive(Deserialize)]
    struct RawEnvelope {
        skip: Option<bool>,
        reason: Option<String>,
        title: Option<String>,
        content: Option<String>,
        entities: Option<Vec<String>>,
    }

    let parsed: RawEnvelope = serde_json::from_str(raw)?;

    // Null-escape-hatch: LLM returned `{"skip": true, ...}` signaling the
    // cluster lacks a meaningful common thread. Return Skipped without
    // validating title/content (they may be absent).
    if parsed.skip == Some(true) {
        return Ok(SynthesisOutcome::Skipped {
            reason: parsed.reason.unwrap_or_default(),
        });
    }

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

    Ok(SynthesisOutcome::Synthesized(SynthesisDraft {
        title,
        content,
        entities: entities.join(","),
        source_memory_ids: source_ids.to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_SYSTEM_PROMPT: &str = "You are consolidating related engineering memories. Output JSON matching one of two shapes. The synthesis shape has keys title, content, entities: title ≤ 80 chars; content 3–6 sentences, self-contained, cites the underlying decisions without naming file paths; entities is an array of 2–6 compound tags (lowercase, hyphen-separated, no spaces). The skip shape has keys skip (must be true) and reason. Only skip if the cluster demonstrates fundamental semantic incoherence (state specifically what prevents synthesis). Otherwise you MUST synthesize even a minimal common thread. No markdown, no prose outside the JSON. Do not invent a consolidation when none exists; do not abuse the skip shape as a shortcut.";

    // Test helper: unwrap SynthesisOutcome::Synthesized variant; panic otherwise.
    // Used by the tests that existed pre-SynthesisOutcome migration and still
    // expect the synthesis (not skip) path. Skip-path tests pattern-match
    // directly.
    fn unwrap_synthesized(outcome: SynthesisOutcome) -> SynthesisDraft {
        match outcome {
            SynthesisOutcome::Synthesized(draft) => draft,
            SynthesisOutcome::Skipped { reason } => {
                panic!("expected Synthesized, got Skipped: {reason}")
            }
        }
    }

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
    fn schema_const_is_flat_object_with_skip_discriminator() {
        // Plan 019 AC1: structural validation of SYNTHESIS_OUTPUT_SCHEMA.
        // Anthropic API rejects top-level `oneOf`/`allOf`/`anyOf` in
        // tool input_schema, so we use a flat object with `skip:bool` as
        // the discriminator (required), and synthesis fields (title,
        // content, entities) as optional. Semantic check that
        // skip:true → reason required, skip:false → title/content/entities
        // required is enforced by `parse_synthesis_response` runtime
        // validation, not by the schema (post-mortem of 2026-05-10
        // Anthropic API probe in docs/spikes/019-rate-limit-measurement.md).
        let parsed: serde_json::Value = serde_json::from_str(SYNTHESIS_OUTPUT_SCHEMA)
            .expect("SYNTHESIS_OUTPUT_SCHEMA must be valid JSON");
        assert_eq!(
            parsed.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "schema must have top-level type: \"object\" (Anthropic API requirement)"
        );
        let required = parsed
            .get("required")
            .and_then(|v| v.as_array())
            .expect("schema must have a top-level required array");
        let required_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(
            required_names,
            vec!["skip"],
            "schema must require exactly the skip discriminator at the structural level; got {required_names:?}"
        );
        let properties = parsed
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("schema must declare properties");
        for field in &["skip", "reason", "title", "content", "entities"] {
            assert!(
                properties.contains_key(*field),
                "schema must declare property `{field}`; got {:?}",
                properties.keys().collect::<Vec<_>>()
            );
        }
        assert!(
            parsed.get("oneOf").is_none()
                && parsed.get("allOf").is_none()
                && parsed.get("anyOf").is_none(),
            "schema must NOT use top-level oneOf/allOf/anyOf — Anthropic input_schema rejects these"
        );
    }

    #[test]
    fn parser_happy_path() {
        let raw = r#"{"title":"X","content":"Y.","entities":["a","b"]}"#;
        let ids = vec!["m1".to_string(), "m2".to_string()];
        let draft = unwrap_synthesized(parse_synthesis_response(raw, &ids).unwrap());
        assert_eq!(draft.title, "X");
        assert_eq!(draft.content, "Y.");
        assert_eq!(draft.entities, "a,b");
        assert_eq!(draft.source_memory_ids, ids);
    }

    // Plan 019 Step 3: parser_tolerates_preamble, parser_tolerates_postamble,
    // and parser_inner_braces_in_content removed — all guarded the deleted
    // brace-depth scanner. Preamble/postamble tolerance + inner-brace
    // robustness are now structurally guaranteed by claude-CLI's
    // --json-schema token-decode constraint, which only emits the schema-
    // validated JSON object. See `extract_structured_output` in llm.rs.

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
    fn parser_empty_string_returns_invalid_json() {
        // Plan 019 Step 3: was `parser_not_json_at_all`. SynthesisError::
        // NoJsonObject deleted with extract_first_json_object; the bare
        // "I refuse." path is now handled by serde_json::from_str failing
        // to find a JSON object. Same observable behavior, different
        // error variant.
        let err = parse_synthesis_response("", &[]).unwrap_err();
        assert!(
            matches!(err, SynthesisError::InvalidJson(_)),
            "expected InvalidJson, got {err:?}"
        );
    }

    // Plan 019 Step 3: parser_malformed_json deleted as dead code under
    // structured-output mode. claude-CLI's --json-schema rejects malformed
    // inner JSON at the schema-validation level; the wrapper would never
    // carry malformed JSON in `.structured_output` while reporting
    // is_error: false. The defense-in-depth value is zero because the
    // path is unreachable.

    #[test]
    fn parser_entities_as_objects_rejected() {
        let raw = r#"{"title":"X","content":"Y.","entities":[{"tag":"x"}]}"#;
        let err = parse_synthesis_response(raw, &[]).unwrap_err();
        assert!(matches!(err, SynthesisError::InvalidJson(_)));
    }

    // Plan 019 Step 3: parser_escaped_quote_with_unbalanced_inner_brace_is_handled,
    // parser_balanced_braces_inside_escaped_string, and
    // parser_markdown_fenced_json_extracts_cleanly all removed — they
    // guarded the deleted brace-depth scanner. Under the new structured-
    // output contract, the parser receives a single, schema-validated JSON
    // object directly from `extract_structured_output` in llm.rs. Inner
    // braces and string-escape edge cases are handled by serde_json's own
    // parser, not by mengdie code.

    // Plan 011 — null-escape-hatch parser tests

    #[test]
    fn parser_skip_happy_path() {
        let raw = r#"{"skip": true, "reason": "unrelated topics"}"#;
        let outcome = parse_synthesis_response(raw, &["a".to_string(), "b".to_string()]).unwrap();
        match outcome {
            SynthesisOutcome::Skipped { reason } => assert_eq!(reason, "unrelated topics"),
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[test]
    fn parser_skip_missing_reason_returns_empty_string() {
        let raw = r#"{"skip": true}"#;
        let outcome = parse_synthesis_response(raw, &[]).unwrap();
        match outcome {
            SynthesisOutcome::Skipped { reason } => assert_eq!(reason, ""),
            other => panic!("expected Skipped with empty reason, got {other:?}"),
        }
    }

    #[test]
    fn skip_response_without_preamble_parses_cleanly() {
        // Plan 019 Step 3: original test name was
        // `parser_skip_with_llm_preamble_still_parses`. Preamble case can
        // no longer arise under --json-schema mode (claude-CLI's
        // structured-output guarantee strips any model-added preamble).
        // Repurposed to confirm that the new clean-input contract still
        // routes a skip-shape JSON through the Skipped outcome — preserves
        // the audit trail of the original design intent (LLM skip behavior
        // is part of the contract).
        let raw = r#"{"skip": true, "reason": "topically adjacent items"}"#;
        let outcome = parse_synthesis_response(raw, &[]).unwrap();
        assert!(
            matches!(outcome, SynthesisOutcome::Skipped { .. }),
            "skip-shape JSON must produce Skipped outcome"
        );
    }

    #[test]
    fn parser_skip_false_is_treated_as_synthesis() {
        // Belt-and-suspenders: explicit skip=false should fall through to
        // synthesis validation, not be treated as a skip signal.
        let raw = r#"{"skip": false, "title": "X", "content": "Y.", "entities": ["a"]}"#;
        let outcome = parse_synthesis_response(raw, &[]).unwrap();
        match outcome {
            SynthesisOutcome::Synthesized(draft) => {
                assert_eq!(draft.title, "X");
                assert_eq!(draft.content, "Y.");
            }
            SynthesisOutcome::Skipped { reason } => {
                panic!("skip=false should NOT be treated as Skipped (got reason={reason:?})")
            }
        }
    }
}
