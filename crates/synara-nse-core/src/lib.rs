//! Notification-extension-only UniFFI boundary.
//!
//! This crate deliberately exports one operation and one read-only callback.
//! It never constructs the full application `SharedCore`, retains a Matrix
//! client after resolution, or permits the extension to mutate the containing
//! app's vault. The narrow request object exists only to provide prompt,
//! idempotent cancellation across UniFFI's Swift async boundary.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio_util::sync::CancellationToken;

uniffi::include_scaffolding!("synara_nse_core");

const VAULT_UNAVAILABLE_CODE: &str = "nse-secret-vault-unavailable";
const VAULT_UNAVAILABLE_DESCRIPTION: &str = "The notification secret store is unavailable.";

pub trait NseSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, NseSecretVaultError>;
}

#[derive(Debug)]
pub enum NseSecretVaultError {
    Unavailable { code: String, description: String },
}

impl std::fmt::Display for NseSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { code, description } => write!(formatter, "{code}: {description}"),
        }
    }
}

impl std::error::Error for NseSecretVaultError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsePreviewDto {
    pub event_type: String,
    pub sender_id: Option<String>,
    pub body: Option<String>,
    pub message_type: Option<String>,
    pub is_agent_approval: bool,
    pub origin_server_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NseCoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for NseCoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { code, description } => write!(formatter, "{code}: {description}"),
        }
    }
}

impl std::error::Error for NseCoreError {}

struct SecretReaderAdapter {
    inner: Box<dyn NseSecretVault>,
}

pub struct NsePreviewRequest {
    store: SecretReaderAdapter,
    user_id: String,
    homeserver_url: String,
    store_root: String,
    room_id: String,
    event_id: String,
    cancellation: CancellationToken,
    started: AtomicBool,
}

impl synara_core::app::nse_preview::NseSecretReader for SecretReaderAdapter {
    fn get(
        &self,
        key: &str,
    ) -> Result<Option<Vec<u8>>, synara_core::app::nse_preview::NsePreviewError> {
        self.inner.get(key.to_owned()).map_err(|_| {
            synara_core::app::nse_preview::NsePreviewError::failed(
                VAULT_UNAVAILABLE_CODE,
                VAULT_UNAVAILABLE_DESCRIPTION,
            )
        })
    }
}

fn map_core_error(error: synara_core::app::nse_preview::NsePreviewError) -> NseCoreError {
    NseCoreError::Failed {
        code: error.code().to_owned(),
        description: error.description().to_owned(),
    }
}

impl NsePreviewRequest {
    pub fn new(
        store: Box<dyn NseSecretVault>,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        room_id: String,
        event_id: String,
    ) -> Self {
        Self {
            store: SecretReaderAdapter { inner: store },
            user_id,
            homeserver_url,
            store_root,
            room_id,
            event_id,
            cancellation: CancellationToken::new(),
            started: AtomicBool::new(false),
        }
    }

    pub async fn resolve(&self) -> Result<NsePreviewDto, NseCoreError> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Err(failed(
                "nse-preview-request-already-started",
                "The notification request has already started.",
            ));
        }

        let cancellation = self.cancellation.clone();
        let result = resolve_with_cancellation(cancellation.clone(), async {
            let preview = synara_core::app::nse_preview::resolve_event_preview(
                &self.store,
                &self.user_id,
                &self.homeserver_url,
                &self.store_root,
                &self.room_id,
                &self.event_id,
            )
            .await
            .map_err(map_core_error)?;

            Ok(NsePreviewDto {
                event_type: preview.event_type,
                sender_id: preview.sender_id,
                body: preview.body,
                message_type: preview.message_type,
                is_agent_approval: preview.is_agent_approval,
                origin_server_ts: preview.origin_server_ts,
            })
        })
        .await;

        // A synchronous foreign callback can become ready during the same
        // poll in which another thread cancels this request. In that narrow
        // race the operation branch may finish before `select!` polls the
        // cancellation branch again. Cancellation must still dominate so a
        // timed-out extension never publishes a late result.
        if cancellation.is_cancelled() {
            Err(cancelled_error())
        } else {
            result
        }
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }
}

impl Drop for NsePreviewRequest {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

async fn resolve_with_cancellation<T, F>(
    cancellation: CancellationToken,
    operation: F,
) -> Result<T, NseCoreError>
where
    F: Future<Output = Result<T, NseCoreError>>,
{
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(cancelled_error()),
        result = operation => result,
    }
}

fn cancelled_error() -> NseCoreError {
    failed(
        "nse-preview-request-cancelled",
        "The notification request was cancelled.",
    )
}

fn failed(code: &str, description: &str) -> NseCoreError {
    NseCoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use tokio::time::{timeout, Duration};

    use super::*;

    #[test]
    fn udl_surface_is_cancelable_and_read_only() {
        let udl = include_str!("synara_nse_core.udl");
        assert_eq!(udl.matches("[Async").count(), 1);
        assert_eq!(udl.matches("bytes? get").count(), 1);
        assert_eq!(udl.matches("void cancel()").count(), 1);
        for forbidden in [
            " put(",
            " delete(",
            "close_read_only_store",
            "\ninterface NseCore {",
        ] {
            assert!(
                !udl.contains(forbidden),
                "forbidden NSE surface: {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn cancellation_drops_a_pending_operation_promptly() {
        struct DropSignal(Arc<AtomicBool>);
        impl Drop for DropSignal {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let cancellation = CancellationToken::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let operation_drop = Arc::clone(&dropped);
        let operation = async move {
            let _drop_signal = DropSignal(operation_drop);
            std::future::pending::<Result<(), NseCoreError>>().await
        };
        let task = tokio::spawn(resolve_with_cancellation(cancellation.clone(), operation));
        tokio::task::yield_now().await;
        cancellation.cancel();

        let result = timeout(Duration::from_millis(250), task)
            .await
            .expect("cancellation must resolve promptly")
            .expect("task must not panic")
            .expect_err("cancellation must fail closed");
        assert!(matches!(
            result,
            NseCoreError::Failed { ref code, .. } if code == "nse-preview-request-cancelled"
        ));
        assert!(dropped.load(Ordering::Acquire));
    }
}
