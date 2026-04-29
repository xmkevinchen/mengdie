---
agent: archaeologist
round: 02
topics: [01-hook-placement, 02-failure-mode]
---

# Round 2 — archaeologist findings

## Findings (with file:line evidence)

### A. Codebase facts relevant to UAG falsification

**Fact A1 — `took_ms` is structurally incomplete under Option A.**
`mcp_tools.rs:197-202`: embedding inference is a `tokio::task::spawn_blocking` call that runs BEFORE `Db::memory_search` is invoked. If `took_ms` is measured inside `Db::memory_search`, it captures only the post-embedding DB time. The embedding time (2-10ms, sometimes 20ms cold-cache per database-optimizer.md §2) is invisible to Option A's hook. Option B (hook at `mcp_tools.rs` after line 244) can measure from before line 197 to after line 245, capturing embedding + search in one `Instant`. This is a structural difference, not a preference.

**Fact A2 — Results variable is moved before Option B hook can read IDs.**
`mcp_tools.rs:247-264`: after the `match query_embedding` block closes at line 245, `results` is consumed by `.into_iter()` at line 247 to build `items`. The Option B hook must be placed BETWEEN line 245 and line 247 to have access to `results` as `Vec<SearchResult>`, from which `entry.id` can be collected. If placed after line 264, the hook must read IDs from `items` (which uses `SearchResultItem.id` — same UUID, just a different struct). Either placement works; the earlier placement is cleaner. This is a minor implementation detail but confirms the exact hook insertion point.

**Fact A3 — FTS-fallback path has no Instant at the Db boundary.**
Under the FTS-fallback path (`mcp_tools.rs:220-244`), `Db::search_fts` at `search.rs:83-148` contains no timing instrumentation. If a `took_ms` column is to be populated for FTS-fallback searches, the measurement must be at `mcp_tools.rs` level, not inside `Db`. There is literally no `Instant::now()` call in `search_fts`. Any effort to add `took_ms` under Option A for FTS-fallback searches would require modifying BOTH `Db::memory_search` AND `Db::search_fts` independently, each with its own timing scope — different semantics and different time origins.

**Fact A4 — CLI does not fall back to FTS-only; all CLI searches are hybrid.**
`cli.rs:607-609`: `embed_text` at line 607 returns `anyhow::Result` and the CLI uses `?` — if embedding fails, the CLI exits with an error before reaching `db.memory_search`. The CLI never reaches the FTS-fallback path. This means:
- Under Option A (hook inside `Db::memory_search`): CLI is auto-covered for all hybrid searches.
- Under Option B (hook in `mcp_tools.rs`): CLI requires a separate wire-up, but that wire-up is a simple call to a shared `record_search_audit(...)` function at `cli.rs:609` — one function call, one test to add.

