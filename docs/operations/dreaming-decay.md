# Operating the Dreaming Decay Pass

The operator procedure for the exponential-decay mechanism that demotes stale
long-term memories during `mengdie dream`. For the CLI surface, see
[`docs/cli.md`](../cli.md). For the higher-level pipeline view, see
[`docs/technical-design.md`](../technical-design.md).

**Context**: Dreaming is Mengdie's periodic filtering pass — it promotes
frequently-recalled memories into a long-term tier and consolidates related
clusters via LLM synthesis. Decay is the counter-force: every long-term
memory accrues a time-decayed effective score, and once that score falls
below the floor (currently 0.20) the memory is demoted out of the long-term
tier on the next `mengdie dream` run. The procedure below is how an
operator avoids surprise demotions on the first run against a populated
corpus and how to recover when a demotion turns out to be premature.

## Quick reference

| Concept | Value |
|---|---|
| Formula | `effective = avg_relevance × 2^(-days_since_last_recalled / 60)` |
| Half-life | 60 days |
| Demotion floor | 0.20 |
| Rough first-demotion trigger | ~77 days of silence at `avg ≈ 0.487` |
| Stored `avg_relevance` | **never mutated** — decay is read-time |
| Age source | `last_recalled` (memories with `NULL` are skipped) |

## Required first-run procedure

Before the first live `mengdie dream` against a populated corpus, an operator
MUST:

1. Run `mengdie dream --decay-dry-run`. This performs the promotion pass as
   usual (writes promotions), then scans long-term memories for the decay
   pass WITHOUT clearing any `is_longterm` flags. Any memories that WOULD
   demote are reported as `decay_floor_breaches` and listed in the
   structured-JSON stderr line's `breaches` array.

2. Inspect the output. The human-readable line looks like:
   ```
   Dreaming pass: 2 promoted, 3 would-demote (DRY RUN) (3 floor breaches, avg effective 0.421 → 0.421)
   ```
   The structured-JSON line on stderr carries the full breach list.

3. **Approval gate**: if `decay_floor_breaches > max(10, 10% of the
   current long-term count)`, HALT. Either (a) recalibrate the floor (edit
   `DEMOTION_FLOOR` in `src/core/decay.rs`, rebuild), or (b) investigate
   each breached memory individually — the list is in the JSON. A spike
   this large on a young corpus means distribution drift; do not proceed
   to a live pass until the cause is understood.

   To compute the threshold without mental math, run the snippet below.
   It queries the **decay-eligible** long-term count (the three primary
   predicates — `is_longterm = 1 AND valid_until IS NULL AND last_recalled
   IS NOT NULL` — match the decay pass's static filter at
   `src/core/dreaming.rs:167-171`; the pass additionally appends an
   optional project-scope predicate when `mengdie dream --project <id>` is
   used, which the default-scope operator procedure does not invoke).
   Rows with NULL `last_recalled` are permanently immune to demotion and
   are correctly excluded from the denominator.

   <!-- threshold-snippet:begin -->
   ```sql
   -- Decay-eligible long-term count (denominator for the 10% threshold).
   -- Filter mirrors src/core/dreaming.rs:167-171.
   SELECT COUNT(*) FROM memory_entries
     WHERE is_longterm = 1
       AND valid_until IS NULL
       AND last_recalled IS NOT NULL;
   ```

   ```bash
   # Run the threshold computation and print both values side-by-side.
   # Pipe breach_count from the `mengdie dream --decay-dry-run` JSON line
   # into the comparison. Threshold = max(10, count/10).
   count=$(sqlite3 ~/.mengdie/db.sqlite \
     "SELECT COUNT(*) FROM memory_entries
        WHERE is_longterm = 1
          AND valid_until IS NULL
          AND last_recalled IS NOT NULL;")
   # Guard: bail with a clear message if sqlite3 returned non-numeric output
   # (e.g., "file is not a database" leaks into $count otherwise and the next
   # line fails with a cryptic "invalid arithmetic operator" error).
   if ! [[ $count =~ ^[0-9]+$ ]]; then
     echo "error: sqlite3 returned non-numeric output: '$count'" >&2
     exit 1
   fi
   threshold=$(( count / 10 > 10 ? count / 10 : 10 ))
   echo "long_term_eligible=${count}, threshold=${threshold}"
   # Compare: HALT if decay_floor_breaches > $threshold.
   ```
   <!-- threshold-snippet:end -->

4. If the breach count is within tolerance, proceed with a live
   `mengdie dream`. The human line will now show `N demoted` instead of
   `N would-demote (DRY RUN)`.

