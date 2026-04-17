---
round: 01
date: 2026-04-17
score: converged
---

# Round 01 — CI scope and triggers

## Discussion

### Architect (Round 1)
Proposed single workflow `ci.yml` separate from `release.yml`, triggering on `push` to main/feature branches + `pull_request`. Single serial job: `cargo build --release → cargo test → cargo clippy -- -D warnings → cargo fmt --check`. Rationale: solo dev, one runner — parallel jobs add scheduler overhead with zero throughput benefit. Cache `~/.cargo/{registry,git}`, incremental `target/deps/`, and `~/.cache/fastembed` (highest value due to ~90MB ONNX model). `cargo audit` defers to v2.

### Rust-archaeologist (Round 1)
Ground truth: existing `.forgejo/workflows/release.yml` has a `test:` job but only on tag push. No push/PR CI. No clippy anywhere in CI. No cache. No toolchain pin. Cargo.toml has no `[features]`, no optional deps — clean single compile unit, no matrix explosion. Tests: 2 `#[ignore]` integration files (`e2e.rs` fastembed, `llm_claude_cli.rs` claude CLI). 3 `#[cfg(unix)]` tests in `llm.rs` run fine on Linux.

### Codex (Round 1)
Forgejo-specific: DEFAULT_ACTIONS_URL resolves bare `actions/*` via `https://data.forgejo.org/`. `actions-rs/*` is archived (2023). Marketplace actions: use explicit full URLs for future-proofing. Proposed skeleton with `container: image: rust:1.90-bookworm` + `rustup component add` + `actions/cache@v4` for fastembed model. Cache key should hash both `Cargo.lock` AND `rust-toolchain.toml`.

### Round 2 tensions resolved

**Runner environment**: TL noted memory/project_infra.md says runner is HOST MODE (no Docker). Container syntax unavailable. Architect and Codex both agreed to drop `container:` and use host Rust (`source ~/.cargo/env` — matches release.yml's proven pattern).

**Actions URL syntax**: Archaeologist confirmed release.yml uses bare `actions/checkout@v4` (works today). Codex suggested full URLs for future-proofing. TL judgment: keep bare for consistency with release.yml; revisit if DEFAULT_ACTIONS_URL ever changes. Reversibility: high.

## Outcome

- Score: **converged**
- **Decision**: Create `.forgejo/workflows/ci.yml` separate from release.yml. Triggers: `push` (all branches except tags) + `pull_request`. Single serial job with steps: `cargo fmt --all -- --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`. Host Rust (runner's pre-installed, pinned via `rust-toolchain.toml`). Cache `~/.cargo/registry`, `~/.cargo/git`, `target/`, `~/.cache/fastembed` keyed on `hashFiles('**/Cargo.lock', 'rust-toolchain.toml')`. Bare `actions/*` syntax matching release.yml. No `cargo audit` in v1 (deferred). No pre-commit hooks (out of scope).
- **Rationale**: Converges three lenses: architect (fail-fast serial, host-mode constraints), archaeologist (verified ground truth — no existing push-time CI, bare action syntax works), codex (cache-fastembed is highest-value, `-D warnings` is Rust CI norm).
- **Reversibility**: high (single YAML file, no stateful migration)
- **Reversibility basis**: splitting jobs later, adding audit, switching to full-URL action refs, or adding pre-commit hooks are all purely additive changes
