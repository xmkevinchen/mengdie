---
agent: minimal-change-engineer
round: 2
topics: [01, 02, 03, 04, 05]
lens: scope-discipline / refuse-over-engineering
timestamp: 2026-05-06
---

# minimal-change-engineer — Round 2

Round 2 lens carry-over from Round 1. The cross-examination targets
are the T4 forward-compat split, the T4 latent bug surfaced by
archaeologist, the T5 028 ACK lock impact, the T2 trigger model now
that "cron is running" has been falsified, and the three contrarian
challenger positions.

I keep my Round 1 verdicts on T3 (ratify §5) and the spirit of T1
(push-only) and T4 (ratify AE-only). I update T2 in light of new
facts. I refine T5 in light of the 028 ACK lock and Goodhart's Law
challenge. Each update is explicitly named.

---

## Topic 1 — Ingest mechanism

### Findings

Round 1 verdict (push-only + `mengdie import` cold-start; watcher kept
as opt-in library) survives Round 2 cross-examination. Three things
have happened that reinforce, not weaken, the verdict.

**(a) archaeologist verified the watcher is fully unwired.** Per
`round-01/archaeologist.md:22-27`, `rg start_watcher` and `rg
watch_loop` across the entire `src/` tree return only hits inside
`watcher.rs` itself. **Zero non-test consumers.** Per
`archaeologist.md:43-48`, `cmd_import` already exists for cold-start
bulk-ingest via `ingest_file` per file. The "push needs a separate
bulk-import path" objection is empirically dead — it exists today and
ships in the v0.x binary.

**(b) system-architect verified push owns features that pull cannot
provide.** Per `round-01/system-architect.md:25-31`, the `resolves:
Option<Vec<String>>` parameter on `IngestParams` (`mcp_tools.rs:96-98`)
is **push-only by construction**: it requires the caller to know
predecessor IDs, which a filesystem watcher cannot derive. Atomic
supersession via `ingest_text_with_resolves` rides the push surface;
moving to pull-primary would either lose this feature or require the
watcher to parse the ingest payload for "this supersedes IDs X, Y" —
parser surface AE produces upstream much more cheaply.

**(c) codex-proxy verified the industry pattern is push-with-async-
queuing, not pull-watcher.** Per `round-01/codex-proxy.md:34-39`:
mem0 v1.0 ships an "async write path" that is **explicitly push** with
server-side queuing; LangMem's ReflectionExecutor coalesces **push
events** within a window; Graphiti MCP server v1.0 exposes **push
tools** with server-side queuing. None of the production frameworks
chose pull-daemon as primary. The "industry trends async background"
intuition is real, but the async is **post-push server-side**, not
pre-push file-watcher.

### Cross-exam: challenger's pull-default position

Per `round-01/challenger.md:60-64`, challenger argues "deployment
surface argument is overstated; mengdie already requires a launchd
plist for dreaming cron — a second plist for the watcher daemon is
marginal additional surface, not a qualitative leap. The precedent
is already set."

This argument **fails on a fact established in the same Round 1**.
Per `archaeologist.md:74-77`, `resources/com.mengdie.dream.plist` is a
**template**, not a system-installed unit (line 9: `<!-- Update this
path to your built binary -->`). Whether the dream plist is loaded in
launchd on the operator's machine **is not verifiable from code**.
Challenger's "precedent is already set" is empty — there is no
production launchd footprint for mengdie. The 13 syntheses came from
**manual on-demand CLI invocation** (archaeologist.md:86-88), not
cron. Adding a watcher plist is therefore not "marginal increment on
existing surface"; it is **the first production daemon mengdie would
deploy**, with all the supervision questions still unanswered.

Per `challenger.md:54-58`: "push errors are equally silent when an AE
skill silently fails to call `memory_ingest`." False symmetry. Push
runs inside a Claude Code session the operator is watching — MCP tool
errors surface in-session. Pull's "silent stop" is the **default**
state of any unsupervised daemon and requires explicit heartbeat /
reconciliation infrastructure. The asymmetry is real.

Per `challenger.md:67-72`: "mem0 v1.0 explicitly chose async write
path; the industry has moved toward write-decoupled background
processing." This is the **same citation codex-proxy** uses to argue
**push** (codex.md:34-39). Both quote mem0 v1.0; codex reads it as
"push-with-async-queuing"; challenger reads it as "decoupled
write-path." Reading the mem0 source: it is push-with-async-queuing
(caller invokes the write API; server queues for processing). The
industry citation **does not support pull**.

**Verdict on challenger T1**: scope-expanding contrarianism. The
position has internal logic but rests on factual misreads
(archaeologist falsified the launchd precedent; codex correctly read
the mem0 / LangMem industry pattern challenger appealed to).

### Recommendation (unchanged from Round 1)

**Minimum: push-only + `mengdie import <dir>` for cold-start.**
Watcher library kept in-tree as opt-in (zero maintenance cost; one
import-line away from being the basis of a future daemon binary if
that ever becomes a real need). One AE-plugin-side BL: each terminal
AE skill (`/ae:plan`, `/ae:discuss`, `/ae:review`, `/ae:retrospect`,
`/ae:analyze`) calls `memory_ingest` after producing its artifact.

### Agreements

- system-architect (`round-01/system-architect.md:111-115`): push as
  v0.0.1 default, watcher kept as opt-in library — same verdict.
  Same reversibility argument (high; daemon can be added later by
  wrapping `start_watcher` + `watch_loop` in a binary).