This procedure lives as long as `mengdie dream` is operator-invoked. When a
scheduled daemon (e.g., macOS launchd) takes over, the approval gate
transitions from an interactive flag to a threshold alarm in
`decay_floor_breaches` — see "Revisit triggers" below for the reversal
shape.

## Rollback: re-promoting a falsely-demoted memory

Incident-response procedure for when a live `mengdie dream` pass demoted
memories that — after the fact — turn out to have been prematurely stale.
Examples: a search-rank regression lands on a demoted memory that the
operator was about to re-use; the operator recognizes an ID in the breach
list as a just-recalled piece of context; a future threshold-alarm fires
on a spike during a known-burst recall pattern.

**Required input**: the `breaches` array from the structured-JSON stderr
line of the offending pass. Capture the JSON line at run time (redirect
stderr to a file or copy from the terminal scrollback) — without it,
exact row-level rollback is constrained.

**Trusted source only**: the `breaches` array must come from a real
`mengdie dream` invocation, NOT from a hand-edited or externally-supplied
JSON. IDs in the array are Rust-populated UUIDs emitted by the binary's
decay pass — the rollback SQL below does NOT defensively escape the
IDs before splicing them into the SQL string. That is safe IFF the
source is the binary's own output. Do not bypass this trust assumption.

### If the breach list is LOST (no captured stderr, no log file)

Surface this branch FIRST because the honest recovery path is limited
and an operator under incident pressure needs to know before attempting
the happy path.

- There is NO persistent dreaming output log by default. The
  structured-JSON line goes to stderr; if the operator did not redirect
  it, it is gone.
- Demotion only writes `is_longterm = 0`. It does NOT set `valid_until`,
  a `demoted_at` timestamp, or any audit-trail column (verified at
  `src/core/dreaming.rs:251-256`). The set of currently-`is_longterm=0`
  rows is a superset of the just-demoted set (includes all rows that
  never reached long-term).
- **Exact row-level rollback is not possible without the captured breach
  list.** Recovery paths require external evidence: shell history
  (`history | grep 'mengdie dream'`), terminal scrollback, a redirected
  stderr file if one exists, or reconstruction from operator memory
  ("which memories was I actively recalling today?").
- If none of those exist, the demotion is accepted as-is. Mitigation
  going forward: the next `mengdie dream --decay-dry-run` captures its
  JSON line to a file (`2>/tmp/dream-$(date +%Y%m%d).log`) so this
  branch never fires again.

### Rollback SQL (with breach list available)

Input: the `breaches` array from the captured JSON, e.g.:

```json
{"schema_version":1,"event":"dreaming_pass",...,"breaches":["abc-123","def-456"]}
```

**JSON → SQL quoting conversion** (required — see failure mode below):
the `breaches` array uses JSON double-quoted strings. SQLite requires
single-quoted literals. If the operator pastes the JSON array directly
into SQL, SQLite parses the identifiers as column names, finds none,
and the UPDATE silently matches zero rows. Convert with:

```bash
# Extract breaches, wrap each in single quotes via jq's @sh filter
# (idiomatic — jq handles the quoting correctly without bash sandwich tricks).
jq -r '.breaches | map(@sh) | join(", ")' < dream-pass.json
# or without jq:
# echo '...' | sed -n 's/.*"breaches":\[\([^]]*\)\].*/\1/p' | tr '"' "'"
# Expected output for breaches=["abc-123","def-456"]:
#   'abc-123', 'def-456'
```

Then paste the result into the UPDATE template. After running, inspect
`SELECT changes();` — if it returns 0 you either forgot to substitute
the example IDs or the IDs aren't in the corpus. A successful rollback
of N memories should print `N`:

<!-- rollback-snippet:begin -->
```sql
-- Re-promote memories by ID. Replace the list below with the
-- jq-converted, single-quoted, comma-separated breach IDs from the
-- captured dream-pass JSON. Running the template LITERALLY against the
-- example IDs 'abc-123', 'def-456' will match zero rows silently —
-- use `SELECT changes();` after the UPDATE to catch the no-op.
UPDATE memory_entries
   SET is_longterm = 1
 WHERE id IN ('abc-123', 'def-456');
SELECT changes();  -- expected: number of memories re-promoted (NOT 0)
```
<!-- rollback-snippet:end -->

### Verification query

Confirm the re-promotion took effect before leaving the incident:

```sql
SELECT id, is_longterm FROM memory_entries
 WHERE id IN ('abc-123', 'def-456');
-- Expected: is_longterm = 1 for each row.
```

