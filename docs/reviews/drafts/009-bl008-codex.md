---
title: BL-008 Correctness Review (Codex)
date: 2026-04-20
reviewer: Codex
scope: "commits 56812cb..HEAD (14 files, +1764/-26 LOC)"
focus: "Mathematical correctness + state-machine logic"
---

# BL-008 Cross-Family Correctness Review

**Reviewer**: Codex (reasoning_effort: medium)  
**Date**: 2026-04-20  
**Scope**: Git commits `56812cb..HEAD` (exponential decay subsystem)  
**Focus Areas**: Half-life math, state-machine semantics, edge cases

## Executive Summary

**Overall Verdict**: PASS with caveat on test portability

The implementation is mathematically and logically sound for production. All five core questions assessed:

1. ✅ **Half-life math**: Correct in production; test portability risk (low, addressable)
2. ✅ **Duration boundary semantics**: No early-demotion bug; lossy conversion is sub-second
3. ✅ **Dry-run state semantics**: Empty-breach shortcut is correct
4. ✅ **Chunked UPDATE correctness**: Missing IDs handled gracefully (SQLite silent skip)
5. ⚠️ **Smoke test coverage**: Verifies boundaries; does not mathematically prove formula uniqueness

**Recommendation**: Soften strict-equality tests to tight epsilon to improve portability. No code changes needed.

---

## Finding 1: IEEE-754 Portability of `powf` Exactness

### Status: **LOW RISK** (test fragility, not production bug)

**Code**: `src/core/decay.rs:34-41` (decay_factor)  
**Tests**: `src/core/decay.rs:105` (d=60 → 0.5 exactly), `src/core/decay.rs:117` (d=120 → 0.25 exactly)

### Issue

The half-life invariant tests use **strict bitwise equality**:

```rust
assert_eq!(decay_factor(60.0), 0.5);
assert_eq!(decay_factor(120.0), 0.25);
```

The implementation uses `f64::powf`:

```rust
(2.0_f64).powf(-days / HALF_LIFE_DAYS)
```

**Rust documents `f64::powf` with "unspecified precision".** The actual binary result can vary across platforms, compiler versions, and libm implementations. If `powf` returns `0.5000000000000001` or `0.4999999999999999` on some CI platform, the tests fail immediately due to bitwise equality.

### Production Impact: None

For actual demotion logic (`effective_relevance < DEMOTION_FLOOR` at line 59), a 1-ULP wobble around 0.5 is negligible. A memory's effective score would need to be sitting within machine-epsilon of 0.20 for this to matter, which doesn't occur in practice.

### Test Impact: Fragile

- ✅ Likely works on mainstream x86_64 (glibc libm) and macOS aarch64
- ⚠️ Not portable to all platforms (wasm, exotic libc, future Rust versions)
- ⚠️ Tight coupling to transcendental implementation detail

### Root Cause

The doc comment at line 11 claims "exactly":

```rust
/// Half-life in days. `decay_factor(HALF_LIFE_DAYS) == 0.5` exactly.
```

This overstates the language guarantee. The design decision (discussion 019) chose the semantic form `2^(-d/H)` for clarity, but did not explicitly address floating-point portability of the test.

### Recommendation

**Soften strict-equality tests to a tight relative epsilon (1e-15)** that still catches the semantic regression (`exp(-d/H)` vs `2^(-d/H)` differ by ~37% at d=60):

```rust
#[test]
fn decay_factor_d60_half_life_is_exactly_half() {
    // Regression catch: detect reintroduction of exp(-d/H) form.
    // Use epsilon instead of strict equality for portability across libm.
    assert!((decay_factor(60.0) - 0.5).abs() < 1e-15);
}

#[test]
fn decay_factor_d120_two_half_lives() {
    // Two half-lives = 0.25 exactly (mathematically).
    assert!((decay_factor(120.0) - 0.25).abs() < 1e-15);
}
```

Update line 11 comment to remove "exactly":

```rust
/// Half-life in days. `decay_factor(HALF_LIFE_DAYS)` yields approximately `0.5`.
```