- ai-engineer (`round-01/ai-engineer.md:447-453`): "push primary;
  watcher archived (or marked experimental, not v0.0.1)" — same
  verdict.
- codex-proxy (`round-01/codex-proxy.md:42-50`): push-primary
  "OpenAI's own Vector Stores API defaults to push; mem0 v1.0,
  LangMem, Graphiti all converged on push-with-async-queuing" — same
  verdict, same industry reading.
- archaeologist (`round-01/archaeologist.md:7-15`, `43-48`): facts
  used to verify the verdict.

### Disagreements

- gemini-proxy (`round-01/gemini-proxy.md:456`, summary table —
  "Hybrid (push-primary, pull-fallback)"): the hybrid adds the
  operator-visibility cost of two failure surfaces (push errors at
  caller, pull errors in daemon log) for no measurable benefit when
  AE plugin is the only producer. The hybrid pattern earns its keep
  in environments where the producer can't be trusted to push;
  v0.0.1's producer is AE plugin, which the operator owns and can
  patch. Defer hybrid to BL with trigger "v0.0.1.x ships an AE-
  external producer that cannot be modified to push."
- challenger (`round-01/challenger.md:28-31`, pull-default): see
  cross-exam above.

### Open Questions

1. Does v0.0.1 commit to deleting `watcher.rs`, or keep it as
   documented opt-in library? Per `system-architect.md:163-166`:
   "Argument for keeping: zero maintenance cost, optional surface.
   Argument for deletion: dead code attracts re-adoption pressure
   later." Minimum: keep, add a comment "experimental — not wired
   to v0.0.1; daemon shell would need supervision before deployment."
   No code change beyond a comment.

---

## Topic 2 — Reflection trigger model (UPDATED)

### What changed since Round 1

Round 1 said "cron + on-demand BOTH already shipped (13 syntheses)."
Per `round-01/archaeologist.md:74-88`:

- The launchd plist is a **template**, not a system-installed unit.
  Line 9 has `<!-- Update this path to your built binary -->`.
- Whether it is loaded in launchd is **not verifiable from code**.
- The "13 syntheses" were produced by **on-demand CLI invocation**
  (`mengdie dream --synthesize`), per CLAUDE.md Project Status —
  **not by cron**.

This factually corrects Round 1's "cron is running" premise. **Cron's
production status is: logic exists in `dreaming.rs:67-313`; CLI entry
point exists at `cli.rs:177`; launchd wiring is template-only and
unverified.**

### Updated position

The fact change is **smaller than it looks**. Round 1's verdict was
"cron + on-demand; both already shipped; zero new code." That verdict
needs one factual edit and zero substantive change:

- **on-demand IS shipped** — `mengdie dream` CLI command is real and
  exercised (it produced the 13 syntheses). Confirmed by
  `archaeologist.md:86-88`.
- **cron logic IS shipped** — `dreaming.rs::run_dreaming` is the
  production entry; the launchd plist template exists.
- **cron deployment is one operator step away** — replace the
  template path with the real binary path + `launchctl load`. **This
  is fifteen minutes of operator configuration, not engineering**.

The Round 1 conclusion holds: the v0.0.1 trigger story = on-demand
(working today) + cron (operator-installable when they want it).
Zero new mengdie code. The only deliverable is a one-paragraph
operator setup doc that explains the plist path substitution.

### Cross-exam: ai-engineer's `ReflectionTrigger` trait

Per `round-01/ai-engineer.md:140-145`:

> v0.0.1 ships with **cron-baseline + on-demand override** (both
> already exist). The trigger module is structured behind a
> `ReflectionTrigger` trait with a single `should_fire(&self,
> context: &ReflectionContext) -> bool` method. v0.0.1 implements
> two: `CronTrigger` (true when scheduler fires) and `OnDemand`
> (true when CLI invoked).

Per `ai-engineer.md:168-179`, the cost is "~80 LoC + 4 tests."

**Karpathy load-bearing test on the trait, applied to v0.0.1 only:**

What does the trait enable that the alternative (cron called from
launchd; on-demand called from CLI; both calling `run_synthesis_pass`
directly) does not?

| Item | Direct call (current) | `ReflectionTrigger` trait |
|---|---|---|
| Cron fires synthesis | launchd → `mengdie dream` → `run_dreaming` | launchd → `mengdie dream` → trait dispatch → `run_dreaming` |
| On-demand fires synthesis | shell → `mengdie dream` → `run_dreaming` | shell → `mengdie dream` → trait dispatch → `run_dreaming` |
| Salience trigger (deferred BL) | not in v0.0.1 | not in v0.0.1 (the trait *would* host it later) |
| Composite trigger (deferred BL) | not in v0.0.1 | not in v0.0.1 |
| Debounced trigger (deferred BL) | not in v0.0.1 | not in v0.0.1 |

The trait abstracts a one-implementation-currently-needed seam.
**v0.0.1 has exactly one trigger implementation that matters
(`OnDemand`)** because cron is just on-demand-from-launchd in disguise
— same code path, different caller. The trait gets exercised by two
callers but ships with one real impl plus one trivial wrapper.

ai-engineer's own defense (`ai-engineer.md:170-175`): "Risk of
premature abstraction: yes. But the abstraction is one method
(`should_fire`) over an existing implementation (cron). If v0.0.2 adds
salience and the trait turns out wrong, the rewrite cost is the same
as if we had cron-only today — the trait gives optionality without
locking shape."

