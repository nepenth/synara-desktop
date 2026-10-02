//! Typed SharedCore operations and projections for nse preview.

use super::*;

/// Privacy-safe NSE store status. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseStoreDto {
    pub read_only: bool,
    pub owners_attached: bool,
    pub sync_started: bool,
}

/// Local-store notification preview. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseEventPreviewDto {
    pub event_type: String,
    pub sender_id: Option<String>,
    pub body: Option<String>,
    pub message_type: Option<String>,
}

/// Static fail-closed NSE store error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NseStoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for NseStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for NseStoreError {}

pub(super) fn nse_failed(code: &'static str, description: &'static str) -> NseStoreError {
    NseStoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn bounded_nse_preview_text(value: &str, maximum_characters: usize) -> String {
    value.chars().take(maximum_characters).collect()
}

pub(super) fn nse_message_type(message: &MessageType) -> Option<&'static str> {
    match message {
        MessageType::Audio(_) => Some("m.audio"),
        MessageType::Emote(_) => Some("m.emote"),
        MessageType::File(_) => Some("m.file"),
        MessageType::Image(_) => Some("m.image"),
        MessageType::Location(_) => Some("m.location"),
        MessageType::Notice(_) => Some("m.notice"),
        MessageType::ServerNotice(_) => Some("m.server_notice"),
        MessageType::Text(_) => Some("m.text"),
        MessageType::Video(_) => Some("m.video"),
        MessageType::VerificationRequest(_) => Some("m.key.verification.request"),
        _ => None,
    }
}

pub(super) fn map_restore_to_nse(error: SessionRestoreError) -> NseStoreError {
    match error {
        SessionRestoreError::Failed { code, .. }
            if code == IDENTITY_INVALID_CODE
                || code == STORE_ROOT_INVALID_CODE
                || code == MATERIAL_MISSING_CODE
                || code == VAULT_UNAVAILABLE_CODE =>
        {
            let description = if code == IDENTITY_INVALID_CODE {
                IDENTITY_INVALID_DESCRIPTION
            } else if code == STORE_ROOT_INVALID_CODE {
                STORE_ROOT_INVALID_DESCRIPTION
            } else if code == MATERIAL_MISSING_CODE {
                MATERIAL_MISSING_DESCRIPTION
            } else {
                VAULT_UNAVAILABLE_DESCRIPTION
            };
            NseStoreError::Failed {
                code,
                description: description.to_owned(),
            }
        }
        _ => nse_failed(NSE_RESTORE_FAILED_CODE, NSE_RESTORE_FAILED_DESCRIPTION),
    }
}

pub(super) fn parse_store_root(store_root: &str) -> Result<&Path, ()> {
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(());
    }
    Ok(path)
}

pub(super) fn validate_store_root(store_root: &str) -> Result<&Path, SessionRestoreError> {
    parse_store_root(store_root)
        .map_err(|_| restore_failed(STORE_ROOT_INVALID_CODE, STORE_ROOT_INVALID_DESCRIPTION))
}

pub(super) fn store_key_for(
    store: &Arc<dyn SecretVault + Send + Sync>,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    get_or_create_store_key(&vault, &StoreKeyId::from_identity(identity)).map_err(|error| {
        match error {
            StoreKeyVaultError::BackendUnavailable { .. } => {
                restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
            }
            StoreKeyVaultError::CorruptPayload => {
                restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION)
            }
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        }
    })
}

/// NSE access must never create or migrate a key. The containing app owns
/// Keychain mutations and publishes readiness only after the current key and
/// shared store are both available.
pub(super) fn store_key_for_read_only(
    store: &Arc<dyn SecretVault + Send + Sync>,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    vault
        .get(&StoreKeyId::from_identity(identity))
        .map_err(|error| match error {
            StoreKeyVaultError::BackendUnavailable { .. } => {
                restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
            }
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        })?
        .ok_or_else(|| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))
}

pub(super) struct SecretStoreKeyVault {
    pub(super) store: Arc<dyn SecretVault + Send + Sync>,
}

impl StoreKeyVault for SecretStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        match self.store.get(id.account()) {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) if bytes.len() == STORE_KEY_LEN => {
                let mut key_bytes = [0u8; STORE_KEY_LEN];
                key_bytes.copy_from_slice(&bytes);
                Ok(Some(StoreKeyMaterial::from_bytes(key_bytes)))
            }
            Ok(Some(_)) => Err(StoreKeyVaultError::CorruptPayload),
            Err(_) => Err(StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            }),
        }
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        self.store
            .put(id.account(), key.as_bytes().as_slice())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store
            .delete(id.account())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })?;
        Ok(existed)
    }
}

