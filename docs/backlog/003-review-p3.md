---
id: "003"
title: "Deep review P3 findings"
status: open
created: 2026-04-05
tags: [review, p3, deferred]
---

# Deep Review P3 Findings

From /ae:review of Plan 001 (MVP Phase 1).

### P3-1: Path traversal via symlink in parser
- **Source**: Security reviewer
- **Issue**: `parse_ae_file` reads path from watcher without verifying it's under expected base directory. Symlink attack could read arbitrary files.
- **Trigger**: When watcher is integrated into daemon (Phase 2).

### P3-2: No enum validation for source_type/knowledge_type
- **Source**: Security reviewer
- **Issue**: Arbitrary strings accepted and stored. Can corrupt search ranking.
- **Trigger**: When unexpected values cause Dreaming/contradiction misbehavior.

### P3-3: User input may appear in tracing logs
- **Source**: Security reviewer
- **Issue**: Error messages transitively include query text. Log aggregators would see user content.
- **Trigger**: When logs are shipped to external service.

### P3-4: E2E test missing #[ignore] for CI
- **Source**: Architecture reviewer
- **Issue**: `test_full_pipeline` downloads 90MB model. Will fail in offline CI or slow down CI.
- **Trigger**: When CI pipeline is set up.

### P3-5: Dead snippet variable in FTS fallback path
- **Source**: Architecture reviewer
- **Issue**: `mcp_tools.rs` FTS fallback constructs unused snippet variable.
- **Trigger**: Next touch of FTS fallback code.

### P3-6: Hand-rolled walkdir doesn't handle symlink cycles
- **Source**: Architecture reviewer
- **Issue**: `cli.rs` recursive walk follows symlinks potentially infinitely.
- **Trigger**: When import is run on directories with symlink cycles.
