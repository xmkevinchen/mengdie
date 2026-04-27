---
id: codex-proxy-round-01
discussionId: "023"
topic: "v0.8.5 scope decision — technical-debt risk assessment"
reviewer: codex-proxy
reasoning_lens: technical-debt prioritization + corruption risk
date: 2026-04-27
---

# Codex Proxy Round 1: v0.8.5 Technical Debt & Corruption Risk

## Findings

### 1. Schema Invariant State: App-Enforced vs DB-Enforced

**FK Enforcement: OFF (Default)**

- `memory_synthesis_links` table declares FKs on `memory_entries(id)` (src/core/schema.rs:193-199):
  ```sql
  FOREIGN KEY (source_memory_id) REFERENCES memory_entries(id),
  FOREIGN KEY (synthesis_memory_id) REFERENCES memory_entries(id)
  ```
- **But**: `PRAGMA foreign_keys = ON` is NOT called anywhere in `Db::open()` or `Db::open_in_memory()` (src/core/db.rs).
- SQLite FKs default to OFF per connection — these are convention-level only, not runtime-enforced.

**Cluster-Hash NOT NULL: App-Enforced Only**

- Invariant: `source_type='synthesis' AND valid_until IS NULL => synthesis_cluster_hash IS NOT NULL`
- Enforced by: `insert_synthesis_with_links` guards source_ids non-empty (src/core/db.rs:354-356), always computes cluster_hash (src/core/db.rs:362).
- **But**: Partial unique index `idx_synthesis_cluster WHERE source_type='synthesis' AND synthesis_cluster_hash IS NOT NULL` silently excludes any NULL-hash row.
- Alternative code paths (batch import, migration fixup, direct SQL, new tools) that bypass `insert_synthesis_with_links` could insert live-synthesis row with NULL cluster_hash without index conflict.

**Source-Type UPDATE Gap: App-Enforced Only**

- Invariant: live synthesis rows cannot have `source_type` UPDATEd to a different value.
- Enforced by: convention (not observed in code review; search/synthesis/dreaming paths treat source_type as stable).
- **But**: No DB constraint or trigger prevents UPDATE of source_type on existing rows.

### 2. Production Evidence: Orphan Row (Plan 017 Step 3)

- Plan 017 v5 migration pre-check caught **one orphan synthesis row**: `529d3212-e809-4b81-a1f5-e15143df5128` with zero links.
- Discovered via manual audit query, not caught by app code (no runtime FK check).
- **Single occurrence, but systemic evidence**: confirms that unenforced FK + orphan-class bugs can accumulate undetected in production.
- Migration did not run to completion due to this blocker; user's `~/.mengdie/db.sqlite` remains on v4 schema.

### 3. BL-009 (MCP Dream Tool) Synthesis-Writing Surface Expansion

BL-009 brings Claude into the synthesis loop **in-session** (user-facing, real-time). This expands the synthesis-writing surface:

- Current writers: `run_synthesis_pass()` (async LLM call) → `insert_synthesis_with_links()` (single entry point, src/core/dreaming.rs:~line 70+).
- BL-009 writers: Unknown, but will include:
  - MCP tool that calls the dream engine in-session
  - Possible batch-synthesis endpoints
  - Possible manual accept/reject paths
  - Possible re-synthesis (Kai updates a prompt; Claude re-runs synthesis loop)

Each new writer is an opportunity to bypass `insert_synthesis_with_links` or buggy re-implementation. Without DB-level constraints:
- Null cluster_hash → invisible to partial index → duplicate siblings allowed
- Orphan FK link → app-level corruption gracefully degraded (`get_synthesis_with_sources` returns `<deleted>` placeholder; audit will see ghosts but not fail loudly)

### 4. Risk Assessment: Which BLs Shield BL-009?

| BL | Corruption Risk Blocked | Priority if BL-009 Ships Soon |
|---|---|---|
| **BL-enable-pragma-foreign-keys** | Orphan FK links at insert time | **P1: CRITICAL** |
| **BL-synthesis-cluster-hash-not-null-enforcement** | Zombie-sibling bug via NULL hash | **P1: CRITICAL** |
| **BL-dreaming-module-split** | Code smell (indirect risk) | **P2: HYGIENE** |
| **BL-v5-migration-operator-docs** | Operator confusion (UX, not corruption) | **P3: NICE-TO-HAVE** |

