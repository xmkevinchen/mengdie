---
discussion: "027"
round: 2
agent: system-architect
lens: "system design — module boundaries, contract design, reversibility"
created: 2026-05-05
---

# Round 2 — system-architect findings

## Round 1 facts to incorporate

Two empirical corrections from archaeologist round-01 force me to update
my Round 1 verdicts:

1. **Cron is NOT actually running.** archaeologist round-01:71-77 verifies
   `resources/com.mengdie.dream.plist` is a template
   ("`<!-- Update this path to your built binary -->`" at line 9), not a
   system-installed unit. archaeologist round-01:131-133 explicitly:
   "Is the launchd plist actually loaded on the operator's machine? Code
   cannot confirm." The "13 syntheses" archaeologist round-01:84-87
   confirms came from `mengdie dream --synthesize` — an explicit
   on-demand CLI invocation, not cron-driven. **My Round 1 T2 verdict
   ("cron is shipped, has produced empirical output") was wrong on the
   first half: cron is shipped as code path but is operationally
   unproven.**

2. **Watcher zero call sites + AE-only is policy not enforcement.**
   archaeologist round-01:22-25: `rg start_watcher` and `rg watch_loop`
   across `src/` returns ONLY hits inside `watcher.rs` itself —
   `bin/mcp_server.rs`, `bin/cli.rs`, no other module imports it.
   archaeologist round-01:258-261: "The MCP `memory_ingest` tool
   accepts any text content from any caller. There is no gate that
   checks 'did this come from an AE pipeline file?' The AE-only
   boundary in CLAUDE.md is an architectural intent, not a technical
   constraint." Both update my Round 1 T1 + T4 verdicts in the same
   direction (less code than I assumed; less enforcement than the
   commitments imply).

A third archaeologist finding deserves explicit pull-forward:
archaeologist round-01:135-139 — synthesis rows are stored with
`embedding = None` (`new_mem.embedding = None` at `dreaming.rs:569`).
This is a material gap that affects T2 verdict choice (a debounced
trigger that re-clusters synthesis output is moot if syntheses can't
participate in clustering).

---

## Topic 1 — Ingest mechanism

### Findings (with file:line evidence)

**Push-default holds against gemini's hybrid push-primary +
pull-fallback (gemini-proxy round-01:69-79).** The gemini argument is
that pull-fallback "handles offline/asynchronous scenarios, naturally
replays cold-start content from docs/." Three counter-points:

1. **Cold-start replay is already covered by push.**
   minimal-change-engineer round-01:73-83 names the mechanism:
   `mengdie import <dir>` walks ingestable files and emits push calls,
   reusing the F-003 `ingest::ingest_text` /
   `ingest_text_with_resolves` helpers. archaeologist round-01:43-48
   confirms this CLI bulk-import path exists today
   (`src/bin/cli.rs:729-742` walkdir + per-file `ingest_file`). The
   "naturally replays" property of pull is matched by an existing
   one-line operator command. Gemini's pull-fallback adds a daemon to
   solve a problem already solved.

2. **AE-not-running is the wrong scenario for v0.0.1.** Gemini's
   pull-fallback rationale (gemini-proxy round-01:79) is "handles
   offline/asynchronous scenarios." But the operator's v0.0.1 use case
   is exactly: AE plugin is the only producer (Topic 4 ratify). An
   environment where AE isn't running is by definition an environment
   where ingest is not happening — pull-fallback would have no work to
   do, because the inputs aren't being produced. Gemini's
   "ingest at rest" pattern (gemini-proxy round-01:36-39) maps to the
   `mengdie import` CLI, not to a watcher daemon.

3. **Hybrid doubles the failure-mode surface.** minimal-change-engineer
   round-01:96-99: hybrid means "every observability story (Topic 5)
   has to cover two surfaces. Every cold-start question has to specify
   which path is authoritative. Net: doubled maintenance for redundant
   delivery." This argument is system-architectural, not just
   minimal-code: every contract that depends on "what facts exist
   when synthesis runs" (ai-engineer round-01:434-441) becomes
   non-deterministic under hybrid.

**Challenger's pull-default argument (challenger round-01:28-95) is
empirically weakened by archaeologist's findings.** Challenger
round-01:74-77: "The real v0.x failure was not choosing pull — it was
not wiring the watcher to a daemon. That is a one-plist + one-service-
binary gap, not an architectural flaw in pull." This understates the
work: archaeologist round-01:57-64 also points out the
`is_ingestable` library uses a blocklist (any `.md` passes unless
specifically excluded), which would ingest `BL-*.md`, `topic-*.md`,
and other structural files unless the watch path is precisely
configured. Wiring the daemon is one plist + one binary + one
allowlist + one supervision policy + one log-rotation strategy. Each
of those is a small thing; together they exceed `mengdie import`'s
single-CLI-subcommand cost by an order of magnitude.