The "rewrite cost is the same" claim is the load-bearing one. It is
**also true if the trait is added when the second impl materializes**
(salience BL fires) rather than now. In Rust, introducing a trait
between an existing function and its callers is a mechanical
refactor — the trait exposes only `should_fire`, the call sites
remain the same shape. Adding it speculatively costs ~80 LoC + tests
**that test nothing functional that isn't already tested** (cron
fires; on-demand fires; both are integration-tested by
`tests/e2e.rs` already).

**Karpathy verdict: scope creep dressed up as engineering
discipline.** The trait is well-shaped, but well-shaped abstractions
that abstract over one real impl are textbook YAGNI. File it as part
of the salience / composite / debounced BLs (those BLs would
naturally introduce the trait at the time the second impl exists),
not as a pre-emptive v0.0.1 deliverable.

### Recommendation (refined)

**Minimum (unchanged in spirit, sharpened in detail):**

1. **on-demand**: ship as-is (`mengdie dream` CLI exists).
2. **cron**: ship the plist template **with a one-paragraph setup
   doc** (`docs/operations/cron-setup.md` — operator path
   substitution + `launchctl load` invocation). This addresses
   archaeologist's "template not loaded" finding without any code
   change. **One markdown file added**.
3. **`ReflectionTrigger` trait**: NOT in v0.0.1. Filed as
   pre-condition of the salience / composite / debounced BLs (when
   any of those fires, the trait lands as a refactor in that BL,
   not as a standalone v0.0.1 commitment).

The other three triggers (salience, composite, debounced) remain
filed as backlog with reopening triggers identical to Round 1.

### Agreements

- ai-engineer (`ai-engineer.md:198-202`) on the **substantive trigger
  choice**: cron + on-demand are the right v0.0.1 default. We agree
  on what triggers ship; we disagree on whether to abstract them.
- system-architect (`system-architect.md:264-271`) on cron + on-demand
  as v0.0.1 default with deferred BLs for the other three — same
  verdict.
- codex-proxy (`codex-proxy.md:83-95`) on cron + on-demand as v0.0.1
  default — same verdict, same operational visibility argument.
- archaeologist (`archaeologist.md:74-88`) on the factual state of
  cron deployment — facts adopted to refine my Round 1 wording.

### Disagreements

- ai-engineer (`ai-engineer.md:140-145`, `ai-engineer.md:168-179`):
  the `ReflectionTrigger` trait is YAGNI for v0.0.1. We agree on
  triggers; we disagree on whether to introduce a trait seam now or
  when the second impl forces it. Cited Karpathy load-bearing test
  above as defense.
- challenger (`challenger.md:107-138`): "cron is sunk-cost reasoning;
  13 syntheses are uncalibrated; on-demand should be the v0.0.1
  default while we validate quality." Partial agreement — on-demand
  IS the working baseline. But challenger's framing implies "do
  nothing about cron" which leaves the operator without a default
  background pass. Adding cron via the existing template + a setup
  doc is not sunk-cost reasoning; it's "ship what works, keep the
  template that the operator can flip on."

### Open Questions

1. Does the cron-setup doc belong in v0.0.1 sprint scope, or in
   `docs/operations/` as a separate small task? Argument for v0.0.1:
   addresses archaeologist's verified "template not loaded" gap.
   Cost: ~30 lines of markdown + plist comment edits. Minimum: yes,
   include it; archaeologist's finding is otherwise an unresolved
   acceptance gap.
2. Per `archaeologist.md:135-139`: synthesis rows are stored with
   `embedding=None`. This is a **separate bug**, not a Topic 2
   concern, but it intersects with Topic 5 below — see Topic 5
   findings.

---

## Topic 3 — Cross-project default retrieval scope

### Findings (unchanged from Round 1, refined cross-exam)

Round 1 verdict (ratify §5 + record reopening trigger) stands.
archaeologist (`round-01/archaeologist.md:174-178`) confirms the
default flip is a **one-line conditional change** in
`mcp_tools.rs:192-195`. Reversibility is therefore high, the bar to
revise is correspondingly low **but needs evidence**, and F-002 audit
data has not yet accumulated.

### Cross-exam: challenger's cross-project-as-default

Per `round-01/challenger.md:182-220`, challenger argues:

1. Single operator = unified identity across projects.
2. Contamination risk is weak for one operator with consistent
   conventions.
3. §5's rationale was migration cost, not correctness — disappears in
   fresh rebuild.
4. Per-project default forces operators to opt-in to mengdie's most
   valuable feature.
5. Provenance at result level (every memory carries `project_id`) is
   already tracked.

**Substantive points; two have problems.**

Point 3 is wrong about §5's rationale. CLAUDE.md §5 says "**Global
storage**, per-project default search — avoid migration cost when
adding cross-project later." Migration cost is the rationale for
**global storage**, not for per-project default. Per-project default
is justified by the contamination concern. Challenger's textual read
inverts the source.

Point 5 (provenance per result) is mitigation **only when the caller
can read provenance and judge applicability**. For interactive operator
use, fine. **For agent-flow callers (ae:work, ae:plan, ae:review
mid-task searches)**, the agent has to be taught to read project_id
labels and discount cross-project results — exactly the per-skill
discipline challenger names as "the strongest argument for per-project
default" (`challenger.md:223-230`). Challenger acknowledges this
internally.

A fact challenger does not address: per `archaeologist.md:210-214`,
`project_id` at search time is the cwd at **mcp_server startup**, not
cwd at query time. For multi-project Claude Code sessions, the default
project_id is **stale** after the first project. Under cross-project-
as-default this stale id matters less (because the search ignores it
in default scope) — but under **either** default, the ingest path
still tags the new ingest with the stale project_id. **This is an
independent ingest-path bug** that exists in both directions; it
doesn't tilt the Topic 3 verdict by itself, but it tells us the
operator's "I work across multiple projects in one session" workflow
is broken at a layer below Topic 3.

