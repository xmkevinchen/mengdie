---
title: "Round 1 — Gemini Proxy (Testing + Observability)"
role: gemini-proxy
timestamp: 2026-04-20
---

# Topic 05: Observability & Testing Strategy — Round 1 Synthesis

This synthesis covers four concrete testing + observability decisions for BL-008 power-law decay, evaluated via Google Gemini (gemma4:26b local).

## Evaluation 1: Deterministic Time in Tests

**Decision: Option (a) — Inject `now: DateTime<Utc>` parameter through `run_dreaming_with_config`**

**Rationale:**
- **No new crate required** — uses existing `chrono` types
- **No change to `DreamingResult` public API** — the return struct is unaffected
- **Test usage**: Tests explicitly construct a `DreamingConfig` with a hardcoded `DateTime<Utc>` value (e.g., parsed from RFC3339 string) and pass it; the frozen clock bypasses the internal `Utc::now()` call
- **Trade-off**: Slightly changes `run_dreaming_with_config` signature (method input), but this is internal to the core module and not a public-facing change for consumers

**Implementation sketch:**
```rust
pub fn run_dreaming_with_config(
    &self,
    project_id: Option<&str>,
    config: &DreamingConfig,
    now: Option<chrono::DateTime<chrono::Utc>>,  // NEW optional param
) -> anyhow::Result<DreamingResult> {
    let now = now.unwrap_or_else(chrono::Utc::now);
    // ... rest of logic
}
```

Tests pass `Some(frozen_time)`, prod passes `None`.

---

## Evaluation 2: Operator Signals (Minimum Counter Set)

**Selected new fields for `DreamingResult` and `metrics.rs`:**

### 1. `demoted_count: usize`
- **Signal**: Directly counts the volume of memories demoted in this pass
- **Operator interpretation**: Spike in demoted_count → decay is over-aggressive (mass demotion event)
- **Located in**: `DreamingResult` struct (per-pass visibility)

### 2. `avg_effective_score_before: f64`
- **Signal**: Average effective relevance score (post-decay) across all eligible memories BEFORE demotion thresholds are applied
- **Operator interpretation**: Compare to baseline (first run establishes the median); shift toward zero → decay is working as intended, but watch for cliff drops
- **Located in**: `DreamingResult` struct
- **Justification**: Accounts for the known compressed distribution (0.5-centered); a distribution shift reveals whether decay is appropriately tuned

### 3. `decay_floor_breaches: usize`
- **Signal**: Count of memories whose effective score fell below the demotion floor threshold
- **Operator interpretation**: Direct evidence of over-aggressive decay; "smoking gun" for tuning decisions
- **Located in**: `DreamingResult` or `metrics.rs` table
- **Justification**: Provides the boundary signal; when combined with `demoted_count`, reveals the ratio of how many demotions were due to floor breach vs. other logic

**Why this set is minimal yet sufficient:**
- **Demotion detection**: `demoted_count` (volume)
- **Distribution monitoring**: `avg_effective_score_before` (baseline + shift)
- **Mechanism validation**: `decay_floor_breaches` (boundary behavior)

Together they form a closed loop: baseline → demotion event → mechanism confirmation.

---

## Evaluation 3: Regression Test Shape

**Table-driven test for decay formula correctness:**

```rust
#[derive(Debug)]
struct DecayTestCase {
    last_recalled: &'static str,  // RFC3339
    avg_relevance: f64,
    now: &'static str,            // RFC3339
    expected_effective: f64,
}

#[tokio::test]
async fn test_decay_formula_correctness() {
    let test_cases = vec![
        // Case 1: Recent memory (1 day old) — minimal decay
        DecayTestCase {
            last_recalled: "2026-04-19T12:00:00Z",
            avg_relevance: 0.8,
            now: "2026-04-20T12:00:00Z",
            expected_effective: 0.795,  // ~0.8 * 0.95^1
        },
        
        // Case 2: Old memory (30 days) — significant decay
        DecayTestCase {
            last_recalled: "2026-03-21T12:00:00Z",
            avg_relevance: 0.8,
            now: "2026-04-20T12:00:00Z",
            expected_effective: 0.216,  // ~0.8 * 0.95^30
        },
        
        // Case 3: Boundary threshold (memory hits the floor)
        DecayTestCase {
            last_recalled: "2026-01-20T12:00:00Z",
            avg_relevance: 0.3,
            now: "2026-04-20T12:00:00Z",
            expected_effective: 0.0,    // Clamped to floor if formula produces negative
        },
    ];
    
    let epsilon = 0.01;
    for case in test_cases {
        let last_recalled = chrono::DateTime::parse_from_rfc3339(case.last_recalled)
            .unwrap()
            .with_timezone(&chrono::Utc);
        let now = chrono::DateTime::parse_from_rfc3339(case.now)
            .unwrap()
            .with_timezone(&chrono::Utc);
        
        let actual = calculate_effective_relevance(
            case.avg_relevance,
            last_recalled,
            now,
        );
        
        assert!(
            (actual - case.expected_effective).abs() < epsilon,
            "Failed for case {:?}: expected {}, got {}",
            case,
            case.expected_effective,
            actual
        );
    }
}
```

