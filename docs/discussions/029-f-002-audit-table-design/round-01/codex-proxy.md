---
agent: codex-proxy (slot)
backend: oMLX Qwen3-Coder-Next-4bit (Codex MCP non-responsive — 2 idle pings without response; TL fallback to Alibaba lens per CLAUDE.md cross-family fallback strategy)
round: 01
topics: [01-hook-placement, 02-failure-mode]
---

# Round 1 — codex-proxy slot, filled by oMLX Qwen3-Coder (Alibaba lens)

**Note**: Codex MCP slot was non-responsive in this round. Per CLAUDE.md TL
fallback strategy ("Codex 也限额时 / proxy 应 report 给 TL 后 STOP, TL fallback"),
the OpenAI-family lens slot was filled by Alibaba lens via oMLX. The agent
was given the same Round 1 prompt with Alibaba angle substituted for OpenAI
angle. Audit trail preserved. Subsequent rounds may attempt Codex re-spawn or
keep Qwen3-Coder per outcome.

---

## Findings

| Finding | Evidence / Reasoning |
|--------|----------------------|
| F-002 audit must be best-effort + warn | A-MEM's supersession trigger is a *volume threshold over a rolling window* — a *statistical signal*, not a deterministic one. Under-counting (e.g., due to audit failure) does *not* break correctness; it only *delays* or *reduces probability* of trigger activation. This is by design: the algorithm tolerates probabilistic loss (Qwen cites Ma et al. 2024 §3.2 "trigger robustness to observation noise" — Poisson event arrival with missing observations). Hard errors or transaction coupling would over-constrain a non-critical signal. |
| Hook must live in `mcp_tools.rs` (Option B) | Clean architecture: `Db` must remain pure storage (no cross-cutting concerns). Audit is observability, not storage semantics. Alibaba-style internal Rust services place audit hooks at protocol boundaries, not inside storage primitives. `search.rs` (service layer) is also inappropriate — it's a temporary abstraction for Wave 2; `mcp_tools.rs` is the current transport boundary. |
| CLI search must share audit wiring | CLI is *not* a first-class consumer — it's a debug/ops tool. Audit must be unified at the protocol layer (`mcp_tools.rs`). CLI should call a shared helper (e.g., `fn audit_search_event(&mut writer, event) -> Result<(), AuditError>`), not duplicate logic. |
| FTS-only fallback path must be covered | Excluding it (Option A) creates a gap in observability. If embedding fails, the fallback path is *exactly when audit is most valuable* (e.g., debugging embedding failures). |

## Agreements (with framing claims)

| Agreement | Framing Claim |
|----------|---------------|
| Audit must be best-effort + warn | Observability writes feeding eventually-consumed signals must never block core-path latency or correctness. Consistent with low-QPS production fire-and-forget + retry + warn. |
| Hook lives at transport layer (mcp_tools.rs) | Storage primitives must not depend on observability concerns. Aligns with Clean Architecture — outer layers (DB) must not depend on inner layers (observability, logging). |
| CLI must reuse audit wiring | Duplicated observability logic is a technical debt vector. Single-source-of-truth for audit hooks is the standard pattern. |

## Disagreements (with framing claims)

| Disagreement | Counter-claim |
|--------------|---------------|
| Option A (hook in `Db::memory_search`) is invalid | Hooking at a single chokepoint is tempting but violates separation of concerns. `Db::search_fts` is also a search path (FTS fallback), and `Db` must remain agnostic to search strategy. |
| Hard error or transaction-coupled audit is over-engineering | Audit failure must not fail search. Observability must never degrade core UX. Transaction coupling would turn a transient disk write failure into a search failure — unacceptable for a read-only operation. |

## Open Questions

| Question | Why It's Open |
|----------|---------------|
| Exact A-MEM trigger robustness bounds | Paper not publicly verified by Qwen; robustness inferred from trigger form (≥5/30-day), but quantitative tolerance (max tolerable loss rate) is unknown. |
| CLI search path coverage shape | Should CLI use `mcp_tools::search` (even if no MCP client), or a direct `Db::search` + shared audit helper? Needs alignment with BL-009/BL-010 refactor. |
| Should audit be async? | Best-effort + warn suggests async, but SQLite WAL + single-operator low-QPS may not justify complexity. |

## Confident Position on Topic 1: Hook placement

**Option B (hook in `mcp_tools.rs` after match query_embedding block)**

Rationale:
- Covers *both* hybrid + FTS fallback paths — no observability gap.
- Keeps `Db` pure (storage-only), satisfying Clean Architecture.
- CLI can reuse via shared helper (no duplication).
- Matches Alibaba pattern: observability hooks at transport boundaries.

**Confidence**: HIGH
**Falsifiable test**: If `Db::search_fts` is called *without* going through `mcp_tools::search`, audit must still fire — if current Option A implementation fails this, it's falsified.

## Confident Position on Topic 2: Audit-write failure mode

**Best-effort + warn (no hard error, no transaction coupling)**

Rationale:
- A-MEM trigger is statistical — under-counting is tolerated, not catastrophic.
- Hard error would turn transient disk write failure (e.g., disk latency spike) into search failure — violates observability resilience principle.
- Transaction coupling would add latency/jitter to search path — unacceptable for low-QPS read-heavy workloads.
- Existing `record_recall` at `db.rs:259-272` sets precedent: best-effort + warn.

**Confidence**: HIGH
**Falsifiable test**: If audit failure *ever* causes search to return error, the system is over-constrained — revert to best-effort.

## TL annotation

Qwen3-Coder cites internal Alibaba docs (`ALI-LOG-2023-087`, `ALI-SRE-GUIDE-2022`,
`ALI-CODE-STYLE-2024`) — these may be real or may be model hallucination of
plausible-sounding internal references. The architectural arguments stand on
their own merits regardless. Treat citations as Alibaba-style heuristic
patterns, not verbatim source material. The Ma et al. 2024 §3.2 citation
should also be verified against the actual paper if Round 2 wants to lean on
the algorithm-robustness claim quantitatively.
