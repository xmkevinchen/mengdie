---
role: archaeologist
round: 1
date: 2026-04-20
---

# Round 1 — Archaeologist Findings

## Findings (with file:line evidence)

### 1. Current dreaming pass: promotion predicates and mutations

`src/core/dreaming.rs:83–89` — promotion predicate (inside `run_dreaming_with_config`):

```sql
WHERE is_longterm = 0
  AND valid_until IS NULL
  AND recall_count >= ?1          -- DEFAULT_MIN_RECALL = 3
  AND avg_relevance >= ?2         -- DEFAULT_MIN_RELEVANCE = 0.45
  AND last_recalled IS NOT NULL
  AND last_recalled >= ?3         -- cutoff = now - window_days (DEFAULT = 14)
```

Promotion write (`dreaming.rs:108`):
```sql
UPDATE memory_entries SET is_longterm = 1 WHERE <same predicates>
```

**No demotion path exists anywhere in the codebase.** The promotion pass only filters
`is_longterm = 0`; once a memory is promoted it is never touched again by Dreaming.
`is_longterm` is set to `1` on promotion and can only return to `0` via a manual
`invalidate_memory` call (`db.rs:162`), which sets `valid_until` rather than clearing
`is_longterm` anyway.

### 2. Schema fields available for decay computation

From `src/core/schema.rs:30–50` and `src/core/db.rs:38–45`:

| Field | Type (SQL) | Rust type | Nullable | Notes |
|---|---|---|---|---|
| `avg_relevance` | REAL NOT NULL DEFAULT 0.0 | f64 | no | running average via `record_recall` |
| `last_recalled` | TEXT | Option<String> | yes — NULL until first recall | RFC3339 string |
| `created_at` | TEXT NOT NULL | String | no | set at insert time |
| `recall_count` | INTEGER NOT NULL DEFAULT 0 | i64 | no | incremented each recall |
| `is_longterm` | INTEGER NOT NULL DEFAULT 0 | bool | no | 0/1 flag |

No `effective_relevance` or decay-related field exists. No migration has added one.
Current schema version is 4 (`schema.rs:4`).

### 3. Who reads `is_longterm`

**Prior-art.md §3 is STALE.** `is_longterm` IS read by search today:

- `src/core/search.rs:9` — `const LONGTERM_BOOST: f64 = 1.2`
- `src/core/search.rs:142–146` — at search result post-processing time, long-term memories
  get their normalized RRF score multiplied by 1.2 (capped at 1.0):
  ```rust
  let boosted = if entry.is_longterm {
      (normalized * LONGTERM_BOOST).min(1.0)
  } else { normalized };
  ```

This boost applies to the **returned score** used for ranking. The `record_recall` call
at line 148 uses the pre-boost `normalized` score to avoid circular amplification.

Additional readers:
- `src/bin/cli.rs:464, 484` — `mengdie list` output (JSON field + "Y/N" column)
- No read in `src/core/mcp_tools.rs` (confirmed by grep — only passes through the
  `MemoryEntry` struct which includes the field)

**Implication for BL-008**: demotion (clearing `is_longterm`) has a real search-ranking
effect today. Demotion would reduce a memory's search score from 1.2× to 1.0× of its
normalized RRF score.

### 4. Who reads `avg_relevance`

Only `dreaming.rs` reads `avg_relevance` as a decision field:
- `dreaming.rs:87` — promotion threshold comparison `avg_relevance >= 0.45`

`search.rs:135` and `search.rs:192` reference `avg_relevance` only in comments
explaining the normalization. `db.rs:238` writes it via `record_recall`.
`synthesis.rs:218` initializes new synthesis memories with `avg_relevance: 0.0`.

No code reads `avg_relevance` for ranking, filtering, or display (beyond the struct
fields serialized in `cli.rs` list output at line 462).

### 5. `last_recalled` vs `created_at`: writes and reliability

**`created_at`**: set at insert time, `db.rs:97,130` (`Utc::now().to_rfc3339()`).
Never updated after insert (the `ON CONFLICT DO UPDATE` at `db.rs:107–115` does NOT
update `created_at`). Always non-null. Fully reliable.

**`last_recalled`**: set only by `record_recall`, `db.rs:233,240`
(`Utc::now().to_rfc3339()`). Schema default is NULL. A memory that has never been
returned by a search call has `last_recalled IS NULL` — confirmed by the dreaming
predicate at `dreaming.rs:88` which guards `AND last_recalled IS NOT NULL`.

**Reliability caveat (from prior-art §1)**: `record_recall` is called on every search
hit with no session deduplication (`db.rs:148` via `search.rs`). `last_recalled` is
thus updated to `now` on every search hit, not once per day or session. For a decay
formula that uses `last_recalled` as the age input, this is actually *favorable* — it
gives the most recent recall timestamp. The inflation problem in prior-art §1 affects
`recall_count` (and thereby `avg_relevance`), not `last_recalled` directly.

