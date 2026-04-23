---
agent: challenger
round: 2
timestamp: 2026-04-23
---

# Challenger — Round 2

## New blind spot (lead item)

**`/ae:roadmap remove` partial-failure leaves the sprint in an inconsistent
state with no recovery path specified.**

The conclusion forming around Topic 2 is "remove both items." Both operations
are assumed to succeed atomically, but they're two separate commands. If the
first `/ae:roadmap remove BL-decay-dreaming-pass-optim` succeeds and the
second fails (tooling crash, bad --reason format, whatever), v0.8.0 has one
item removed and one still present. The sprint frontmatter is now asymmetric:
one "filed for trigger" item gone, one remaining. The remaining item still
blocks sprint-close under codex-proxy's reading (codex-proxy.md:31–32: "gate
is absolute: 'All 7 review-originated follow-ups closed'").

There is no recovery procedure: do you re-add the first item and retry both?
Do you leave the asymmetry? Can `/ae:roadmap add` even restore a previously-
removed item without re-filing from scratch?

This is not a hypothetical. The roadmap Notes section (v0.8.0.md:60–64)
already records one close-scope-delta that happened mid-plan (006 and
BL-ci-full-clippy-test both closed by plan 014). The precedent for roadmap
mutation during a sprint is established. The question is whether two mutations
in sequence have an atomic guarantee, and nothing in the discussion or the
roadmap skill documentation answers this.

**Recommendation**: before running `/ae:roadmap remove`, verify whether the
skill processes both removes in one invocation (atomic) or requires two
separate invocations. If two invocations, name the recovery path in the
decision so the executor isn't improvising.

---

## Topic 3 stress-test: shipping actions 2+4 only

The archaeologist's factual correction (archaeologist.md:lines 113–135)
confirms: actions 1 and 3 are already implemented. Action 1 is present at
`scripts/verify-decay.sh:35–38`; action 3 is `RUST_LOG="${RUST_LOG:-info}"`
at line 47. The effective work drops to actions 2 + 4.

**Failure mode in shipping action 2 (`--db-path` flag):**

Gemini-proxy raised a test isolation concern (gemini-proxy.md:lines 27–28).
I'll sharpen it: `--db-path` is a new flag on `scripts/verify-decay.sh`. When
action 4's CI test exercises the script, it will need to pass `--db-path` to
avoid hitting the operator's default `~/.mengdie/db.sqlite` (which may not
exist on the CI runner). This is not a problem IF action 2 and action 4 are
co-planned (they would share the test harness that also passes the flag).
But if someone reads the BL as "action 2 is trivial, action 4 is the
interesting part" and sequences them separately — even within one plan — the
test written for action 4 must be written *after* action 2 is committed, or
the test silently falls back to the default DB path and validates nothing.

The planning implication: the implementation order inside the plan must be
explicit — **action 2 before action 4**. This is a constraint that should
appear in the plan's step sequencing, not be left implicit. Nobody in Round 1
named this intra-plan ordering risk.

**On the BL close state:**