---

## Finding 2: Duration Boundary Conversion — Sub-Second Truncation

### Status: **NO BUG**

**Code**: `src/core/decay.rs:52-54` (effective_relevance)

### Scenario Examined

A memory inserted 77 days + 1 second ago: does it demote prematurely?

- `(now - last_recalled).num_seconds()` returns `6652801` seconds (i64)
- Cast to f64: `6652801.0`
- Divide by 86400: `77.00001157...` days
- `decay_factor(77.00001157) ≈ 0.20008...` (still above floor 0.20)
- **Does NOT demote**

The correct demotion boundary for `avg_relevance = 0.487` is at d ≈ 77.0353 days (about 77d 50m 50s).

### Why No Early Demotion

The lossy part is **sub-second truncation**, not precision loss in the division. `num_seconds()` rounds down to the last full second, which delays demotion by less than 1 second — never accelerates it.

### Conclusion

The duration conversion is safe. The floor-boundary tests (`d=75, d=77, d=78`) all pass correctly because the seeded timestamps in the e2e test use exact integer-day offsets, which have no sub-second component to lose.

---

## Finding 3: Dry-Run State Semantics — Empty-Breach Shortcut

### Status: **CORRECT**

**Code**: `src/core/dreaming.rs:252-253`

```rust
let avg_effective_score_after = if !write_demotions || breached_ids.is_empty() {
    avg_effective_score_before
} else {
    // second scan
}
```

### Semantics Verified

- **Dry-run (`!write_demotions`)**: No UPDATE runs. The long-term set is unchanged. Setting `_after = _before` is correct.
- **Live run + no breaches (`write_demotions && breached_ids.is_empty()`)**: No UPDATE runs because the loop at line 236 never executes. The set is unchanged. Setting `_after = _before` is correct.
- **Live run + breaches**: Full second scan to recompute the mean across the survivors.

The short-circuit is both an optimization and a semantic simplification: if nothing changed, the metric is identical.

---

## Finding 4: Chunked UPDATE Loop — Missing ID Handling

### Status: **CORRECT**

**Code**: `src/core/dreaming.rs:237-246`

```rust
for chunk in breached_ids.chunks(500) {
    let placeholders = std::iter::repeat_n("?", chunk.len()).collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE memory_entries SET is_longterm = 0 WHERE id IN ({placeholders})");
    let params_dyn: Vec<&dyn ToSql> = chunk.iter().map(|s| s as &dyn ToSql).collect();
    demoted += conn.execute(&sql, params_dyn.as_slice())?;
}
```

### Concurrent Deletion Scenario

If an ID in `breached_ids` was concurrently deleted (not in the table):

- SQLite `UPDATE ... WHERE id IN (...)` **silently skips** the missing ID
- `conn.execute()` returns the affected-row count (0 for the missing ID)
- Accumulated `demoted` count is correct: it reflects actual rows updated

### Correctness

This is correct behavior. Under concurrency, the demotion count can be less than `decay_floor_breaches` (which counts would-demote IDs regardless of presence). This is an observability nuance, not a logic error. The plan accommodates this: `demoted <= decay_floor_breaches` is documented in the result struct.

### Chunking Justification

Default SQLite SQLITE_MAX_VARIABLE_NUMBER is 999. Chunking at 500 is conservative and safe.

---

## Finding 5: Smoke Test — Boundary Verification, Not Formula Proof

### Status: **LOW COVERAGE**, but acceptable for acceptance criteria

**Code**: `tests/e2e.rs:139` onwards (decay_smoke_on_seeded_corpus)

### What It Verifies

The test:
- Seeds 6 long-term memories with controlled `avg_relevance = 0.487` and `last_recalled` at d=0, 15, 75, 77, 78, 137
- Freezes time at 2026-10-01 (clock is injected, not wall-clock dependent)
- Runs one dreaming pass
- Verifies exactly 2 demote (d=78, d=137) and 4 survive (d=0, 15, 75, 77)

