---
id: BL-ae-work-closes-parent-discussion
status: open
origin: /ae:discuss 014 topic-01 + Doodlestein-strategic
created: 2026-04-18
scope: upstream (AE plugin, not mengdie)
---

# AE `/ae:work` should close parent discussion status on completion

## Finding

When `/ae:work` finishes a plan (all checkboxes ticked, review passed,
plan `status: done`), the parent discussion's `status` and
`pipeline.work` fields do NOT get updated automatically. They stay
`active` and `pending` respectively.

Observed consequence in mengdie: multiple discussions ended up with
`status: active` and `pipeline.work: pending` (or `work: done` but
`status: active`) even after all their spawned plans shipped. Examples:

- discussion 002 (MVP Phase 1) — work: done but in the 2026-04-16 audit was `status: active, work: pending`
- discussion 016 (Dreaming Evolution) — plans 007, 009, 010 all shipped; index.md stayed `status: active`
- discussion 017 (CI + Lint) — plan 008 `reviewed` with pending steps; index stayed `status: active`

The progress-audit cleanup (this discussion) added an advisory rule to
CLAUDE.md: "When /ae:work completes a plan, update the discussion
`index.md` status + pipeline fields in the same commit." But CLAUDE.md is
advisory documentation — agents running /ae:work mid-session don't
re-read CLAUDE.md at completion time.

Doodlestein-strategic flagged this during the progress-audit Doodlestein
pass. The real fix is in the AE plugin skill definition itself.

## Trigger

Fires when:

1. The AE plugin (upstream at
   `/Users/ckai/Workspace/Projects/agentic-engineering/plugins/ae/skills/work/SKILL.md`)
   is next edited for any reason — bundle this fix in.
2. A new mengdie plan ships and — despite the CLAUDE.md advisory — its
   parent discussion's frontmatter is NOT updated in the completion commit.
   Concrete signal: `/ae:dashboard` shows a "phantom active" discussion
   for a feature that's already shipped.
3. A second project adopts the AE pipeline and hits the same drift
   pattern — signal that the skill needs the fix, not just documentation.

## Fix direction

Add to `plugins/ae/skills/work/SKILL.md` under Step 8 (Commit / finalize):

```
8c. After updating plan status to `done`, update the parent discussion's
    `index.md`:
    - Set pipeline.work: done
    - If all pipeline stages are done, set status: done
    - If plan is part of multi-plan discussion (multiple plans share a
      parent), list all plans in the `plan:` field or append to an array
    Include this update in the same commit as the plan status flip.
```

Plus: `/ae:work` Step 8 pre-commit check should refuse if the parent
discussion hasn't been updated (unless `no-discussion-link` is explicit
in plan frontmatter).

## Why this is an upstream (AE plugin) item, not mengdie

The mengdie repo cannot directly edit the AE plugin's skill definitions
(those live in the external `agentic-engineering` repo). The
mengdie-side CLAUDE.md rule is the best we can do here; the real
enforcement must land upstream. File this backlog entry in mengdie to
preserve the resume signal; when touching the AE plugin upstream, apply
the fix there.