**Critical reasoning**: BL-009 introduces new synthesis writers. If those writers ship WITHOUT FK enforcement and cluster-hash NOT NULL triggers:
- **Worst case 1**: New MCP tool writer forgets to call `insert_synthesis_with_links` or calls it incorrectly. FK pragma OFF means no error. Orphan link accumulates silently. `get_synthesis_with_sources` gracefully returns `<deleted>` placeholders. Kai's audit output shows ghosts. Trust in synthesis output degrades.
- **Worst case 2**: Batch-import or migration path inserts synthesis with NULL cluster_hash. Partial index is blind. Duplicate `insert_synthesis_with_links` call for same cluster succeeds (no index conflict). Corpus ends up with two live siblings. Plan 017 spent effort eliminating this exact bug — zombie-sibling reborn.

### 5. Trigger-Discipline Evaluation: Is the Rule Too Conservative?

**Current rule** (discussion 021): "don't schedule items whose trigger conditions haven't fired."

**BL-enable-pragma-foreign-keys trigger status**:
- Stated triggers: (a) first observed corruption, (b) next FK-bearing schema add, (c) schema v6 migration.
- **What has fired**: (a) YES — orphan row observed.
- **What hasn't fired**: (b) no new FK tables pending, (c) v6 migration not scheduled.
- **Verdict**: Trigger *has* fired (production evidence). The "next FK-bearing schema add" was a backup condition; observing the corruption is sufficient.

**BL-synthesis-cluster-hash-not-null-enforcement trigger status**:
- Stated triggers: (a) any non-`insert_synthesis_with_links` writer, (b) observation of violation, (c) next memory_entries migration, (d) synthesis-audit --all lands.
- **What hasn't fired**: (a) no evidence of bypass writers yet, (b) no NULL-hash violation observed (27-row corpus is small), (c) v5 just landed (v6 unknown), (d) audit --all not shipped.
- **What is about to fire**: BL-009's new writers are **incoming** non-`insert_synthesis_with_links` paths.
- **Verdict**: Trigger condition (a) is about to fire **within days of BL-009 landing**. Shipping BL-009 without the trigger is deferring a known invariant leak into the exact period that matters most.

**Conclusion on trigger discipline**: The rule is not too conservative. It's well-calibrated. However, the rule allows **anticipatory squashing** of trigger-adjacent items (condition imminent) — and the risk/reward here is lopsided. Shipping BL-009 with unenforced cluster-hash invariant is accepting known zombie-sibling risk for no upstream reason (the triggers are ready now).

### 6. Sole Operator / Hand-Coded Synthesis Risk Tolerance

Kai is solo dev building for himself, not shipping to a user base. This _usually_ means higher risk tolerance.

**But**: Kai explicitly built Mengdie as an AI-native tool to feed back into his AI workflows (discussion 022, user motivation). If the synthesis corpus accumulates silent corruption:
- Kai's trust in the tool degrades ("why is audit full of ghosts?").
- BL-009 (Claude in-session synthesis) becomes less useful if the synthesis loop can produce hard-to-debug orphans.
- Debugging corruption after the fact is harder than preventing it (especially with FKs — they give you loud errors, not silent deadness).

**Verdict**: Solo-dev status argues against deferring structural bugs. Kai will own the consequences, and he cares about correctness. The high-trust nature of the tool (feeding LLM context) amplifies the value of integrity.

---

## Agreements