### 6. Existing time handling: `chrono::Utc::now()` usage

All time usage is **inline `chrono::Utc::now()` calls**. No clock abstraction
(trait-object, injected `now` parameter, test helper) exists anywhere:

- `db.rs:97, 162, 179, 233, 311` — insert, invalidate, insert_with_resolves,
  record_recall, insert_synthesis_with_links
- `dreaming.rs:62` — the dreaming pass cutoff
- `search.rs:52` — FTS search (used to filter `valid_until IS NULL OR valid_until > now`)
- `vector.rs:52` — same validity filter
- `contradiction.rs:60` — age check for "recent conflict" window
- `metrics.rs:15` — metric timestamp

**No test helper or injectable clock.** Tests that call `run_dreaming` or `record_recall`
implicitly use wall clock time. For decay testing, any deterministic assertion about a
decay formula output requires either:
(a) injecting a `now: DateTime<Utc>` parameter into the dreaming function, or
(b) computing expected values from `chrono::Utc::now()` at test time (fragile for
    assertions on specific decay amounts unless tested relative to just-inserted rows).

### 7. Corpus stats (from `~/.mengdie/db.sqlite`)

DB is accessible. All figures are for `valid_until IS NULL` rows (live memories):

**Total / longterm**:
- Total live memories: **323**
- `is_longterm = 1`: **41** (12.7%)
- Memories with `last_recalled` set: **110** (34%)

**`avg_relevance` distribution** (among the 110 recalled memories):
| Range | Count |
|---|---|
| < 0.30 | 0 |
| 0.30–0.50 | 95 (86%) |
| 0.50–0.70 | 14 (13%) |
| ≥ 0.70 | 1 (< 1%) |
- Min: 0.462, Max: 0.746, Mean: 0.487

This confirms prior-art §2: distribution is tightly compressed just above the 0.45
promotion threshold. The *entire promoted corpus* sits in the range 0.45–0.75 with
~86% in the 0.45–0.50 band.

**`last_recalled` age distribution** (days since last recall):
| Days | Count |
|---|---|
| 0 (today) | 8 |
| 1 | 27 |
| 2 | 26 |
| 3 | 22 |
| 4 | 20 |
| 14 | 2 |
| 15 | 5 |

All recalled memories were recalled within 15 days. No memory has a `last_recalled`
older than 15 days — this is consistent with the corpus being relatively young
(first ingestion was ~15 days ago).

**`created_at` age distribution**:
| Age (days) | Total | is_longterm |
|---|---|---|
| 0 | 1 | 0 |
| 1 | 27 | 0 |
| 2 | 75 | 3 |
| 3 | 18 | 1 |
| 4 | 157 | 13 |
| 11 | 2 | 1 |
| 14 | 17 | 12 |
| 15 | 26 | 11 |

The 41 long-term memories are spread across days 2–15. None of the recently-ingested
(day 0–1) memories are long-term yet.

## Agreements / Disagreements

N/A Round 1 — no peer findings to compare against yet.

**One stale prior-art finding to flag**:
Prior-art.md §3 claims "`is_longterm` … never read by `search.rs` or `mcp_tools.rs`."
This is incorrect for the current code. `search.rs:142` applies `LONGTERM_BOOST = 1.2`
to long-term memories at search result post-processing time. The memory was written
2026-04-06; the boost was added after that date.

## Open Questions

1. **Decay formula anchor**: with `avg_relevance` compressed 0.46–0.75 (86% in 0.46–0.50),
   a decay multiplier like `0.95^days` applied for 14 days yields `0.95^14 ≈ 0.49`.
   A memory with `avg_relevance = 0.48` and `last_recalled = 14 days ago` would have
   `effective = 0.48 × 0.49 ≈ 0.24`. The proposed demotion floor of `effective < 0.01`
   would NOT trigger here — but would the *promotion* threshold of `0.45` become
   unreachable for most of the corpus under the decayed value after just 2–3 days?
   Needs a concrete pass simulation.

2. **`last_recalled` NULL for 213/323 memories**: 66% of live memories have never been
   recalled. What age input drives decay for these? `created_at` is always set, but
   using it would decay a memory that was ingested but never searched — which may or
   may not be intended.

3. **LONGTERM_BOOST and demotion**: since `is_longterm = 1` currently gives a 1.2×
   search score boost, demotion has a concrete user-visible effect (reduces search
   ranking). This is worth stating explicitly in the design — demotion is not just
   bookkeeping.

4. **No clock abstraction**: any decay implementation in `dreaming.rs` that needs
   deterministic tests will require injecting `now` as a parameter. This is a ~5 LOC
   change to the public API of `run_dreaming_with_config` but it's load-bearing for
   testability.
