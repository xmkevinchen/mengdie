# Round 1 — Codex-Proxy Response

**Reasoning effort**: medium
**Model**: Codex (OpenAI-family)
**Date**: 2026-04-22

---

## 1. Verification Feasibility

Realistic cost: **30-60 minutes**. If CI queueing or cold Cargo downloads are slow, call it **90 minutes**. `dtruss` can easily turn this into a **2-3 hour detour**.

Minimum useful verification:

```sh
rtk which cc
rtk cc --version
rtk xcrun --find cc
rtk xcrun --show-sdk-path
rtk rustc -vV
rtk cargo tree -i ring
```

Then reproduce outside Forgejo/act first:

```sh
rtk rustup target add x86_64-unknown-linux-gnu
rtk env CC_ENABLE_DEBUG_OUTPUT=1 cargo build -vv --target x86_64-unknown-linux-gnu --locked
```

Also test whether inherited Xcode env is the whole problem:

```sh
rtk env -u SDKROOT -u DEVELOPER_DIR -u MACOSX_DEPLOYMENT_TARGET CC_ENABLE_DEBUG_OUTPUT=1 cargo build -vv --target x86_64-unknown-linux-gnu --locked
```

And ask Apple Clang what it would do without Cargo:

```sh
rtk cc -### -x c -c /dev/null --target=x86_64-unknown-linux-gnu
rtk cc -v -x c -E /dev/null --target=x86_64-unknown-linux-gnu
```

`CC_ENABLE_DEBUG_OUTPUT=1` is the key: `cc-rs` documents it as logging compiler invocations and exit codes, and it also documents target-specific `CC_x86_64_unknown_linux_gnu` / `AR_x86_64_unknown_linux_gnu` style overrides [cc-rs docs](https://docs.rs/cc/latest/cc/#external-configuration-via-environment-variables). `ring` uses `cc::Build` in its build script, so this is the right level to inspect [ring build.rs](https://docs.rs/crate/ring/latest/source/build.rs).

`strace`: not applicable on macOS. `dtruss`: technically possible with `sudo dtruss -f <command>`, but poor ROI for act-spawned builds. It uses DTrace and needs root; Apple's SIP docs state DTrace cannot inspect system processes, and SIP must be changed from Recovery OS for broader tracing [Apple SIP runtime protections](https://developer.apple.com/library/archive/documentation/Security/Conceptual/System_Integrity_Protection_Guide/RuntimeProtections/RuntimeProtections.html), [Apple SIP configuration](https://developer.apple.com/library/archive/documentation/Security/Conceptual/System_Integrity_Protection_Guide/ConfiguringSystemIntegrityProtection/ConfiguringSystemIntegrityProtection.html). Use `cc-rs` logging plus `cc -###` first.

## 2. Bypass Mechanism Ranking

### 1. Compiler replacement in current host runner: **best first move for solo-dev TCO**.

Variants:

```sh
rtk brew install zig
rtk cargo install --locked cargo-zigbuild
rtk rustup target add x86_64-unknown-linux-gnu
rtk cargo zigbuild --target x86_64-unknown-linux-gnu.2.28 --tests --no-run --locked
```

Or Linux GCC toolchain:

```sh
rtk brew tap messense/macos-cross-toolchains
rtk brew install x86_64-unknown-linux-gnu
rtk env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar cargo build --target x86_64-unknown-linux-gnu --locked
```

Time: **30-120 minutes**. Maintenance: low to medium. Exit cost: low. Failure modes: `cargo zigbuild` is mainly a build wrapper, not a general `cargo clippy` wrapper; Linux `cargo test --target ...` will compile Linux test binaries but cannot execute them on macOS without a runner; Zig glibc targeting has caveats, including static glibc not supported [cargo-zigbuild README](https://github.com/rust-cross/cargo-zigbuild). Extra C deps may need headers/libs because Zig uses `-nostdinc` for configured targets.

### 2. Docker executor on same Mac mini: **best if actual Linux `cargo test` execution matters**.

Forgejo supports `docker`, `lxc`, and `host` label types; host has no isolation, while docker runs steps inside the configured image [Forgejo runner config](https://forgejo.org/docs/next/admin/actions/configuration/).

Example label:

```yaml
runner:
  labels:
    - ubuntu-22.04:docker://ghcr.io/catthehacker/ubuntu:act-22.04
```

Time: **2-4 hours** if Docker/Colima already works; longer if not. Maintenance: medium. Exit cost: medium. Failure modes: Docker Desktop/Colima lifecycle, slower volume mounts, cache quirks, nested Docker access if workflows build images, and x86_64-on-Apple-Silicon emulation if the Mac mini is ARM.

### 3. External CI: **pragmatic fallback**.

Time: **1-3 hours** for a GitHub Actions mirror or similar. Maintenance: low. Exit cost: low-medium. Failure modes: external dependency, quota/availability, secrets policy, split source of truth, Forgejo status integration drift.

### 4. Runner relocation to Linux: **cleanest mechanics, highest ops burden**.

Time: **3-8 hours** for a Linux VM/mini box plus runner registration/cache/secrets. Maintenance: medium. Exit cost: medium-high. Failure modes: another host to patch, secure, back up, and monitor; loses direct macOS host coverage unless the Mac runner remains for macOS-specific jobs.

## 3. Missed Options

Yes: use **Zig directly as the target compiler/linker for normal Cargo subcommands**, without changing runner topology and without relying only on `cargo zigbuild`.

Create wrappers such as:

```sh
#!/bin/sh
exec zig cc -target x86_64-linux-gnu "$@"
```

Then configure Cargo/cc-rs:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "ci/zigcc-x86_64-linux-gnu"

[env]
CC_x86_64_unknown_linux_gnu = { value = "ci/zigcc-x86_64-linux-gnu", relative = true }
AR_x86_64_unknown_linux_gnu = { value = "ci/zigar", relative = true }
```

That lets these work in the existing host runner:

```sh
rtk cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings
rtk cargo test --target x86_64-unknown-linux-gnu --no-run --locked
```

Important limit: this gets **Linux compile coverage**, not Linux test execution. Cargo's `--no-run` explicitly compiles tests without running them [Cargo test docs](https://doc.rust-lang.org/cargo/commands/cargo-test.html). To run Linux test binaries, Cargo needs a target runner, and Cargo documents `target.<triple>.runner` / `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER` for that [Cargo config docs](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner). On macOS, a real Linux container/VM is still the practical answer for executing `x86_64-unknown-linux-gnu` tests.
