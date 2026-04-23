---
agent: minimal-change-engineer
round: 1
created: 2026-04-23
lens: minimum-viable diffs, refuse scope creep, 3 similar lines beats premature abstraction
---

# Round 1 — Minimal-change Engineer

## Findings (per topic — argue for minimum)

### Topic 1: Smallest bundle that ships

**Position: 1 plan, 2 BLs (json-schema-contract + verify-decay-hardening). Ops-doc-polish splits out.**

The "3-BL bundle" framing treats "decay operator surface hardening" as a
unit because the files overlap. But file overlap is not the right
criterion; **review-risk coupling** is. Ask: if the plan fails review,
which BLs get held back unnecessarily?

- json-schema-contract + verify-decay-hardening have a HARD test
  coupling — the integration test asserts on `schema_version: 1`.
  These MUST ship together or the test has to mock the field. Bundling
  them saves one useless mock. Legitimate unit.
- ops-doc-polish has only a SOFT coupling (the doc references
  `breaches[]`, which already exists regardless of schema_version).
  Its 3 actions (SQL snippet, arrow fallback, rollback section) are
  pure documentation + one-line format string. Zero test interaction.
  Reviewers for a code+test plan and reviewers for a docs-polish plan
  look at different things. Bundling inflates the code plan's
  review-surface without benefit.

**The counter-argument "plan overhead = 1 S-class item" applies only
when plans are near-identical in kind.** A docs-only plan is ~20 min
of review (no drift-check churn, no CI iteration). Plan 014 ran 4 CI
iterations because it had code changes — a docs plan won't. So the
"3× overhead" math in the analysis is pessimistic for this specific
split.

Additional minimum-change observation: the integration test in the
schema-contract BL spawns `mengdie dream --decay-dry-run` via
`std::process::Command`. That's non-trivial test infrastructure. If
verify-decay-hardening's CI coverage action also needs a shell test
harness, **the SAME harness serves both** — strong reason to co-plan
those two. Ops-doc has no such shared infrastructure.

