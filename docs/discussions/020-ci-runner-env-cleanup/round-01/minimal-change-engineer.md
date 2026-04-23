---
role: minimal-change-engineer
round: 1
discussion: "020"
created: 2026-04-22
---

# Round 1 — Minimal-Change Engineer

Lens: smallest diff that unblocks CI beyond fmt for a solo-dev project.
Challenge any mechanism that adds ongoing maintenance surface or couples
unrelated decisions. Challenge verification work that doesn't change the
action.

## Topic 1 — Verify-then-decide vs. accept-bypass-now

**Position: accept-bypass-now, strictly time-boxed.** The minimal path
is: ship `cargo zigbuild` in the workflow (one-line change), observe CI
turn green, close the BL. Do **not** run the verification first.

### Why not verify

The framing leans on "verification is cheap, 30 min, reversible" — but
cheap isn't free, and "reversible" is misapplied. The real question from
topic-01 key questions: **does verification change which bypass we
pick?** Walking through each outcome:

1. **Hypothesis confirmed** (`/usr/bin/cc` = Apple Clang, synthesizes
   `-isysroot` internally via xcrun). Action: pick a bypass that replaces
   the C compiler for the Linux target. `cargo zigbuild` does exactly
   that. Already installed. One workflow line.
2. **Hypothesis refuted** (`-isysroot` comes from a different source —
   `CFLAGS_*` env block, a `cc` shim, broken `TARGET` propagation).
   Action: still need a bypass that sidesteps whatever macOS-host env
   quirk this is. `cargo zigbuild` still works — it brings its own
   bundled libc headers and zig-cc, sidesteps the host cc and its env.

So the bypass choice is **the same** under both outcomes. Verification
is epistemically satisfying but action-identical. That makes it
optional, not cheap — and optional investigation on a solo-dev sprint
costs real time, not zero.

### The "institutional knowledge" argument is weak here

Framing asks: "Is there institutional value in having the mechanism on
record?" For mengdie specifically, no:

- Solo dev. Only reader of the docs is Kai, who will re-hit this if it
  recurs regardless.
- The `analysis.md` already narrows this to 4 candidate mechanisms with
  source-level cc-rs citations. That's the durable knowledge; the 5th
  exact-line-number pinpoint doesn't unlock new action.
- If ring/cc-rs/Apple-Clang versions change and this re-breaks, the
  reproduction cost is identical whether we verified today or not —
  the verification artifact isn't runnable against future versions.
- BL-ci-full-clippy-test (L, 5pt) already committed. Verification
  expands the sprint surface without reducing scope elsewhere.

### Time-box discipline

Sprint v0.8.0 is active. If the team wants a safety net: cap any
verification at the 30-min estimate *and* require that verification
only runs if `cargo zigbuild` trial itself fails. That is: ship the
bypass first; if it fails, *then* verify. Verification is a recovery
tool, not a prerequisite.

### Edge case I'll flag

Topic-01 lists an edge case: "strace unavailable on macOS for act
subprocesses, verification expands beyond 30 min." This is not
hypothetical — `strace` is Linux-only; macOS uses `dtruss`, which
requires SIP accommodations or root + codesigned targets. `dtruss`
on act-spawned bash subprocesses on a modern macOS is not a guaranteed
30-min exercise. The framing's 30-min estimate is optimistic; the
realistic band is 30 min–unbounded depending on SIP/signing friction.
That asymmetry alone tilts toward bypass-first.

## Topic 2 — Bypass mechanism selection

**Position: `cargo zigbuild` in the workflow step.** Rank against the
topic-02 enumeration:

### Ranking by solo-dev TCO

| Mechanism | Setup | Ongoing maint | Robust to hypothesis-wrong? | Verdict |
|---|---|---|---|---|
| `cargo zigbuild` in workflow | 1 line | zero — already installed, same install path as release builds | Yes — replaces the C compiler entirely, env-independent | **Pick this** |
| `CC_x86_64_unknown_linux_gnu=<linux-gcc>` env | Brew install + env line | pkg update on brew upgrades | Partial — still relies on host env cleanliness elsewhere | Second choice if zigbuild blows up |
| Runner-mode → Docker executor on Mac mini | Docker Desktop install + runner reconfig + VM tuning | **ongoing: Docker Desktop updates, VM disk growth, license terms (Docker Desktop paid for commercial), image pulls on CI** | Yes — full isolation | Reject. I already pushed back on this in Round 0; the "10-min install" framing understates ongoing cost. |
| Runner relocation → Linux VPS | VPS config, secrets migration, Forgejo instance risk | Forgejo-serving CPU contention is a live concern | Yes — native Linux | Reject. Couples CI reliability to the Forgejo instance's health. Non-local-reversible. |
| External CI provider (GH Actions mirror) | Repo mirror + secrets duplication | Two CI surfaces forever | Yes | Reject. Privacy (private repo), split surface. Also flagged by analysis cross-team consensus. |

