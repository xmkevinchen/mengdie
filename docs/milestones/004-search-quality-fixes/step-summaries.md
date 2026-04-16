## Step 1 — Lower Dreaming threshold from 0.65 to 0.45 (commit: 17222ba)
**Decisions**: Changed DEFAULT_MIN_RELEVANCE to 0.45 (Option A from discussion 013 — preserves dual-signal semantics). Linked all CLI defaults via default_value_t to prevent future drift.
**Rejected**: Option B (change RRF_MAX to 1/61) — math proof showed it destroys dual-signal differentiation. Resetting avg_relevance — unnecessary since 11+ entries already qualify at 0.45.
**Cross-step deps**: Threshold change enables Step 4 Dreaming promotion verification.
**Actual files**: src/core/dreaming.rs, src/bin/cli.rs, tests/e2e.rs

## Step 2 — Replace FTS5 phrase wrapping with AND-term matching (commit: 17222ba)
**Decisions**: Used allowlist approach (char::is_alphanumeric only) instead of denylist. Added sanitize_fts_query as public function with 7 unit tests covering edge cases.
**Rejected**: Denylist approach (missed +, ^, :, {, }, NEAR/N). NEAR matching (unnecessary complexity). OR fallback (monitor during validation first).
**Cross-step deps**: sanitize_fts_query behavior enables Step 3 test updates.
**Actual files**: src/core/search.rs

## Step 3 — Update existing search tests (commit: 17222ba)
**Decisions**: Updated dual-ranker assertion from >0.4 to >0.5 to verify FTS5 contribution. Added non-adjacent match test as AC3 evidence.
**Rejected**: None.
**Cross-step deps**: None.
**Actual files**: src/core/search.rs

## Step 4 — Build, test, and verify promotions (commit: 17222ba)
**Decisions**: All 86 tests pass, no clippy errors. Dreaming promoted 13 entries (exceeded expected 11 — DB gained entries since analysis). Verified is_longterm=1 in DB.
**Rejected**: None.
**Cross-step deps**: None.
**Actual files**: (verification only)