The right move: ratify §5; file the project_id-staleness as a
separate bug; let F-002 audit data accumulate; check the reopening
trigger in 30+ days.

**Verdict on challenger T3**: deserves serious consideration on
operator-identity grounds (point 1 is real). But the evidence to
overturn is not there yet — cwd-stale ingest bug needs fixing before
cross-project default would even be **safe**, and F-002 data is the
right substrate for the verdict.

### Recommendation (unchanged from Round 1)

Ratify §5. Record reopening trigger: when F-002 audit data shows
either (a) ≥10% of operator searches use `scope: 'global'` and return
non-empty cross-project results AND the same operator manually
reissued a project-scoped query for the same intent in the same
session, or (b) operator explicitly reports ≥3 cross-project
rediscovery incidents in a retrospect cycle. Reversibility: HIGH
(one-line change at `mcp_tools.rs:192-195`).

### Agreements

- codex-proxy (`codex-proxy.md:117-125`): ratify §5 — same verdict,
  same reasoning (per-namespace isolation pattern from OpenAI).
- ai-engineer (`ai-engineer.md:478-490`): ratify §5 — same verdict,
  with additional ML-invariant note that cluster contamination is a
  real synthesis-quality risk if cross-project clustering ever fires.
- system-architect (`system-architect.md:371-378`): ratify §5 — same
  verdict, with the same reopening trigger structure (≥30% global
  opt-in across 60-day window OR ≥3 retrospect-reported incidents).
- gemini-proxy (`gemini-proxy.md:457`): ratify per-project default —
  same verdict, with global search index reframe (analytical detail,
  doesn't change architectural decision).

### Disagreements

- challenger (`challenger.md:182-220`): cross-project as default. See
  cross-exam above. Substantive but evidence-underweight; revisit
  when F-002 data is in.

### Open Questions

1. Should the project_id cwd-stale bug
   (`archaeologist.md:210-214`) be filed as a separate BL now, or
   treated as a known-limitation comment? Minimum: file as a BL
   with trigger "operator runs multi-project Claude Code sessions
   and reports incorrect project_id tagging on ingest." Independent
   of Topic 3 outcome.

---

## Topic 4 — Ingest source boundary (CROSS-EXAM HEAVY)

### Findings

Round 1 verdict (ratify AE-only + reopening trigger; **NO**
forward-compat scaffolding) survives Round 2. Two pieces of new
evidence reinforce it.

### Cross-exam: codex-proxy's forward-compat split

Per `round-01/codex-proxy.md:158-163`:

> **Forward compatibility (v0.0.1 API design):**
> - Add a `source` enum in ingest schema: `{ source: "ae_plan" |
>   "ae_review" | "ae_conclusion" | ... }`.
> - Store source tag on every ingested memory.
> - When broader sources are added (commit messages, issue text), the
>   schema already supports typed provenance.

**The proposal as stated already exists in v0.x.** Per
`round-01/archaeologist.md:36-42`, `mcp_tools.rs:38-58` defines
`SourceType` enum (`Conclusion | Review | Plan | Retrospect |
Synthesis`) with serde `rename_all = "lowercase"` enforcement.
`schema.rs:108-110` stores `source_type TEXT NOT NULL` per
`memory_entries` row. **Codex's three bullet points describe the
current schema.**

Reading codex charitably, what codex *might* mean by "forward-compat"
is: **broaden the enum to a free-form-but-validated string** so that
adding `commit_message` or `chat_summary` later doesn't require an
enum-variant addition. But system-architect already costed this
exact alternative and rejected it. Per
`round-01/system-architect.md:457-470`:

> Forward-compat option B: now generalize to `source_type: String`
> and add a `source_schema_version: i64` for per-source dispatch.
> Cost on adoption: zero (just write a new parser). Cost today: enum
> guarantees lost (caller can submit garbage); per-source dispatch
> indirection added with no current consumer.

system-architect's verdict (`system-architect.md:466-468`): "Option A
is the v0.0.1-correct choice. The 028 architecture conclusion already
chose enum-shape decisions of this kind in Topic 1."

**Cost-benefit per Karpathy load-bearing test**:

| Forward-compat artifact | Lines | What it enables now | What it enables later |
|---|---|---|---|
| `source_type` enum (already exists) | ~20 | Caller supplies one of 5 AE-style values | Adding a 6th variant when a real new source lands = 1-line enum addition |
| Generalize to free-form string | ~10 | Nothing | Adding a 6th source = 0 schema work; **but** lose enum guarantees |
| Add `source_schema_version` column | ~30 schema + migration + read paths | Nothing (no consumer at v6) | Per-source dispatch when a 6th source lands; **wasted column for v0.0.1** |
| Typed source markers + per-source filters | ~150-300 (schema + parser + filter pipeline) | Nothing | Polymorphic ingest pipeline when N>1 source types exist |

The "v1 API break insurance" framing (codex's load-bearing argument)
**does not apply to v0.0.1**, because:

1. v0.0.1 has **zero external consumers**. Per
   `docs/v0.0.1-rebuild-plan.md:25-30`: "v0.0.1 is personal-use
   rebuild, not yet accountable to external users." There is no
   external API contract to break.
2. Adding an enum variant to `mcp_tools.rs` is a non-breaking change
   in Rust serde: existing callers continue to work; new callers can
   use the new variant. **The "v1 API break" cost is hypothetical
   and bounded by AE plugin updates — both repos are operator-owned.**
3. Schema migrations are first-class in mengdie (`schema.rs` migration
   ladder; v6 just shipped). Adding a new column is `ALTER TABLE` +
   one migration step, not an "API break."

**Verdict on T4 forward-compat**: scope-creep / reinvention failure
mode. Codex's specific proposal **already exists** (enum + tag
column). Anything beyond it (free-form string, source_schema_version,
typed source markers) is either lost-guarantees (B) or speculative
infrastructure (typed markers + filters) for sources we are not adding
in v0.0.1 and have no concrete plan to add post-v0.0.1.

### Cross-exam: archaeologist's latent bug

Per `round-01/archaeologist.md:269-275`:

> `infer_source_type` returns `"unknown"` for non-matching filenames
> and `knowledge_type="factual"`. Files named `notes.md` or `BL-007.md`
> would be ingested as `source_type="unknown"`, `knowledge_type=
> "factual"`. The schema's `ALLOWED_SOURCE_TYPES` trigger (v5
> migration) would reject `"unknown"` — this is a latent bug in the
> file-ingest path for non-AE files that `is_ingestable` passes
> through.

