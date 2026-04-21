---
reviewer: architect-proxy (claude-sonnet-4-6)
plan: 013-exponential-decay (BL-008)
commits: 7245994..405f3b8
diff_range: 56812cb..HEAD
files_changed: 14
loc_added: 1764
loc_removed: 26
date: 2026-04-20
---

# BL-008 Architecture Review

## Scope

Four targeted questions from TL:
1. Module boundaries — `decay.rs` constants export location; same-age-clock invariant hold
2. `DreamingResult` contract — hidden invariants, enforcement
3. Dependency direction — acyclicity
4. CLI vs library split — `format_structured_json` placement

---

## Q1: Module Boundaries

### Constants export location: CORRECT

`HALF_LIFE_DAYS` and `DEMOTION_FLOOR` are `pub const` in `src/core/decay.rs`. Both `dreaming.rs` and `search.rs` access them through the `decay::` module path (via `use super::decay`). This is the right place: they are pure decay parameters with no dependency on DB or search structure, and a caller that needs the floor value for display or alerting can reach `mengdie::core::decay::DEMOTION_FLOOR` directly.

**One gap**: `search.rs` does NOT use `DEMOTION_FLOOR` — it only uses `decay_factor`. The LONGTERM_BOOST constant (`1.2`) in `search.rs` is defined locally and not exported. That's fine for now because boost is a search-path concern, not a decay concern. No issue here.

### Same-age-clock invariant: PARTIALLY HOLDS, with a divergence

The invariant is stated as: both the Dreaming pass and the search path derive the age clock from `MemoryEntry::last_recalled_as_datetime()`.

**Search path** (`search.rs:apply_boost_and_decay`): uses `entry.last_recalled_as_datetime()` — correctly routed through the shared helper.

**Dreaming demotion path** (`dreaming.rs:191-195`): does NOT use `last_recalled_as_datetime()`. Instead it does raw `DateTime::parse_from_rfc3339(last)` inline on the raw `String` fetched from SQL. This is a **same-age-clock invariant violation at the code level** — the semantics are identical (`parse_from_rfc3339` + `with_timezone(&Utc)`) but the call sites are duplicated.

The practical risk is low today (both produce the same output), but it is a maintenance hazard: if `last_recalled_as_datetime` ever gains additional logic (timezone normalization, a different format fallback), the demotion path silently diverges. The comment in `search.rs:44-46` explicitly names this invariant, but the demotion code does not call the helper.

**P2 finding (non-blocking)**: The demotion loop (lines 191-195) and the `avg_effective_score_after` re-scan (lines 273-275) should both call `last_recalled_as_datetime()` on a `MemoryEntry`-shaped struct, or the inline logic should be extracted to a free function in `decay.rs`. As written, the invariant is documented but not mechanically enforced.

---

## Q2: DreamingResult Contract

### Five new fields

`demoted`, `avg_effective_score_before`, `avg_effective_score_after`, `decay_floor_breaches`, `breached_ids`.

### Hidden invariants and enforcement status

**Invariant A: `demoted == decay_floor_breaches` in live mode (`write_demotions=true`)**

This is the most critical hidden invariant. The code sets `decay_floor_breaches = breached_ids.len()` before the UPDATE loop, and `demoted` accumulates `conn.execute(...)` return values (affected rows). These can diverge if any row in `breached_ids` was already `is_longterm=0` at UPDATE time. The promotion pass runs first in the same function body — could a promoted memory immediately get added to `breached_ids`? Only if it was just set to `is_longterm=1` by the promotion UPDATE and its `avg_relevance × decay_factor` fell below `DEMOTION_FLOOR`. With default thresholds (`min_relevance ≥ 0.45`, floor = 0.20), this would require `decay_factor < 0.444`, i.e., `d > 71` days. But the recency window (`window_days = 14`) means newly promoted memories had `last_recalled` within 14 days — decay_factor > 0.89 — so this race cannot happen under default config.

**However**, with custom config (`window_days` set large), a memory recalled 71+ days ago could promote and immediately breach the floor in the same pass. If that happened, the promotion UPDATE sets `is_longterm=1`, the decay scan sees it and adds it to `breached_ids`, then the demotion UPDATE sets it back to `is_longterm=0`. The final DB state is `is_longterm=0` (correctly), `demoted=1`, `decay_floor_breaches=1`. These agree in this edge case — but the memory was promoted and immediately demoted in the same pass, which is semantically odd and unlogged.

**More importantly**: the demotion UPDATE is an `IN (...)` filter but does NOT add `AND is_longterm = 1` to the WHERE clause. So it will "successfully" execute `SET is_longterm = 0` on a row that was never `is_longterm=1` to begin with — and `conn.execute()` will return 1 for that row. This is not a bug in normal operation (the decay scan already filtered to `WHERE is_longterm = 1`), but it is a subtle assumption.

**P2 finding**: The invariant `demoted == decay_floor_breaches` in live mode is not enforced or asserted in tests. The test `decay_demotes_below_floor_and_preserves_above` checks `result.demoted == 2` and `result.decay_floor_breaches == 2` separately, which is good coverage, but there is no test that constructs a scenario where they could differ and verifies they do not.

