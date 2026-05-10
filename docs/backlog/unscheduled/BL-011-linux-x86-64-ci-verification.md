---
id: BL-011
title: "Linux x86_64 CI verification of sqlite-vec static-link claim (PASS_STATIC transfer)"
type: backlog
created: 2026-05-09
admission_status: defer-until-trigger
trigger: "Forgejo CI runner provisioned for Linux x86_64 builds (currently no Linux runner exists; mengdie CI is macOS-only on Forgejo)"
related: [F-006, F-001]
source: F-001 spike `docs/spikes/sqlite-vec-compat.md` + F-006 spike `docs/spikes/sqlite-vec-distribution.md` (PASS_STATIC verdict on macOS arm64) + F-006 plan Step 7
---

# BL-011: Linux x86_64 CI verification of sqlite-vec static-link claim

## What

Verify that the **PASS_STATIC** verdict from the F-006 sqlite-vec distribution spike (`docs/spikes/sqlite-vec-distribution.md`, 2026-05-08; macOS arm64) transfers to Linux x86_64. F-001 spike `docs/spikes/sqlite-vec-compat.md` filed this verification as a follow-up but never ran it because no Linux CI runner existed at the time. F-006 ships sqlite-vec adoption to mainline relying on the macOS-only PASS_STATIC verdict; this BL closes the cross-platform verification gap when Linux CI becomes available.

## Why it matters

The sqlite-vec adoption (F-006, commit `8b15146`) added a runtime dependency on the sqlite-vec extension being statically linked into the mengdie binary. The static-link mechanism is structural (cc-rs builds sqlite-vec C source from the bundled crate; no `.dylib` / `.so` to link against at runtime) — it should be platform-independent in mechanism — but has only been **empirically confirmed on macOS arm64**.

If Linux x86_64 builds turn out to require dynamic-library linking (e.g., due to a linker-flag difference in the cc-rs build script, or a glibc/musl quirk), users on Linux would hit a runtime `no such module: vec0` error on first DB open and mengdie would refuse to start. This is the worst-case failure mode F-006 explicitly does not currently catch.

## Why deferred

No Forgejo CI runner exists for Linux x86_64 today (verified per `project_infra.md` Mengdie memory + spike doc inline note). Filing this as `defer-until-trigger` means we surface the gap explicitly without blocking F-006 ship on infrastructure that doesn't exist.

## Trigger condition

Move this BL to a sprint when the Forgejo CI runner is provisioned for Linux x86_64 (or when Linux x86_64 GitHub Actions / equivalent is set up for the project — any CI environment that gives Linux x86_64 build coverage).

## Pass criteria

When this BL is picked up, verification is:

1. **Static-link confirmation**: run `cargo build --release` on Linux x86_64; verify the produced binary has no dynamic dependency on a sqlite-vec shared library. Equivalent of macOS spike's `otool -L` check is `ldd target/release/mengdie | grep -i vec` returning empty (no `libsqlite_vec.so` or similar).
2. **Symbol presence**: `nm target/release/mengdie | grep sqlite3_vec_init` returns a non-empty match (the symbol is present in the static text section, as on macOS).
3. **Runtime tests pass**: full `cargo test --release` passes on the Linux x86_64 runner. Specifically, `core::vector::*` tests + `core::schema::*` v7 migration tests must pass — these exercise the vec0 module loading path through both auto-extension registration (`db.rs::ensure_sqlite_vec_registered`) and the schema-v7 migration's `CREATE VIRTUAL TABLE … USING vec0(…)`.
4. **No `cargo build` warnings about dynamic linking**: the build output must not contain `-lvec` or similar dynamic-link flags. The cc-rs build script is supposed to compile sqlite-vec C source directly into the binary; any indirect-link path would be a regression vs the macOS PASS_STATIC verdict.

If all four pass, mark BL-011 closed and amend `docs/spikes/sqlite-vec-distribution.md` outcome to `PASS_STATIC_CROSS_PLATFORM` (or whatever verbiage the spike author prefers); F-001 spike's follow-up obligation is then resolved.

If any step fails, BL-011 escalates to a real engineering issue (root-cause why static-link doesn't transfer; potentially blocks Linux distribution of mengdie).

## Out of scope

- Windows x86_64 verification — separate concern; mengdie is not currently distributed for Windows.
- Cross-compilation — verifying static-link specifically on a *native* Linux x86_64 build is the goal here. Cross-compilation from macOS is a different verification with its own caveats.
- Performance comparison — this BL is correctness/distribution-mechanism only. Performance benchmarking of vec0 on Linux is a separate topic.

## F-006 relationship

F-006's verdict does NOT depend on BL-011 closing. F-006 ships sqlite-vec adoption to mainline today (Phase 1 of the v0.0.1 OSS-replacement scope) on the strength of the macOS PASS_STATIC verdict. BL-011 is the post-ship verification gate filed for the future Linux CI sprint, NOT a blocker for F-006 ship. If BL-011 later fails (Linux turns out to need dynamic linking), the response would be a new F-007-shaped feature to fix the linkage on Linux — not a retroactive F-006 rollback.
