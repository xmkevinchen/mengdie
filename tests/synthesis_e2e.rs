//! End-to-end shape-equivalence tests for the structured-output synthesis
//! path (plan 019 Step 4 / AC5).
//!
//! These tests load hand-crafted minimal claude-CLI wrapper-JSON fixtures
//! from `tests/fixtures/`, walk them through the same extraction sequence
//! that `ClaudeCliProvider::complete_structured` uses internally
//! (parse wrapper → take `.structured_output` → re-serialize), and pass
//! the result to `parse_synthesis_response`. The goal is to confirm that
//! a real claude-CLI wrapper of either shape (synthesis or skip) routes
//! to the expected `SynthesisOutcome` variant.
//!
//! Fixtures are hand-crafted (not captured from live runs): a live capture
//! would carry non-deterministic `session_id`, `uuid`, `duration_ms`, and
//! `total_cost_usd` fields that go stale on every CLI update. The hand-
//! crafted fixtures contain only the fields the parser reads (`is_error`,
//! `result`, `structured_output`) — see plan 019 Step 4 for rationale.
//!
//! Match on **shape**, not exact values: variant + field types +
//! non-emptiness. Specific values (titles, content) come from the LLM
//! and are non-deterministic across runs.

use mengdie::core::synthesis::{parse_synthesis_response, SynthesisOutcome};

/// Extract the `.structured_output` field from a wrapper JSON file and
/// re-serialize it as a string — same shape as
/// `ClaudeCliProvider::complete_structured` returns to its caller.
fn extract_structured_output_from_fixture(path: &str) -> String {
    let body = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read fixture at {path}: {e}"));
    let envelope: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("fixture {path} is not JSON: {e}"));
    let structured = envelope
        .get("structured_output")
        .unwrap_or_else(|| panic!("fixture {path} missing `.structured_output`"));
    serde_json::to_string(structured)
        .unwrap_or_else(|e| panic!("failed to re-serialize structured_output: {e}"))
}

#[test]
fn wrapper_success_fixture_routes_to_synthesized_outcome() {
    let payload =
        extract_structured_output_from_fixture("tests/fixtures/synthesis-019-wrapper-success.json");
    let outcome = parse_synthesis_response(&payload, &["mem-a".to_string(), "mem-b".to_string()])
        .expect("success fixture must parse without error");

    match outcome {
        SynthesisOutcome::Synthesized(draft) => {
            // Shape assertions: types + non-emptiness, NOT exact values.
            assert!(
                !draft.title.is_empty(),
                "title must be non-empty in synthesis shape"
            );
            assert!(
                draft.title.chars().count() <= 80,
                "title must respect schema's maxLength: 80 (got {} chars)",
                draft.title.chars().count()
            );
            assert!(
                !draft.content.is_empty(),
                "content must be non-empty in synthesis shape"
            );
            assert!(
                !draft.entities.is_empty(),
                "entities (joined string) must be non-empty"
            );
            // Source IDs propagate from the call.
            assert_eq!(draft.source_memory_ids, vec!["mem-a", "mem-b"]);
        }
        SynthesisOutcome::Skipped { reason } => {
            panic!("success fixture must NOT route to Skipped (got reason: {reason:?})")
        }
    }
}

#[test]
fn wrapper_skip_fixture_routes_to_skipped_outcome() {
    let payload =
        extract_structured_output_from_fixture("tests/fixtures/synthesis-019-wrapper-skip.json");
    let outcome = parse_synthesis_response(&payload, &["mem-x".to_string()])
        .expect("skip fixture must parse without error");

    match outcome {
        SynthesisOutcome::Skipped { reason } => {
            assert!(
                !reason.is_empty(),
                "skip fixture's reason must be non-empty (schema enforces minLength: 20)"
            );
            assert!(
                reason.chars().count() >= 20,
                "skip reason must respect schema's minLength: 20 (got {} chars)",
                reason.chars().count()
            );
        }
        SynthesisOutcome::Synthesized(draft) => {
            panic!(
                "skip fixture must NOT route to Synthesized (got title: {})",
                draft.title
            )
        }
    }
}
