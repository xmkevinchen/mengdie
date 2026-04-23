# Round 3 — Codex-Proxy Response

**Reasoning effort**: low
**Model**: Codex (OpenAI-family)
**Date**: 2026-04-22

---

## Root Cause Mechanism Confirmation

Cargo reads `.cargo/config.toml` from the repo root (and parent directories via directory walk) and injects `[env]` vars into every process Cargo spawns: build scripts, rustc invocations, clippy-driver, and test binaries. This is documented behavior [Cargo config reference](https://doc.rust-lang.org/cargo/reference/config.html#env).

The global `CFLAGS = "-isysroot /Applications/Xcode.app/..."` in the repo's `.cargo/config.toml` (added 2026-04-16, commit af303d5) is injected into all build-script subprocesses. This bypasses shell-level env unsets entirely because Cargo injects **after** shell command construction.

Key point: `[env]` does not override an already-present env var unless `force = true`. So `unset CFLAGS` in the workflow does not suppress the Cargo-injected value; the Cargo-level injection wins.

---

## Recommended Fix Form

**Best fix: delete the global `CFLAGS` entirely** — unless every Cargo build in this repo (Apple targets, Linux targets, everywhere) requires the macOS SDK flag.

If the flag is genuinely needed for Apple targets only, scope it via cc-rs target-specific env vars:

```toml
[env]
CFLAGS_x86_64_apple_darwin = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
CFLAGS_aarch64_apple_darwin = "-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
```

`cc-rs` uses target-suffixed vars first before falling back to plain `CFLAGS` [cc-rs configuration](https://docs.rs/cc/latest/cc/#external-configuration-via-environment-variables).

Cargo does **not** support `[target.<triple>.env]` syntax. The `[target.<triple>]` table supports `linker`, `runner`, `rustflags`, `rustdocflags`, and `[target.<triple>.<links>]` for build-script overrides only [Cargo target config](https://doc.rust-lang.org/cargo/reference/config.html#target).

---

## Sanity-Check Without Full CI

Run locally on macOS:

```sh
rtk cargo check -vv --target x86_64-unknown-linux-gnu
```

Then search the output for:

```
-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk
```

If that string is absent, the leak is fixed.

Caveat: `cargo check --target x86_64-unknown-linux-gnu` may still fail if a native C dependency requires a Linux C compiler (unrelated cross-toolchain issue). That's acceptable for this sanity check as long as the `-isysroot` flag is absent from the output.