**Invariant B: `demoted == 0` in dry-run (`write_demotions=false`)**

Enforced in code (`demoted` is initialized to 0 and only incremented inside `if write_demotions`). The test `decay_dry_run_counts_breaches_but_never_writes` asserts this. SOUND.

**Invariant C: `breaches.len() == decay_floor_breaches`**

`decay_floor_breaches = breached_ids.len()` is set on line 208, before the struct is constructed on line 294. These are always equal by construction. The JSON formatter also derives both fields independently from the same struct (`result.decay_floor_breaches` and `result.breached_ids.len()`) — the CLI test `format_structured_json_breaches_array_length_matches_decay_floor_breaches` covers this. SOUND.

**Invariant D: `avg_effective_score_after == avg_effective_score_before` exactly in dry-run**

Enforced in code (the `else` branch explicitly returns `avg_effective_score_before`). Test `decay_dry_run_counts_breaches_but_never_writes` uses `assert_eq!` on both values. SOUND.

### Public surface soundness

`DreamingResult` is `pub` with all fields `pub`. The `breached_ids` field is a `Vec<String>` of memory IDs — it is not clear from the type whether these are row `id` values or some other key. In context it is always `id` (the UUID primary key), but nothing in the type enforces this. **Minor doc gap, not a blocker.**

`DreamingResult` does not implement `Clone` or `Serialize`. The CLI tests construct it directly using a struct literal (in `sample_result`), which means adding a new field to `DreamingResult` will produce a compile error at every construction site — an acceptable and intentional forcing function. SOUND for a `#[derive(Debug)]`-only struct.

---

## Q3: Dependency Direction

Import graph for BL-008 additions:

```
decay.rs        → chrono only (no internal deps)
dreaming.rs     → decay, clustering, db, llm, synthesis
search.rs       → decay, db, vector
db.rs           → chrono only (for last_recalled_as_datetime)
cli.rs (bin)    → mengdie::core::dreaming, mengdie::core::decay (transitively)
```

`decay.rs` imports nothing from the `core::` tree — only `chrono`. No cycle is possible from it.

`dreaming.rs` → `decay` is fine: decay does not import dreaming or search.

`search.rs` → `decay` is fine: decay does not import search.

**The dependency graph is a DAG. No circular imports.** I verified by grepping all `use super::` and `use crate::` statements in `decay.rs` — it has none outside its own test module.

---

## Q4: CLI vs Library Split for `format_structured_json`

**Current placement**: `format_structured_json` and `format_dreaming_line` are private `fn`s in `src/bin/cli.rs`.

**Is this right?** Yes, for the current state of the project.

The structured JSON contract is specifically an **operator-facing CLI contract** — it targets `scripts/verify-decay.sh`, which greps stderr for a bare `{"event":"dreaming_pass",...}` line. This contract is meaningfully different from what an MCP tool or library consumer would need: an MCP tool would return `DreamingResult` fields as a typed JSON-RPC response, not as a raw string on stderr with `eprintln!`. The `dry_run` boolean is also a CLI-layer concept — downstream consumers receive `DreamingResult` and can inspect `demoted == 0` to determine dry-run mode.

Moving `format_structured_json` into `core::dreaming` would couple the core crate to `serde_json` (for `serde_json::json!` macro) and to the CLI's operator-facing wire format. The core crate currently has no direct `serde_json` dependency for `DreamingResult` serialization.

**The one real risk** is if a second binary (e.g., a future daemon or MCP tool) needs to emit the same event format. At that point, the right move is a `DreamingResult::to_operator_event(dry_run: bool) -> serde_json::Value` method on the struct in `core::dreaming`, not a free function in `cli.rs`. This is a reasonable deferral — no second consumer exists today.

**P2 consideration (not a blocker)**: document at the call site that the JSON shape is the stable contract (not just the function comment), and add a version field to the emitted JSON (`"schema_version": 1`) before advertising it externally. Currently `verify-decay.sh` owns the parser side — if the shape ever changes, the script silently breaks. But that is an ops concern, not an architectural one.

---

## Summary

**P1 blockers**: None found.

**P2 items** (non-blocking, worth tracking):

| # | Location | Finding |
|---|----------|---------|
| P2-1 | `dreaming.rs:191-195, 273-275` | Same-age-clock invariant documented but not mechanically enforced: demotion loop and after-scan re-implement `last_recalled_as_datetime()` inline instead of calling the shared helper on `db.rs`. Low practical risk today; refactor hazard if the helper gains logic. |
| P2-2 | `dreaming.rs:235-246` | `demoted == decay_floor_breaches` in live mode is a hidden invariant without a dedicated test for divergence. Covered incidentally by existing tests, but no adversarial fixture (e.g., row deleted between scan and UPDATE). |
| P2-3 | `cli.rs:200-224` | `format_structured_json` carries no schema version. Acceptable at MVP; add `"schema_version": 1` before any external consumer other than `verify-decay.sh`. |

**No P1 blockers. Architecture is sound for the BL-008 scope.**