**Minimum shape**: Plan 015 = json-schema-contract + verify-decay-hardening
(M+M, shared test harness). Ops-doc-polish ships as its own S plan (or
as a trivial follow-up PR if the project's discipline allows it).

**Reject**: "bundle all three to save cycles" — ignores that the shared
harness benefit only couples 2 of the 3 BLs.

**Reject even harder**: any suggestion to expand the bundle to include
`release.yml` follow-ups, other decay-adjacent tidy-ups. The framing's
Topic 1 key question #3 already smells like scope creep ("should the
bundle also include..."). Answer: no.

### Topic 2: Smallest action on defer-items

**Position: do nothing. Leave both BLs where they are.**

The topic asks "keep / remove / close". The minimum action is
**none of those** — leave them in v0.8.0 with status:open and do not
touch the sprint frontmatter. Both BL bodies already contain:

- The trigger condition (machine-readable in the text).
- The reason "not now".
- The fix direction for when the trigger fires.

What concrete cost does leaving them impose?

- **Velocity accounting**: only matters if someone is computing
  velocity. Mengdie is solo-dev. No velocity consumer exists. Cost = 0.
- **Sprint close ambiguity**: see Topic 4 — this is the symptom that
  Topic 4 wants to regulate. But if Topic 4's answer is "no policy"
  (see below), then sprint close just... closes. Two BLs stay open
  across the boundary, which is the normal state of a backlog anyway.
- **Discoverability**: keeping them in v0.8.0 means `ls .ae/backlog/v0.8.0/`
  shows them. `/ae:roadmap remove` back to `unscheduled/` actively
  REDUCES discoverability (into a bigger directory). The supposed
  cleanup makes things harder to find.
- **Future re-filing cost**: if a trigger fires, an item in
  `v0.8.0/` is easier to re-assign than one buried in `unscheduled/`.

The only argument for action is cosmetic: "the sprint frontmatter
looks cleaner if all items are closed." That's not a concrete cost;
that's aesthetic tidying, which is exactly the "policy ceremony
dressed up as housekeeping" the topic warned about.

**One qualifier**: if `/ae:roadmap close v0.8.0` actively REFUSES to
close with open items (check the skill spec), then the minimum action
becomes `/ae:roadmap remove` on both — forced by tooling, not by
policy preference. I don't know the skill's current close behavior.
This is my one open question.

**Reject**: closing as "superseded-by-trigger" — introduces a new
status convention (none exists in mengdie). Policy ceremony for a
2-item problem.

**Reject**: `/ae:roadmap remove` as default — the `--reason` requirement
is already a trivial paperwork tax; skipping it by not removing is
strictly cheaper.

### Topic 3: Smallest hardening-action set

Apply "only fix what was asked" to each of the 5 actions in
BL-verify-decay-script-hardening:

1. **Binary preflight** (action 1) — **SHIP**. "Already partially done;
   clarify the 'proceeding anyway' branch." This is a 3-line change
   to an existing check. Pure finish-what-you-started.

2. **DB path param `--db-path`** (action 2) — **SHIP**. Single flag,
   single default. Takes <10 lines of shell. The motivation (silent
   wrong-DB validation) is a real correctness problem TODAY, not a
   future concern. Trivial and justified.

3. **RUST_LOG normalization** (action 3) — **SHIP**. One line:
   `env RUST_LOG=info mengdie dream ...` at the subprocess call site.
   The motivation is "the script silently fails when operator env has
   RUST_LOG=warn." That IS a silent failure today, not hypothetical.
   Trivial fix, real current problem.

4. **CI coverage** (action 4) — **SHIP** but with constrained scope.
   The shell test should be the minimum that catches "JSON line
   regressed to log line with timestamp prefix" — the exact regression
   that already happened once in plan 013. NOT a full matrix of exit
   codes, NOT a parameterized test over threshold values. One positive
   case + one negative case (mocked non-JSON stderr) is enough. The
   BL body says "a shell test... or a cargo integration test" — pick
   shell test. Cargo integration test would reinvent the infrastructure.

   **Important**: this shares the test harness with BL-decay-json-schema-contract's
   stderr integration test. That's the bundling argument in Topic 1.

5. **Threshold-mode for daemon** (action 5) — **DEFER**. Paradigmatic
   premature abstraction.
   - BL-010 doesn't exist.
   - The `--threshold=N` flag's semantics depend on BL-010's daemon
     model, which is undesigned.
   - Shipping action 5 now creates dead code in the script AND
     commits us to a flag shape we might regret when BL-010 actually
     happens.
   - The BL's own trigger list says "BL-010 daemon work starts
     (mandatory)" — the mandatory trigger is exactly action 5's
     precondition. Action 5 should move.

**Minimum set**: actions 1, 2, 3, 4. BL-verify-decay-script-hardening
closes as "4 of 5 shipped, action 5 awaits BL-010" — OR action 5
splits out into its own new BL (`BL-verify-decay-threshold-mode`)
filed under `unscheduled/` with the BL-010-starts trigger. I prefer
the split: it cleanly closes the v0.8.0 BL without an "almost-done"
asterisk.

**On Topic 3 sub-question "cheaper form of action 5"**: the framing
asks "is there a cheaper form that gives BL-010 a hook?" The honest
answer is **no** — the cheapest hook is "nothing, because BL-010
will touch this script anyway." Don't pre-shape a script for a
design that hasn't happened.

### Topic 4: Smallest policy (probably: none)

**Position: no policy. Case-by-case per Topic 2.**

A policy is an abstraction over data points. We have 2 data points
(the 2 defer-until-trigger items currently in v0.8.0). Drawing a
policy from 2 points is premature abstraction by exactly the same
standard as BL-010 threshold-mode.

The claimed value of a policy:
- **Admission gate for v0.9.0**: a pre-check that filters BLs whose
  body matches "not now" / "deferred" / "filed for trigger."
  - **Cost**: someone maintains the heuristic regex or tag scheme.
    False positives reject legitimate sprint items; false negatives
    let trigger-gated items through anyway.
  - **Benefit**: avoids repeating the current situation.
  - **But**: when Kai plans v0.9.0, he'll read each BL body anyway
    (the sprint plan is ~10 items, not 1000). The "heuristic" is
    already his eyes. Automating it adds maintenance without
    automating a bottleneck.

- **Close-state allowance**: permit `/ae:roadmap close` with
  "explicitly deferred" items.
  - If the skill already allows this, no change needed.
  - If the skill rejects this, fix the workflow for v0.8.0 (Topic 2's
    forced-remove branch), don't codify it into a policy.

The second-order cost of "policy must be maintained" IS larger than
the first-order cost of "case-by-case on 2 items, one decision" —
because the case-by-case decision is already trivial (Topic 2: do
nothing). A trivial first-order cost always beats a maintained
second-order abstraction.

**The minimum policy**: Kai reads each BL body before including it in
a sprint. That's not a policy, that's normal work, and it already
happens implicitly. Codifying it would be ceremony.

**One exception I'd accept**: add a single line to the
`/ae:roadmap plan` skill's instructions — "if a BL body contains the
phrase 'NOT pursuing this now' or 'filed for trigger', confirm with
the user before including." That's a linting hint, not a policy, and
it costs ~2 minutes to add once. But even that is borderline — it
presumes a future where Kai forgets what he already decided. I'd
rather leave it out.

**Strong position**: none.

## Scope-creep risks identified

1. **Bundle expansion in Topic 1**: the topic's own key question asks
   "should the bundle also include `release.yml` follow-ups...?" That
   question should be deflected, not answered. `release.yml` has no
   dependency on decay operator surface.

2. **New status convention in Topic 2**: "close as superseded-by-trigger"
   introduces a status that doesn't exist in mengdie today. Inventing
   status enums to solve a 2-item labeling problem is textbook scope
   creep.

3. **Action 5 in Topic 3**: pre-building a flag for a daemon that
   doesn't exist. Classic premature abstraction — ships code to
   support a design that might never exist in the shape assumed.

4. **Policy in Topic 4**: generalizing from n=2. An admission-gate
   heuristic would require regex maintenance, false-positive
   handling, and occasional review — all for a problem that solves
   itself when Kai reads the BL body.

5. **Discussion ceremony itself**: Topic 2 and Topic 4 are two topics
   for what is arguably one decision ("what do we do with these 2
   items"). Splitting into Topic 2 (tactical) and Topic 4 (policy)
   structurally invites the premature-generalization answer. The
   minimum-change position is that Topic 4 reduces to "no" and can be
   closed quickly, freeing Round 2 energy for Topic 1 and Topic 3.

6. **Integration test over-engineering**: both the schema-contract
   BL and verify-decay-hardening action 4 call for integration tests.
   Risk is that the test harness grows parameter matrices ("test all
   valid schema versions", "test all threshold values"). One happy
   path + one regression case per BL is sufficient.

## Open Questions

1. **What does `/ae:roadmap close` actually do with open items?**
   This is load-bearing for Topic 2's "do nothing" position. If the
   skill REFUSES to close v0.8.0 while open items exist, then my
   "do nothing" answer is wrong and the minimum becomes
   `/ae:roadmap remove` forced by tooling. Would like Codex or
   another agent with the skill docs in front of them to answer.

2. **Does `format_structured_json` ALREADY emit a bare JSON line, or
   does it still wrap in `tracing::info!`?** The schema-contract BL
   references a post-ship fixup commit `32e11ef` — I want to confirm
   current state before agreeing the integration test is the minimum
   diff. If the bare-JSON format is already shipped, the test is
   pure defense against regression. If it's not, the test has to
   co-ship with the format change, which affects the plan shape.

3. **Is there prior art in mengdie for "shell test with CI hook"?**
   If not, action 4's "CI coverage" is more infrastructure than the
   action description suggests. Would change my scope judgment — it's
   still in-scope, but the infrastructure cost should be named
   upfront in the plan.

4. **Split vs bundle for ops-doc-polish (Topic 1)**: my "split" answer
   assumes docs-only plans have lower review overhead. If this
   project's `/ae:plan` + `/ae:work` cycle has fixed overhead
   regardless of content (e.g., 4-agent review runs unconditionally),
   then the split math flips and bundling wins. Would like someone
   with recent `/ae:work` history to calibrate.