**Codex round-01:24-39 corroborates push.** codex-proxy round-01:38-39
explicitly: "None of the production frameworks chose pull-daemon as
primary. All chose push-with-async-queuing." This is independent
evidence from outside the v0.x inertia challenger flagged
(challenger round-01:411-413).

**Resolves contract is push-only by construction.** My Round 1
finding holds: `mcp_tools.rs:96-98` `resolves: Option<Vec<String>>`
requires the caller to know predecessor IDs. Pull-mode would have to
parse each file for "this supersedes IDs X, Y" markers — adding
parser surface that AE is far better positioned to compute.

### Architectural updates from Round 1 facts

- archaeologist round-01:258-261's "AE-only is policy not enforcement"
  observation does NOT change the T1 verdict. The mechanism question is
  about delivery shape; the enforcement question is about source
  validation. They are decoupled. Push-as-default still wins on
  delivery shape; the enforcement question lands in T4.
- ai-engineer round-01:447-453 agrees ("watcher library should be
  archived (or explicitly marked 'experimental, not v0.0.1') rather
  than wired to a daemon"). I would NOT archive — the library is 130
  LoC of tested code with no maintenance load. Mark as opt-in;
  document its non-canonical status. Deletion would be a correctness
  loss if the discussion later flips on a future post-v0.0.1
  generic-AI-tool integration.

### Verdict update

**Push as v0.0.1 default. Confirmed.** Reversibility high. Watcher
library kept in-tree, marked experimental/opt-in via module doc
comment. Defer the daemon entirely; v0.0.1 ships zero new daemon
surface.

### Agreements

- Agree with codex-proxy round-01:43-46 ("Push should be v0.0.1
  primary"; rationale: OpenAI Vector Stores, Responses API, mem0,
  LangMem, Graphiti convergence).
- Agree with ai-engineer round-01:447-451 ("push primary; watcher
  library should be archived" — modulo my preference for mark-as-opt-in
  rather than archive).
- Agree with minimal-change-engineer round-01:74-83 (push-only +
  `mengdie import` already exists; cold-start is solved).
- Agree with archaeologist round-01:22-25 / round-01:43-48 on the
  empirical state.

### Disagreements

- Disagree with gemini-proxy round-01:69-79 (hybrid push-primary +
  pull-fallback). Disagreement is on the cost-benefit: gemini's
  rationale for pull-fallback ("offline / asynchronous scenarios")
  doesn't apply to mengdie's actual v0.0.1 single-producer
  configuration. The hybrid earns its keep only when a second
  producer that bypasses the push contract exists; v0.0.1 has none.
- Disagree with challenger round-01:46-50 (the "decoupling is a
  structural virtue" argument for pull). For a single-producer system
  with a richer push API contract (`resolves`), decoupling produces
  loss of contract surface, not a virtue.

### Open Questions

- Should `mengdie import <dir>` ship in v0.0.1 sprint as part of T1
  resolution, or is it already shipped (archaeologist round-01:43-48
  confirms `walkdir` + `ingest_file` exists in `cli.rs`)? If shipped,
  the T1 deliverable is purely a documentation update plus the
  per-skill push-call wiring on AE side. If only partially shipped,
  one BL files the gap.
- minimal-change-engineer round-01:120-124 raises whether AE-side
  push wiring lives in a thin shared helper or per-skill. This is
  AE-plugin scope, not mengdie scope. System-architect view: a thin
  helper crate inside the AE plugin reduces N skill-side bugs to one
  testable site. File as AE-plugin BL.

---

## Topic 2 — Reflection trigger model

### Findings (with file:line evidence)

**Round 1 fact correction is decisive: cron is unverified.**
archaeologist round-01:71-77 + 131-133 collapses the "cron is
shipped" defense. The plist is template-shaped, not installed; the
"13 syntheses" came from on-demand CLI (archaeologist
round-01:84-87). My Round 1 verdict ("cron + on-demand both running")
was operationally wrong — only on-demand is empirically operational.

**Restated trigger inventory after correction:**

| Candidate | Code state | Empirical state |
|---|---|---|
| cron | shipped as `dreaming.rs::run_dreaming` + `cmd_dream` + plist template | **template not installed; never run on operator's machine via launchd; runs only when operator manually fires the CLI** |
| on-demand | shipped, exercised | produced 13 syntheses (the one empirical data point we have) |
| salience-threshold | not present | not present |
| composite (SCM) | not present | not present |
| debounced submit-dedupe | not present | not present |

**Should I update to ai-engineer's `ReflectionTrigger` trait
(ai-engineer round-01:140-179)?** ai-engineer round-01:170-179
proposes a trait seam at ~80 LoC + 4 tests, parallel to the existing
`LlmProvider` trait, with `CronTrigger` and `OnDemandTrigger` as
v0.0.1 implementations, salience/composite/debounced as v0.0.2+
plug-ins.

The trait proposal is in tension with the **028 conclusion**.
`docs/discussions/028-v0.0.1-architecture-design/conclusion.md:17`
explicitly: "**Do NOT introduce `Reflector` trait** in v0.0.1,
regardless of sqlite-vec spike outcome. ANN-based clustering is
similarity-primitive swap inside one strategy, not a 2nd reflection
strategy." That decision was 5-of-5 unanimous + UAG-passed in 028
(`conclusion.md:17`).

But the 028 trait under discussion was `Reflector` (over the
synthesis pass itself), not `ReflectionTrigger` (over when to fire
the pass). These are different trait scopes:

- 028's rejected `Reflector`: abstracts "what synthesis does"
  (the cluster + LLM-summarize algorithm). Rejected because there's
  no v0.0.1 call site selecting between ≥2 synthesis algorithms
  (`conclusion.md:17`).
- ai-engineer's proposed `ReflectionTrigger`: abstracts "when
  synthesis fires." There ARE ≥2 v0.0.1 candidates (cron,
  on-demand) per ai-engineer round-01:142-145. The trait's
  `should_fire(&self, context: &ReflectionContext) -> bool` method
  (round-01:141-145) has a real polymorphic call site.

So 028's argument doesn't transfer 1:1. The trait's premise (≥2
strategies at v0.0.1) is satisfied for `ReflectionTrigger` even
though it failed for `Reflector`.

