---
id: "sqlite-vec-distribution"
type: spike
status: accepted
date: 2026-05-08
spike_for: BL-026
outcome: PASS_STATIC
relates_to:
  - docs/spikes/sqlite-vec-compat.md  # F-001 spike (2026-04-28; PASS_WITH_CONDITIONS)
  - docs/backlog/unscheduled/BL-026-sqlite-vec-adoption-replace-vector-rs.md
  - docs/backlog/unscheduled/BL-011-linux-x86-64-ci-verification.md  # follow-up filed by F-001
environment:
  rust: "1.95.0"
  sqlite_vec_version: "0.1.9"
  rusqlite_version: "0.39"
  macos: "26.4 (Tahoe; Darwin 25.4.0)"
  arch: "arm64"
---

# Spike: sqlite-vec distribution model — PASS_STATIC re-confirmation

This spike is **BL-026 Step 1** (the embedded compatibility gate). It
re-runs the F-001 spike against the current toolchain to confirm the
PASS_WITH_CONDITIONS verdict still holds on Rust 1.95 / macOS Tahoe
26.4 / current crate versions.

**Note**: this is partially redundant with F-001's earlier
`docs/spikes/sqlite-vec-compat.md` (2026-04-28). F-001 established the
core compatibility verdict and the `distance_metric=cosine` adoption
caveat. This spike adds a current-toolchain re-run and answers the
narrower distribution question (static vs dynamic linking) explicitly
called out in BL-026 step 1.

## Method

Fresh workspace at `/tmp/bl026-spike` (cleaned after spike — outputs
captured here):

```toml
# Cargo.toml
[dependencies]
rusqlite = { version = "0.39.0", features = ["bundled", "load_extension"] }
sqlite-vec = "0.1.9"
```

`.cargo/config.toml` mirrors mengdie's CFLAGS isysroot injection (per
discussion 020 — without this, cc fails to find `assert.h`/`stdio.h`
on macOS Tahoe SDK).

Two tests in `src/lib.rs`:

1. `vec_version_works` — register sqlite-vec via `sqlite3_auto_extension`,
   query `select vec_version()`.
2. `vec0_virtual_table_knn_top3` — create `vec_memories USING
   vec0(memory_id, embedding float[384])`, insert 10 deterministic
   unit-normalized vectors, KNN top-3 against a query identical to
   id=3.

Plus a binary `bl026-spike` for `otool -L` + `nm` symbol scan.

## Result

```
running 2 tests
sqlite-vec version: v0.1.9
test tests::vec_version_works ... ok
KNN top-3 for query ≈ id=3:
  id=3 distance=0.000000
  id=2 distance=0.053367
  id=4 distance=0.076422
test tests::vec0_virtual_table_knn_top3 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.00s
```

KNN ranking is correct (self-match is closest, distance ≈ 0).

### Distribution model — STATIC confirmed

```
$ otool -L target/release/bl026-spike
target/release/bl026-spike:
    /usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1356.0.0)

$ nm target/release/bl026-spike | grep -i 'vec_init\|sqlite3_vec'
000000010000c7ac T _sqlite3_vec_init

$ stat -f '%z' target/release/bl026-spike
2332112  # 2.22 MB
```

- **otool -L**: only `/usr/lib/libSystem.B.dylib` (system libc). No
  `sqlite-vec.dylib` / `libsqlite3.dylib` external dependency.
- **nm**: `_sqlite3_vec_init` is `T` (text section, defined locally) —
  statically linked into the Rust binary.
- **Binary size**: 2.22 MB — reasonable for static rusqlite (bundled
  SQLite ~1 MB) + sqlite-vec (~340 KB C source) + minimal Rust
  runtime.
- **At runtime**: `target/release/bl026-spike` prints
  `sqlite-vec linked statically: version=v0.1.9` with no separate
  install step on the host machine.

### Mechanism

`sqlite-vec` crate v0.1.9 ships a `build.rs`:

```rust
fn main() {
    cc::Build::new()
        .file("sqlite-vec.c")
        .define("SQLITE_CORE", None)
        .compile("sqlite_vec0");
}
```

The C source `sqlite-vec.c` (~340 KB, vendored in the crate) compiles
via `cc-rs` into a static library archive `libsqlite_vec0.a`, which
the Rust linker pulls into the final binary. `SQLITE_CORE` define
ensures the extension links against statically-bundled SQLite (from
rusqlite's bundled feature) rather than expecting a separate libsqlite3
at runtime.

This is the cleanest possible distribution: single-binary, no operator
`.dylib` install, no runtime `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH`
mucking.

## Verdict: **PASS_STATIC**

Per BL-026 step 1 outcome enum:
- ✅ PASS_STATIC — sqlite-vec compiles statically into the Rust binary
  via cc-rs build script. No external `.dylib` dependency.
- (PASS_DYNAMIC: not the case here.)
- (FAIL: not the case here.)

## Implications for BL-026 Step 2 (Adoption)

Step 2 (mengdie src/ integration) is unblocked. ~50 LoC change per
BL-026 plan. Required adaptations:

1. **Carry forward F-001 caveat** (per
   `docs/spikes/sqlite-vec-compat.md`): adoption MUST use
   `distance_metric=cosine` column-declaration override to match
   mengdie's existing cosine-similarity semantics:
   ```sql
   CREATE VIRTUAL TABLE vec_memories USING vec0(
       memory_id integer primary key,
       embedding float[384] distance_metric=cosine
   );
   ```
   Without override, default L2 distance produces ranking that is
   correct under rank-based RRF but semantically inconsistent with
   the rest of the codebase.

2. **Score conversion**: cosine distance `[0, 2]` → similarity
   `1.0 - distance / 2.0` for RRF merge consumption (or stay rank-
   based; current `src/core/search.rs` is rank-based RRF per F-001
   finding so adoption-time conversion may not be strictly needed,
   but explicit conversion is safer).

3. **Schema migration v7**: add `vec_memories` virtual table +
   populate from existing `memory_entries.embedding` BLOB rows. F-001
   AC6 enforced "no src/ modification regardless"; this spike does
   not modify src/ either (work in /tmp/). Step 2 begins src/ change.

4. **Linux x86_64 CI verification** (BL-011, filed by F-001): MUST
   pass before BL-026 closes. Same smoke binary on Forgejo runner
   `runs-on: ubuntu-latest`. Currently macOS arm64 only is verified.

## Follow-ups

- BL-026 Step 2 (Adoption) is **runnable**. Operator may proceed
  immediately or pause for separate planning cycle (~50 LoC + schema
  migration v7 + tests; ~1-2 hours).
- BL-011 (Linux CI verification) — runnable on Forgejo runner;
  blocking gate before BL-026 final close.
- v0.0.1.prd.md AC1 disposition: **resolved as PASS** for sqlite-vec
  spike side. BL-026 Step 2 integration is the next AC1-relevant work
  unit.
