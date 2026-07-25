//! Opaque [`crate::matrix::supervisor::ClientHandle`] wrapping a live SDK Client.
//!
//! Construction of the inner `Client` remains in [`super::open`]; this type only
//! owns the handle for supervisor install/drop cycles. No login/sync APIs.
//!
//! Callers that install a live Client must ensure a Tokio runtime is available
//! when the handle is dropped (SQLite/deadpool cleanup). Harness tests use a
//! multi-thread runtime entered for the full lifecycle.

use matrix_sdk::Client;

use crate::matrix::supervisor::ClientHandle;

/// Production-shaped client handle for a built (possibly unauthenticated) SDK Client.
pub struct SdkClientHandle {
    id: u64,
    client: Option<Client>,
    shutdown_count: u64,
}

impl SdkClientHandle {
    pub fn new(id: u64, client: Client) -> Self {
        Self {
            id,
            client: Some(client),
            shutdown_count: 0,
        }
    }

    pub fn client(&self) -> Option<&Client> {
        self.client.as_ref()
    }

    pub fn take_client(&mut self) -> Option<Client> {
        self.client.take()
    }

    pub fn shutdown_count(&self) -> u64 {
        self.shutdown_count
    }

    /// Homeserver URL currently configured on the client (privacy-safe; no tokens).
    pub fn homeserver(&self) -> Option<String> {
        self.client.as_ref().map(|c| c.homeserver().to_string())
    }
}

impl ClientHandle for SdkClientHandle {
    fn handle_id(&self) -> u64 {
        self.id
    }

    fn shutdown(&mut self) {
        // Drop the Client (closes stores / HTTP resources). Idempotent.
        // Requires a Tokio runtime in context for SQLite pool teardown.
        self.client = None;
        self.shutdown_count = self.shutdown_count.saturating_add(1);
    }
}

impl Drop for SdkClientHandle {
    fn drop(&mut self) {
        if self.client.is_some() {
            self.shutdown();
        }
    }
}
