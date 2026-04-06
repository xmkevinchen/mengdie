# Step 6: End-to-End Loop Validation Results

Date: 2026-04-05
Plan: docs/plans/002-close-the-loop.md

## Phase A: analyze write→read loop (5+ sessions)

| Session | Topic | Memories Ingested | Prior Findings Surfaced | AI Referenced |
|---------|-------|-------------------|------------------------|---------------|
| 006 SQLite Concurrency | Arc<Mutex>, tokio, spawn_blocking | 3 | 0 (first session) | N/A |
| 007 Embedding Models | MiniLM quality, download progress bug, model provenance | 3 | 3 (from 005, 006) | Yes — cited in Prior Art |
| 008 Contradiction Detection | Resolution workflow missing, entity-tag FP, valid_from dead code | 2 | 5 (from 005, 006, 007) | Yes — cited in Prior Art |
| 009 Dreaming/Promotion | is_longterm disconnected from search, recall inflation | 2 | 5 (from 005, 006, 007, 008) | Yes — cited in Prior Art |
| 010 Cross-Project Sharing | FTS5 IDF contamination, missing global tier | 2 | 5 (from 005, 007, 008, 009) | Yes — cited in Prior Art |

**Aggregate**: 12 memories ingested. 4/5 sessions surfaced prior findings (session 1 was first, expected 0). Recall counts reached 5 on earliest finding (RRF analysis from 005). **Gate: PASS (4/5 > 3/5 threshold).**

## Phase B: discuss→analyze cross-loop

| Step | Action | Result |
|------|--------|--------|
| ae:discuss 008 | "Should mengdie add memory_resolve_conflict?" | Decision: no new tool, 4 fixes. Memory ingested as decisional (id: 99bc8ea4) |
| mengdie list | Verify ingestion | `99bc8ea4 [discuss]: No new memory_resolve_conf... decisional 0 N` ✓ |
| ae:analyze 011 | "MCP tool API design patterns" (related topic) | Step 3.5 memory_search returned discuss decision as **rank #1** (score 0.5) |
| Analysis 011 Prior Art | Cited in synthesis | "[discuss]: No new memory_resolve_conflict tool" cited as first prior art item ✓ |

**Loop confirmed**: discuss writes → analyze reads → knowledge spiral demonstrated. **Gate: PASS.**

## Conflict Detection Test

| Step | Action | Result |
|------|--------|--------|
| Seed | Ingested conflicting memory: "should add memory_resolve_conflict tool" with same entities as discuss 008 | entry_id: 8b98fe33 |
| Detect | memory_ingest returned conflicts | `[{ id: "99bc8ea4", reason: "evolution candidate (similarity: 0.85)", title: "[discuss]: No new memory_resolve_conflict tool..." }]` |
| Verify | EvolutionCandidate fired (both decisional, entity overlap, cosine 0.85 > 0.7 threshold) | ✓ |
| Cleanup | Invalidated test memory with reason | ✓ |

**AE skill protocol**: Updated to emit conflict summary in closing output. Format: `Knowledge capture: N items ingested, conflicts detected with: [titles]`. **Gate: PASS.**

## Search Quality Observations

- Normalized RRF scores range 0.47-0.50 across all sessions (consistent)
- FTS5 + vector hybrid effectively retrieves related content across sessions
- Compound entity tags (protocol fix) eliminated false positive conflicts in analysis 011 (0 conflicts on 2 ingestions)
- Prior single-word tags produced false positives in analyses 007-010 (before protocol fix)

## Issues Found and Fixed During Validation

| Issue | Found In | Fixed |
|-------|----------|-------|
| `image-models` Cargo feature unused | 007 | b59fbe0 |
| `show_download_progress(true)` corrupts MCP stdio | 007 | b59fbe0 |
| `is_longterm` not wired into search | 009 | b59fbe0 |
| `record_recall` received boosted score (circular amplification) | code review | 5a4b542 |
| Entity tags too broad → false positive conflicts | 008 observed | AE skill update d661118 |
| `reason` field silently dropped in memory_invalidate | 008 discuss | 64f13fc |
| `superseded_by` write-only (not in outputs) | 008 discuss | 64f13fc |
| `insert_memory_resolving` missing SQLite transaction | code review | 64f13fc |

## Overall Verdict

**All gates passed. The knowledge loop is closed.**

- Write side: AE skills (analyze, discuss) ingest knowledge via memory_ingest ✓
- Read side: AE skills surface prior knowledge via memory_search in Step 3.5 ✓
- Cross-loop: discuss decisions are retrievable by subsequent analyze sessions ✓
- Conflict detection: contradictory ingestions correctly flagged ✓
- Knowledge accumulation: recall_count tracks usage, compound tags reduce noise ✓