**However:** at v0.0.1 with cron operationally unproven, cron and
on-demand are not really 2 strategies — they're 1 strategy
(on-demand) plus 1 documented-but-unverified strategy. ai-engineer's
trait abstracts a difference that empirically reduces to "did the
operator type the command, or did launchd fire it." Both call into
the same `mengdie dream --synthesize` invocation today; the
plist-template being the only difference.

**The architecturally cleaner v0.0.1 framing.** Drop cron-as-default;
ship **on-demand as the v0.0.1 default**, plus an opt-in plist
template (already exists, no new code) that operators can install
when they want nightly background reflection. No trait. No new
module. Cron becomes opt-in operational deployment, not a
first-class trigger model.

This matches ai-engineer's stated lean (round-01:196-201): "I lean
on-demand (lowest ambient surface, no plist to maintain, aligns
with operator-attention-aligned reflection). Cron is the *fallback*
— operator opts in via launchd plist if they want nightly runs."

It also matches challenger's falsification challenge
(round-01:111-127): "13 syntheses are an output count, not a
quality or timeliness measure." With on-demand-default, the
operator IS the trigger and IS the quality auditor — both in the
same decision moment. Cron defers the audit to "next morning's
log review" which observably never happens.

**Trait deferred — minimal-change-engineer round-01:153-159 +
codex-proxy round-01:88-95 reasoning is decisive.**
minimal-change-engineer round-01:204-209: "The other three triggers
are answers to questions v0.0.1 has not asked. File them; do not
build them." codex-proxy round-01:90-94: cost control argument
(predictable LLM spend) plus "salience, entropy, conflict-density
all require runtime metrics mengdie doesn't compute." Adding a
trait scaffold for two strategies that empirically reduce to one
fires the same YAGNI test 028 fired against `Reflector`.

If a future v0.0.2 BL adds salience or debounced, the trait can
land then — when there's actually a 3rd-or-greater concrete
strategy + a runtime call site that picks between them. v0.0.1
should not pre-pay that cost.

### Architectural updates from Round 1 facts

- ai-engineer round-01:135-139's "synthesis stored with embedding =
  None" finding is critical for T2: any trigger model that depends
  on synthesis rows participating in subsequent clustering passes
  (debounced, composite-with-conflict-density) requires the
  embedding gap to be filled FIRST. This adds another reason to
  defer the metric-bearing triggers — they are not just
  "expensive runtime metrics" but "expensive runtime metrics
  blocked behind an unfixed schema/pipeline gap."

### Verdict update

**v0.0.1 default: on-demand only. Cron is opt-in operational
deployment via the existing plist template (operator installs
manually if they want nightly runs).** No `ReflectionTrigger` trait.
Salience, composite, debounced filed as v0.0.2+ BLs with explicit
triggers. Reversibility high — adding a trait later is purely
additive.

**Architectural rationale:** at v0.0.1 with one operationally
proven trigger (on-demand) and one operationally unproven one
(cron-via-plist-template), positing a trait abstracts a
non-existent strategy gap. The trait makes sense in v0.0.2 when a
concrete second strategy lands. For v0.0.1, on-demand is the
operator-attention-aligned default; cron is opt-in. dreaming.rs
remains trigger-agnostic (it already is — `run_dreaming_with_config`
takes `now: Option<DateTime<Utc>>`, not a trigger object).

### Agreements

- Agree with ai-engineer round-01:196-201's stated lean (on-demand
  as default; cron as opt-in fallback).
