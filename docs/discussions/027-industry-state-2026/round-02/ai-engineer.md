---
discussion: "027"
round: 2
agent: ai-engineer
lens: "ML / LLM-driven reflection, embedding clustering, synthesis pipelines"
created: 2026-05-05
focus: [topic-02, topic-05, topic-01, topic-03, topic-04]
peers_read: [system-architect, archaeologist, minimal-change-engineer, codex-proxy, gemini-proxy, challenger]
---

# Round 2 — AI-Engineer cross-examination

Read in this round: all 6 peer Round 1 files + `round-01/synthesis.md`
(orientation only). Verified facts from `archaeologist.md` are
load-bearing for the cross-examination below; I cite them by file:line.

Two facts from Round 1 reshape the topology:

1. **Cron is NOT actually running** (archaeologist.md:75-79 verified
   `resources/com.mengdie.dream.plist` is a TEMPLATE with placeholder
   path `<!-- Update this path to your built binary -->`; the "13
   syntheses" first-real-run was on-demand CLI invocation, not
   cron). This falsifies the framing's claim that cron is "stable,
   predictable; already operational" (framing.md:99). Cron is
   half-shipped: the `dreaming.rs` logic exists, the launchd
   trigger does not.
2. **Synthesis rows are stored with `embedding: None`**
   (archaeologist.md:131-139 + verified `dreaming.rs:569`). Synthesis
   rows are invisible to vector search (`vector.rs:62`:
   `WHERE embedding IS NOT NULL`). They ARE visible to FTS5 (text
   match on title/content). This means reflection's output is
   *partially* queryable — text-match yes, semantic-match no. This
   has direct implications for Topic 5's "synthesis-influencing-
   search" measurement.

These two facts are ground truth for what follows.

---

## Topic 2 — Reflection trigger model

### Findings (with peer-cited evidence)

**The "cron + on-demand both shipped" position is empirically
falsified by archaeologist.md.** Four agents (codex-proxy,
gemini-proxy, system-architect, minimal-change-engineer) wrote
their Round 1 verdicts on the assumption that cron is the running
default. archaeologist.md:75-79 establishes the plist is a
template, not a deployable unit; the "13 syntheses" came from
operator-typed `mengdie dream --synthesize` (archaeologist.md:84-88).
This means:

