---
id: BL-038
title: "VEC_DIM compile-time consistency check across schema.rs (i64) and vector.rs (usize)"
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "any future change to schema.rs::VEC_DIM or vector.rs::VEC_DIM (single-source-of-truth migration would land here too); OR a fastembed model swap (the only realistic event that would change either constant)"
related: [F-006]
source: F-006 /ae:review (architect Q5-A)
---

# BL-038: compile-time enforce VEC_DIM consistency between schema.rs and vector.rs

## What

`VEC_DIM` is currently declared twice in the codebase:

- `src/core/schema.rs:15` — `pub const VEC_DIM: i64 = 384;` (binds to `rusqlite::params!` for SQLite INTEGER).
- `src/core/vector.rs:12` — `pub const VEC_DIM: usize = 384;` (used as a Rust slice index / length comparison).

The vector.rs doc comment at lines 7-11 explicitly says "MUST match schema.rs::VEC_DIM (single source of truth lives in schema.rs)" — but this is **aspirational**, not mechanical. Nothing prevents one from drifting out of sync with the other.

The type difference is load-bearing: schema.rs's i64 binds to SQLite parameters; vector.rs's usize is required for slice operations. They cannot share a single typed constant directly. The fix is a compile-time assertion that the two values agree.

## Why deferred

Current state is correct: both constants are 384, the system works. Drift would require an explicit edit to either file. At v0.0.1 personal-KB scale with one fastembed model (AllMiniLML6V2 → 384-d), the realistic drift trigger is a fastembed model swap (e.g., to a 768-d or 1024-d model), which is not on the immediate roadmap.

Filing as `defer-until-trigger` documents the latent risk without blocking F-006 close.

## Trigger condition

Move this BL to a sprint when ANY of:

- Any future commit modifies `schema.rs::VEC_DIM` or `vector.rs::VEC_DIM`. The act of modifying either constant should pull this BL into the same commit / sprint to ensure the consistency check is added before drift is possible.
- A fastembed model swap (Cargo.lock change to fastembed feature flags or a `model: AllMiniLML6V2` → `BGEBaseEnV15` change in code).
- Multi-model support introducing variable-dim embeddings (this would obsolete the single-VEC_DIM design entirely; the fix shape would change).

## Hint at fix shape

Add a compile-time assertion in either file (or in a new top-level `lib.rs` invariant block):

```rust
// At the bottom of src/core/vector.rs, or top-level lib.rs:
const _: () = {
    // Compile-time assert that schema.rs::VEC_DIM (i64) and vector.rs::VEC_DIM (usize) agree.
    // If a future edit drifts these constants, this fails at compile time, not at runtime.
    assert!(
        crate::core::schema::VEC_DIM == crate::core::vector::VEC_DIM as i64,
        "VEC_DIM mismatch between schema.rs and vector.rs"
    );
};
```

Note: `const fn` assertions in const context are stable since Rust 1.57 (mengdie pins Rust 1.95 per F-006 spike, so this is available). The assertion runs at compile time; failure prevents the build.

Alternative — single-source-of-truth refactor: declare only one `VEC_DIM_USIZE: usize = 384` and derive the i64 form via `VEC_DIM_USIZE as i64` at every i64 use site. This is cleaner long-term but requires touching every `params![..., VEC_DIM]` call site. Pick at sprint pickup.

## Out of scope

- Multi-model variable-dim support (separate concern; would require redesign of VEC_DIM as a constant).
- Refactoring `params!` call sites to use a derived i64 form (out of this BL's minimal-fix scope).

## F-006 relationship

F-006 close-out preserved the dual-VEC_DIM definition because it works correctly today. The architect's review flagged this as a latent risk; the fix is small but the trigger is sufficiently rare that filing-with-trigger is the right cost-aware response for v0.0.1.