**Key points:**
- Tests use frozen `now` (from Evaluation 1 approach)
- Epsilon tolerance of 0.01 accounts for floating-point arithmetic
- Cases span low decay (1 day), high decay (30 days), and boundary (floor clamp)

---

## Evaluation 4: Dry-Run Flag Trade-Off

**Decision: `mengdie dream --dry-run-decay` (Option A)**

**Reasoning:**

The core risk is **population-level regression**: the compressed `avg_relevance` distribution (0.5-centered) could be crushed toward zero if decay is over-aggressive.

- **Option B (`inspect <id>`)** is too narrow. A single-memory view cannot reveal whether the aggregate `demoted_count` will drop from 100 candidates to 5 candidates.
- **Option A (`--dry-run-decay`)** allows an operator to see:
  - `demoted_count` across the entire corpus
  - `avg_effective_score_before` and `avg_effective_score_after` (distribution shift)
  - `decay_floor_breaches` count
  - All without mutating the single-writer DB

This is the only way to validate the "mass demotion" risk in the face of the known distribution compression before making permanent changes.

**Scope note:** If implementation is <20 LOC (test a decay formula in a loop, print metrics), worth doing. If it requires a new schema or background task, defer to BL-008 Phase 2.

---

## Gemini Raw Output

```
[gemma4:26b local inference — full output trimmed for brevity]

### Evaluation 1: Deterministic time in tests

**Pick: a) Inject `now: DateTime<Utc>` parameter through `run_dreaming_with_config`**

* **Does it change `DreamingResult`'s public API?** No. `DreamingResult` is the *output* of the process; changing the input configuration does not alter the returned struct.
* **Does it require a new crate?** No. It uses existing `chrono` types.
* **How do tests set a frozen clock?** Tests explicitly construct a `DreamingConfig` with a hardcoded `DateTime<Utc>` (e.g., `DateTime::parse_from_rfc3339(...)`), bypassing the internal `Utc::now()` call.

---

### Evaluation 2: Operator signals (NEW fields only)

**Selected Fields:**
1. **`demoted_count`**: Directly measures the volume of the "mass demotion" signal. If this spikes unexpectedly, decay is too aggressive.
2. **`avg_effective_score_before`**: Provides the baseline relevance distribution. Essential to detect if the decay is pushing the entire 0.5-centered distribution toward zero.
3. **`decay_floor_breaches`**: The count of memories that specifically fell below the minimum threshold. This is the "smoking gun" for over-aggressive decay.

**Justification**: This set provides a complete closed-loop: Baseline (to see the shift) → Result (the demotion event) → Boundary (the mechanism of failure).

---

### Evaluation 3: Regression test shape

[pseudocode test case with 3 concrete DecayTestCase examples]

---

### Evaluation 4: Dry-run flag trade-off

**Winner: A) `mengdie dream --dry-run-decay`**

**Reasoning**:
The core risk is **population-level regression** (the compressed 0.5 distribution being crushed to 0).
* `inspect <id>` (Option B) is too narrow; it cannot reveal if the *aggregate* number of `promoted` memories will drop from 100 to 5.
* `--dry-run-decay` (Option A) allows an operator to see the `demoted_count` and `avg_effective_score_after` across the entire dataset before the single-writer DB is mutated. This is the only way to validate the "mass demotion" risk in the face of the known distribution compression.
```

---

## Cross-Family Integration Notes

The Gemini analysis prioritized **population-level signal visibility** (dry-run flag), **determinism without new crates** (inject parameter), and **boundary-aware metrics** (floor breaches). These reflect a testing lens: regression prevention before mutation, distribution-aware thresholds, and minimal churn to existing surfaces.

Recommend pairing with architect's (Claude) formula/design decisions on the shape of decay and location of computation.

---

## Deferred Questions

- **Exact epsilon values** for float comparison — empirical calibration after first decay run
- **Floor threshold tuning** — whether 0.01 is the right minimum or if it should be data-driven
- **Dry-run scope** — exact output format and whether to integrate with existing `menconfigdie dream` command or create a sibling `mengdie dream inspect`
