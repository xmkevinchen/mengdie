---
id: BL-029
title: "Formalize supersession protocol — template + checklist"
status: open
created: 2026-05-08
origin: F-004 council review (codex-proxy + challenger)
trigger: "5+ document supersessions accumulated within a 6-month window"
size: S
v_target: vNext
---

# BL-029 — Formalize supersession protocol

## Origin

F-004 council review (2026-05-08) found that the supersession protocol
in `docs/blueprint.md §14` was established by example using 3 documents
in F-004:
- `docs/discussions/001-product-vision/` → `status: superseded`
- `docs/backlog/005-phase2-roadmap.md` → `status: superseded`
- `docs/prd-ae-integration.md` → `status: archived`

Two patterns (`superseded` vs `archived`) emerged inline. F-004 review
fixup #9 documented the 2-pattern distinction in §14, but the protocol
remains "by example" not "by template + checklist."

## Problem

If 5+ more documents need supersession over the next few months,
consistency will drift:
- Frontmatter field order may vary
- Reason wording may vary (long prose vs short tag)
- Body blockquote placement / format may vary
- `superseded_on:` vs `archived_on:` confusion when both apply
- Pattern selection rule ("which one?") relies on §14 prose, not a
  decision tree

## Proposed scope

Write `docs/conventions/supersession-protocol.md` containing:

1. **Decision tree**: when to use `superseded` vs `archived`
   - Successor exists fulfilling same role → `superseded`
   - Out-of-scope without direct replacement → `archived`
2. **Frontmatter templates** (copy-paste-ready):
   - Template A — superseded
   - Template B — archived
3. **Body blockquote templates** (copy-paste-ready):
   - Top-of-body markdown blockquote with successor link
4. **Checklist** for executing supersession:
   - [ ] Frontmatter updated with required fields
   - [ ] Top blockquote prepended to body
   - [ ] Body preserved (audit trail)
   - [ ] Cross-references in other docs updated (or noted as historical)
   - [ ] Commit message follows `docs(<scope>): supersede <doc> via <successor>` pattern

## Trigger

**Fires when**: 5+ documents in `docs/` carry `status: superseded` OR
`status: archived` frontmatter accumulated in a rolling 6-month window.

Mechanical check (operator runs occasionally):
```bash
rg -l '^status: (superseded|archived)' docs/ | wc -l
```

If count ≥ 5 with creation/supersession dates within 6 months → fire
this BL.

## Reversibility

High. Convention doc is non-binding until referenced; can adopt or drop
without affecting existing supersessions.

## Why deferred (not in F-004)

F-004 had 3 supersessions, which is enough to surface the 2-pattern
distinction but not enough to require formal template. Karpathy "wait
until 4th occurrence" applies. Re-evaluate at trigger.