### Why `cargo zigbuild` specifically

- `cargo-zigbuild` is **already installed** (per analysis: shims bound
  for aarch64-apple-darwin release builds). Zero new install surface.
- Single workflow-line change: `cargo zigbuild --target
  x86_64-unknown-linux-gnu` wherever cargo build/test would fire.
  Clippy is a frontend check that doesn't hit the C compiler path —
  may work without zigbuild wrapping, but wrapping it costs nothing.
- `zig cc` bundles glibc/musl headers; it sidesteps
  `/usr/bin/cc`-is-Apple-Clang entirely. Works under **both** outcomes
  of topic-01.
- Failure signal is clean: if zig-cc can't find a header, it says so
  explicitly. Not a silent `-isysroot` mystery.
- Reversible at the workflow level — delete the `zigbuild` prefix, no
  infrastructure to tear down.

### What I'll explicitly push back on

If anyone argues for Docker executor on the Mac mini as "just one
install":

- Docker Desktop on macOS is a full Linux VM with ongoing disk
  growth, periodic auth loops, and license implications (Docker
  Desktop is paid for commercial use over $10M revenue — not
  mengdie's concern today, but a real policy surface).
- `rust:latest` pulls on every run unless cached — CI run time grows.
- Runner reconfiguration means testing the host-mode → docker-mode
  switch, and host-mode is the canonical pattern in this repo (per
  `memory/project_infra.md`). Changing it affects every future CI
  workflow, not just this one.
- "Ongoing maintenance" is the decisive criterion for solo-dev per
  topic-02 constraints. Docker-on-Mac fails that test.

If anyone argues for verifying first "because it's only 30 min":
see topic-01 position above. The cost isn't 30 min; it's 30 min plus
the context-switch back into bypass selection plus the sprint schedule
slip if `dtruss`/SIP friction bites.

### Out-of-scope watchlist

Per framing, these are explicitly **out** of this discussion — I'll
flag if anyone tries to fold them in:

- 006 vs. BL-ci-full-clippy-test scope split (handled at `/ae:plan`)
- `release.yml` race fix (drive-by in next `release.yml` touch)
- CI target matrix (settled in 017)

Analysis.md line 174 ("ship unconditionally: release.yml race fix")
tempts scope creep. Resist. Keep this plan focused on the clippy+test
unblock only.

## Findings

1. The bypass choice is invariant under topic-01's two outcomes →
   verification is action-neutral for a solo-dev sprint.
2. `cargo zigbuild` is the unique candidate that: (a) requires no new
   install, (b) adds no ongoing maintenance, (c) is robust to both
   hypothesis outcomes, (d) is reversible at the workflow-line level.
3. The framing's 30-min verification estimate is optimistic due to
   `dtruss`/SIP/codesigning friction on macOS — the true band is
   wider and asymmetric upward.
4. Docker-on-Mac-mini carries real ongoing cost that "10-min install"
   framings hide (license, VM disk, image pulls, runner-mode
   precedent change). Reject for solo-dev.
5. Runner relocation couples CI health to Forgejo serving — rejected
   for adjacent-system risk.

## Agreements

N/A — Round 1 independent research, no prior agents' positions yet.

## Disagreements

N/A — Round 1 independent research, no prior agents' positions yet.

## Open Questions

1. Does `cargo zigbuild` work transparently for `cargo clippy`, or
   only for `cargo build`/`cargo test`? If clippy runs lints without
   linking, it may not need the C compiler at all for Linux targets
   — need to confirm the first time we wire it in. (Low-risk: if
   clippy works without zigbuild, we just don't wrap it.)
2. Does `cargo zigbuild` need explicit glibc vs. musl target
   selection? Default is glibc-compatible; confirm against mengdie's
   deployment target (VPS glibc). (Low-risk: matches release.yml
   current practice.)
3. If the bypass ships and CI still fails for a different reason,
   do we then run verification, or iterate on bypass? Proposal:
   one-failure retry with logging, then verify. Avoids infinite
   bypass-juggling.
