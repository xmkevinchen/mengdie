---
agent: architect
round: 1
created: 2026-04-23
---

# Architect — Round 1 Findings

## Findings (per topic, with file:line evidence)

### Topic 1: Bundle boundary

**Position: Keep the 3-BL decay bundle. Do not split ops-doc-polish.**

Evidence for bundling all three:

- `cli.rs format_structured_json` is touched by BOTH `BL-decay-json-schema-contract`
  (adds `schema_version` field) AND `BL-decay-ops-doc-polish` (arrow regex/ASCII
  fallback in the format string). Same function, same file — two separate plans
  would produce a conflicting diff or force artificial sequencing.
  See analysis.md:70–74 ("same-file work") and topic-01-bundle-boundary/summary.md:53–55.

- The hard dependency (schema-contract → verify-decay test) and the soft dependency
  (schema-contract → ops-doc rollback referencing `breaches[]`) are both resolved
  within one plan by ordering the steps internally. No inter-plan coordination needed.
  See analysis.md:49–57.

- Plan 014 overhead observation: "plan/review overhead is non-trivial (plan 014 ran
  4 CI iterations for 90 LOC of code changes + 3 file config)" — topic-01/summary.md:51–52.
  The 3 BLs collectively touch 4 files + 1 new schema doc + 1 integration test. That's
  a well-bounded scope for a single review cycle. Splitting into 3 plans would triple
  the overhead for what amounts to one coherent "harden the decay operator surface" unit.

**On splitting ops-doc-polish specifically**: the argument for splitting would be that
ops-doc-polish is "pure docs" with lower review stakes. However, this is incorrect:

1. ops-doc-polish has a format-string change in `cli.rs` (arrow/ASCII fallback — item 2
   in BL-decay-ops-doc-polish:44–48). That's a code change, not pure docs.
2. The rollback procedure (item 3) references `breaches[]` which is defined by the
   schema contract. Writing the rollback section without the schema being locked first
   risks documenting a field layout that the schema contract then changes.
3. ops-doc-polish is S-sized. Splitting an S out of an M+M+S bundle saves roughly
   zero plan cycles (it generates its own plan + review overhead equivalent to what
   it saves).

**Verdict**: 3-BL bundle ships as one plan. ops-doc-polish stays in.

---

### Topic 2: Defer-trigger items

**Position: `/ae:roadmap remove` both. Do not leave in v0.8.0. Do not close as "superseded".**

Evidence:

- `BL-decay-dreaming-pass-optim` body (lines 68–71): "NOT pursuing this now. The
  current implementation's clarity is worth more than the premature optimization at
  current scale. This backlog item exists so the trigger is recorded."
  None of its 3 triggers are v0.8.0 concerns (corpus > 50k, p95 > 1s, BL-010 daemon).
  Current corpus is 323 memories / ~41 long-term — analysis.md:85–90.

- `BL-synthesis-preload-db-miss-edge` body (lines 53–61): "Until any of these trigger,
  the edge is documented in `src/core/dreaming.rs`." Its triggers require a `mengdie
  delete` subcommand (doesn't exist) or observed arithmetic mismatch (never observed).

**Why remove over close-as-superseded**: "superseded" is a resolution status implying
the item's concern was addressed by something else. These items' concerns are NOT
addressed — they remain valid concerns with explicit trigger conditions. Closing as
"superseded-by-trigger" would be semantically wrong. `/ae:roadmap remove` returns
the BL file to `unscheduled/` intact, preserving full body text including the trigger
conditions. The BL is still discoverable when the trigger fires.

**Why remove over leave-in-sprint**: leaving them open in v0.8.0 creates ambiguity
at sprint close. The v0.8.0 gate (roadmap:28–34) says "All 4 BL-decay-* items closed"
and "All 3 BL-synthesis-* items closed" — technically these count. Leaving them
creates a close-blocking or a forced wontfix-close that loses the trigger semantics.
topic-02/summary.md:52–56 identifies this as the crux.

**Symmetry**: both items have identical "filed for trigger" character. Handle identically.
No reason for asymmetry between the decay and synthesis items.

**Action**: `/ae:roadmap remove BL-decay-dreaming-pass-optim --reason "body says not-now;
triggers are corpus>50k, p95>1s, or BL-010 daemon — none are v0.8.0 scope"` and
same for `BL-synthesis-preload-db-miss-edge`.

---

### Topic 3: Hardening scope

**Position: Actions 1–4 ship in v0.8.0. Action 5 (threshold-mode) defers.**

Evidence per action:

**Actions 1–3 (binary preflight, DB path param, RUST_LOG normalization)**: all fix
environmental fragility that exists TODAY, independently of BL-010. The trigger
"first operator-reported issue" is more likely than BL-010 daemon landing. These
are unconditionally useful.
See BL-verify-decay-script-hardening.md:41–53.

