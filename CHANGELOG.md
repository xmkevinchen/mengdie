# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

## v0.8.0 — 2026-04-24

Theme: Decay + Synthesis Hardening + CI unblock. Review-originated
follow-ups from plans 010/012/013 closed, CI expanded past
`cargo fmt`-only. 7 of 9 committed items shipped (2 descoped mid-sprint
to `unscheduled/` when their triggers did not fire).

### Added
- **Exponential decay for Dreaming** (BL-008, plan 013). Formula:
  `effective_relevance = avg_relevance × 2^(-days_since_last_recalled / 60)`
  with half-life of 60 days and a demotion floor of 0.20. Long-term
  memories whose effective relevance falls below the floor have their
  `is_longterm` flag cleared; stored `avg_relevance` is never mutated.
  The same decay multiplier is applied at search time as a post-fetch
  re-rank so stale memories rank lower before the next Dreaming pass
  demotes them. Adds `mengdie dream --decay-dry-run` for operator
  preview and 4 new counters + a `breached_ids` list on `DreamingResult`.
  Structured-JSON event emitted on stderr per pass for machine
  consumers. Operator procedure:
  [`docs/operations/dreaming-decay.md`](docs/operations/dreaming-decay.md).
  Design record: [discussion 019](docs/discussions/019-power-law-decay/conclusion.md).
- **Decay structured-event schema + verify-decay hardening**
  (BL-decay-json-schema-contract + BL-verify-decay-script-hardening,
  plan 015): locked the structured-JSON shape of the decay dry-run
  event so machine consumers can rely on it; hardened
  `scripts/verify-decay.sh` with an explicit approval gate.
- **Decay operations doc polish** (BL-decay-ops-doc-polish, plan 016):
  rewrote `docs/operations/dreaming-decay.md` — added Rollback section,
  plan 013 AC5 post-ship correction (stored avg_relevance is never
  mutated), doc-SQL drift-guard test.
- **Synthesis cluster-hash dedup** (BL-synthesis-dedup-key, plan 017):
  replaced unstable `content_hash` dedup key for synthesis rows with a
  new `synthesis_cluster_hash` column derived from sorted+deduped
  source IDs. Re-synthesis of the same cluster now UPSERTs the
  existing row instead of producing a zombie sibling. Schema bumped
  v4→v5 with 4 pre-checks, transactional migration via
  `execute_batch`, CHECK-via-trigger on `source_type` allowlist,
  PRAGMA integrity_check. `idx_memory_content_hash` is now partial
  (excludes synthesis rows).
- **Synthesis audit subcommand** (BL-synthesis-provenance, plan 017):
  `mengdie synthesis-audit <id>` prints a synthesis row alongside its
  source memories for operator fidelity spot-checks. Graceful
  placeholder for hard-deleted source memories.
- **Surface source_type in search + list** (BL-synthesis-provenance
  option 4 reinterpreted, plan 017): `mengdie search` + `mengdie list`
  output now include a `type:` column distinct from the `source:` file
  path so operators can visually distinguish syntheses from primary
  sources.

### Changed
- **CI runner env unlocked** (006-ci-runner-env-cleanup +
  BL-ci-full-clippy-test, plan 014): fixed the `.cargo/config.toml`
  `-isysroot` CFLAGS leak blocking the Forgejo runner; expanded
  `ci.yml` from `cargo fmt --check` only to full fmt + clippy + test +
  cross-check jobs. Extracted `Embed` trait + `MockEmbedder` so the
  pipeline test suite runs on any CPU without loading the fastembed
  ORT runtime (works around the Ivy Bridge runner's AVX2 SIGILL).

### Descoped (moved to `unscheduled/`, trigger not fired)
- BL-decay-dreaming-pass-optim: premature at current corpus size.
- BL-synthesis-preload-db-miss-edge: depends on a `mengdie delete` /
  `memory_invalidate` CLI subcommand that does not exist.

