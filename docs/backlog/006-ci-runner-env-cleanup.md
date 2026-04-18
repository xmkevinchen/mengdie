---
id: "006"
title: "CI — expand ci.yml past `cargo fmt` once Forgejo runner CFLAGS leak is fixed"
status: open
created: 2026-04-17
source: "Plan 008 Step 3 partial deferral"
---

# Expand ci.yml past `cargo fmt` once runner env is fixed

## What ships in plan 008 Step 3 (partial)

`.forgejo/workflows/ci.yml` runs `cargo fmt --all -- --check` on push + PR. That's it. Full fmt/clippy/test CI was the original plan but had to be dropped after diagnosis hit a wall.

## What's blocked

On the Forgejo runner (`ckai-macmini.local`, Linux x86_64, host-mode forgejo-runner v6.3.1), the following FAILS inside every act-spawned job step:

```
cargo build  # or any cargo command that compiles `ring`
```

with:

```
error occurred in cc-rs: ... "cc" ... "-isysroot" "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk" ... curve25519.c
→ stdint.h: No such file or directory
```

## What I ruled out over ~30 min of hunting

- **Env leak via shell rc**: `~/.bashrc`, `~/.zshrc`, `~/.profile`, `~/.zshenv`, `/etc/environment`, `/etc/profile`, `/etc/zsh/*`, systemd user env — none export `CFLAGS` / `SDKROOT` / `MACOSX_*`.
- **Env leak via systemd unit**: forgejo-runner.service `Environment=` is empty beyond defaults.
- **Env leak via forgejo-runner config**: `/mnt/disk1/appdata/forgejo-runner/config.yaml` has only the stub `A_TEST_ENV_NAME_*` values. No `.env` file present.
- **act-spawned job env**: dumped via `env | sort` as the first run step in ci.yml (debug commit 4014ee3). Clean. No CFLAGS, no SDKROOT, no HOST_CC, nothing Darwin-flavored.
- **cargo config**: no `~/.cargo/config.toml`, no `.cargo/config.toml` in the repo, no `/etc/cargo/*`.
- **rustup override**: `rustup override list` says none. `rustup show active-toolchain` = `stable-x86_64-unknown-linux-gnu`.
- **PATH cc shim**: `which cc` = `/usr/bin/cc` (GCC 13.3). `~/.cargo/bin/` has no `cc`, no `gcc`, no `cpp` — just rustup/cargo/cargo-zigbuild entries.
- **cargo-zigbuild interference**: the zigbuild shims (`~/.cache/cargo-zigbuild/.../zigcc-aarch64-apple-darwin-*.sh`) exist ONLY for aarch64-apple-darwin target. They would only fire with `cargo zigbuild --target aarch64-apple-darwin`. They don't hook a plain linux `cargo build`.
- **cc-rs target detection**: cc-rs's `rerun-if-env-changed` lines show it watches `CFLAGS_x86_64_unknown_linux_gnu`, `HOST_CFLAGS`, etc. — Linux-targeted. So cc-rs knows the target is Linux. Yet it emits a Darwin `-isysroot`.
- **Explicit `CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu`**: set inside the run step (commit 4a12931), did not change the outcome.

## What reproduces AND what doesn't

- In the runner's user shell, directly via SSH: `cargo build` on a minimal crate that depends on `ring` → **succeeds**, builds clean.
- Inside an act-spawned run step (plain workflow, host mode): `cargo build` of the same minimal crate → **fails** with the `-isysroot` error.

So the problem is specific to act's subprocess context, despite act running `bash --noprofile --norc -e -o pipefail` and the bash env showing no Darwin vars.

## Triggers to resume

Any one of:
1. **Root-cause the act leak**: strace the act-spawned bash process to see what's setting the env, OR bisect act runner versions / configs. This was outside the patience window for plan 008.
2. **Replace the runner host**: move forgejo-runner to a clean VM / container where the Mac-dev shell environment doesn't exist to leak from.
3. **Run the existing ci.yml on a non-host-mode runner**: switch to docker mode for that workflow specifically (requires Docker on the runner, which memory/project_infra notes is currently absent).
4. **Explicitly wrap cc**: write a shell wrapper at `~/.cargo/bin/cc` that strips `-isysroot` before calling `/usr/bin/cc`. Hacky but effective. Only do this if root-cause hunting continues to fail.

When resumed: the reverted commits (e4b8cbf through 6658248) are in git history and can be cherry-picked for re-application once the env issue is fixed.

## What's in place meanwhile

- `ci.yml` = `cargo fmt --check` only. No C compilation → no ring leak.
- `release.yml` unchanged from its original shape (standalone `test:` + `build-linux:` jobs). The pre-existing race between those two is documented in plan 008 Step 3's "reverted" note but tolerated.
- `.githooks/pre-commit` enforces fmt + clippy locally. Solo-dev workflow relies on this as the primary gate.
