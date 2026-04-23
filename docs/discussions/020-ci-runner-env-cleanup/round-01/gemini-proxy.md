# Gemini Proxy Response: CI Runner Env Cleanup (Discussion 020)

**Round 1: Cross-Family Lens on Bypass vs. Investigation**

---

## 1. Cost-Benefit of Investigation vs. Bypass

**Verdict: Investigate for ~30 minutes, then pivot to bypass.**

### The Real Risk: The Rabbit Hole

The danger isn't the initial 30 minutes—it's the 3-hour descent into `cc-rs` source code or LLVM/linker internals. If you spend half a day debugging a compiler flag issue, you haven't just lost time; you've lost the momentum required to build the actual ML project.

### Why Verification Matters (But Quickly)

For a solo dev, "knowledge debt" is high-interest. If you use a bypass (e.g., cargo-zigbuild) without understanding *why* the current setup fails, you will eventually hit a situation where the bypass fails (e.g., a new C dependency is added) and you'll be forced to debug the original issue under much higher pressure.

**Decision Rule:**
- If you can prove/disprove the hypothesis via a `printenv` or a simple `CC` override in <30 mins → **Investigate**.
- If you find yourself reading rustc internals → **Abort and Bypass**.

The hypothesis is almost certainly correct: `/usr/bin/cc` → Apple Clang. If verified, the fix is likely just setting `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER`.

---

## 2. Ranking of Bypass Mechanisms

Ranked by **infrastructure debt** (how much complexity you're adding for your future self):

| Rank | Mechanism | Complexity to Undo | Future Risk | Verdict |
|:---|:---|:---|:---|:---|
| **1** | **Runner Relocation (Linux VPS)** | **Low**—just stop using the Mac runner. | **Lowest**—aligns build env with target env. | **The "Golden Path"**—eliminates cross-compilation entirely. |
| **2** | **cargo-zigbuild** | **Very Low**—just remove from PATH. | **Low**—adds one dependency (Zig) but keeps logic local. | **The "Quick Win"**—best if the VPS is too weak for heavy builds. |
| **3** | **External CI (GHA)** | **Medium**—requires managing secrets/workflows in two places. | **Medium**—creates "Split Brain" (Dev thinks code is fine, GHA says it's broken). | **The "Offloader"**—use only if you want to stop managing your own infra. |
| **4** | **Docker Executor** | **High**—requires managing Docker networks, volumes, Mac/Linux context switching. | **Highest**—hides the host's reality. Debugging "it works in the container but not on my Mac" is a massive time sink. | **"Avoid at all costs"**—too much overhead for a solo dev on macOS. |

### Summary for Kai

Move the runner to the Linux VPS. It is the only option that solves the problem by **removing the problem**, rather than **layering a workaround** on top of it.

---

## 3. Non-Obvious Options

### A. The "Explicit Toolchain" Fix (The Middle Ground)

Instead of investigating *why* `cc-rs` is looking at Apple Clang, simply force the target-specific linker in your `.cargo/config.toml` or CI environment:

```toml
# .cargo/config.toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"  # Or whatever your cross-compiler is
```

This is a "fix" that doesn't require a "bypass"—just explicit configuration.

### B. The "Hybrid Runner" Strategy

Don't treat runners as interchangeable. Use a **Multi-Runner Pipeline**:

- **Runner A (macOS):** Runs `cargo check`, `clippy`, and `cargo test` (for native macOS targets). This provides instant feedback on code quality.
- **Runner B (Linux VPS):** Runs the actual `cargo build --target aarch64-unknown-linux-gnu` and integration tests.

**Benefit:** Separates "Code Quality" (fast, local) from "Deployment Readiness" (slow, target-specific).

### C. The "SSH-Trigger" Pattern

If the Linux VPS is already hosting Forgejo, don't even set up a formal "Runner" if it's too heavy. Use a simple script on the Mac that `ssh`'s into the VPS, pulls the latest code, and runs `cargo build`. This bypasses the entire "Runner Configuration" headache while keeping the build on native Linux hardware.

---

## Recommendation Chain

**Short-term (next week):** Spend 30 mins verifying the `/usr/bin/cc` hypothesis. If confirmed → set `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER` in CI.

**Medium-term (this sprint):** If verification shows the fix is not trivial, move the runner to the Linux VPS (Option 1, Rank 1). It's the lowest-debt solution.

**Fallback:** If the VPS can't handle the load, use cargo-zigbuild as a quick win (Option 2, Rank 2).

---

**Cross-family note (Google lens):** Compared to Codex's likely focus on prompt caching or API optimization, the Google family emphasizes pragmatic DevOps tradeoffs: *which infrastructure decision scales with minimal cognitive load?* For a solo dev, the answer is almost always "eliminate the problem, don't layer workarounds."
