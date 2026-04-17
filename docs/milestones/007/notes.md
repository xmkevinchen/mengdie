# Plan 007 — Notes

## Accumulated Doodlestein Checkpoint (post-Step 3)

### Fixed in follow-up commit

CHECKPOINT [Strategic]: `llm.provider` runtime selector was ignored — `provider = "openai"` with `[llm.claude_cli]` silently built a Claude provider.
Disposition: FIXED. Added `build_provider(cfg) -> Result<Box<dyn LlmProvider>, LlmError>` dispatch fn + `LlmError::UnknownProvider(String)` variant. Direct `ClaudeCliProvider::from_config` still available for Claude-specific callers.

CHECKPOINT [Adversarial]: help-smoke test had a hard-coded flag list duplicated from `build_command` — silent drift risk.
Disposition: FIXED. Extracted `pub const CLAUDE_CLI_FLAGS: &[&str]` as single source of truth. Both `build_command` argv (checked by a new unit test) and the opt-in help-smoke test reference it.

### Deferred to BL-007 / later

CHECKPOINT [Regret]: `LlmError` and `ExitKind` public API committed before any real caller exists. BL-007 synthesis may demand renamed or new buckets (e.g. splitting `Model` into `UnsupportedModel` vs `ModelNotFound`, or new `Overloaded` bucket).
Disposition: WAIVED: plan 007 explicitly defers retry policy to the call site (non-goal section). API churn when BL-007 lands is expected; renames are a refactor, not a correctness bug. Revisit when BL-007 has a real retry loop and an empirical view of which `ExitKind` values callers actually branch on.