### What It Does NOT Verify

The test does not prove `2^(-d/H)` is the correct formula — it only checks that the six fixture points land in the right demotion state. A different polynomial or exponential that coincidentally matched those boundaries could pass.

### Acceptable?

**Yes, for this acceptance criterion.** The plan's AC1 states:

> "Verify the exact demotion outcome across the boundary cases from plan 013 AC1: d=0 and d=15 stay promoted; d=75 rides the floor boundary and survives; d=77 is just above the floor; d=78 just crosses; d=137 demotes hard."

The smoke test directly satisfies that criterion. The formula correctness is anchored in:
- Discussion 019 Round 2 codex analysis (explicit 2^(-d/H) derivation)
- Plan 013 Step 1 derivation with floor calibration
- Code doc comments

So while the test is not a mathematical **proof**, it aligns with the stated acceptance criteria.

### Observation

If residual-rate issues emerge (67% residuals noted in the backlog), a follow-up might want a targeted test of the formula at arbitrary day boundaries (not just the six fixtures). For now, this is acceptable.

---

## Test Coverage Gap: Null `last_recalled` Skip Rule

### Status: **MISSING TEST**

**Code**: `src/core/dreaming.rs:212-231` (null_skip logging)  
**Search symmetry**: `src/core/search.rs:52-58` (skip without decay penalty)

The dreaming pass explicitly skips long-term memories with `last_recalled IS NULL`:

```sql
WHERE is_longterm = 1
  AND valid_until IS NULL
  AND last_recalled IS NOT NULL  -- <-- the skip rule
```

And logs the count as an info-level observability event.

**No explicit test exercises this path.** The e2e test uses `days_before()` to set every memory's `last_recalled`, so the skip rule is never invoked.

### Recommendation

Add a minimal test to verify:
1. A long-term memory with `last_recalled IS NULL` is excluded from demotion
2. It remains `is_longterm = 1` after the pass
3. The info log fires with the correct skip count

Example sketch:

```rust
#[test]
fn test_decay_skips_null_last_recalled() {
    // Insert a long-term memory with last_recalled = NULL
    // Run the dreaming pass
    // Verify: it is not included in the decay scan, not demoted
    // Verify: null_skip count in logs > 0
}
```

### Risk Level

Low. The code path is simple (SQL WHERE clause), and the search.rs layer has the symmetric skip rule. But explicit coverage would remove any doubt.

---

## Soft-Delete (`valid_until`) Coverage

### Status: **MISSING EXPLICIT TEST**

**Code**: `src/core/dreaming.rs:166` and `src/core/search.rs` decay multiplier

Both scan-demote and search apply `valid_until IS NULL` to exclude soft-deleted memories.

No e2e test explicitly verifies that a soft-deleted long-term memory is excluded from demotion. The decay smoke test does not seed a `valid_until` row.

### Risk Level

Very low. The SQL filter is straightforward, and the same rule is used consistently across all memory queries. The test gap is observational, not indicative of a logic error.

---

## Summary of Findings

| Finding | Category | Severity | Recommendation |
|---------|----------|----------|---|
| 1. powf portability | Test fragility | Low | Soften to epsilon 1e-15 |
| 2. Duration boundary | Numeric | None | No change |
| 3. Dry-run shortcut | State logic | None | No change |
| 4. Chunked UPDATE | Concurrency | None | No change |
| 5. Smoke test | Coverage | Low | Acceptable as-is |
| 6. NULL skip rule test | Coverage gap | Very low | Optional nice-to-have |
| 7. valid_until test | Coverage gap | Very low | Optional nice-to-have |

---

## Verdict

**PASS** with **P3 (nice-to-have) follow-up**: soften half-life tests from strict equality to epsilon for portability.

**Production safety**: ✅ All correctness invariants hold. Math is sound. State machines are correct. Concurrency is handled safely.

**Test quality**: ✅ Smoke test aligns with acceptance criteria. Coverage gaps (null skip, valid_until) are low-risk observability, not logic gaps.

