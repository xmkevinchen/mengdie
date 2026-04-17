## Step 1 — Content hash dedup + source_file optional + score normalization (commit: 299b4e6)
**Decisions**: 
- Used `#[serde(default)]` on `String` instead of `Option<String>` for source_file — avoids nullable JSON schema, keeps DB column NOT NULL with empty string default
- Backfill content_hash in Rust code (not SQL) since SQLite lacks built-in SHA-256
- `INSERT ... ON CONFLICT DO UPDATE ... RETURNING id` for atomic upsert — replaces SELECT+conditional INSERT/UPDATE (fixes TOCTOU from backlog 002-13)

**Rejected**:
- `Option<String>` for source_file — schemars generates `{"type": ["string", "null"]}` which some MCP clients may not handle
- Keeping raw RRF scores — raw values ~0.01-0.03 make `min_score` filter unusable for callers

**Cross-step deps**:
- `compute_content_hash()` in schema.rs is pub — used by db.rs, available for future callers
- Normalized scores (0-1) now flow to callers — Steps 3-6 depend on this for quality gates
- Schema v2 migration must complete before any data import (Step 3)

**Actual files**: Cargo.toml, Cargo.lock, src/core/schema.rs, src/core/db.rs, src/core/search.rs, src/core/mcp_tools.rs

## Step 2 — CLI list + dry-run (commit: 3b1690e)
**Decisions**:
- Dry-run returns before loading embedding model (~90MB) — fast preview
- Table format with right-aligned numeric columns, title truncation at 40 chars
- JSON format includes all fields needed for AC3 verification

**Rejected**:
- Dynamic column sizing — overkill for CLI tool, fixed widths sufficient

**Cross-step deps**:
- `list_memories()` in db.rs available for future callers
- `mengdie list` and `mengdie import --dry-run` used in Step 3 validation

**Actual files**: src/bin/cli.rs, src/core/db.rs