impl SharedCore {
    /// Open the persisted store for NSE preview. Never attaches owners or
    /// starts SyncService. An already-retained planted/restored client is
    /// adopted as read-only; otherwise this restores from the vault.
    pub async fn nse_open_read_only_store(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<NseStoreDto, NseStoreError> {
        self.nse_open_read_only_store_with_room(user_id, homeserver_url, store_root, None)
            .await
    }

    pub(super) async fn nse_open_read_only_store_with_room(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        room_id: Option<matrix_sdk::ruma::OwnedRoomId>,
    ) -> Result<NseStoreDto, NseStoreError> {
        if self.owners_attached() {
            return Err(nse_failed(
                NSE_OWNERS_ATTACHED_CODE,
                NSE_OWNERS_ATTACHED_DESCRIPTION,
            ));
        }
        AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| nse_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        validate_store_root(&store_root).map_err(map_restore_to_nse)?;
        if self.has_retained_client() {
            self.set_nse_read_only(true)?;
            return self.nse_store_dto();
        }
        self.restore_persisted_session_with_policy(
            user_id,
            homeserver_url,
            store_root,
            true,
            room_id.map(RoomLoadSettings::One),
        )
        .await
        .map_err(map_restore_to_nse)?;
        self.set_nse_read_only(true)?;
        self.nse_store_dto()
    }

    pub async fn nse_store_status(&self) -> Result<NseStoreDto, NseStoreError> {
        if !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            ));
        }
        self.nse_store_dto()
    }

    /// Drop the short-lived NSE client while this async call is still executing
    /// on the Rust runtime. SQLite pool cleanup may require that runtime; leaving
    /// the retained client for UniFFI object deallocation can abort the extension.
    pub async fn nse_close_read_only_store(&self) -> Result<(), NseStoreError> {
        if self.owners_attached() || !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_CLOSE_FAILED_CODE,
                NSE_CLOSE_FAILED_DESCRIPTION,
            ));
        }
        let retained = {
            let mut guard = self
                .restored_client
                .lock()
                .map_err(|_| nse_failed(NSE_CLOSE_FAILED_CODE, NSE_CLOSE_FAILED_DESCRIPTION))?;
            std::mem::replace(&mut *guard, RestoredClientSlot::Empty)
        };
        drop(retained);
        self.set_nse_read_only(false)
            .map_err(|_| nse_failed(NSE_CLOSE_FAILED_CODE, NSE_CLOSE_FAILED_DESCRIPTION))?;
        Ok(())
    }

    /// Resolve one push notification with the Matrix SDK's dedicated
    /// multi-process notification client. This may run the SDK's bounded,
    /// short-lived notification/decryption sync, but it never starts the
    /// product `SyncService` or attaches product session owners.
    pub async fn nse_resolve_event_preview(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        tokio::time::timeout(NSE_RESOLUTION_TIMEOUT, async {
            if room_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
                return Err(nse_failed(
                    NSE_PAYLOAD_OVERSIZE_CODE,
                    NSE_PAYLOAD_OVERSIZE_DESCRIPTION,
                ));
            }
            let parsed_room =
                matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()).map_err(|_| {
                    nse_failed(
                        NSE_EVENT_NOT_IN_STORE_CODE,
                        NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
                    )
                })?;
            self.nse_open_read_only_store_with_room(
                user_id,
                homeserver_url,
                store_root,
                Some(parsed_room),
            )
            .await?;
            self.nse_event_preview_unbounded(room_id, event_id).await
        })
        .await
        .map_err(|_| {
            nse_failed(
                NSE_RESOLUTION_TIMEOUT_CODE,
                NSE_RESOLUTION_TIMEOUT_DESCRIPTION,
            )
        })?
    }

    pub async fn nse_event_preview(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        tokio::time::timeout(
            NSE_RESOLUTION_TIMEOUT,
            self.nse_event_preview_unbounded(room_id, event_id),
        )
        .await
        .map_err(|_| {
            nse_failed(
                NSE_RESOLUTION_TIMEOUT_CODE,
                NSE_RESOLUTION_TIMEOUT_DESCRIPTION,
            )
        })?
    }

    pub(super) async fn nse_event_preview_unbounded(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        if room_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
            || event_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
        {
            return Err(nse_failed(
                NSE_PAYLOAD_OVERSIZE_CODE,
                NSE_PAYLOAD_OVERSIZE_DESCRIPTION,
            ));
        }
        if !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            ));
        }
        let client = self.retained_client()?;
        let Ok(parsed_room) = matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()) else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let Ok(parsed_event) = matrix_sdk::ruma::OwnedEventId::try_from(event_id.trim()) else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let notification_client =
            NotificationClient::new(client, NotificationProcessSetup::MultipleProcesses)
                .await
                .map_err(|_| {
                    nse_failed(
                        NSE_CLIENT_INIT_FAILED_CODE,
                        NSE_CLIENT_INIT_FAILED_DESCRIPTION,
                    )
                })?;
        let status = notification_client
            .get_notification(&parsed_room, &parsed_event)
            .await
            .map_err(|_| {
                nse_failed(
                    NSE_EVENT_FETCH_FAILED_CODE,
                    NSE_EVENT_FETCH_FAILED_DESCRIPTION,
                )
            })?;
        let NotificationStatus::Event(item) = status else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let NotificationEvent::Timeline(event) = &item.event else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) =
            event.as_ref()
        else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let Some(original) = message.as_original() else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let body = Some(bounded_nse_preview_text(original.content.body(), 240));
        let message_type = nse_message_type(&original.content.msgtype)
            .map(|value| bounded_nse_preview_text(value, 64));

        Ok(NseEventPreviewDto {
            event_type: "m.room.message".to_owned(),
            sender_id: Some(bounded_nse_preview_text(
                item.sender_display_name
                    .as_deref()
                    .unwrap_or_else(|| item.event.sender().as_str()),
                255,
            )),
            body,
            message_type,
        })
    }

    pub(super) fn is_nse_read_only(&self) -> bool {
        self.nse_read_only
            .lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    pub(super) fn set_nse_read_only(&self, value: bool) -> Result<(), NseStoreError> {
        let mut guard = self
            .nse_read_only
            .lock()
            .map_err(|_| nse_failed(NSE_FAILED_CODE, NSE_FAILED_DESCRIPTION))?;
        *guard = value;
        Ok(())
    }

    pub(super) fn nse_store_dto(&self) -> Result<NseStoreDto, NseStoreError> {
        Ok(NseStoreDto {
            read_only: self.is_nse_read_only(),
            owners_attached: self.owners_attached(),
            sync_started: self.core.sync_service_started(),
        })
    }
}