### `last_recalled` note — shield from immediate re-demotion

The rollback SQL above flips `is_longterm` back to 1 but does NOT touch
`last_recalled`. The rolled-back memory is subject to the same decay
schedule as before — if its `last_recalled` is already 80+ days old, the
NEXT dream pass will demote it again. To shield a memory from immediate
re-demotion, also update `last_recalled` to "now":

```sql
UPDATE memory_entries
   SET is_longterm = 1,
       last_recalled = datetime('now')
 WHERE id IN ('abc-123', 'def-456');
```

Use with care: resetting `last_recalled` masks the real recall-age of
the memory and delays legitimate future demotion. This is an operator
decision, not an automatic follow-up.

### Why this rollback is complete

Demotion only modifies `is_longterm` (`src/core/dreaming.rs:251-256`).
Setting `is_longterm = 1` is a full reversal at the row level — no
`avg_relevance`, `valid_until`, or other field needs resetting beyond
the optional `last_recalled` callout above.

## Metric interpretation guide

All four `DreamingResult` fields are visible in the human CLI line and
the structured JSON line on stderr.

| Field | Meaning | Operator signal |
|---|---|---|
| `demoted` | Memories whose `is_longterm` was cleared in this pass. Always `0` in dry-run. | A live-run spike (`demoted > 10%` of long-term count) means the corpus just crossed the staleness threshold en masse — expected after a long hiatus, worth a closer look on a young corpus. |
| `decay_floor_breaches` | Memories whose effective relevance is below the floor. In live mode, equal to `demoted`. In dry-run, `demoted = 0` but this count reflects what WOULD demote. | The steady-state signal. A sustained non-zero here means stale memories keep arriving at the floor — watch the age-profile trend across successive passes. |
| `avg_effective_score_before` | Mean effective relevance across all `is_longterm = 1 AND last_recalled IS NOT NULL` memories, computed BEFORE any demotion write. | A slowly-falling series indicates the corpus is aging without fresh recall — a signal to examine why upstream recall isn't re-surfacing old-but-relevant memories. |
| `avg_effective_score_after` | Mean across SURVIVORS after demotions write (live) OR identical to `_before` (dry-run — no writes occurred). | `(after - before)` quantifies how much the demotion raised the per-memory mean. A large positive delta means demotion is working as intended; a zero delta over time means nothing is ever being demoted. |

The `breaches` array in the structured JSON line lists the specific
memory IDs that fell below floor this pass — use this to inspect
individual memories via `mengdie list --format json | grep <id>`.

## Data freshness: what "corpus age" means

In the revisit triggers below, "corpus age > 90 days" is shorthand for
**the longest `last_recalled` gap** among `is_longterm = 1` memories —
NOT ingest age. An ingest-heavy recent week does not count as "fresh" for
decay purposes; what matters is whether memories are being recalled.

To check the current longest gap:

```sql
SELECT
  id,
  ROUND((julianday('now') - julianday(last_recalled)) * 86400 / 86400, 1) AS days_since_recall
FROM memory_entries
WHERE is_longterm = 1
  AND valid_until IS NULL
  AND last_recalled IS NOT NULL
ORDER BY days_since_recall DESC
LIMIT 5;
```

If the top row exceeds 90 days, revisit the floor calibration (see
revisit triggers below).

## The LONGTERM_BOOST cliff

When Dreaming demotes a stale memory, its search-time score drops from
`normalized × 1.2 × decay_factor` to `normalized × decay_factor` on the
very next query. This is a **one-time discontinuity** and is the
mechanism by which demotion becomes user-visible. Subsequent queries
just apply the decay without the boost; the scoring then evolves
continuously from that new level.

This is intentional. If you see a search-rank regression correlated with
a Dreaming pass, cross-reference the demoted id against the `breaches`
array from that pass's JSON line. A regression on a NON-demoted memory
is a real bug; a regression on a demoted memory is the system working.

## Revisit triggers

Re-evaluate the decay floor + formula if any of:

- `avg_effective_relevance` across the corpus drops below 0.25 on a
  Dreaming pass.
- Longest `last_recalled` gap exceeds 90 days (see "Data freshness").
- `avg_relevance` IQR widens past 0.05 — the compressed-distribution
  assumption no longer holds; a percentile-based floor becomes viable.

First reversal shape: lower `DEMOTION_FLOOR` to 0.15, OR switch to a
corpus-relative rule `mean − 1.5 × IQR`. Both are one-line edits + a
regression-table refresh in `src/core/decay.rs`.