The emerging consensus (architect.md:lines 98–102, minimal-change-engineer.md:
lines 149–154) is to close `BL-verify-decay-script-hardening` after shipping
actions 2+4, re-filing action 5 as a new BL. I support this over "leave BL
open with asterisk." However, the close creates a documentation gap: the
original BL was filed from a specific review finding (BL body: "origin:
BL-008 /ae:review (challenger C2 MEDIUM-HIGH + gemini P3)"). If we close the
BL and file a new one for action 5, the new BL must carry forward the
original review citation. Otherwise the provenance trail breaks — future
readers won't know action 5 originated from the plan 013 adversarial review,
they'll see a bare BL with no origin. The re-filed BL should explicitly say
"split from BL-verify-decay-script-hardening, originally filed from plan 013
review" in its origin field.

---

## Topic 4: `admission_status: defer-until-trigger` — ceremony in disguise?

The architect-codex convergence (architect.md:lines 113–120, codex-proxy.md:
lines 54–59) proposes `admission_status: defer-until-trigger` frontmatter as
"lightweight" policy. Attack:

**It is not low-ceremony. It is medium-ceremony dressed as low-ceremony.**

The proposal has three parts: (a) a new field on new BLs, (b) a scan filter
in `/ae:roadmap plan`, and (c) implicit maintenance of the convention going
forward. Part (b) requires an AE plugin change (both architect.md:line 123
and framing.md:46–50 acknowledge this is out of scope). Part (c) requires
that every future BL author remembers to set the field — and that future
sprint planners remember to run the filter.

The minimal-change-engineer's position (minimal-change-engineer.md:lines 196–205)
is more honest: a single linting hint in the `/ae:roadmap plan` skill
instructions costs two minutes to add and never drifts, because it's a
human-readable prompt guard rather than a machine-enforced field. The
`admission_status` field only adds value if the tooling enforces it. Without
tooling enforcement, it's a convention that future sprints will skip — exactly
the "ghost planning" risk gemini-proxy named (gemini-proxy.md:lines 33–34).

**The actual value case for `admission_status`**: it would be useful if the
AE roadmap skill could surface a warning like "2 items have
`admission_status: defer-until-trigger` — confirm before committing." That's
a genuine UX improvement. But that requires AE skill work, which is out of
scope. Without it, the field is annotation noise.

**Verdict**: if architect + codex want to pursue `admission_status`, the
right action is to file an AE upstream backlog item for the skill change (as
architect.md:line 124 suggests) and do nothing in this discussion beyond
naming the pattern. Don't put the field on current BLs without the tooling
to enforce it — half-baked conventions are worse than no conventions.

---

## Topic 1 bundle shape: concrete review-depth evidence from plan 013

Plan 014's 4 CI iterations are the wrong comparison. Plan 014 had CI
*environment* debugging — the AVX2 SIGILL, the ORT dependency, the trait
extraction. That's a qualitatively different failure mode from review-depth
degradation on a bundled plan. CI churn from environment issues is not a
proxy for reviewer attention dilution.

The correct comparison is plan 013, which was a bundled multi-step feature
(5 steps: decay primitive + demotion pass + search re-rank + CLI flag + e2e
test). Plan 013's review archive (docs/plans/reviews/013-*.md) shows:

- `013-doodle-adversarial.md:verdict: conditional-pass` — found a blocker
  (DreamingResult missing `breached_ids` field) that required a plan change
  BEFORE `/ae:work`. This was a cross-step dependency that reviewers caught
  because the review covered all 5 steps together. If steps had been split
  into separate plans, the blocker would have been found at Step 5
  implementation time, not at plan review time. **Bundling caught the blocker
  earlier.**

- `013-architect.md:verdict: approved-with-notes` — found two structural
  concerns (helper placement and dry-run function divergence), both cross-step
  issues. Same pattern: a split plan would have missed these at review time.

This is the concrete evidence minimal-change-engineer asked for
(minimal-change-engineer.md:lines 270–272 "would like someone with recent
/ae:work history to calibrate"). Plan 013's review shows that bundled plans
catch cross-step dependency issues that split plans defer to implementation.
For the current 3-BL bundle, the hard dependency (schema-contract →
verify-decay test) is EXACTLY this class of issue — a bundled plan review
will catch test/contract misalignment before `/ae:work`; a split plan review
won't.

**Qualification**: this argues for bundling json-schema-contract +
verify-decay-hardening (the M+M with the hard dependency). It does NOT
automatically argue for also bundling ops-doc-polish. The minimal-change-
engineer's split (minimal-change-engineer.md:lines 46–49) is "M+M together,
S separately" — and the plan 013 evidence actually supports this shape, not
the full 3-BL bundle. The adversarial and architect reviewers caught issues
that spanned Steps 1–5 because those steps had tightly-coupled code
dependencies. ops-doc-polish's coupling to the other two BLs is a soft doc
reference — a class of dependency that reviewers can check without co-planning.

**Revised position**: 2-BL bundle (json-schema-contract + verify-decay-
hardening) is the evidence-supported shape. ops-doc-polish splits out as
an S plan. This aligns with minimal-change-engineer.md:47–49 and contradicts
architect.md's "keep all three."

---

## Summary of Round 2 positions

| Topic | Round 1 challenge | Round 2 update |
|-------|-------------------|----------------|
| T1 bundle | Hidden assumption: bundle ≠ fewer review errors | Plan 013 evidence supports M+M bundle, NOT 3-BL. Ops-doc-polish should split. |
| T2 defer | 4th option (conditional BL) foreclosed | New blind spot: partial-failure on two-step remove has no recovery path. |
| T3 action 5 | First-caller anti-pattern | Still correct. Additional: action 2 must precede action 4 in plan sequencing (--db-path needed by CI test). Re-filed BL must carry origin provenance. |
| T4 policy | Ceremony | `admission_status` is medium-ceremony without AE tooling enforcement. File as upstream AE BL; don't apply to current BLs. |
