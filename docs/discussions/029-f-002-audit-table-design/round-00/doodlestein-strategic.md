---
agent: doodlestein-strategic
verdict: REVISE
timestamp: 2026-04-28T16:39:43Z
---

# Doodlestein-strategic framing review verdict

Verdict: REVISE. Three concrete issues found.

1. **Topics 1 and 4 collapse into one.** Topic 1 (hook placement for coverage) and Topic 4 (failure contract for audit-write) are entangled: if the hook lives at a single choke point (Db::memory_search), failure semantics are decided once. If the hook must cover two sites (hybrid path + FTS fallback in mcp_tools.rs), Topic 4's answer potentially differs per site. The framing treats them as parallel independent choices, which means a researcher can resolve Topic 4 without first settling Topic 1, producing an incoherent result. Recommend: merge into one topic "audit hook placement and failure contract," with placement as the prerequisite question.

2. **FK enforcement state is a missing prior question inside Topic 2.** Topic 2 asks about ON DELETE policy given that PRAGMA foreign_keys is currently OFF — but buries the more fundamental question: does F-002 turn PRAGMA foreign_keys = ON (even per-connection), or leave it OFF globally? If enforcement stays OFF, the ON DELETE declaration is documentation-only and the cascade/restrict debate is moot at runtime. If F-002 turns it ON, it risks breaking rename_project's DELETE FROM memory_entries pattern. The framing allows a researcher to recommend RESTRICT without confronting that RESTRICT is a no-op unless enforcement is ON. This should be the explicit first question in Topic 2, not a sub-assumption.

3. **Upstream acceptance asymmetry: supersession SQL locked but its consumer (read path) is deferred.** The framing treats the supersession SQL from analysis.md as settled contract (out-of-scope to re-litigate), while Topic 5 asks whether v0.0.1 needs a read path at all. If Topic 5 resolves to "no read path until A-MEM trigger," the SQL has no v0.0.1 consumer and locking it as contract now adds a constraint without value. Recommend: add a sequencing note in Scope that Topic 5 is decided before the SQL becomes binding; if no read path in v0.0.1, the SQL is provisional (not contract).
