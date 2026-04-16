---
id: "005"
title: "Human-Readable Project Naming"
type: plan
created: 2026-04-16
status: reviewed
discussion: ""
---

# Feature: Human-Readable Project Naming

## Goal

Replace opaque hash-based project_id (`proj_5ec117a5e361ef6a`) with human-readable names, so memories survive git remote changes and are identifiable across projects.

## Prior Art

- `.mengdie.toml` config reading already implemented in `src/core/project.rs` (this session)
- `hash_project_id()` exported for backward compat
- Backlog 004-18: `--project-name` flag (trigger: multi-machine/monorepo) — this plan addresses the root cause

## Steps

### Step 1: Test .mengdie.toml support (AC1)
- [x] Add unit tests for `read_project_name`: found in current dir, found in ancestor, not found (fallback to hash), empty name ignored, malformed TOML handled (8a3bf60)
- [x] Add integration test: create temp dir with `.mengdie.toml`, verify `infer_project_id` returns the name (8a3bf60)
Expected files: `src/core/project.rs` (test module)

### Step 2: Add `rename_project` to Db (AC2)
- [x] `pub fn rename_project(&self, old_id: &str, new_id: &str) -> anyhow::Result<(usize, usize)>` returns (renamed, merged) (0de7f2e)
- [x] Implementation: SELECT collisions → DELETE old duplicates → UPDATE remaining (0de7f2e)
- [x] Log each merged entry (id + title) via tracing::info (0de7f2e)
- [x] Unit test: insert 3 memories, rename, verify all 3 updated (0de7f2e)
- [x] Unit test: collision case — old row deleted, new row preserved (0de7f2e)
Expected files: `src/core/db.rs`

### Step 3: Add `rename` CLI subcommand (AC3)
- [x] `mengdie rename <from> <to>` — calls `db.rename_project`, prints renamed + merged counts (d8305d7)
- [x] `mengdie rename --list` — shows all distinct project_ids with memory counts (d8305d7)
- [x] `--dry-run` flag — show what would happen without writing (d8305d7)
- [x] `--yes` flag — skip confirmation prompt (d8305d7)
- [x] Confirmation prompt with merge count (d8305d7)
- [x] Post-rename MCP restart warning (d8305d7)
Expected files: `src/bin/cli.rs`

### Step 4: Create .mengdie.toml + rename existing data (AC4)
- [x] Run `mengdie rename --list` to verify actual project_ids in DB
- [x] Create `.mengdie.toml` for both projects (mengdie + agentic-engineering)
- [x] Renamed proj_35a0a24ad8956900 → mengdie (32 memories), proj_879f9676c4a5a472 → agentic-engineering (30 memories)
- [x] Verified search returns results under named project_id
Expected files: `.mengdie.toml` (both projects, outside repo)

Note: Steps 1 and 2 are parallel-safe (no shared state). Step 3 depends on Step 2. Step 4 depends on Step 3 + binary rebuild.

## Acceptance Criteria

### AC1: .mengdie.toml Resolution
- `infer_project_id` returns name from `.mengdie.toml` when present
- Falls back to hash when `.mengdie.toml` absent
- Walks up to git root, stops there

### AC2: Rename DB Operation
- `rename_project("old", "new")` returns `RenameResult { renamed, merged }`
- Collisions (same content_hash under both project_ids) are merged: old duplicate deleted, new row preserved
- Non-colliding rows updated to new project_id
- FTS index stays consistent (triggers handle UPDATE automatically)

### AC3: Rename CLI
- `mengdie rename --list` shows project_id → count mapping
- `mengdie rename <from> <to>` prints renamed + merged counts
- `--dry-run` shows impact without writing
- `--yes` skips confirmation
- Post-rename warning about MCP server restart

### AC4: Live Migration
- Both projects (mengdie + AE) have `.mengdie.toml` with human-readable names
- `mengdie search` in each project dir returns results under the named project_id
- No `proj_*` hash entries remain in DB (merge handles duplicates)
