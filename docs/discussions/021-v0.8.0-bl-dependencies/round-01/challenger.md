---
agent: challenger
round: 1
timestamp: 2026-04-23
---

# Challenger — Round 1

## Findings (per topic)

### Topic 1: What's the hidden assumption?

The analysis says "shipping separately would duplicate plan/review cycle
overhead" and concludes bundling is cheaper. The hidden assumption is that
**plan overhead is the dominant cost**, not review-depth degradation.

Plan 014 is the counter-evidence in this same codebase: 90 LOC of code
changes + 3 config files → 4 CI iterations. That overhead was partly due to
CI environment debugging, not plan complexity per se. The analysis implicitly
assumes Plan A's bundle (cli.rs + verify-decay.sh + ops doc + integration
test) won't suffer the same dynamic — but there's no basis for that
confidence.

The actual question is: what degrades in a bundled 5-6 file plan vs. three
1-2 file plans? A 4-agent review covering M+M+S simultaneously has to spread
attention across: (a) a JSON schema correctness question, (b) shell script
robustness, (c) documentation accuracy. These are three different skill
domains. Splitting ops-doc-polish (pure docs, no code) from the other two
would let docs review land fast while code review focuses on the two
code-touching BLs.

The framing accepted "bundle = cheaper" without asking: **cheaper in what
unit?** Fewer plan invocations, yes. Fewer review errors caught, possibly.
That trade-off is not explored.

### Topic 2: What options got foreclosed?

The three options presented — (a) leave in sprint, (b) remove to
unscheduled, (c) close as superseded-by-trigger — all treat the BL as an
atomic unit: either it's in the sprint or it's not.

**Foreclosed option 4**: convert to a conditional BL with automatic
promotion. Instead of removing or leaving, mark the BL with a machine-
readable `trigger:` field and a `promote_to: v0.9.0` stanza. When the
trigger fires, the item auto-promotes. This preserves sprint hygiene (v0.8.0
closes clean), preserves the BL body (no information loss), and creates a
traceable promotion path instead of a manual re-file.

The analysis didn't ask whether the existing `/ae:roadmap` tooling supports
or could support this — it jumped straight to the three options that fit
current tooling. That's a constraint, not a given.

A second foreclosed angle: **why were these items scheduled in the first
place?** The v0.8.0 roadmap frontmatter says `committed_at: 2026-04-22`.
Both BL bodies already said "not now" at that date. If the admission error is
obvious in retrospect, the question is not "what do we do now" but "what gate
failed." The topic options all paper over that question.

### Topic 3: What risk is being understated?

The BL body says action 5 (threshold-mode) "paves the way for BL-010." But
BL-010 design doesn't exist. No discussion, no framing, no interface spec.

This is precisely the "first-caller rework" anti-pattern that retrospects in
this project have flagged: you stub an interface based on a vague future
requirement, then when the real design arrives, the stub's assumptions are
wrong and you rewrite anyway. The stub was net negative — it created a
maintenance surface with no current consumers, and it anchored BL-010's
design to the stub's semantics instead of the other way around.

The topic's key question acknowledges this risk ("is action 5 in-scope?")
but the framing's only pushback is procedural ("BL-010 internals are out of
scope"). That leaves action 5's inclusion as a live option when it should
probably be a default-no with an explicit reversal argument required.

The more important understated risk: if action 5 ships and BL-010 makes
incompatible interface assumptions, the script must be modified twice — once
now, once when BL-010 lands. That's more total work than deferring action 5,
not less. The analysis treats "paves the way" as net-positive without costing
the rework risk.

### Topic 4: Is this a real problem or ceremony?

This is ceremony. The honest answer is that this topic exists because
doodlestein-strategic flagged the process signal in Round 0 and it got
elevated to a full topic. The underlying problem — two BLs in a sprint whose
bodies already said "not now" — has a one-time fix (Topic 2) and doesn't
require a policy.

The cost-benefit here: solo dev, ~7-item sprints, sprint cadence is roughly
one sprint per 2-3 weeks. A sprint-commitment policy has a fixed reading cost
every time `/ae:roadmap plan` is invoked. That cost is real and recurring.
The failure mode it prevents (accidentally scheduling "defer-until-trigger"
BLs) happened once and is obvious in hindsight. The same outcome — noticing
the mismatched BLs before commit — would be achieved by a single sentence in
the `ae:roadmap plan` checklist, not a standalone policy document.

The framing asks "is a project-level policy actually justified, or is
case-by-case sufficient?" The answer is almost certainly case-by-case, and
adding policy machinery here is the kind of meta-work that slows a solo
project without adding velocity.

## Missing Topics / Blind Spots

**Missing: Sprint gate coherence after item removal.**

The v0.8.0 gate says: "All 7 review-originated follow-ups closed, CI runs
cargo clippy + cargo test on PR." If Topic 2 removes the 2 defer-until-
trigger items, the gate's "All 7" count becomes incoherent — the sprint
committed to 7 but now closes with 5. Does the gate update? If not, the
close condition is vacuously true (removed items can't be "closed"). If yes,
who updates it and what's the new statement?

This is a concrete consistency problem, not a hypothetical. None of the
topics address it.

**Missing: Whether Plan B (synthesis) is actually v0.8.0.**

The analysis recommends Plan B (synthesis dedup-key + provenance) "after
Plan A lands." But the synthesis cluster is independent — no cross-cluster
deps. Plan B could happen in v0.9.0 just as easily. The framing treats
synthesis as a v0.8.0 commitment, but the roadmap's gate says "no open P2
residuals from plans 010/012 reviews" — if those synthesis BLs are P3 or
lower, the gate could close without them. Nobody verified the priority
classification against the close gate.

## Open Questions

1. Does bundling (Topic 1) actually save net reviewer-hours, or does it just
   save plan invocation count? These are different optimization targets.

2. If 2 items are removed from v0.8.0 (Topic 2), does the sprint gate text
   update, and who owns that update?

3. Action 5 in Topic 3: what's the default — in or out? If "in" requires
   justification, the discussion should say so explicitly rather than leaving
   it as an open option.

4. For Topic 4: if a one-sentence checklist item in `/ae:roadmap plan` would
   prevent the same failure, why is a full policy topic warranted?

5. Are the synthesis BLs (dedup-key + provenance) actually gated by the
   v0.8.0 close criteria, or could they slip to v0.9.0 without affecting the
   gate?
