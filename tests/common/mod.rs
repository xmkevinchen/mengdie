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
    EntityFactsOutput, EntityFactsParams, GetOutput, GetParams, IngestOutput, IngestParams,
    InvalidateOutput, InvalidateParams, MengdieServer, SearchOutput, SearchParams, StatusOutput,
    StatusParams,
};
use rmcp::handler::server::wrapper::Parameters;

/// Process-wide pre-warm marker — once this OnceLock is initialized, the
/// fastembed model is guaranteed to be downloaded (cached at
/// `~/.cache/fastembed/`). Subsequent `Embedder::new()` calls only pay the
/// in-memory ONNX session-load cost (~100-200ms post-download), not the
/// ~90MB download cost.
///
/// Parallel-test caveat (F-013 review F2): `OnceLock::get_or_init` allows
/// multiple concurrent first-callers to all execute the init closure; only
/// the first to finish wins, the rest are discarded. Under cargo's default
/// parallel test execution, N concurrent `Harness::new()` calls on a
/// cold-cache system will each run `Embedder::new()` in parallel. This is
/// NOT a correctness bug — the download step inside fastembed is
/// filesystem-cache-safe — but the "first test pays the load, rest are
/// free" framing only strictly holds under serial execution. In parallel
/// runs, the first BATCH of concurrent tests share the redundant load
/// cost; later tests (after the first batch completes) get the warm path.
///
/// We can NOT share a single Embedder instance across harness instances
/// because `MengdieServer::new` consumes `Embedder` by value (moves into
/// `Arc<Mutex<Embedder>>` internally). A test-only constructor accepting
/// a pre-wrapped `Arc<Mutex<Embedder>>` would fix both this caveat AND the
/// per-Harness load cost — filed as BL-051 (trigger: mengdie published
/// as library OR per-test load becomes painful).
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
    /// Clone of the same `Db` handle (`Arc<Mutex<Connection>>`) that the
    /// server holds — exposes raw DB access for tests that need to set
    /// up scenarios `MengdieServer`'s tool surface can't construct
    /// (e.g., F-009 collision path needs 2 facts with shared ID prefix,
    /// achievable only via `Db::insert_memory_with_id`).
    pub db: Db,
    /// Default project_id the server was constructed with. Exposed so
    /// tests can ingest into other projects for cross-project tests.
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
        let db_for_server = db.clone(); // Arc<Mutex<Connection>> share
        let embedder = Embedder::new().expect("Embedder::new failed in Harness");
        let server = MengdieServer::new(db_for_server, embedder, project_id.to_string());
        Self {
            server,
            db,
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

    pub async fn status(&self, params: StatusParams) -> StatusOutput {
        self.server.status(Parameters(params)).await.0
    }

    pub async fn entity_facts(&self, params: EntityFactsParams) -> EntityFactsOutput {
        self.server.entity_facts(Parameters(params)).await.0
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}
