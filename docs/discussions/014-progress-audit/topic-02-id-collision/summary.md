---
id: "02"
title: "ID collision — 003-memory-credibility and 003-tech-stack share id: 003"
status: converged
current_round: 1
created: 2026-04-18
decision: "Leave as-is. Do not rename directory, do not delete. Add a one-line note to 003-memory-credibility/index.md explaining the historical mismatch. Frontmatter id is authoritative; directory name is cosmetic."
rationale: "Archaeologist verified zero external references to the 003-memory-credibility directory outside 014-progress-audit itself. Challenger: rename or delete is churn for no user-visible benefit. Architect's Option C (delete) rejected as over-aggressive given zero cost to keep. Renaming creates commit noise; deleting loses the Phase 2+ concept sketch."
reversibility: "high — can rename or delete later if the mismatch ever surfaces in a real tool"
reversibility_basis: "Non-action is fully reversible; the mismatch has existed for 2+ weeks with no downstream effect observed."
---

# Topic: ID collision — two discussions share `id: "003"`

## Current Status

No collision — fixed in commit 17cb083 (2026-04-15). Directory name / frontmatter id mismatch retained. Round 1 team: not worth fixing; add a header note for future readers.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | Leave as-is, annotate with one-line note about historical renumber |

## Context

The audit (2026-04-16) called out this as an id collision requiring renumber-to-015 OR sub-id. A grep of frontmatter shows the renumber has already happened in the frontmatter — but the DIRECTORY NAME is still `003-memory-credibility`. All references to this discussion (in backlog files, for instance) use the directory name.

## Constraints

- Directory renames break all relative links (other discussions, backlog items, retrospect files).
- Discussion is `status: deferred` (pinned as Phase 2+ concept). No active work depends on its directory name.
- Other discussions have id == directory-number (e.g., directory `002-mvp-phase1` has `id: "002"`). This is the only exception.

## Key Questions

1. Is the current state (frontmatter `id: 015` but directory `003-memory-credibility`) actually a problem? Do any tools consume the directory name as an ID?
2. If renaming — options: (a) rename directory to `015-memory-credibility` and fix references, (b) leave as-is and add a CLAUDE.md footnote, (c) delete the directory entirely and file to backlog.
3. Is this worth fixing at all, given the discussion is deferred / unlikely-to-activate?
