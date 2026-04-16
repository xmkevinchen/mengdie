---
id: "03"
title: "Post-Fix Validation Protocol"
status: converged
current_round: 1
created: 2026-04-16
decision: "Fix MCP permissions → apply code fixes → run Dreaming → 2-week forced-use validation with manual scorecard + 1 counterfactual"
rationale: "MCP permissions blocker confirmed (not in allow list). Circular validation broken by real-project sessions. Codex stripped 4-week protocol to minimal 2-week version."
reversibility: "high"
reversibility_basis: "Process protocol, not code — adjustable mid-validation"
---

# Topic: Post-Fix Validation Protocol

## Current Status
Pending — analysis confirmed existing validation is circular (Mengdie tested on itself). Need to define what non-circular validation looks like.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
After fixing RRF normalization (Topic 01) and FTS5 tokenization (Topic 02), the project needs a validation protocol before entering "use it" mode. The core loop claim ("AI knowledge → better AI output → richer knowledge → spiral upward") has step 4 ("better output") completely unverified.

Challenger flagged: all existing validation ran ae:analyze on Mengdie itself. The project that ingests AI knowledge about itself was tested with AI knowledge about itself. This is circular.

Product-strategy recommends: "forced-use mode — route all AE pipeline research through Mengdie for 2 weeks."
Codex recommends: target metric "10 real AE sessions/week invoke Mengdie, 70% produce useful top-3."

## Constraints
- Kai is sole user — no external testers
- Mengdie is registered in Claude Code MCP — it's already "in the loop" for any project in the same Claude Code session
- The AE repo may not be on this machine — write side of loop potentially disconnected
- Dreaming needs organic search data to calibrate — validation period is also data collection
- 46 memories exist across 3 projects — enough for basic recall testing

## Key Questions
- What does non-circular validation look like concretely? (Which project, what queries, what "success" means)
- Should there be a structured eval (scorecard) or just organic usage with friction logging?
- What's the minimum validation that's convincing before building more features?
- How long should the validation period be before deciding "use it" works or doesn't?