**Action 4 (CI coverage)**: the trigger "CI pipeline gets a full-clippy+test stage"
fired — plan 014 delivered exactly this (roadmap notes 2026-04-22). The dependency
is satisfied. CI coverage for the script belongs in v0.8.0 alongside the code that
the CI is supposed to cover. Shipping the hardening without the CI test is the
original defect (BL-decay-json-schema-contract:23–28, Challenger C1 HIGH).

**Action 5 (threshold-mode for daemon)**: explicitly flagged as "paves the way for
BL-010" in the BL body (line 59). BL-010 daemon design is not v0.8.0 scope (framing.md:67–68:
"BL-010 daemon internal design (Out)"). Shipping `--threshold=N` now creates dead
code with no caller and with semantics tied to an unspecified daemon design.
topic-03/summary.md:57–59: "coupling a v0.8.0 ship to BL-010's (unknown) design is risk."

**On "cheaper form of action 5"**: the topic-03 summary asks whether a hook without
committed threshold semantics could be cheaper. My position: no. A `--threshold=N`
flag with no daemon to call it and no defined semantics for what N means in daemon
context is pure speculation. Better to let BL-010's design dictate the interface
than to pre-bake an interface that may need revision. The script is simple enough
that adding `--threshold=N` when BL-010 lands is a 10-line change.

**BL close state**: ship actions 1–4 as the v0.8.0 plan. When shipped, close
`BL-verify-decay-script-hardening` with a note "action 5 (threshold-mode) not
included; re-file as BL-decay-threshold-mode targeting BL-010 sprint." This is
cleaner than leaving the BL open with "4 of 5 shipped" — a partially-shipped BL
is ambiguous at sprint close.

---

### Topic 4: Sprint-commitment policy

**Position: `/ae:roadmap remove` handles the v0.8.0 case. Light documentation of
the "defer-until-trigger" exclusion pattern is sufficient. No automated gate needed.**

Reasoning:

The doodlestein-strategic reframe asks whether a process policy is warranted
(framing.md:42–44: "commitment-semantics question may be higher-leverage"). The
pattern here is specific: both items were committed to v0.8.0 despite their bodies
already containing explicit "not now" language. That's an admission-gate failure.

But the corrective cost analysis matters for a solo-dev project. The constraint is
explicit at topic-04/summary.md:53: "Mengdie is a solo-dev project — policy overhead
must justify itself."

**What actually happened**: both BL bodies had "not now" language BEFORE v0.8.0
committed them. The roadmap's gate language (roadmap:32–34) says "All 4 BL-decay-*
closed" and "All 3 BL-synthesis-*" — it's counting by prefix pattern, not by BL
trigger-readiness. That's the actual gap.

**Two options for correction**:

A. Automated admission gate in `/ae:roadmap plan`: scan body for language patterns
   ("NOT pursuing", "filed for trigger", "defer until"). This is tooling work with
   false-positive risk (e.g., a BL that says "defer until Round 2 of this discussion"
   wouldn't mean "don't commit").

B. Lightweight convention: when writing a BL that explicitly says "not now",
   add `admission_status: defer-until-trigger` to the YAML frontmatter. The
   `/ae:roadmap plan` command can then filter on this field. Zero ambiguity, zero
   NLP parsing.

**Verdict**: Option B is the right future policy, but implementing it requires an
AE plugin change (out of this discussion's scope per framing.md:46–50). For now:

1. `/ae:roadmap remove` both items (Topic 2 handles the immediate case).
2. Document the pattern in the sprint's Notes section: "items with body-language
   'not now' / 'filed for trigger' should not be committed unless the trigger is
   expected to fire within the sprint."
3. File a lightweight backlog item in AE's upstream backlog for the
   `admission_status: defer-until-trigger` frontmatter convention. This is an AE
   plugin feature, not a mengdie feature.

**On close-state semantics (sub-question 1)**: v0.8.0 should NOT be allowed to close
while any item remains "open" without explicit resolution. The cleanest sequence is:
remove both items first (Topic 2 action), then the remaining items are all actively
workable, and the sprint closes cleanly when they're done.

---

## Open Questions

1. **`BL-decay-ops-doc-polish` arrow/ASCII fallback**: the BL says "commit to one and
   update the AC" or emit both. Which choice does the plan make? This affects whether
   the ops-doc plan changes cli.rs or just the docs. The decision should be made in the
   plan step, not deferred.

2. **`BL-synthesis-provenance` option commitment**: analysis.md:61–65 flags this as
   unresolved — "discussion needed to resolve" which of the 4 options provenance
   picks before a plan can be written. Is there a pending discussion for this, or
   does it need a mini-discuss before `/ae:plan` for synthesis?

3. **BL close note for action 5**: if we close `BL-verify-decay-script-hardening`
   with a note about action 5, where does action 5's trigger get re-recorded? In a
   new BL filed immediately, or in a note on the BL-010 discussion when it opens?
   Leaving it in comments in the closed BL risks it being overlooked.