**Fact A5 — 028 trigger condition is a count threshold, not a rate.**
The framing cites "≥5 events per 30-day window" as the A-MEM trigger. This is a volume metric over time. Under-counting from best-effort failures means the trigger fires slightly later (delayed, not wrong-direction). For this to produce a wrong-direction false positive (trigger fires when it shouldn't), audit-write failures would have to add phantom rows — which is impossible under best-effort (failures add zero rows, never extra rows). For a false negative (trigger never fires despite enough real events), the under-count rate would have to persistently suppress the count below 5 per 30 days. At single-operator low-QPS, audit-write failures are disk-full or SQLITE_BUSY events (per database-optimizer.md §3 failure taxonomy) — transient, not continuous. A sustained failure rate that kills 5 out of 5 searches per month is a DB that's entirely broken, not an audit-write contract failure.

---

## Agreements

1. **Option B covers both paths; Option A covers only hybrid path**: Confirmed by code inspection. `mcp_tools.rs:220` is unreachable via `Db::memory_search`. All peer reports agree on this factual basis. (architecture-reviewer.md, codex-proxy.md, database-optimizer.md all confirm.)

2. **`record_recall` best-effort is at the call site**: All five R1 agents confirmed this — `db.rs:259-272` propagates errors; `search.rs:188-190` is the best-effort wrapper. database-optimizer.md §4 provides the cleanest characterization: "at the `record_recall` layer: NO — it propagates `?` errors. At the caller layer: YES."

3. **Codex's three-index design is correct**: database-optimizer.md §5 provides textbook-fit analysis with SQLite query planner reasoning. Composite `(searched_at, id)` for window scan + covering index `(fact_id, audit_id)` for FK-direction join + partial `WHERE valid_until IS NOT NULL` for supersession filter. No counterargument surfaced across 5 reports. Adopted.

4. **Transaction-coupled requires non-trivial `Db::memory_search` restructuring under both options**: Confirmed by Round 1 archaeologist.md §1 and cross-confirmed by database-optimizer.md §6 ("To put search + audit in one transaction, the function would need to be restructured to hold the lock across the entire body"). The framing's "transaction-coupled is only feasible under Option A" was an overstatement — it requires restructuring regardless.

5. **architecture-reviewer.md's Wave 2 migration cost comparison is correct**: Option B hook at `mcp_tools.rs` is already co-located at the site BL-009/BL-010 will refactor; Option A hook is entangled with `Db::memory_search` which must be moved to a free function, requiring hook extraction as a separate surgical step (architecture-reviewer.md §4).

---

## Disagreements

### D1 — database-optimizer's claim: "transaction-coupled is only feasible under Option A"

database-optimizer.md (final paragraph of §6 synthesis): "Topic 1's hook placement decision is constrained by Topic 2: option B (mcp_tools.rs) is incompatible with transaction-coupled because the connection mutex is per-method-call, not held across `Db::memory_search`."

This claim is partially correct but overstated. Under Option B, transaction-coupled IS technically achievable via a new Db-level method (e.g., `Db::memory_search_transactional`) that holds one `MutexGuard` and wraps FTS + vector + audit in one `BEGIN IMMEDIATE` transaction. `lock_conn()` is `pub(crate)` — `mcp_tools.rs` is in the same crate and can call it. However, this requires the SAME non-trivial restructuring that Option A requires, AND it requires `mcp_tools.rs` to pass the embedding result down into the Db layer — which violates the architecture-reviewer's finding that embedding is not a Db concern. So: transaction-coupled is incompatible with Option B for architectural reasons (embedding outside Db boundary), not purely for mutex reasons. The mutex argument alone is not the load-bearing blocker.

This disagreement is not material to the UAG outcome (transaction-coupled is being dropped from consideration given best-effort convergence), but the framing's coupling claim should be corrected to: "transaction-coupled is incompatible with Option B for architectural reasons (embedding is not a Db concern), not purely for mutex ownership reasons."

### D2 — gemma's Option A vote (TL-annotated reasoning inversion)

Already flagged by TL and confirmed in Round 1. gemma-proxy.md's "Option A" vote is based on a reasoning inversion: gemma says "use Option A because Option B misses FTS-fallback" — but per the framing and confirmed by code, Option A is the one that MISSES the FTS-fallback and Option B COVERS it. gemma's Topic 2 reasoning (best-effort) is sound; Topic 1 vote is discarded as inverted.

---

## UAG — Topic 1

**Falsification attempt**: Find a concrete v0.0.1 scenario where Option A (hook inside `Db::memory_search`) produces a strictly better outcome than Option B (hook in `mcp_tools.rs` after the match block).

**Scenario 1 — "Internal caller bypass"**: A future internal caller of `Db::memory_search` (e.g., a dreaming-time auto-search or a contradiction-check sub-search) would be auto-audited under Option A but would bypass the audit under Option B. However, as confirmed in Round 1 archaeologist.md §7 and framing §pre-decided #2, there are ZERO internal callers at v0.0.1. This scenario only arises if a new caller appears, which triggers a v7 BL. Not a v0.0.1 scenario.

**Scenario 2 — "CLI coverage without extra code"**: Option A auto-covers the CLI (`cli.rs:609`) without any additional wiring. Under Option B, the CLI requires a shared helper call. Is this a strictly better outcome? No — the CLI wire-up under Option B is one function call added at `cli.rs:609`. The cost is approximately 3 lines of code and one test. This is a trivial cost, not a material quality difference. It does not constitute "strictly better."

**Scenario 3 — "Simpler implementation"**: Option A requires adding audit logic only in one function (`search.rs:152`). Option B requires either: (a) placing the hook in `mcp_tools.rs` and adding a shared helper call in `cli.rs`, or (b) accepting that CLI searches are not audited until the helper is wired. Is Option A simpler? At initial implementation yes — one insertion point. But Option A's `took_ms` is incomplete (misses embedding time, Fact A1), making the audit data less useful for operator debugging. The simpler implementation produces less correct data.

**Scenario 4 — "No import dependency changes"**: Option A places audit writes in `search.rs`, which already imports `db.rs`. Option B may require `mcp_tools.rs` to call an audit helper that lives in `db.rs` or a new module — no new import dependency since `mcp_tools.rs` already imports `db::Db`. Not a material concern.

**Conclusion**: No concrete v0.0.1 scenario found where Option A produces a strictly better outcome than Option B. Every apparent advantage of Option A either (a) does not apply at v0.0.1 scope, (b) comes at the cost of incomplete `took_ms` data, or (c) has a trivially cheap Option B equivalent.

**UAG-PASS: Topic 1 — Option B (hook in `mcp_tools.rs` after line 245) is the unanimous decision.**

---

## UAG — Topic 2

**Falsification attempt**: Find a concrete A-MEM trigger scenario at v0.0.1 corpus (~200-1000 facts, single-operator, low QPS) where best-effort under-counting causes a wrong-direction trigger outcome.

**Precondition — trigger form**: "≥5 events per 30-day window" is a count threshold. Under-counting from audit-write failures produces fewer rows than actual searches. The trigger fires late (needs more real searches to cross threshold) — never early.

**Scenario 1 — "False positive (trigger fires when it shouldn't)"**: Under best-effort, audit failures add ZERO rows (no phantom insertions). Every row that exists corresponds to a real search call. The count can only be lower than reality, never higher. A false positive where the trigger fires inappropriately (count says ≥5 but real events < 5) is structurally impossible under best-effort. Best-effort CANNOT produce a false positive by audit failure.

**Scenario 2 — "False negative (trigger never fires despite enough real events)"**: For best-effort to suppress the count persistently below 5 per 30 days, audit-write failures would need to be both frequent AND sustained. At v0.0.1 (single-operator laptop, NVMe SSD, ~<10 QPS), the failure modes that fire are: `SQLITE_FULL` (disk must be completely full) and `SQLITE_BUSY` (requires concurrent writer holding lock > 5000ms). Both are extraordinary conditions. Normal operation produces zero audit-write failures. The scenario where "enough real events exist but audit never captured 5 of them in 30 days" requires persistent failure — at which point `METRIC_AUDIT_WRITE_FAILURES` is non-zero and observable. This is detectable and actionable by the operator; it is not silent.

**Scenario 3 — "Partial window corruption"**: Could a burst of failures at month-boundary create a window where the 30-day count undershoots by exactly the margin needed to suppress the trigger? E.g., if the "true count" is exactly 5 and one audit row is lost, count = 4 and trigger suppressed. This is the weakest plausible false negative. However: (a) the trigger fires on the NEXT search that writes successfully, making the false negative a delay of at most one search interval (hours to days for low-QPS); (b) `METRIC_AUDIT_WRITE_FAILURES > 0` flags that counts are unreliable, giving the operator information; (c) this is a delay, not a permanent miss. Under-counting delays A-MEM activation by days, not prevents it.

**Scenario 4 — "Adversarial clock skew causes window to misalign"**: Could a WAL stall cause an audit row to be written with a `searched_at` timestamp that's outside the 30-day window (e.g., writes queued for >30 days)? No — WAL stalls at v0.0.1 are bounded by `busy_timeout=5000ms` (`schema.rs:87`). A write either succeeds within 5 seconds or fails. A 5-second delay does not shift a row outside a 30-day window.

**Conclusion**: No concrete A-MEM trigger scenario found where best-effort under-counting causes wrong-direction trigger outcome (false positive is structurally impossible; false negative is at most a brief delay that is observable via counter).

**UAG-PASS: Topic 2 — best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES` is the unanimous decision.**

---

## Ratifications

### Index design: Codex's three-index proposal

**Ratified without EXPLAIN verification.** database-optimizer.md §5 provides sufficient analytical basis:

1. `idx_memory_search_audit_searched_id ON memory_search_audit(searched_at, id)` — composite covering the 30-day window range scan and the `id` join column. Order matters: `searched_at` first enables the range filter. Correct.

2. `idx_audit_returned_facts_fact_audit ON audit_returned_facts(fact_id, audit_id)` — covering index in reverse FK direction. Enables the supersession join (fact-driven) without a table lookup and also protects FK maintenance if PRAGMA is ever flipped on. Correct.

3. `idx_memory_entries_valid_until_id ON memory_entries(valid_until, id) WHERE valid_until IS NOT NULL` — partial index matching the supersession query's filter condition. Shrinks index to only superseded facts (the minority). Correct per SQLite partial index semantics ([SQLite Partial Indexes §3](https://www.sqlite.org/partialindex.html)).

EXPLAIN verification would be desirable for production systems with data variance. At v0.0.1 corpus size (~200-1000 facts), any reasonable index design produces sub-5ms query time. The analytical basis is sufficient; EXPLAIN is a v1+ refinement. **Adopt Codex's three-index design as-is.**

### Async vs sync audit write

**Sync ratified.** The chain of evidence is sufficient:

- database-optimizer.md §2: existing `record_recall` already does 10 fsyncs per `memory_search` call (one per result hit). The total audit write (1 `memory_search_audit` row + N `audit_returned_facts` rows) adds at most 2 fsyncs (or 1 if multi-row INSERT is used). This is within the existing cost envelope.
- database-optimizer.md §1A: implicit transactions in WAL mode guarantee the audit INSERT is atomic without needing async machinery.
- codex-proxy.md §open questions: "Should audit be async? ... SQLite WAL + single-operator low-QPS may not justify complexity." This is codex-proxy's own open question; the db-optimizer's analysis answers it.
- architecture-reviewer.md §3: "best-effort explicitly separates the concerns: search succeeds independently of whether the side effect completed." Under sync best-effort, the audit write runs after the search result is in `results` (Fact A2), before the result is returned to the MCP caller. The wait is sub-1ms (database-optimizer.md §2 latency analysis). No async complexity needed.

**Async audit write is over-engineering at v0.0.1.** A `tokio::task::spawn` for a sub-1ms SQLite write adds task-scheduler overhead (context switch + channel send) that likely exceeds the write cost. Sync is correct here. **Ratified: sync audit write in a best-effort `if let Err(e) { tracing::warn! }` wrapper.**

### CLI wiring shape: `mcp_tools::audit_search_event` vs `Db`-level helper

**Recommendation: Db-level helper, not `mcp_tools`-level helper.**

codex-proxy.md §findings: proposes `fn audit_search_event(&mut writer, event) -> Result<(), AuditError>` shared between `mcp_tools.rs` and `cli.rs:609`. synthesis.md §2 integration note: "CLI wiring cost under Option B is concrete: one shared function called from 2 sites."

The question is: where does the shared function live?

- **`mcp_tools::` namespace**: `mcp_tools.rs` is a MCP protocol handler module. A function that `cli.rs` imports from `mcp_tools.rs` creates a dependency from the CLI binary on the MCP server module — conceptually backwards (CLI depends on the MCP protocol handler for a Db-write operation). This coupling would make Wave 2 BL-009/BL-010 refactoring harder: the audit helper would live in a module that Wave 2 may consolidate into `search.rs`.

- **`db.rs` or `search.rs` (new `Db` method or module-level helper)**: `cli.rs` already imports `db::Db` directly (`cli.rs:609` calls `db.memory_search`). A `Db::record_search_audit(query, scope, took_ms, fact_ids)` method follows the existing `Db::record_recall` pattern exactly: takes minimal params, acquires the lock, executes the INSERT, returns `anyhow::Result`. The call site in `cli.rs` and `mcp_tools.rs` both use this method. Wave 2 BL-009/BL-010 wraps this Db method in a free function `search::memory_search_audited` — the Db method doesn't move, only the caller consolidates.

**Verdict**: shared audit write belongs in `Db` as `Db::record_search_audit(...)`, matching the `record_recall` pattern. Both `mcp_tools.rs` and `cli.rs` call it with `if let Err(e) { tracing::warn! }` wrappers. Wave 2 consolidates the callers; the Db method is stable.

This also resolves codex-proxy.md's CLI open question: CLI should call `Db::record_search_audit(...)` directly, NOT `mcp_tools::audit_search_event`. The CLI already uses `Db` directly; adding one more `Db` method call is consistent with its existing pattern.

---

## Open Questions

None load-bearing for convergence. Both UAGs pass. The following are implementation details for the plan author:

1. **Exact hook insertion point in `mcp_tools.rs`**: hook goes between line 245 (`}` closing the match block) and line 247 (`let items: Vec<SearchResultItem> = results.into_iter()`), before `results` is moved. Alternatively, collect fact IDs from `items` after line 264 if the hook is placed later — minor style choice.

2. **`Db::record_search_audit` signature**: needs `(query: &str, scope: Option<&str>, took_ms: i64, fact_ids: &[String]) -> anyhow::Result<()>`. Uses two INSERTs: one into `memory_search_audit` (returning `last_insert_rowid()`) + N into `audit_returned_facts`. Whether to use a multi-row VALUES INSERT or N separate prepared statements is a micro-optimization (N≤10 at default limit; either is fine).

3. **`took_ms` start point in `mcp_tools.rs`**: `Instant::now()` before line 197 (embedding start) or after line 202 (embedding done). Decision: before line 197 captures end-to-end operator-visible latency; after line 202 captures search-only latency. Either is defensible. The supersession-rate query doesn't use `took_ms`; the operator-debug use case prefers end-to-end. Recommend: before line 197.

4. **`Db::record_search_audit` in `cli.rs`**: at CLI, `took_ms` should be measured from before `embed_text` at `cli.rs:607` to after `db.memory_search` at `cli.rs:609`. Same end-to-end semantic as MCP.