1. **Industry standard for 0.x.5 is correct** (analysis.md): patches carry bugfixes / hardening / docs. New features (Gemini-proxy's Transparency Pivot items) belong in v0.9.0, not v0.8.5. Analysis is right to flag this.

2. **Module split has fired its trigger and is lower-priority** (analysis.md, archaeologist): BL-008 shipped 2026-04-20; the split was "should preferentially" at that moment. It didn't happen, so it's deferred. It's good refactoring work, not urgent. The 1326-line file (reported in analysis) is large but not unmaintainable yet. Lower priority than schema integrity.

3. **v5 migration blocker is real** (analysis.md): orphan row on production DB is a hard blocker for v5 adoption. This is independent of v0.8.5 scope and should be resolved first (per codex's direction-validation point in analysis).

4. **Trigger-discipline has been earning its keep** (v0.8.0 incident cited in analysis): 2 trigger-gated items committed and retroactively removed. The sweep-before-commit discipline is working.

---

## Disagreements

1. **Analysis.md's characterization of BL-enable-pragma-foreign-keys as "weak fire"** — I assess this as **strong fire** based on production evidence (orphan row). The "next FK-bearing schema add" was listed as _alternative_ trigger condition, not the only one. Observing corruption is the primary condition.

2. **BL-synthesis-cluster-hash-not-null-enforcement as "arguable"** — I assess this as **imminent fire**. The trigger condition "any code path other than insert_synthesis_with_links writes synthesis rows" is about to become true (BL-009 landing within days). Deferring this trigger is accepting zombie-sibling risk during the highest-leverage period (live synthesis + Claude in-session).

3. **"Module split has been deferred 7 days" suggests low urgency** — Agreed with analysis's assessment, but note: if v0.8.5 happens, this is the natural place to land it (hardening sprint shape, clean 100-LOC refactor). Bundling with BL-009 is worse (blast radius).

---

## Open Questions

1. **Is BL-009 the _only_ writer for synthesis rows post-ship?** — If BL-009 establishes a new user-facing API surface, are there follow-on tools/batch modes that would land later in Phase 2? Helps judge whether the trigger condition is imminent or speculative.

2. **What's the migration plan for existing production DB with orphan row?** — Once v0.8.5 (or v0.9.0) ships with FK enforcement, how does Kai run `PRAGMA foreign_keys = ON` on a DB that has existing orphans? Plan 017 noted: "FK PRAGMA activation itself doesn't fail — it only constrains future writes. Existing orphan links survive." So the answer is: allow FKs on, audit to find orphans, fix manually. But has this been tested?

3. **Does the partial unique index silently exclude NULL rows in SQLite, or is the exclusion explicit/documented?** — I'm assuming standard behavior (NULL violates NOT NULL in uniqueness checks, NULL rows excluded from partial index WHERE clause). But worth confirming this is the exact mechanism before shipping the trigger.

---

## Recommendation

**Ship BL-enable-pragma-foreign-keys + BL-synthesis-cluster-hash-not-null-enforcement in v0.8.5 (before BL-009).**

**Rationale**:

- **Corruption risk is asymmetric**: Deferring these costs ~0 (two small XS/S items, pure hardening). Deferring the risk costs non-zero (BL-009's new writers ship without guards).
- **Trigger conditions have fired / are imminent**: FK pragma trigger fired (production orphan). Cluster-hash trigger about to fire (BL-009 new writers landing).
- **Idiomatic 0.x.5 scope**: Both items are hardening (no new features, no new user-visible API). Small diffs (pragma one-liner, trigger pair for cluster-hash). Documentation cost is pre-check script for existing data.
- **BL-009 benefit scales with integrity**: If synthesis corpus can accumulate silent corruption, Kai's trust in in-session synthesis loops degrades. These guards let BL-009 ship with confidence.
- **Supports Phase 2 roadmap**: BL-009 → BL-010 → BL-011/BL-013. Protecting BL-009 from known corruption paths is prerequisite care.

**Conditional on resolving the production orphan first**: Once v0.8.5 ships with FK enforcement enabled, Kai's existing DB will need a one-time audit + manual fix pass. Best to resolve the `529d3212-...` row before running v5 migration (plan 017 pre-check already caught it). This is an ops task, not a code task, but sequencing matters.

**Optionally include BL-dreaming-module-split if time permits** (it's a small, clean refactor), but if v0.8.5 scope is tight, defer it to v0.9.0 opening (doesn't unblock anything; it's code hygiene that will be valuable when BL-008 decay paths are added).

**Explicitly defer BL-v5-migration-operator-docs to v0.8.5 opening or v0.9.0 scope survey** (it's doc polish, not critical path; can ride v0.9.0's wave).

---

## Supporting Evidence (File:Line)

- **FK declaration**: `src/core/schema.rs:193-199` — FKs declared, never enforced at connection level.
- **Db::open methods**: `src/core/db.rs:91-107` — `Db::open` and `Db::open_in_memory` do NOT call `PRAGMA foreign_keys = ON`.
- **Synthesis insert guard**: `src/core/db.rs:354-356` — `anyhow::ensure!(!source_ids.is_empty())` guards empty source_ids.
- **Cluster-hash compute**: `src/core/schema.rs:66-78` — deterministic computation of cluster-hash from source IDs.
- **Graceful degradation on orphan**: `src/core/db.rs:489-508` — `get_synthesis_with_sources` returns `<deleted>` placeholder for missing sources (no hard error, but audit will show ghosts).
- **Partial index definition**: `src/core/schema.rs:~v5 migration` — index is `idx_synthesis_cluster WHERE source_type='synthesis' AND synthesis_cluster_hash IS NOT NULL` (excludes NULL rows).
