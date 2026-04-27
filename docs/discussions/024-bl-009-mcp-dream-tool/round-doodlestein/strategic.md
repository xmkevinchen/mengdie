---
reviewer: doodlestein-strategic
type: post-conclusion-review
scope: single-smartest-improvement
reopen: false
---

# Strategic Review — Discussion 024 Conclusion

## Verdict

The conclusion is sound on mechanism (McpSessionProvider + thin wrapper +
ClaudeCliProvider fallback), convergence quality (4/5 with tracked concessions),
and sequencing. No reopen warranted. The decision table stands.

## Single Smartest Improvement

**Expose the initialization-ordering contradiction hidden inside "construction-time
capability check."**

### The gap

The conclusion commits to: "construction-time capability check at
`MengdieServer::new` — single resolution per process selecting
McpSessionProvider (if host advertises sampling) or ClaudeCliProvider fallback."
(Decision table, Topic 1 column, Rationale row.)

The open questions block acknowledges "Whether `MengdieServer` gets a per-server
or per-request `Peer<RoleServer>` handle" as plan-time detail. It isn't. It is a
structural initialization-ordering problem:

`src/bin/mcp_server.rs:37-39` shows the actual startup sequence:

```
let server = MengdieServer::new(db, embedder, project_id);   // line 37
let transport = rmcp::transport::io::stdio();                 // line 38
let service = rmcp::serve_server(server, transport).await?;  // line 39
```

`MengdieServer::new` runs at line 37. The `Peer<RoleServer>` handle — the object
through which `peer_info().capabilities.sampling` can be queried — is produced by
`rmcp::serve_server` at line 39. The peer does not exist before the server is
initialized; the server is initialized before the peer exists. The conclusion's
"construction-time" plan assumes the peer is queryable during construction, but
that ordering is structurally impossible under the current startup sequence and
the rmcp v1.3 stdio transport pattern.

The capability check therefore cannot happen at `MengdieServer::new` as specified.
It must happen elsewhere. The two structural options are:

1. **Post-handshake lazy init** — store a `provider: OnceLock<Arc<dyn LlmProvider>>`
   on `MengdieServer`; populate it on the first `memory_dream` call by calling
   `self.peer.peer_info()` at that point. The check is still "once per process"
   (via `OnceLock`) but happens after the peer is established, not at construction.
   This preserves the "single resolution" property the conclusion wants but requires
   the `Peer` handle to be accessible from inside tool handlers — which rmcp does
   provide via `self` access to the server struct if the `Peer` is stored on it
   after `serve_server`.

2. **Two-phase construction** — refactor `mcp_server.rs` startup to:
   (a) call `rmcp::serve_server`, receive service + peer,
   (b) query `peer.peer_info().capabilities.sampling`,
   (c) inject the resolved provider into `MengdieServer` before the service
   starts handling requests. This matches the conclusion's intent literally but
   requires that rmcp's `serve_server` return the peer before dispatching — which
   needs verification at plan time.

Option 1 is almost certainly what the plan will land on (lower rmcp API surface
risk), but it contradicts the "at `MengdieServer::new`" language explicitly in
the conclusion's decision table. If left unresolved, the plan author will either
(a) re-litigate this at plan-time as a design dispute or (b) silently pick a
resolution that differs from what the conclusion specifies without documenting
why — losing the audit trail.

### Classification

**Framing tightening + downstream linkage.** The decision is correct; the
implementation shape that follows from it needs one clarification to be
plan-actionable without ambiguity. This is additive: replace "construction-time
at `MengdieServer::new`" in the conclusion with "post-handshake, once-per-process
(via `OnceLock` or equivalent) at first `memory_dream` invocation, after the
`Peer<RoleServer>` is available from the initialized transport."

The open questions block (conclusion.md lines 49-53) should also be updated to
make this an explicit plan-time verification target, not just a shape question:
confirm which of the two-phase patterns rmcp's API supports, and resolve before
writing `McpSessionProvider`.

No topic reopen required.
