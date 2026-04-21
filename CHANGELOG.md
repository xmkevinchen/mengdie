# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

### Added
- Exponential decay for Dreaming (BL-008, plan 013). Formula:
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