- Agree with minimal-change-engineer round-01:204-209 ("File them;
  do not build them" for salience/composite/debounced).
- Agree with codex-proxy round-01:88-95 (operator cost control;
  predictable cron > aggressive metric-driven triggers).
- Agree with archaeologist round-01:131-133 (the launchd plist may
  not be loaded; cron is documented but unverified).
- Agree with challenger round-01:144-150 (the right question is
  "is burst-activity mismatch a real problem?" — and the answer is
  "we don't know yet"; on-demand-default avoids needing to answer
  it).

### Disagreements

- Disagree with ai-engineer round-01:140-179's `ReflectionTrigger`
  trait introduction. Same argument as 028's `Reflector`
  rejection (`conclusion.md:17`): no v0.0.1 runtime call site
  selecting between ≥2 distinct strategies. Cron-via-plist and
  on-demand-via-CLI both call the same `run_synthesis_pass`; the
  trait abstracts a deployment difference, not an algorithmic one.
- Disagree with my own Round 1 verdict on T2 ("cron + on-demand
  both v0.0.1 defaults"). Updated to on-demand default + cron
  opt-in, per the empirical correction.
- Partially disagree with challenger round-01:131-138 ("on-demand
  as v0.0.1 default — not as a permanent answer, but as the
  minimum-complexity option"). Agree on the default; disagree that
  it should be framed as "until we commit to a trigger model."
  Permanent answers are out of scope for v0.0.1; the trigger model
  question is genuinely open and stays so.

### Open Questions

- Should the plist template be removed from the tree (since it's
  unverified) or kept as opt-in documentation? Architectural view:
  keep it as opt-in. Removal would force a re-creation when the
  next operator wants nightly runs. minimal-change-engineer
  round-01:170-173's documentation route ("v0.0.1 default: on-demand
  + plist template ships as opt-in") covers this.
- Does on-demand need an MCP tool wrapper (`dream_run`) so AE skills
  can invoke synthesis after a batch of related artifacts (e.g.,
  `/ae:retrospect` triggers synthesis at end of cycle)?
  Architecturally trivial; v0.0.1 sprint scope question. Filing as a
  small follow-up BL is fine — not gating Topic 2 resolution.

---

## Topic 3 — Cross-project default retrieval scope (ratify-or-defer)

### Findings (with file:line evidence)

**The architectural state is unchanged from Round 1.** archaeologist
round-01:174-178 confirms: changing the default is a one-line diff
(`mcp_tools.rs:192-195`). Storage is global; per-project filter is
the only conditional; reversibility is at single-line scope.
Provenance per result is already tracked (every memory has
`project_id`).

**Challenger round-01:188-220 surfaces a real architectural
question, not just a preference.** Three points deserve direct
engagement:

1. **The §5 rationale was migration cost, not correctness**
   (challenger round-01:204-208). Concrete: CLAUDE.md §5: "avoid
   migration cost when adding cross-project later." Challenger is
   correct that this is a deferred-design argument, not a
   designed-for-correctness argument. archaeologist round-01:174-178
   confirms the migration cost is now zero (one-line diff).
2. **Single operator, unified cognitive identity**
   (challenger round-01:189-195). For mengdie's actual v0.0.1
   operator, "what did we decide about MCP transport patterns?"
   spans projects. Per-project default returns the subset.
3. **AE-skill caller-shape question** (challenger round-01:223-238).
   AE skills are mostly callers; ae:work mid-task wants narrow
   scope; ae:analyze pre-research wants wide scope.

The third point dissolves the first two as a global default
question. **The right architectural shape is per-call, not per-server-
default.** Specifically:

- Each MCP-tool caller specifies scope at call time
  (`mcp_tools.rs:24-35` `scope: Option<String>` already supports
  this).
- The "default when scope omitted" is only relevant for callers
  that omit it. v0.0.1 callers are: AE skills (ae:analyze,
  ae:work, ae:plan, ae:discuss, ae:review, ae:retrospect) and
  the operator's interactive `mengdie search`.
- AE skills can be wired explicitly to specify scope per skill:
  ae:analyze passes `scope: "global"`; ae:work passes
  `scope: "project"`. This is challenger's "caller-type-aware"
  resolution (round-01:236-238).
- The operator's `mengdie search` keeps per-project default for
  the cwd-anchored case.

The default-when-omitted is then a fallback for the rare
unparameterized call. Either choice (per-project default or global
default) is acceptable because the high-volume callers have made
the choice explicit upstream.

**This shifts T3's locus.** It's not "is per-project the right
default"; it's "do AE skills specify scope per skill, or do they
inherit the server default?" The latter is fragile (one server-
config flip silently changes all skills' behavior). The former is
verbose at the caller side but explicit.

**minimal-change-engineer round-01:289-296 raises legitimate
counter-friction** ("'per-call config decided by calling skill' →
each AE skill needs to be taught when to opt cross-project. This is
N skill changes on the AE side"). Concrete, but this is one BL on
the AE plugin side that wires N skills with skill-appropriate scope
constants. Not an open-ended program.

### Architectural updates from Round 1 facts

- archaeologist round-01:209-214 surfaces a stale-project-id bug:
  "the project_id at search time is the cwd at mcp_server startup,
  not the cwd at query time. For operators who open multiple
  projects in the same Claude Code session without restarting the
  MCP server, the default project_id is stale." This is independent
  of the T3 default-scope question but exacerbates per-project
  default's failure mode in multi-project sessions. Worth filing as
  a separate BL (per-call cwd resolution). Doesn't change the T3
  verdict.

### Verdict update

**Ratify §5 unchanged for v0.0.1 server default. Add explicit
guidance: AE skills should specify `scope` per call. The default-
when-omitted (per-project) is a fallback for callers that don't
parameterize.** This addresses challenger's correctness argument
without a default flip.

The reopening trigger remains the F-002-audit-data-driven trigger I
proposed in Round 1, with one refinement from minimal-change-engineer
round-01:267-282: the BL-014 audit-stats subcommand should compute
the per-project-vs-global ratio explicitly, so the trigger condition
is mechanically observable.

**Reversibility:** high — server default flip is one line; AE-side
per-skill scope wiring is N independent edits, each independently
reversible.

### Agreements

- Agree with codex-proxy round-01:117-125 (ratify; per-namespace
  isolation is the OpenAI converged pattern, AND keep cross-project
  observable in audit logs).
- Agree with minimal-change-engineer round-01:267-282 (ratify §5;
  reopening trigger via BL-014 audit-stats).
- Agree with ai-engineer round-01:472-490 (cross-project synthesis is
  a different, dangerous shape from cross-project search; synthesis
  must remain per-project even if search default changes).
- Agree with challenger round-01:222-238 that the resolution should
  be caller-type-aware (AE skills specify scope per skill).

### Disagreements

- Disagree with gemini-proxy round-01:240-251's "ratify per-project
  + global search index must exist." The "global search index must
  exist" framing implies a separate index; archaeologist
  round-01:158-161 verifies storage is global and filter is per-call,
  so no separate index is needed. Gemini's spirit is right (cross-
  project capability must be available); the mechanism is already
  there.
- Partially disagree with challenger round-01:182-220 ("cross-project
  should be the default"). Challenger's correctness argument lands
  for the operator's interactive use case but mis-fits AE-skill
  callers (challenger acknowledges this himself at round-01:222-238).
  The right resolution is per-skill explicit scope, not a global
  default flip.

### Open Questions

- Should the AE plugin's per-skill scope wiring be a v0.0.1 sprint
  BL or a follow-up? Architecturally trivial (1-2 lines per skill);
  worth including in v0.0.1 because the audit data won't be
  meaningful without correctly-attributed scope. File as v0.0.1
  AE-side BL.
- The stale-project-id bug (archaeologist round-01:209-214) — is it
  a v0.0.1 sprint item? My view: yes, it's a small fix
  (resolve project_id per call rather than at server startup) and
  it materially improves T3's per-project semantics. File as small
  v0.0.1 BL.

---

## Topic 4 — Ingest source boundary (ratify AE-only)

### Findings (with file:line evidence)

**Forward-compat tension is the real Round 2 question.** Three
positions in Round 1:

- codex-proxy round-01:158-163: "Add a `source` enum in ingest
  schema: `{ source: 'ae_plan' | 'ae_review' | 'ae_conclusion' |
  ... }`. Store source tag on every ingested memory. When broader
  sources are added (commit messages, issue text), the schema
  already supports typed provenance." Codex frames this as cheap
  insurance.
- gemini-proxy round-01:310-313: "Build ingest pipeline with
  source-type markers from day one (e.g., `source_type: 'ae_conclusion'`,
  not just `text`). Make per-source filtering/validation pluggable."
- minimal-change-engineer round-01:374-388: "Build flexibility for
  hypothetical future ingest sources is the exact failure mode the
  v0.0.1 rebuild is correcting against v0.x. ... v1 API breakage
  cost (the steel-man for forward-compat) is hypothetical; v0.0.1
  has zero external consumers."

**archaeologist round-01:36-42 is decisive on cost.** The source
type enum already exists (`mcp_tools.rs:38-58`):
`Conclusion | Review | Plan | Retrospect | Synthesis`. Adding a
sixth variant is a one-line change + display impl + serialization
test. The schema column is already TEXT (archaeologist
round-01:251-252), so zero schema migration is needed for new
variants.

This means **codex's forward-compat scaffolding is already in the
codebase**. It's not "build forward-compat for sources we may add";
it's "we have a typed enum today, adding new variants later is
trivial." Both codex and minimal-change are partially right:
- codex is right that typed source markers are the right shape.
- minimal-change is right that NO new infrastructure should be
  built for hypothetical future sources.
- The synthesis: ratify AE-only with the existing enum. The enum
  is the typed marker. New variants land when sources do. Don't
  preemptively widen the enum to accommodate sources nobody is
  adding.

**My Round 1 YAGNI position holds, with tightening.** I argued
"Option A: keep as-is; if a new source ever lands, add a variant +
parser. Cost on adoption: linear with sources." Round 1 evidence
confirms: cost on adoption is one enum line + one parser stanza,
which is essentially free.

**The challenger round-01:266-323 reframe is the more interesting
Round 2 question.** challenger round-01:268-275: distinguish two
claims being conflated:

> Claim A (defensible): AE-only for v0.0.1 because the extraction
> discipline (LLM-mediated structured extraction in the AE plugin)
> is the quality gate.
> Claim B (problematic): AE-only because mengdie = AE 的大脑 and
> anything outside the AE pipeline is out of scope.

challenger round-01:281-291 names the failure mode:
> The operator has a 90-minute debugging session that produces a
> clear fact ("SQLite WAL mode is incompatible with our embedded
> use case"). This session is outside the AE pipeline. No
> conclusion.md is written. Under AE-only, it is lost to mengdie.

Architecturally, archaeologist round-01:235-241 shows this is
already permissible: the MCP `memory_ingest` tool accepts any
caller-supplied content. Today's enforcement is policy-level, not
tool-level. The operator can `mengdie ingest --content "SQLite WAL
incompatible..."` (or via the MCP tool) and the fact lands in the
DB.

So challenger's challenge resolves to: **what does "AE-only" mean
operationally?** Two readable interpretations:

1. "Only the AE plugin's MCP-tool calls produce ingest events."
   This is the AE 的大脑 reading. Implementing it requires either
   (a) per-caller authentication (out of scope; no auth in v0.0.1
   per CLAUDE.md secrets-delegation), or (b) hand-on-discipline by
   the operator. Today's reality is (b), unenforced.
2. "AE-extraction-discipline is the quality bar." Anything that
   passes equivalent extraction (LLM-mediated structured
   propositional fact, with entity tags) is acceptable. The
   manual `memory_ingest` for the WAL fact passes this bar
   because the operator is doing the human-mediated extraction.

Interpretation 2 aligns with codex round-01:155-156's "boundary +
filtering is strength" lens. It also aligns with the actual code
(archaeologist round-01:258-261 confirms there's no enforcement of
file-source).

**System-architect verdict:** ratify AE-extraction-discipline as
the boundary, NOT AE-files-only. The blueprint's §3.1 list
("`conclusion.md`, `plan.md`, `review.md`, ...") is an
**enumeration of the v0.0.1 producers** that satisfy the
extraction discipline — it should not be misread as the only
permitted entry point. The MCP `memory_ingest` tool is the
contract surface; whoever calls it (AE skill, operator, future
producer) must satisfy the extraction discipline. The discipline
is the boundary; the enumeration of current producers is just
that — an enumeration.

This is a meaningful tightening of my Round 1 verdict. I had
"ratify AE-only" with no nuance; challenger's distinction
(round-01:268-275) is correct and worth absorbing into the Round 2
verdict.

### Architectural updates from Round 1 facts

- archaeologist round-01:269-275 surfaces a latent bug: file-ingest
  with non-AE filenames produces `source_type="unknown"`,
  `knowledge_type="factual"`, but the schema's
  `ALLOWED_SOURCE_TYPES` trigger (v5 migration) would REJECT
  `"unknown"`. This is an inconsistency in the file-ingest path
  for non-AE files that `is_ingestable` admits. **Worth filing as a
  separate BL** — independent of T4 verdict, but related (it's the
  enforcement gap challenger named, manifested as a real bug).

### Verdict update

**Ratify the EXTRACTION DISCIPLINE (LLM-mediated structured
propositional facts, with entity tags) as the v0.0.1 boundary.
Enumerate AE pipeline outputs as the canonical producers. Do NOT
build per-caller enforcement (auth, source filtering) into
v0.0.1.** The existing typed `source_type` enum stays as-is; new
variants land when concrete new sources land.

The re-open trigger from Round 1 holds with one refinement: a new
discussion is required IF a new producer of meaningful volume
emerges that is not AE-pipeline-shaped. Manual `memory_ingest`
(operator distilling an ad-hoc finding) does NOT trigger
re-opening — it falls within "AE-extraction-discipline applied by
human."

Reversibility is high: the typed enum + free-text storage column
admits new sources additively.

### Agreements

- Agree with codex-proxy round-01:158-163's typed source markers —
  noting that they already exist (archaeologist round-01:36-42).
  No NEW scaffolding required.
- Agree with minimal-change-engineer round-01:374-388's YAGNI on
  building forward-compat infrastructure — clarified that "the
  typed enum exists, don't widen it preemptively" is the correct
  reading.
- Agree with challenger round-01:294-311's distinction: ratify the
  extraction discipline, not AE-files-only.
- Agree with ai-engineer round-01:514-520 (signal quality dominates
  at sub-1000-fact scale; admission filtering matters more than
  source breadth).
- Agree with archaeologist round-01:258-261 on the enforcement-
  versus-policy distinction.

### Disagreements

- Partially disagree with my own Round 1 ("ratify AE-only" with no
  nuance). Updated to "ratify extraction discipline." The tighter
  framing is challenger's contribution.
- Partially disagree with gemini-proxy round-01:310-313 ("make
  per-source filtering/validation pluggable" as v0.0.1 design
  work). The pluggable filtering is the YAGNI minimal-change
  rightly resists. Today's enum + free-text storage is enough.
- Partially disagree with codex-proxy round-01:158-159 ("Add a
  `source` enum in ingest schema") to the extent it implies new
  scaffolding. The enum exists; nothing to add.

### Open Questions

- Should the file-ingest `"unknown"` -> `knowledge_type="factual"`
  bug (archaeologist round-01:269-275) be fixed in v0.0.1 sprint?
  My view: yes, small fix, on-trigger because it's a latent bug
  rather than scope creep. File as small BL.
- Does the discussion need to explicitly enumerate "manual
  `memory_ingest` is in-scope" so future operators don't read
  AE-only as files-only? My view: yes, write it into the
  conclusion's verdict text. Without this explicit, the
  challenger-named ad-hoc-discovery failure mode recurs.

---

## Topic 5 — Loop-closure signal

### Findings (with file:line evidence)

**Round 1 surfaced a productive disagreement on whether F-002
suffices.** Three camps:

- ai-engineer round-01:289-298 / minimal-change-engineer
  round-01:418-466 / archaeologist round-01:283-359: F-002 is
  sufficient substrate; what's missing is a READ procedure that
  surfaces the data. The new work is querying + display, not new
  schema.
- codex-proxy round-01:191-196: track "was a result cited in
  downstream agent output (yes/no flag in ae:analyze post-research
  injection)." This requires **ACK-from-caller** which 028
  explicitly rejected (`conclusion.md:22-32`).
- gemini-proxy round-01:412-444: "Thumbs up/down on every search
  result or synthesis" — also ACK-from-caller-shape.
- challenger round-01:339-371: warns about Goodhart on any
  search-call-count or synthesis-count metric.

**028 conclusion locks the contract.** `conclusion.md:22-32`:
"MCP `memory_search` ACK feedback channel — NO in v0.0.1
contract. ... All Topic 4 triggers must be server-side observable
from the persisted domain audit table." This is a hard constraint.
codex's "result cited in downstream agent output" and gemini's
"thumbs up/down on every result" both require caller ACK and
violate this constraint. **Their proposals are out-of-scope for
v0.0.1 by 028's own authority, regardless of whether the system-
architect lens favors them.**

This eliminates two of the three Round 1 camps. The remaining one
(F-002-derived metrics) splits across:

- ai-engineer round-01:300-316: per-search nonempty rate +
  qualitative retrospect-hook.
- minimal-change-engineer round-01:431-444: BL-014 `mengdie
  audit-stats` derived purely from F-002.
- challenger round-01:372-381: contradiction-detection events
  trending down + qualitative "did Round 0 inject relevant facts"
  per-session check.

**Challenger's Goodhart concern (round-01:339-388) deserves direct
engagement.** Challenger names search-call-count and
synthesis-count as gameable. The metrics I proposed in Round 1
(search-with-results-rate + synthesis-influencing-search rate) are
both ratios, not raw counts. Ratios resist the "I'll just call
more" gaming because the denominator inflates with the numerator.
Two refinements:

- Search-with-results-rate is the **inverse** of "calls returning
  empty," which is what the operator can falsify by inspection
  (challenger round-01:373-376 "if injected facts are ignored,
  loop is open"). minimal-change-engineer round-01:496-516 names
  the same falsification surface (empty-result-rate +
  zero-row-days).
- Synthesis-influencing-search rate (`source_type='synthesis'`
  fraction of returned facts) is a Goodhart-resistant variant: it
  measures whether mengdie's OWN synthesis output is being
  consumed by its OWN retrieval. The operator cannot game this
  without making syntheses that are then artificially recalled
  (which would manifest as high `recall_count` on a small set —
  separately observable).

**Challenger's hard-to-game proxy (round-01:380-386):**
"contradiction-detection events trending down over time within a
project." This is computable from existing schema (`superseded_by`
+ `valid_until` + per-day grouping). archaeologist round-01:329-340
notes that today's `mengdie stats` reads only the running-total
metrics counters; for time-series view, BL-014 would need to query
the actual rows in `memory_entries` filtered by date. The data is
there; only the read path is missing. This makes challenger's
proposal **architecturally consistent with F-002-as-substrate**.

**Updated minimum metric set for v0.0.1.** Three F-002-derived
signals, all decay-weighted, all surfaced via `mengdie stats`
(extending the existing CLI subcommand, BL-014):

1. **Per-search nonempty rate (7d rolling).** Measures whether the
   loop is being fed but mengdie is mute. Falsification rule
   (ai-engineer round-01:344-346 + minimal-change round-01:496-509):
   < 30% → loop is broken on retrieval side.
2. **Synthesis-influencing-search rate (30d rolling).** Measures
   whether synthesis output (the self-evolving promise) is being
   consumed. Falsification: < 5% → synthesis is doing work nobody
   benefits from.
3. **Contradiction-event-rate trend (per-day series, 30d window).**
   Measures whether the corpus is converging (lower trend) or
   churning (high trend). challenger round-01:380-386 notes
   direction ambiguity: low rate could be "consistent" or "narrow
   usage." The RATE alone is ambiguous, but the TREND is more
   informative.

All three are computable from existing F-002 schema +
`memory_entries` table. None require ACK feedback. None require new
event streams. All are ratio-shaped or trend-shaped (Goodhart-
resistant per challenger's lens).

**Surfacing channel:** `mengdie stats` (existing CLI; BL-014
extends it). Optionally inline a one-line summary in
`IngestOutput` so the operator sees the headline metric on every
push. minimal-change-engineer round-01:474-481 endorses this
("pinning `mengdie audit-stats` to the existing launchd plist...
zero engineering"). ai-engineer round-01:301-310 endorses
extending `mengdie stats` ("extending an existing CLI output with
one section is the smallest possible measurement surface").

### Architectural updates from Round 1 facts

- archaeologist round-01:329-359 confirms that `mengdie stats`
  today reads only the `metrics` table running totals, NOT the
  audit tables. So the v0.0.1 work for T5 is a non-trivial
  addition to BL-014 — querying the audit data, not just exposing
  it. Still small (~3 SQL queries + formatting), but not zero.
- codex-proxy round-01:172-177's OpenAI evals reference
  ("Production data is your most authentic source for evolving
  evaluation and training datasets") is structurally compatible
  with F-002-derived metrics. The audit table IS the production
  data; the eval is the operator's `mengdie stats` read.

### Verdict update

**v0.0.1 minimum loop-closure signal: three F-002-derived metrics
(nonempty-rate, synthesis-influence-rate, contradiction-trend),
surfaced via `mengdie stats` (BL-014). No ACK contract. No
separate event stream.** Reversibility high — any of the three can
be replaced or extended without schema change.

**Architectural rationale:** F-002 audit table is the substrate
designed for the 028 Topic 4 supersession trigger; loop-closure
metrics ride on the same substrate without sprawl. ACK-feedback
proposals (codex's citation rate, gemini's thumbs) violate the 028
locked contract and are out-of-scope. Ratio-and-trend shapes
resist challenger's Goodhart concern; raw counts would not.

### Agreements

- Agree with ai-engineer round-01:300-316 (per-search nonempty
  rate + qualitative retrospect-hook).
- Agree with minimal-change-engineer round-01:418-516 (extend
  `mengdie stats` / BL-014; no new schema).
- Agree with archaeologist round-01:329-359 on the enforcement gap
  (today's `mengdie stats` doesn't read audit tables — that's the
  v0.0.1 BL).
- Agree with challenger round-01:380-386 on contradiction-trend as
  hard-to-game; absorbed into the metric set.
- Agree with codex-proxy round-01:204-211 ("two separate signal
  streams that may sometimes disagree" — three is the same shape
  argument).

### Disagreements

- Disagree with codex-proxy round-01:191-196 ("was a result cited
  in downstream agent output yes/no flag in ae:analyze
  post-research injection"). Violates 028's ACK rejection
  (`conclusion.md:22-32`). Codex's metric is desirable in v1+
  but blocked at v0.0.1.
- Disagree with gemini-proxy round-01:412-415 ("Thumbs up/down on
  every search result or synthesis"). Same 028 violation.
  Gemini's intent (force-confront the operator) is achievable via
  the F-002-derived metrics surfaced inline in
  `IngestOutput` — same coercion shape, no caller-ACK contract.
- Disagree with challenger round-01:347-356's framing of
  search-call-count as the only-counter metric. The proposed
  metrics are ratios, not counts; Goodhart concern is mitigated
  by the denominator-inflates-with-numerator property.

### Open Questions

- Should `mengdie stats` output be surfaced as an MCP tool
  (`loop_status`) so AE skills can call it? ai-engineer
  round-01:402-405 raises this; minimal-change-engineer
  round-01:526-528 punts to "minimum is stdout." My view: file as
  small follow-up BL — not gating T5 verdict.
- Does the qualitative retrospect-hook (ai-engineer round-01:308-316,
  codex-proxy round-01:198-201) need AE-plugin-side support to be
  asked? Yes (ai-engineer round-01:411-413 acknowledges this) —
  AE-plugin-side BL, gates the qualitative-side measurement. v0.0.1
  can ship the quantitative side without it.
- minimal-change-engineer round-01:530-533: "Should retrospect
  explicitly include a 'mengdie loop' prompt from AE plugin side?
  Out of mengdie scope; this is an AE plugin-side BL if anything."
  My view: agree. v0.0.1 mengdie ships the three quantitative
  metrics; the qualitative retrospect-hook is an AE-plugin BL,
  filed but outside the mengdie sprint.
