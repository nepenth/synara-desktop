//! Opaque client handle slot and sole construction factory.
//!
//! P2.3 supplies [`crate::matrix::client_builder::PlannedClientFactory`] which
//! validates config and installs a planned handle. Live `matrix_sdk::Client`
//! construction remains test-harness-only (guardrails). No production
//! login/sync.

use crate::transport::MatrixIpcErrorCategory;

/// Opaque Matrix client slot owned exclusively by [`super::MatrixSupervisor`].
///
/// Implementations must not perform production login/sync. Planned handles
/// (P2.3) carry config/plan only; live SDK adapters remain harness-scoped.
pub trait ClientHandle: Send {
    /// Stable id for leak/tracking tests (not a Matrix device id).
    fn handle_id(&self) -> u64;

    /// Release subordinate resources. Idempotent.
    fn shutdown(&mut self);
}

/// Builds a client handle. **Only** [`super::MatrixSupervisor`] may call this
/// in product architecture; unit tests may exercise the factory directly only
/// when proving factory behaviour in isolation.
pub trait ClientFactory: Send {
    fn build(&self, generation: u64) -> Result<Box<dyn ClientHandle>, FactoryError>;
}

/// Privacy-safe factory failure (no tokens / secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryError {
    pub category: MatrixIpcErrorCategory,
    pub diagnostic_id: &'static str,
}

/// Default factory: refuses construction. Used when no config is available.
/// Prevents accidental silent client creation.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullClientFactory;

impl ClientFactory for NullClientFactory {
    fn build(&self, _generation: u64) -> Result<Box<dyn ClientHandle>, FactoryError> {
        Err(FactoryError {
            category: MatrixIpcErrorCategory::SdkInvariant,
            diagnostic_id: "p2.1-null-factory-no-client",
        })
    }
}

/// Test double: installs a trackable handle that records shutdown.
#[derive(Debug)]
pub struct TestClientHandle {
    id: u64,
    shutdown_count: u64,
}

impl TestClientHandle {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            shutdown_count: 0,
        }
    }

    pub fn shutdown_count(&self) -> u64 {
        self.shutdown_count
    }
}

impl ClientHandle for TestClientHandle {
    fn handle_id(&self) -> u64 {
        self.id
    }

    fn shutdown(&mut self) {
        self.shutdown_count = self.shutdown_count.saturating_add(1);
    }
}

/// Factory that mints monotonically increasing test handles.
#[derive(Debug, Default)]
pub struct TestClientFactory {
    next_id: std::sync::atomic::AtomicU64,
}

impl TestClientFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&self) -> u64 {
        self.next_id.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ClientFactory for TestClientFactory {
    fn build(&self, _generation: u64) -> Result<Box<dyn ClientHandle>, FactoryError> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        Ok(Box::new(TestClientHandle::new(id)))
    }
}

/// Factory that always fails with a store-class error (harness).
#[derive(Debug, Default, Clone, Copy)]
pub struct FailingClientFactory;

impl ClientFactory for FailingClientFactory {
    fn build(&self, _generation: u64) -> Result<Box<dyn ClientHandle>, FactoryError> {
        Err(FactoryError {
            category: MatrixIpcErrorCategory::StoreUnavailable,
            diagnostic_id: "p2.1-test-factory-store-unavailable",
        })
    }
}