- **codex-proxy.md:83-94** ("Cron + on-demand … hybrid,
  operator-controlled" as v0.0.1 default) operates from
  `archaeologist.md:75-79`-falsified premise.
- **gemini-proxy.md:140-159** ("Cron-only is defensible for
  v0.0.1 because [1] it's already shipping") cites the v0.x
  baseline as evidence; archaeologist establishes the v0.x
  baseline is NOT cron.
- **system-architect.md:243-260** ("v0.0.1 default: cron +
  on-demand … both running today") similarly inherits the
  framing's false premise.
- **minimal-change-engineer.md:140-157** ("v0.x already shipped
  cron via `resources/com.mengdie.dream.plist` … Cron is not a
  proposal; it is the running default") is the most explicit
  about the false premise. Reading
  archaeologist.md:75-79 forces a position update.

Only **challenger.md:108-167** anticipated this, calling
"cron-because-it-13-times-fired" sunk-cost reasoning
(challenger.md:139-143) before archaeologist's verification.

**My Round 1 trait-based proposal becomes the natural answer
once the falsification is absorbed.** The argument:

1. The fact-on-the-ground is "on-demand is the only running
   trigger." That is the v0.x evidentiary baseline — not cron.
2. Both my Round 1 framing (`ai-engineer.md` round-01 §"Minimum
   viable trigger framing") and challenger.md:130-167 converge
   on **on-demand as the v0.0.1 default**. Different lenses
   (mine: ML-defensibility; challenger's: sunk-cost-rejection)
   produce the same verdict.
3. The trait-based seam (~80 LoC, mirrors `LlmProvider` at
   `src/core/llm.rs`) makes cron pluggable when the operator
   chooses to write a real launchd plist. It does NOT prevent
   cron — it makes cron one of several `ReflectionTrigger`
   implementations rather than THE default.

**Position update from Round 1**: I now lean *more strongly*
toward on-demand-as-v0.0.1-default than my Round 1 file did.
Round 1 had on-demand as a soft preference. Round 2 with the
archaeologist evidence: on-demand is the empirically grounded
choice; cron is aspirational without operator-side launchd
configuration.

### The synthesis-embedding bug — Topic 2 gating issue or separate BL?

**archaeologist.md:131-139 + dreaming.rs:569** establishes that
synthesis rows are inserted with `embedding: None`. Combined with
**vector.rs:62** (`WHERE embedding IS NOT NULL` in
`search_vector`), this means:

- Synthesis rows ARE findable via FTS5 search (text match on
  title/content).
- Synthesis rows are NOT findable via vector search (semantic
  similarity).
- Hybrid search (`search.rs:230-231`) combines both, so
  syntheses appear in results IF the FTS5 leg returns them.
  Effective coverage: text-match queries find syntheses;
  paraphrased / semantic queries do not.

**From the ML lens, this is a partial regression of the
synthesis pass's value.** The whole point of clustering and
synthesizing was to produce a higher-order memory the operator
can find via *meaning*, not just keywords. Without the
embedding, the synthesis is ~50% as discoverable as a regular
memory — it survives FTS but fails the very semantic-similarity
retrieval that justified the LLM-summarization expense.

**Is this a Topic 2 gating issue?** I argue **yes, partially**.
Here's the subtle reason:

- If trigger choice is **on-demand**, the operator runs
  `mengdie dream` and reads the synthesis output directly
  (stdout). They see what was produced. If they want to find it
  later, FTS5 is sufficient because the operator typed the
  query knowing what they synthesized — keyword recall works.
  Operator-attention-aligned trigger compensates for the
  embedding gap.
- If trigger choice is **cron** (or salience / composite /
  debounced), the synthesis is produced silently while the
  operator is not watching. The next time it should surface is
  via search — at which point the embedding gap halves the
  retrieval rate, AND the operator does not know what was
  synthesized so cannot keyword-target it.

**Therefore: the synthesis-embedding bug makes background
triggers worse than on-demand triggers**. On-demand survives the
bug because the operator's attention closes the loop manually.
Cron does not. **This strengthens the Topic 2 verdict for
on-demand: it is the ONLY trigger model that compensates for the
embedding gap without first fixing the gap.**

Concretely: if v0.0.1 ships cron-as-default with the synthesis
embedding gap unfixed, the operator runs cron at 03:00, syntheses
land with null embeddings, semantic search misses them, and the
loop is invisibly broken. Filing the embedding gap as a separate
BL is fine; choosing cron-as-default while it's open is not.

**Recommendation update**: ship on-demand as v0.0.1 default
behind the trait seam; file a P1 BL for "synthesis re-embedding
on insert" with explicit trigger ("v0.0.1 ships; revisit before
adding any non-on-demand trigger"). Do NOT block v0.0.1 on the
embedding fix; DO block cron-as-default on it.

### Cross-exam: minimal-change-engineer's "cron + on-demand, zero new code"

minimal-change-engineer.md:166-186 argues for cron + on-demand
because both are "already shipped; zero new code." Round 2
update needed:

- minimal-change-engineer.md:171 says "Document that cron is the
  v0.0.1 default (`resources/com.mengdie.dream.plist`)." But
  archaeologist.md:75-79 establishes that plist is a template
  with `<!-- Update this path -->` placeholder; documentation
  cannot make a template-with-placeholder into a deployable
  unit. The minimum fix is "operator runs `cp + sed +
  launchctl load`" — that's a deployment task, not "zero new
  code."
- minimal-change-engineer's broader point — "do not build
  salience / composite / debounced for v0.0.1" — I fully agree
  with. We diverge ONLY on whether cron is the right default
  versus on-demand. The trait seam reconciles us: shipping a
  one-line trait + two impls (CronTrigger, OnDemandTrigger)
  costs ≤ 50 LoC and lets minimal-change preserve cron-as-
  documented-baseline while letting me preserve on-demand-as-
  empirical-baseline. Both are real. Operator picks via config.

This is the case where minimal-change's "scope discipline"
actually argues *for* a small abstraction: writing one new line
of trait code now prevents both of us from re-litigating the
default later, AND makes the synthesis-embedding gap fixable
without touching trigger code.

### Cross-exam: codex-proxy's "ChatGPT Memory is debounced-submit-dedupe"

codex-proxy.md:62-72 argues from observed ChatGPT Memory
behavior that the silent-debounce-with-capacity-gate pattern is
defensible. I agree with the *observation* but disagree with the
implication for mengdie:

- ChatGPT Memory is a foreground-process pattern (the chat IS
  the user-attention surface). Debounce-at-conversation-end maps
  to "save when user closes the tab" — operator attention is
  the de facto trigger.
- mengdie is a background-process pattern (stdio MCP server,
  invoked by Claude Code, no user-facing chat surface). There is
  no equivalent "conversation-end" event for mengdie — `/ae:work`
  produces 3 ingest calls, then the operator opens another
  session, then another `/ae:work`. There is no natural
  "debounce window" without inventing one.
- Codex's own "Critical absence: OpenAI does NOT ship a 'is the
  loop closing?' metric that's visible to solo operators"
  (codex-proxy.md:186) reinforces my argument: their patterns
  are not transferable to solo-operator scale without
  modification.

### Updated verdict — Topic 2

**v0.0.1 default: on-demand**, behind a small `ReflectionTrigger`
trait that admits cron as a future implementation. The
trait-based seam costs ~50 LoC and resolves the genuine
disagreement between my lens and minimal-change-engineer's
without re-architecting `dreaming.rs`. Salience / composite /
debounced filed as P1+ BLs with explicit triggers (corpus > 1000,
daemon decision, AE-side importance hook).

**Synthesis-embedding gap is a P1 BL with explicit gating: any
non-on-demand trigger MUST first fix the embedding gap or the
loop is silently lossy.** This is a Topic 2 architectural
finding even though the implementation lives elsewhere.

### Agreements

- **challenger.md:130-167** on "cron is sunk-cost reasoning."
  Round 2 archaeologist verification fully validates this.
- **system-architect.md:182-205** on the API being "clean about
  coupling … `dreaming.rs` is trigger-agnostic." This is exactly
  the substrate that makes a trait-based trigger seam cheap.
- **minimal-change-engineer.md:175-186** on filing salience /
  composite / debounced as deferred BLs with explicit triggers.
  Full agreement; my position adds the trait seam as the
  enabling abstraction.
- **codex-proxy.md:91-94** on "metric burden" (salience /
  composite require runtime metrics mengdie does not compute).
  Full agreement.

### Disagreements

- **codex-proxy.md:83-87, gemini-proxy.md:142-145,
  minimal-change-engineer.md:140-142, system-architect.md:243-244**
  all assert "cron is shipped / running / operational." Each is
  falsified by archaeologist.md:75-79 (plist is a template, not
  deployed). Round 2 must update these positions.
- **minimal-change-engineer.md:171** ("Document that cron is the
  v0.0.1 default"): documenting a template-with-placeholder as
  the default is not consistent with the lens's "every line has
  to earn its keep" principle. The honest documented default is
  on-demand; cron is opt-in until the operator runs the plist
  install steps.

### Open Questions

1. **Should v0.0.1 ship the trait seam, or is on-demand-only
   sufficient?** I lean trait-seam because cron WILL come back
   the moment the operator wires the plist; landing the trait
   now (50 LoC) prevents a refactor. But I acknowledge
   minimal-change-engineer's reasonable counter: "no caller =
   no need." If Round 2 closes with on-demand-only, the trait
   seam is a follow-up BL.
2. **Synthesis-embedding gap: fix in v0.0.1 or P1?** My current
   lean: P1 — but with the explicit gate that no non-on-demand
   trigger ships before it lands. v0.0.1 can be on-demand-only
   AND have null synthesis embeddings; the operator-attention
   compensation works for that combination.

---

## Topic 5 — Loop-closure signal

### Findings

The Round 1 split among the 5 ratifiers (everyone except
challenger) is not as wide as it looks. Reading peers' Round 1:

- **My Round 1**: per-search nonempty rate (F-002) + ae:retrospect
  qualitative + falsification rule (nonempty < 20% over 14d AND
  two "idk" verdicts in a row).
- **minimal-change-engineer.md:454-516**: F-002 audit + `mengdie
  audit-stats` CLI (BL-014) + ae:retrospect implicit-by-existence.
- **system-architect.md:592-614**: Two F-002-derived metrics —
  search-with-results-rate (≡ my nonempty rate) +
  synthesis-influencing-search rate.
- **codex-proxy.md:188-211**: Search-result utilization rate
  (cited rate, requires ACK) + operator retro verdict.
- **gemini-proxy.md:434-446**: Thumbs up/down on every result +
  weekly forced report.
- **challenger.md:328-403**: Goodhart's Law on count-based
  metrics; prefer contradiction-trend + Round 0 citation rate.

Three of these (mine, minimal-change, system-architect) converge
on F-002-derivable signals. codex-proxy and gemini-proxy diverge
toward ACK-based signals. challenger diverges toward
hard-to-game proxies.

### Defending my falsification specificity

challenger.md:373-381 ("The cleanest falsification test is: 'Run
ae:analyze on a topic that mengdie should know something about.
Did the Round 0 injection contain relevant facts? Did the
research output build on them?' This is qualitative,
operator-executed, and cannot be gamed.") is structurally
similar to my "two idk verdicts in a row" rule. The difference:
mine names a specific cadence (twice in 14d) and a specific
quantitative companion (nonempty < 20%). Specificity is the
defense.

Why specific numbers matter:
- Without a number, "the loop isn't working" is opinion. The
  operator-as-implementer (challenger.md:391-393) has every
  incentive to rationalize a marginally-failing loop as
  succeeding. A specific threshold short-circuits that — the
  number says yes or no.
- 20% nonempty floor: derived from analysis.md "Perplexity:
  77% → 95% recall by storing half as many." If a well-tuned
  memory system can hit 95%, a poorly-tuned-but-functional one
  should hit > 20%. Below 20% is "clearly broken."
- "Two idk verdicts" beats "any idk verdict" because operator
  fatigue produces random "idk" responses; two-in-a-row
  filters single-week noise.

The numbers are not calibrated; they are floors. The
falsification rule is a *clearly broken* test, not a
*performing well* test. Different question, different number
class.

### Cross-exam: system-architect's "synthesis-influencing-search rate"

system-architect.md:604-608 proposes: "of the facts returned by
`memory_search`, what fraction had `source_type = 'synthesis'`?
… Joinable via `audit_returned_facts → memory_entries.source_type`."

**This is a different signal than the cited-rate / ACK signal**
that 028 locked out. system-architect is measuring "did syntheses
appear in search results," not "did the agent USE the search
results." The former is purely server-side observable
(F-002 audit + memory_entries source_type); the latter requires
ACK.

**Does this conflict with 028's no-ACK lock?** I read peer files
and `docs/discussions/028-v0.0.1-architecture-design/conclusion.md`
(quoted in system-architect.md:556 "028 explicitly rejected ACK
as v0.0.1 contract"). The 028 lock is on caller-side ACK ("did
the agent use this fact"). system-architect's metric does NOT
require that — it asks "did the search engine return any
synthesis rows?" which is computable from F-002 alone.

**However, the synthesis-embedding gap I covered in Topic 2
breaks this metric.** If syntheses have null embeddings,
syntheses appear in FTS5 results (text match) but never in
vector results. The "fraction of returned facts that are
syntheses" metric will be biased down by exactly the embedding
gap. Specifically:
- A keyword-matching query: syntheses appear at
  uncalibrated-but-non-zero rate (FTS5 fires).
- A semantic-paraphrase query: syntheses appear at zero rate
  (vector fires only against non-null embeddings).

So system-architect's metric is computable, but its denominator
shifts based on query type, and its absolute value is biased low
until the embedding gap is fixed. **Recommendation**: adopt this
metric BUT note the bias and pair it with the embedding-gap fix.

### Cross-exam: gemini-proxy's "thumbs up/down per result"

gemini-proxy.md:434-438 ("Thumbs up/down on every search result
or synthesis. This is non-negotiable; it's the operator actively
grading the system's output.") is **structurally an ACK signal**
— the operator's vote is exactly the kind of caller feedback
that 028 locked out. minimal-change-engineer.md:485-487 ("Search-
result-cited rate: violates the 028 ACK-feedback constraint.
Either v0.0.1 keeps that constraint and skips the metric, or
v0.0.1 reopens 028 — high cost") names this constraint
correctly.

**My read**: gemini-proxy's thumbs proposal is well-motivated
(forces engagement) but conflicts with a hard 028 lock. To ship
it would require reopening 028 — out of v0.0.1 scope. The
substantive concern (operator must confront the signal) is
better served by:
- Quantitative (mine + minimal-change + system-architect): a
  number the operator sees
- Qualitative (mine + codex-proxy): retrospect-hook prompting
  the operator weekly

Both surfaces force confrontation without per-result ACK.

### Cross-exam: challenger's Goodhart concern on count metrics

challenger.md:343-368 lists the failure modes of count-based
signals. I take this seriously and update:

- **Search call count alone**: gameable by running more
  searches. challenger is right. **Mitigation**: my proposal is
  not search-call-count; it's search-with-results-rate
  (computed against a denominator that grows when the operator
  makes more queries). Gaming the numerator (results) is hard
  because results come from the corpus, not the operator's
  intent. Operator can game total-call-count by running more
  searches, but the *rate* requires the corpus to actually
  return facts.
- **Synthesis count**: gameable by lowering thresholds.
  challenger is right. My Topic 2 proposal of on-demand-default
  partially mitigates: the operator chooses when to synthesize
  and reads the output, so the count-vs-quality discrepancy is
  visible at the moment of generation. challenger's fix
  (reviewing 13 syntheses for quality before declaring cron
  works) applies regardless of trigger.
- **Re-research reduction**: hard to game but hard to measure.
  challenger.md:354-358 acknowledges this. I deferred this to
  P1 in Round 1 for the same reason.
- **Round 0 injection citation rate**: hard to game, but
  requires AE-side instrumentation that does not exist
  (challenger.md:367-370 explicitly notes this). Cannot ship
  in v0.0.1.

**Updated position**: my proposed signal (per-search nonempty
rate) is *less gameable* than challenger's central worry
because the rate's denominator includes the operator's own
query volume. Adding the qualitative retrospect-hook compounds
the protection — the operator's gut verdict is hard to game
because the operator is the same person who would benefit from
gaming.

### Coupling between Topic 2 and Topic 5 — concrete answer

In Round 1 I said "the trigger seam and the measurement surface
should ship together." Round 2 makes this concrete via the
synthesis-embedding gap:

- If Topic 2 ships on-demand-only AND Topic 5 ships
  search-with-results-rate, the embedding gap is invisible
  (operator-attention closes the loop) and the metric is
  meaningful.
- If Topic 2 ships cron-default AND Topic 5 ships
  search-with-results-rate, the embedding gap reduces the rate
  by the FTS-vs-vector miss percentage on syntheses. The
  metric still functions but with a bias term that the operator
  cannot distinguish from "cron-fired-but-syntheses-suck."
- If Topic 2 ships ANY trigger AND Topic 5 ships
  synthesis-influencing-search rate (system-architect's
  proposal), the embedding gap directly breaks the metric (zero
  syntheses in vector results means the metric never fires
  positive on semantic queries).

**The cleanest combination is**: Topic 2 = on-demand
(trait-seam), Topic 5 = search-with-results-rate +
ae:retrospect hook, embedding gap = P1 BL gating
non-on-demand triggers. This minimizes the surface area where
the bug bites.

### Updated verdict — Topic 5

**Ship**:
1. Per-search nonempty rate (computable from F-002 audit;
   surfaced via `mengdie audit-stats` per minimal-change's
   BL-014 + via inline `IngestOutput.loop_status` per
   system-architect.md:587-591).
2. Qualitative ae:retrospect hook ("did mengdie short-circuit
   anything this week?").
3. Falsification rule: nonempty < 20% over 14d AND two "idk"
   retrospect verdicts in a row → loop not delivering.

**File as P1 BLs**:
- Synthesis-influencing-search rate (system-architect's metric)
  — gated on synthesis-embedding-gap fix.
- Round 0 citation rate (challenger's hard-to-game proxy) —
  gated on AE-side instrumentation.
- Per-result ACK (gemini-proxy's thumbs) — gated on 028 reopen.

### Agreements

- **minimal-change-engineer.md:454-472** on F-002-as-substrate +
  ae:retrospect-as-qualitative. Full alignment; the BL-014
  CLI subcommand is the correct surfacing channel.
- **system-architect.md:548-561** on "F-002 IS the measurement
  substrate" and "no separate event stream needed." Full
  alignment.
- **codex-proxy.md:204-209** on "Disagreement forces
  investigation" — quantitative + qualitative as separate
  streams is correct.
- **challenger.md:339-368** on Goodhart risk for count-based
  metrics. Full alignment in principle; my rate-not-count
  formulation is the mitigation.

### Disagreements

- **gemini-proxy.md:434-438** on per-result thumbs up/down. This
  conflicts with 028's no-ACK lock per minimal-change-engineer.md:485-487.
  Disagree: this metric requires reopening 028, which is out
  of v0.0.1 scope. File as P1 BL gated on 028 reopen.
- **codex-proxy.md:192-197** on "search-result utilization
  rate" if read as ACK-requiring (cited-rate). If read as
  server-side observable (search-call utilization volume,
  similar to my nonempty rate), we agree. Codex's wording is
  ambiguous on this — Round 2 should pin which.
- **challenger.md:381-385** on "contradiction-detection events
  trending down over time" as the preferred quantitative
  proxy. Direction-ambiguity (low could mean consistent OR
  unused, per challenger's own caveat at challenger.md:362-365)
  makes this less actionable than nonempty rate. Defer to P1.

### Open Questions

1. **Where does `IngestOutput.loop_status` live?** system-
   architect.md:587-591 proposes inlining the headline metric in
   ingest responses (forced confrontation per ingest). I lean
   toward this for one specific reason: ingest happens 3-5
   times per `/ae:work` session, so the operator sees the
   metric on the same cadence as their work. Surfacing only via
   `mengdie audit-stats` (separate CLI) requires the operator
   to remember to run it. **Recommendation**: ship both — CLI
   for deep dives, IngestOutput for ambient awareness.
2. **Should the qualitative retrospect-hook be one prompt or
   three?** Round 1 proposed one ("did mengdie short-circuit
   anything?"). codex-proxy.md:201 proposes three options
   (yes/uncertain/no). gemini-proxy proposes a 4-option modal.
   I lean three options (yes/uncertain/no): "yes/no" forces
   binary judgment that the operator may not feel; "uncertain"
   is the load-bearing third option that surfaces "I don't
   know" honestly.
3. **Embedding-gap fix scope**: backfill existing 13 syntheses
   too, or only fix forward (new syntheses get embeddings)?
   Forward-only is simpler; backfill is one batch script.
   Defer to the BL author.

---

## Topic 1 — Ingest mechanism (lighter touch from ML lens)

### Findings

My Round 1 said push-primary on signal-determinism grounds.
Round 2 with peer reading: 4 of 7 agents (system-architect,
codex-proxy, ai-engineer, minimal-change-engineer) converge on
push-primary. gemini-proxy goes hybrid (push-primary +
pull-fallback). challenger argues for pull.

**Cross-exam: challenger.md:34-78's pull-default position.**
challenger's strongest argument is decoupling — "AE writes
files; mengdie observes them" (challenger.md:39). From the ML
lens this is appealing: pull would automatically capture every
AE artifact without per-skill ingest plumbing.

But challenger.md:36-43 itself names the failure mode in a
different topic ("the operator silently fails to call
`memory_ingest`") — and the fix challenger proposes there is
push with explicit per-skill discipline. Pull's "automatic
capture" claim depends on AE writing files in the right format
to the right path; if AE writes to a non-watched path or with
a non-matching filename, pull silently misses it. The failure
mode flips from "skill forgot to call" to "watcher's path
config didn't match" — same shape, different layer.

archaeologist.md:32-34 verifies that `is_ingestable`
(`parser.rs:159-180`) is a blocklist not allowlist; pulling
over `docs/` would ingest every `.md` file UNLESS the path
config carefully restricts it. This is the "watcher catches
unintended files" failure that challenger does not address.

**Updated verdict**: push-primary, watcher kept as opt-in
library with explicit "experimental, not v0.0.1" doc string.
gemini-proxy's hybrid earns its keep only if push has measured
reliability problems; archaeologist evidence does not show
those problems exist.

### Synthesis-quality consequence (per Round 1)

None at the mechanism level. Confirmed by reading peers — no
agent's Round 1 file claimed mechanism affects synthesis input
quality. The cluster algorithm operates on `memory_entries`
rows; ingest mechanism is upstream and orthogonal.

### Agreements

- **system-architect.md:130-145** on `resolves` parameter being
  "push-only by construction." Good architectural argument I
  hadn't surfaced in Round 1.
- **minimal-change-engineer.md:73-99** on "post-hoc markdown
  re-extraction" as the key cost of pull-as-primary. Full
  alignment; the watcher would re-derive what AE already did,
  exactly the v0.x pattern the rebuild rejects.
- **codex-proxy.md:34-39** on industry convergence to
  push-with-async-queuing.

### Disagreements

- **challenger.md:31-78** on pull-as-default. Disagree on
  archaeologist-evidence grounds (blocklist `is_ingestable`)
  and on synthesis-quality grounds (mengdie does NOT have
  AE-grade extraction logic). The decoupling appeal is real
  but the implementation cost is not "marginal."
- **gemini-proxy.md:69-79** on hybrid (push-primary +
  pull-fallback). The "fallback" surface doubles maintenance
  for a failure mode (push silently fails) that the F-002
  audit table can detect more cheaply than running a watcher.

### Open Questions

1. Per Round 1: "Should `memory_ingest` validate AE-shaped
   inputs at the MCP boundary, or trust AE plugin?" Round 2
   archaeologist evidence shifts this: archaeologist.md:269-275
   shows `infer_source_type` returns `"unknown"` for non-AE
   filenames, and the v5 schema trigger REJECTS unknowns. This
   is a latent bug that constrains how Topic 4 (ratify AE-only)
   ships. From the ML lens: do not validate beyond what F-002
   catches — over-validation creates rejection paths that the
   operator cannot debug.

---

## Topic 3 — Cross-project default (lightest touch)

### Findings

Round 1: I voted ratify on ML grounds (cluster contamination
risk if synthesis went cross-project). Round 2 with peer
reading: 5 ratifiers, 1 challenger.

**challenger.md:172-251's reframe** ("§5 was a migration-cost
deferral, not a design claim") deserves engagement. The ML
question: under unified-operator-identity, does cross-project
default produce better or worse synthesis input?

**Still worse, from the ML lens**, because:

1. Cluster-contamination is a structural problem regardless of
   operator identity. The operator may be one person, but
   project A's "use Arc<Mutex<T>>" decision and project B's
   "Avoid Arc<Mutex<T>>" decision are not the same fact —
   they are correctly different decisions in different
   stacks. A cross-project cluster on these two memories
   produces a contradictory synthesis prompt. The synthesis
   LLM either picks one (silently wrong for the other) or
   produces a hedged consolidation (semantically empty).
2. challenger.md:215-220 ("Provenance at the result level …
   reduces contamination risk to 'read the source label'") is
   true for *retrieval* but not for *synthesis*. The synthesis
   pass passes cluster contents to the LLM; provenance labels
   in the prompt do not prevent the LLM from blending opposed
   decisions. Synthesis is the load-bearing consumer of
   per-project scoping.

**Position update**: ratify §5; add explicit ML-invariant note
that **synthesis is per-project ONLY** (already enforced by
`clustering.rs:71`'s `project_id: Option<&str>` parameter, but
this is invisible policy). Make it explicit. challenger's
reframe is correct for retrieval but wrong for synthesis.

### Agreements

- **codex-proxy.md:115-126** on per-namespace isolation as
  industry pattern. Same direction as my Round 1.
- **minimal-change-engineer.md:236-282** on ratify + reopening
  trigger. Trigger formulation slightly differs from mine
  (their ≥10% global vs my "30% global opt-in" — both are
  honest floor numbers).
- **system-architect.md:341-353** on "policy on a single query
  parameter; storage is global so the policy can be flipped
  per call or per default with no migration." Architectural
  cleanness validated.

### Disagreements

- **challenger.md:181-225** on cross-project as default.
  Disagree on synthesis-pollution grounds (above). For
  retrieval-only the argument has merit; for the loop as a
  whole it does not.

### Open Questions

1. Should the explicit "synthesis is per-project ONLY" rule
   live in the conclusion text, or in the Topic 2 trait seam
   (`ReflectionTrigger` knows its project scope)? Light
   preference for conclusion-text + trait param: the rule is a
   cross-cutting invariant.

---

## Topic 4 — Ingest source boundary (lightest touch)

### Findings

Round 1: ratify AE-only on Perplexity admission-filtering
grounds. Round 2: 5 ratifiers, 1 challenger.

**challenger.md:269-307's reframe** ("AE-only should mean
extraction discipline, not physical AE files") is the most
substantive challenger argument across all topics. It maps onto
my Round 1 admission-filtering frame: what mengdie cares about
is fact-quality, not file-path.

**Updated verdict**: I now lean toward absorbing
challenger's reframe rather than rejecting it. Specifically:

- v0.0.1 ratifies AE-only-as-extraction-discipline.
- The MCP `memory_ingest` tool ALREADY accepts arbitrary
  caller-supplied content (archaeologist.md:235-241). The
  policy is enforced by AE plugin discipline (only AE skills
  call it), not by mengdie-side filtering.
- archaeologist.md:269-275 surfaces a latent bug: non-AE
  filenames pass `is_ingestable` but get rejected by the v5
  schema trigger as `"unknown"`. This bug INTERSECTS with
  Topic 4: under "AE-files-only" reading, the rejection is
  correct (with a wrong error message); under "extraction-
  discipline" reading, the rejection is a real bug.

**The right fix** is to ratify Topic 4 as
"extraction-discipline" + treat the latent bug as a real bug
to fix in the F-001 ingest plan: rename `unknown` → `direct`
(or `manual`) and accept it as a valid source_type for
operator-distilled propositional facts.

### Position update from Round 1

Round 1 I leaned strict ratify-AE-only. Round 2 with
challenger's reframe + archaeologist's latent-bug verification:
absorb the reframe. The ML lens supports this — admission
filtering is about *signal quality* not *file path*; an
operator-distilled fact via `memory_ingest` CLI passes the same
quality bar.

This does NOT change anything about ingest mechanism (Topic 1
push) or signal-quality discipline. It DOES change the ratify
text from "AE files only" to "AE extraction discipline."

### Agreements

- **challenger.md:294-306** on extraction-discipline-as-
  identity-claim. Strong agreement after Round 2 reflection.
- **codex-proxy.md:152-156** on "boundary + filtering is
  strength."
- **gemini-proxy.md:300-318** on NotebookLM precedent for
  strict initial boundaries.

### Disagreements

- **codex-proxy.md:158-161** on baking in `source: enum`
  forward-compat with explicit `ae_*` prefixes. After
  challenger's reframe, the enum-with-prefix design is
  premature — the discipline is "extraction quality," not
  "file source." A simple `source_type` enum (no `ae_*`
  prefix) suffices.
- **minimal-change-engineer.md:357-393** on rejecting any
  forward-compat work. We agree on no forward-compat code,
  but I'd add the textual reframe to the conclusion (cost: 0
  LoC, value: closes the latent-bug discussion).

### Open Questions

1. **Latent-bug fix scope**: rename `"unknown"` → `"direct"` in
   `infer_source_type` + update the v5 trigger? This is
   F-001 ingest plan scope, not Topic 4 conclusion. Note it in
   the conclusion as a Topic 4-derived plan-time TODO.

---

## Cross-topic synthesis

### What the two new facts changed

1. **archaeologist.md:75-79 (cron not actually running)**
   reshaped Topic 2: 4 of 7 Round-1 verdicts inherited a false
   premise; on-demand-as-default is the empirically grounded
   choice; the trait seam reconciles minimal-change's "no new
   code" with my "trigger optionality" lens.
2. **archaeologist.md:131-139 + dreaming.rs:569 (synthesis
   embeddings null)** is a Topic 2 architectural finding (it
   constrains which trigger model is sound) AND a Topic 5
   measurement constraint (system-architect's
   synthesis-influencing-search metric is biased by it). The
   bug also bridges Topics 2 and 5: fixing it precedes any
   non-on-demand trigger AND precedes any synthesis-coverage
   metric.

### Recommended v0.0.1 commitments

- **Topic 1**: push-primary; watcher kept as opt-in library
  with explicit "experimental, not v0.0.1" doc string.
- **Topic 2**: on-demand-as-default, behind a small
  `ReflectionTrigger` trait (~50 LoC seam, mirrors
  `LlmProvider`); cron is a future trait impl, not the v0.0.1
  shipped default.
- **Topic 3**: ratify §5; add explicit "synthesis is
  per-project ONLY" ML invariant to the conclusion text.
- **Topic 4**: ratify AE-only-as-extraction-discipline (absorb
  challenger's reframe); fix the `"unknown"` → `"direct"`
  latent bug in F-001 plan.
- **Topic 5**: per-search nonempty rate (F-002) +
  ae:retrospect qualitative hook + falsification rule
  (nonempty < 20% over 14d AND two "idk" verdicts → loop
  broken). Defer thumbs / ACK / synthesis-influencing-search
  to P1+ BLs.

### P1 BLs to file

1. **Synthesis-embedding gap**: insert `embedding: ...` for
   synthesis rows. Trigger: gates any non-on-demand trigger
   addition AND gates synthesis-influencing-search metric.
2. **Synthesis-influencing-search rate**: system-architect's
   metric. Trigger: synthesis-embedding fix lands.
3. **Cron trigger plist**: real launchd plist with deployable
   path. Trigger: operator runs mengdie for ≥1 month and
   wants ambient reflection.
4. **Round 0 citation rate**: AE-side instrumentation hook.
   Trigger: ae:analyze emits structured "I used memory X"
   provenance.
5. **Per-result ACK**: gemini-proxy's thumbs. Trigger: 028
   reopens with evidence the no-ACK lock blocks loop signal
   convergence.
