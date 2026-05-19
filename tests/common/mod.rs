//! Shared integration-test harness for the MCP tool surface (F-013).
//!
//! Constructs a `MengdieServer` against an in-memory `Db` and a lazily-
//! initialized shared `Embedder` (one-time per test binary via
//! `std::sync::OnceLock` — first test pays the fastembed model load,
//! rest are free).
//!
//! ## Usage
//!
//! ```ignore
//! mod common;
//! use common::Harness;
//! use mengdie::core::mcp_tools::{InvalidateParams, IngestParams, SourceType, KnowledgeType};
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let h = Harness::new();
//!     // ingest, then invalidate by prefix, etc.
//! }
//! ```
//!
//! ## Why this exists
//!
//! F-009 + F-010 added prefix-resolution code paths in `memory_invalidate`
//! and `memory_get`. `tests/e2e.rs` exercises only the `db::*` layer, so
//! the MCP-dispatch wrapping (param shape, error formatting, cross-project
//! guard, recall bump semantic) was unreachable from tests. F-013 closes
//! the gap retroactively and provides scaffolding for F-008 / F-011 / F-012
//! to add their tests from day one.

use std::sync::OnceLock;

use mengdie::core::db::Db;
use mengdie::core::embeddings::Embedder;
use mengdie::core::mcp_tools::{
    GetOutput, GetParams, IngestOutput, IngestParams, InvalidateOutput, InvalidateParams,
    MengdieServer, SearchOutput, SearchParams,
};
use rmcp::handler::server::wrapper::Parameters;

/// Process-wide pre-warm marker — once this OnceLock is initialized, the
/// fastembed model is guaranteed to be downloaded (cached at
/// `~/.cache/fastembed/`). Subsequent `Embedder::new()` calls only pay the
/// in-memory ONNX session-load cost (~100-200ms post-download), not the
/// ~90MB download cost.
///
/// We can NOT share a single Embedder instance across harness instances
/// because `MengdieServer::new` consumes `Embedder` by value (moves into
/// `Arc<Mutex<Embedder>>` internally). A test-only constructor accepting
/// a pre-wrapped `Arc<Mutex<Embedder>>` could enable sharing — filed
/// as out-of-scope follow-up if per-test load cost becomes painful.
fn ensure_embedder_warm() {
    static WARM: OnceLock<()> = OnceLock::new();
    WARM.get_or_init(|| {
        // Trigger a download (no-op if already cached) by constructing once.
        let _ = Embedder::new().expect("Embedder::new failed during pre-warm");
    });
}

/// Reusable integration-test harness wrapping a `MengdieServer` instance.
///
/// Each `Harness::new()` returns a fresh in-memory `Db` (no cross-test
/// state bleed). The `Embedder` is shared across all tests in the binary
/// (read-only inference; no mutable state per call).
pub struct Harness {
    server: MengdieServer,
    /// Default project_id the server was constructed with. Exposed so
    /// tests can ingest into other projects for cross-project tests
    /// (used by F-010 cross-project guard test, not by smoke test).
    #[allow(dead_code)]
    pub default_project_id: String,
}

#[allow(dead_code)] // some tests only use a subset of the helpers
impl Harness {
    /// Construct a fresh harness with default project_id `"test-project"`.
    pub fn new() -> Self {
        Self::with_project_id("test-project")
    }

    /// Construct a harness with a specific default project_id.
    pub fn with_project_id(project_id: &str) -> Self {
        ensure_embedder_warm();
        let db = Db::open_in_memory().expect("Db::open_in_memory failed");
        let embedder = Embedder::new().expect("Embedder::new failed in Harness");
        let server = MengdieServer::new(db, embedder, project_id.to_string());
        Self {
            server,
            default_project_id: project_id.to_string(),
        }
    }

    /// Access the underlying server for direct method calls in tests
    /// that need shape beyond the typed helpers below.
    pub fn server(&self) -> &MengdieServer {
        &self.server
    }

    pub async fn search(&self, params: SearchParams) -> SearchOutput {
        self.server.search(Parameters(params)).await.0
    }

    pub async fn ingest(&self, params: IngestParams) -> IngestOutput {
        self.server.ingest(Parameters(params)).await.0
    }

    pub async fn get(&self, params: GetParams) -> GetOutput {
        self.server.get(Parameters(params)).await.0
    }

    pub async fn invalidate(&self, params: InvalidateParams) -> InvalidateOutput {
        self.server.invalidate(Parameters(params)).await.0
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}