**Does this change my "no forward-compat" stance?**

**No — the bug fix actually reinforces it.** The bug is:
`is_ingestable` (parser.rs:159-180) is a blocklist that lets non-AE
markdown through, then `infer_source_type` returns `"unknown"`, which
v5 trigger rejects. Three fix shapes:

1. **Tighten `is_ingestable` to AE-only allowlist.** Reject the file
   at the parser stage with a clear error before `infer_source_type`
   is reached. Schema unchanged. Code change: ~10 lines.
2. **Add `"unknown"` to `ALLOWED_SOURCE_TYPES`.** Lets `notes.md`,
   `BL-007.md` ingest with `source_type="unknown"`. **This broadens
   the boundary** (files outside the AE pipeline now in scope). Bad
   fix — silently changes ratify decision.
3. **Surface a clear error message.** Currently the operator sees a
   schema constraint violation. Add a check before insert that says
   "file X did not match AE pipeline filename conventions; ingest
   refused." Schema unchanged. Code change: ~15 lines.

**Fix shape 1 + 3 enforce the AE-only policy at the MCP boundary.**
Both are no-schema-change, ~25-line bug fixes. They make the policy a
technical constraint instead of "policy not enforcement"
(archaeologist's phrase). Neither requires forward-compat.

In other words: the bug surfaces because v0.x **half-implements**
AE-only enforcement (file-ingest path lets non-AE files through, then
v5 trigger catches them with an unhelpful error). The fix is to
**finish enforcement, cleanly**. Forward-compat scaffolding is
orthogonal to this and irrelevant.

This actually **strengthens** the no-forward-compat case: the bug
proves that the operational boundary already exists in the schema
(v5's `ALLOWED_SOURCE_TYPES` trigger). v0.0.1's job is to make that
boundary cleanly observable at the MCP boundary, not to add a
parallel forward-compat surface for sources nobody is adding.

### Cross-exam: challenger's extraction-discipline reframe

Per `round-01/challenger.md:294-311`, challenger argues "AE-only
should mean AE pipeline extraction is the required extraction
discipline, not AE files as the only physical source."

The example case (`challenger.md:280-288`): a 90-minute debugging
session produces a clear fact ("SQLite WAL mode is incompatible with
our embedded use case"). This session is outside the AE pipeline.
Under "AE-files only" the fact is lost.

**The reframe has a real concern but the proposed fix is broader than
needed.** Two things are true:

1. The operator *does* have ad-hoc discoveries outside the AE
   pipeline. Real concern.
2. The MCP `memory_ingest` tool already accepts caller-specified
   structured fact submissions (per `archaeologist.md:235-241`). The
   `source_type` enum is restricted to 5 AE values, but the operator
   *could* submit a manually-distilled fact under one of those tags
   (e.g., write the WAL discovery into a project's `discussions/NNN-
   wal/conclusion.md` and `mengdie import` it).

The challenger's proposal — broaden ingest to any "AE-extraction-
discipline-equivalent" source — would loosen the gate:

- "Extraction discipline" is operator-self-reported quality. There is
  no automated way to verify a fact submitted via MCP carries
  AE-equivalent extraction.
- AE pipeline outputs are reviewed (plan review, conclusion review).
  Manual ad-hoc submissions are not. Treating them as quality-
  equivalent **dilutes the corpus**.
- The Perplexity 77% → 95% recall improvement (per `analysis.md` cited
  by ai-engineer in `round-01/ai-engineer.md:506-510`) came from
  **storing fewer memories**. Loosening admission is the wrong
  direction.

**The right route for the WAL discovery**: write it into an AE
artifact (a discussion, a retrospect entry, a small `analysis.md`).
This routes through the existing AE quality gate. Cost to operator:
the same minute they would have spent submitting via MCP.

**Verdict on challenger T4**: deserves consideration as a reopening
trigger (operator observes ≥3 high-value facts that genuinely cannot
be retrofitted into AE workflow). Does not warrant v0.0.1 scope
expansion. Updated reopening trigger:

> Reopen Topic 4 when (a) operator identifies ≥3 specific
> high-value fact incidents per quarter that (b) are produced
> outside any AE skill AND (c) cannot be retrofitted into an AE
> skill (e.g., `/ae:discuss` or a `/ae:retrospect` entry) without
> distorting the workflow.

### Recommendation (unchanged in spirit, sharpened in detail)

1. **Ratify AE-only.**
2. **Fix the latent bug** via `is_ingestable` tightening + clear
   error at MCP boundary (~25 LoC, no schema change). **This is the
   only T4-related code change in v0.0.1.**
3. **NO forward-compat scaffolding.** No free-form `source_type`
   string, no `source_schema_version`, no typed source markers, no
   per-source filters.
4. **Reopening trigger** (per challenger reframe): record the
   "≥3 high-value facts unfit for AE" trigger explicitly, in
   addition to the original v0.0.1 trigger.

### Agreements

- system-architect (`system-architect.md:466-470`, `system-architect.md:486-495`):
  ratify AE-only; YAGNI on forward-compat; explicit re-open
  discipline. Same verdict, same reasoning.
- ai-engineer (`ai-engineer.md:506-518`): ratify AE-only on
  signal-quality grounds (Perplexity empirical evidence).
- gemini-proxy (`gemini-proxy.md:458`): ratify AE-only — but with
  "forward-compat architecture." See disagreement below.
- archaeologist (`archaeologist.md:269-275`): the latent bug fact —
  used to refine the v0.0.1 acceptance criteria (bug fix in scope,
  forward-compat out of scope).

### Disagreements

- codex-proxy (`codex-proxy.md:158-163`): the forward-compat proposal
  describes the **already-existing** schema; anything *beyond* it
  (free-form string, source_schema_version, typed markers) is
  speculative infrastructure for nonexistent consumers. See cross-
  exam above.
- gemini-proxy (`gemini-proxy.md:458`): "ratify AE-only + forward-
  compat architecture" — same load-bearing-zero objection as codex.
  Forward-compat without a concrete second source to design against
  is dead surface area.
- challenger (`challenger.md:294-311`): the extraction-discipline
  reframe deserves a recorded reopening trigger but does not justify
  v0.0.1 scope expansion. See cross-exam above.

### Open Questions

1. Should the `is_ingestable` allowlist live in `parser.rs:159-180`
   (current location) or move to a single source-of-truth alongside
   `ALLOWED_SOURCE_TYPES` in `schema.rs`? Minimum: tighten in place;
   refactor only if the bug fix BL surfaces another reason to move.

---

## Topic 5 — Loop-closure signal (REFINED)

### Findings

Round 1 verdict (F-002 audit + `mengdie audit-stats` CLI from BL-014;
quantitative empty-result rate / repeat-query density / zero-row
days; qualitative `/ae:retrospect`) survives Round 2. Refinements
below per cross-exam.

### Cross-exam: 028's "no ACK feedback" lock — exact language

Per `docs/discussions/028-v0.0.1-architecture-design/conclusion.md:22-27`:

> **MCP `memory_search` ACK feedback channel — NO in v0.0.1
> contract.** challenger's argument (Round 2): the "used" signal is
> ambiguous — an AI that reads and discards facts by exclusion has
> still "used" them. Contractual burden on every integrator is not
> worth a noisy precision estimate. **All Topic 4 triggers must be
> server-side observable from the persisted domain audit table.**

Two locks in this language:

- **Lock 1**: no caller-side ACK feedback channel as v0.0.1 contract.
- **Lock 2**: all triggers (and by extension, by 028's reasoning, all
  quantitative loop signals) **must be server-side observable from
  the persisted domain audit table** — F-002.

**Applied to peer T5 proposals:**

| Proposal | Source | Crosses lock 1 (ACK)? | Crosses lock 2 (server-side)? |
|---|---|---|---|
| empty-result rate | minimal-change (Round 1) | NO | NO — derives from `audit_returned_facts` empty/non-empty |
| repeat-query density | minimal-change (Round 1) | NO | NO — derives from `memory_search_audit.query` |
| nonempty rate | ai-engineer (`ai-engineer.md:303-309`) | NO | NO — derives from `audit_returned_facts` join |
| supersession rate | ai-engineer (`ai-engineer.md:241-242`) | NO | NO — `audit_returned_facts` join `memory_entries.valid_until` |
| synthesis-influencing-search rate | system-architect (`system-architect.md:606-609`) | NO | NO — `audit_returned_facts` join `memory_entries.source_type='synthesis'` (server-side observable) |
| search-result-cited rate | codex-proxy (`codex-proxy.md:192-196`) | **YES** — requires "yes/no flag in ae:analyze post-research injection" | YES — would need new caller-side write |
| Round 0 injection citation rate | challenger (`challenger.md:367-370`) | **YES** — "requires explicit tracking of did the agent respond to Round 0 content" | YES — same |
| thumbs up/down on every result | gemini-proxy (`gemini-proxy.md:411-414`) | **YES** — "operator actively grading the system's output" | YES — needs new ACK write path |
| qualitative ae:retrospect verdict | minimal-change, ai-engineer (`ai-engineer.md:312-316`) | NO — operator-side, not MCP ACK | N/A — qualitative, no metric |

**Conclusion**: 028's lock 1 + 2 rules out three of the eight peer
proposals (codex's cited-rate, challenger's R0-citation rate, gemini's
thumbs up/down). The five remaining are all server-side derivable
from F-002.

### Cross-exam: can system-architect's "synthesis-influencing-search
rate" be computed without crossing the no-ACK lock?

Per `system-architect.md:606-608`:

> **Synthesis-influencing-search rate (lifetime, decay-weighted to
> last 30d)**: of the facts returned by `memory_search`, what
> fraction had `source_type = 'synthesis'`? If syntheses are never
> returned (or returned at < 5% rate), the synthesis pass is doing
> work that nobody benefits from. Joinable via `audit_returned_facts
> → memory_entries.source_type`.

**Yes — the metric is server-side observable**: `audit_returned_facts`
gives the fact_ids returned per search; `memory_entries.source_type`
is already populated; the join is one SQL query. **The metric does
NOT cross 028's no-ACK lock.**

**However, there is an independent problem with the metric in v0.0.1.**
Per `round-01/archaeologist.md:135-139`:

> Synthesis pass embeddings: `new_mem.embedding = None` at line 569 —
> synthesis rows are stored WITHOUT an embedding at creation time.
> This means they cannot be clustered or found via vector search
> until a re-embedding pass runs. No such re-embedding pass exists in
> the code.

So synthesis rows in v0.0.1 can only surface via FTS5 hit
(text-keyword match), never via vector similarity. The
synthesis-influencing-search rate is **structurally suppressed** in
v0.0.1: it will read close to 0 not because syntheses are useless,
but because syntheses are not vector-searchable.

This is a **separate bug** (file as a Topic-2-or-storage BL: synthesis
embeddings at insert time). Until that bug is fixed, the metric
**cannot be a v0.0.1 falsification signal** — its zero reading is
explained by the bug, not by loop dysfunction.

**Refined Topic 5 position**: I include
synthesis-influencing-search rate **conditionally** — it lands in v0.0.1
**only if** the synthesis-embedding bug is fixed in the same sprint.
Otherwise it is a deferred BL that fires when the bug is fixed.

The unconditional v0.0.1 minimum remains:
- **Quantitative**: empty-result rate, repeat-query density, zero-row
  days — all derivable from F-002, all server-side, all already
  present in `audit_returned_facts` / `memory_search_audit`.
- **Qualitative**: `/ae:retrospect` operator verdict — already exists.

### Cross-exam: challenger's Goodhart's Law concern

Per `round-01/challenger.md:340-370`, challenger warns: "Whatever
metric is chosen will become the proxy for 'the loop is working.' The
operator will optimize for that proxy. The question is whether the
proxy can be gamed without the loop actually closing."

**Goodhart's Law has limited applicability to a solo-operator system,
but the concern is partially valid.** challenger's own falsification
path (`challenger.md:391-393`) names the issue: "the operator has no
incentive to produce call volume artificially — which may be true,
but should be stated, not assumed."

**Stated.** A solo operator who is also the implementer has no
external observer rewarding higher counts. There is no career, money,
or social signal attached to "more searches per day." Goodhart's Law
applies to **systems where measurer ≠ implementer**; mengdie inverts
this — the operator both measures and acts on the metric.

**More importantly**, my Round 1 minimum signals are **inverse-
gameable**:

- **empty-result rate**: high = bad signal. To "game" it, operator
  would have to load fake facts into the corpus to make searches
  succeed — that is corpus health, not corpus gaming.
- **repeat-query density**: high = bad signal (rediscovery).
  Operator cannot game it down without actually solving rediscovery.
- **zero-row days**: zero count days = bad signal (loop dead).
  Operator cannot game by avoiding searches without observably
  killing the loop.

challenger's worry applies to **counts going up** (search count,
synthesis count). My signals go up = bad. That **inverts** the
Goodhart pressure. Challenger's concern dissolves when applied to the
specific signals proposed.

**Verdict on challenger T5**: real concern, well-named. Applies to
naive metrics (raw search count). Does not apply to the proposed
inverse-gameable signal set. Worth recording as a "do not propose
naive count metrics" guardrail in the conclusion.

### Recommendation (refined)

**Minimum (lock-compatible, inverse-gameable, F-002-derived):**

1. **Quantitative** (via `mengdie audit-stats` CLI = BL-014 already
   filed, zero new schema):
   - **empty-result rate** (rolling 7d): `(searches with empty
     `audit_returned_facts`) / (total searches)` — high = bad.
   - **repeat-query density** (rolling 30d): count of distinct
     `query` strings appearing ≥3 times — high = bad (rediscovery).
   - **zero-row days** (count over rolling 30d): days with zero
     `memory_search_audit` rows — high = bad (loop unused).

2. **Qualitative**: `/ae:retrospect` already prompts the operator
   weekly to reflect on prior period; one prompt-line addition on the
   AE plugin side ("Did mengdie short-circuit anything this period?
   yes/idk/no") is the minimum AE-side coupling and lives in the AE
   plugin BL, not in mengdie code.

3. **Conditionally added in v0.0.1 if synthesis-embedding bug is
   fixed in the same sprint**: synthesis-influencing-search rate
   (from system-architect). Otherwise filed as deferred BL with
   trigger "synthesis-embedding bug fixed."

4. **Explicitly excluded from v0.0.1** (cite locks):
   - search-result-cited rate (codex): crosses 028 ACK lock.
   - Round 0 injection citation rate (challenger): crosses 028 ACK
     lock.
   - thumbs up/down on every result (gemini): crosses 028 ACK lock.
   - All filed as deferred BLs with trigger "028 ACK lock revisited
     in a future discussion with new evidence."

### Agreements

- ai-engineer (`ai-engineer.md:300-316`): nonempty rate + qualitative
  retrospect-hook + falsification rule. Substantively identical to
  my Round 1 + Round 2 position; ai-engineer's phrasing of the
  falsification rule (`ai-engineer.md:336-345`: "nonempty < 20% over
  14d AND two 'idk' retrospect verdicts → loop is not delivering")
  is sharper than mine — adopt the falsification rule wording.
- system-architect (`system-architect.md:592-614`): the
  synthesis-influencing-search rate is server-side observable;
  conditionally agree pending synthesis-embedding bug fix (see
  cross-exam above).
- archaeologist (`archaeologist.md:283-360`): F-002 schema facts —
  used to verify proposal lock-compatibility above.
- codex-proxy (`codex-proxy.md:198-202`) on the **qualitative
  retrospect-hook** as Signal 2 — same shape, different phrasing.
  Disagreement on Signal 1 (cited rate vs nonempty rate) recorded
  separately.

### Disagreements

- gemini-proxy (`gemini-proxy.md:411-414`): thumbs up/down per
  result. **Crosses 028 ACK lock; v0.0.1 must reject.** Either 028
  is reopened with new evidence, or the proposal is filed as deferred
  BL.
- codex-proxy (`codex-proxy.md:192-196`): search-result-cited rate
  via "yes/no flag in ae:analyze post-research injection." **Crosses
  028 ACK lock**; same disposition.
- challenger (`challenger.md:367-370`): Round 0 injection citation
  rate. **Crosses 028 ACK lock**; same disposition. Goodhart's Law
  concern partially valid but does not apply to inverse-gameable
  metrics — see cross-exam.

### Open Questions

1. Where does the `mengdie audit-stats` CLI output land for
   operator visibility? Per `system-architect.md:570-584`, four
   options: stdout, MCP tool `loop_status`, ingestion-time inline,
   daily report file. Minimum: stdout + launchd capture (same
   pattern as dream); inlining in `IngestOutput` is a low-cost
   addition if the ingest BL has spare scope. Defer to F-002 read
   path BL-014 specification.
2. Should the synthesis-embedding bug
   (`archaeologist.md:135-139`) be filed as a v0.0.1 sprint item or
   deferred? If deferred, system-architect's synthesis-influencing-
   search metric is also deferred. Minimum: file the bug; sprint
   placement decided at roadmap stage.

---

## Cross-topic discipline summary (Round 2 update)

| Topic | Round 1 verdict | Round 2 update | New v0.0.1 code |
|---|---|---|---|
| 1 ingest mechanism | push-only + `mengdie import` | UNCHANGED | one CLI subcommand (already exists as `cmd_import`) + AE-plugin BL |
| 2 reflection trigger | cron + on-demand (already shipped) | REFINED — cron logic shipped, plist is template; add a one-paragraph cron-setup doc | one markdown doc; **NO `ReflectionTrigger` trait** |
| 3 cross-project scope | ratify §5 + reopening trigger | UNCHANGED | zero (one-line reopening trigger note); + separate BL for cwd-stale `project_id` bug |
| 4 ingest source boundary | ratify AE-only + reopening trigger | EXPANDED — ratify + latent bug fix (~25 LoC); reopening trigger refined per challenger reframe | ~25 LoC bug fix + one docs note |
| 5 loop signal | F-002 + audit-stats CLI | REFINED — three inverse-gameable signals (empty-rate, repeat-query, zero-rows) explicitly enumerated; synthesis-influencing-search conditional on separate embedding bug fix | one CLI subcommand (BL-014, already filed) |

**Total new v0.0.1 code these decisions imply (Round 2)**:

- 1 AE-plugin-side BL: each terminal AE skill calls `memory_ingest`
- ~25 LoC: T4 latent bug fix (`is_ingestable` allowlist + clear MCP
  error)
- 1 markdown doc: `docs/operations/cron-setup.md`
- 1 CLI subcommand: `mengdie audit-stats` (BL-014, already filed)
- 1 separate bug BL: `project_id` cwd-staleness on ingest
- 1 separate bug BL: synthesis embeddings at insert time

The `mengdie import <dir>` from Round 1 is already implemented as
`cmd_import` per `archaeologist.md:43-48` — **this Round 1 deliverable
collapses to zero new code**.

**Total Round 2 reductions vs Round 1**: one CLI subcommand removed
(`mengdie import` already exists); one markdown doc added (cron-setup);
two separate bug BLs surfaced (cwd-stale project_id; synthesis
embedding). Still ~one engineering week of work for v0.0.1, less than
any peer's proposal that adds traits / forward-compat / new metrics
infrastructure.

**Total `more` we are explicitly NOT doing** (carried from Round 1
+ Round 2 cross-exam):

- Watcher daemon (Round 1, reinforced by archaeologist falsifying
  challenger's "launchd precedent")
- `ReflectionTrigger` trait (Round 2 reject — Karpathy load-bearing
  fail; abstraction over one impl)
- Salience / composite / debounced trigger metrics (Round 1)
- Per-call / per-skill / per-type cross-project policy (Round 1)
- `source_type` free-form string / `source_schema_version` /
  typed source markers / per-source filter pipeline (Round 2 reject
  — codex's proposal already exists; further forward-compat is
  speculative scaffolding for nonexistent consumers)
- Caller-ACK metrics (search-cited rate, R0 citation rate, thumbs
  up/down) — all blocked by 028's lock 1 + 2.

Each "NOT doing" item has either a recorded reopening trigger
(T2/T3/T4/T5 reopening triggers) or a structural lock (028 ACK lock).
None is forgotten or hidden.

This Round 2 position holds the line: **two CLI subcommands worth of
code, one bug fix, one doc, plus AE-plugin wiring** — for a v0.0.1
that closes the core promise loop end-to-end. Anything more than
this, in any topic, is for v0.0.2+.
