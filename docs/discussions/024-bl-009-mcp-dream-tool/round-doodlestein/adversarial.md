---
type: doodlestein
discussion: "024"
author: doodlestein
date: 2026-04-24
---

# Adversarial Post-Conclusion Review — Discussion 024 (BL-009 MCP Dream Tool)

## Verdict: BLOCK (structural)

One failure mode is not a caveat — it breaks the entire construction-time capability check design.

---

## The Failure: `Peer<RoleServer>` Is Not Available at `MengdieServer::new`

### What the conclusion assumes

> **Construction-time capability check** at `MengdieServer::new` — single resolution per process selecting McpSessionProvider (if host advertises sampling) or ClaudeCliProvider fallback.

The conclusion treats "construction-time" as equivalent to "before any tool is called." This is correct in spirit but wrong about *which* construction time matters.

### What the rmcp lifecycle actually is

`MengdieServer::new` is called in `main()` **before** `rmcp::serve_server()`:

```rust
// src/bin/mcp_server.rs (current)
let server = MengdieServer::new(db, embedder, project_id);   // ← no peer yet
let transport = rmcp::transport::io::stdio();
let service = rmcp::serve_server(server, transport).await?;  // ← handshake happens here
```

The MCP initialization handshake — where the client sends its `ClientCapabilities` (including whether sampling is supported) — happens **inside** `serve_server()`. The `Peer<RoleServer>` handle, which carries `supports_sampling_tools()`, does not exist until after `serve_server()` completes the initialize exchange.

`MengdieServer::new` receives no peer, no session, no capabilities. There is no `Peer<RoleServer>` argument to pass in, and rmcp provides no global accessor to retrieve it from outside a request context.

The rmcp API surface confirms this. `Peer<RoleServer>::supports_sampling_tools()` is a method on a live peer handle. That handle is available in:

- `RequestContext<RoleServer>` passed to `call_tool` / `on_initialized`
- `NotificationContext<RoleServer>` passed to `on_initialized`

It is **not** available in `MengdieServer::new`.

### What actually happens if the plan is implemented as written

The plan calls for `MengdieServer::new` to query sampling capability and permanently set a provider field. There are only two ways to attempt this:

**Option A — Panic / compile error**: `MengdieServer::new` has no peer to query. The field `provider: Box<dyn LlmProvider>` cannot be set. The code does not compile without a peer argument, and there is no peer to pass.

**Option B — Use `on_initialized`**: Move the capability check into the `on_initialized` hook (which does receive a `NotificationContext` containing the peer). But then `provider` must be a late-initialized field — `Option<Box<dyn LlmProvider>>` set after init, with a `Mutex` or `OnceLock`. This is not "construction-time" anymore. It is post-handshake initialization, deferred and fallible.

**Option C — Check per-call from `RequestContext`**: The peer is available inside each tool call via `context.peer`. The capability check becomes per-call, not construction-time. This contradicts the conclusion's explicit claim that "no per-call branching" is a design property.

### Why this matters beyond a naming quibble

The conclusion's "construction-time, single resolution" is the specific claim that justified the "no test-matrix doubling" and "no per-call branching" benefits that swayed the architect and minimal-change positions (Round 2 movements). If the check cannot be done at construction time:

- The `provider` field must be `OnceLock<Box<dyn LlmProvider>>` or similar, adding late-init failure modes.
- `on_initialized` must be overridden (not mentioned in conclusion or next steps).
- Every tool call that fires before `on_initialized` completes (a race that rmcp does not rule out) will hit an uninitialized provider.
- The fallback logic is now structurally more complex than "two impls, one selected at new."

### What the conclusion's open questions reveal

The conclusion already admitted:

> Exact rmcp v1.3 client-capability detection API surface (`peer_info().capabilities.sampling` is plausible but needs verification at code time)

This is not a minor API spelling question. It is the load-bearing mechanism. The method is `peer.supports_sampling_tools()` on a `Peer<RoleServer>` — and that peer does not exist at `new()` time. The conclusion deferred the one question that determines whether "construction-time" is achievable at all.

---

## Recommended Fix

The plan step that reads "Implement construction-time provider selection in `MengdieServer::new`" must be replaced with:

> Implement **post-handshake provider selection** in `on_initialized`: store `Peer<RoleServer>` in a `OnceLock<Peer<RoleServer>>` field on `MengdieServer`; in `on_initialized`, call `peer.supports_sampling_tools()` and write the selected provider to a `OnceLock<Box<dyn LlmProvider>>`. Tool handlers read from the `OnceLock` (defaulting to ClaudeCliProvider if somehow unset).

This preserves the "one selection per process, no per-call branching" intent, but requires:
1. `on_initialized` override (not mentioned anywhere in conclusion or next steps)
2. `OnceLock` field (not `direct field`)
3. A fallback for the window between process start and `on_initialized` firing

The conclusion's "~50-150 LOC" estimate was based on a construction-time model that cannot be implemented. Actual LOC will be higher.

---

## Not a Blocker on the Verdict to Build

The conclusion's **decision to build** (yes-ship in v0.9.0 with McpSessionProvider + silent fallback) remains sound. The blocker is only on the **mechanism description**: the plan, as written in Next Steps items 2 and 3, will fail at implementation time. The plan must be corrected before `/ae:work` begins.
