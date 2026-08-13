//! Outbound attachment / media send queue (P7.4 harness foundation).
//!
//! Tracks file/image/video/audio/voice send intents by **media handle id**
//! only — never file bytes. No SDK `Room::send`, no dual-backend, no tokens.

use std::collections::HashMap;

use crate::dto::{LocalEchoState, RoomId};

use super::error::SendError;
use super::queue::LocalTxnId;

/// Soft cap on concurrent active attachment sends.
pub const MAX_ACTIVE_ATTACHMENTS: usize = 16;

/// Soft cap on caption length (chars).
pub const MAX_CAPTION_CHARS: usize = 2_048;

/// Soft cap on media handle / filename length.
pub const MAX_HANDLE_CHARS: usize = 2_048;

/// Kind of timeline attachment send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentKind {
    File,
    Image,
    Video,
    Audio,
    Voice,
}

impl AttachmentKind {
    pub const ALL: &'static [AttachmentKind] = &[
        Self::File,
        Self::Image,
        Self::Video,
        Self::Audio,
        Self::Voice,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Voice => "voice",
        }
    }
}

/// Input fields for [`AttachmentSendQueue::enqueue`] (avoids arg-count thrash).
#[derive(Debug, Clone)]
pub struct AttachmentEnqueue {
    pub room_id: String,
    pub kind: AttachmentKind,
    pub media_handle_id: String,
    pub file_name: Option<String>,
    pub caption: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Privacy-safe queued outbound attachment (handle id only, no bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundAttachment {
    pub local_txn_id: LocalTxnId,
    pub room_id: RoomId,
    pub session_generation: u64,
    pub kind: AttachmentKind,
    /// Upload/cache handle or mxc — never raw bytes / data: URIs.
    pub media_handle_id: String,
    /// Optional display filename (not path to secrets).
    pub file_name: Option<String>,
    /// Optional plain-text caption (not ciphertext).
    pub caption: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub state: LocalEchoState,
    pub failure_diagnostic_id: Option<&'static str>,
}

impl OutboundAttachment {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            LocalEchoState::Sent | LocalEchoState::Failed | LocalEchoState::Cancelled
        )
    }
}

/// Per-session-generation attachment send queue.
#[derive(Debug, Default)]
pub struct AttachmentSendQueue {
    session_generation: u64,
    order: Vec<LocalTxnId>,
    items: HashMap<LocalTxnId, OutboundAttachment>,
    next_seq: u64,
}

impl AttachmentSendQueue {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            order: Vec::new(),
            items: HashMap::new(),
            next_seq: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.items
            .values()
            .filter(|i| matches!(i.state, LocalEchoState::Sending))
            .count()
    }

    fn alloc_txn_id(&mut self) -> LocalTxnId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("attach-txn-{}", self.next_seq)
    }

    /// Enqueue attachment send by media handle (no bytes).
    pub fn enqueue(&mut self, req: AttachmentEnqueue) -> Result<&OutboundAttachment, SendError> {
        let room_id = req.room_id.trim().to_owned();
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-invalid-room-id",
            });
        }
        let media_handle_id = req.media_handle_id.trim().to_owned();
        validate_handle(&media_handle_id)?;
        if let Some(ref n) = req.file_name {
            if n.chars().count() > MAX_HANDLE_CHARS {
                return Err(SendError::Invalid {
                    diagnostic_id: "p7.4-file-name-cap",
                });
            }
        }
        if let Some(ref c) = req.caption {
            if c.chars().count() > MAX_CAPTION_CHARS {
                return Err(SendError::Invalid {
                    diagnostic_id: "p7.4-caption-cap",
                });
            }
            let lower = c.to_ascii_lowercase();
            if lower.contains("access_token") || lower.contains("refresh_token") {
                return Err(SendError::Invalid {
                    diagnostic_id: "p7.4-forbidden-caption",
                });
            }
        }
        if let Some(sz) = req.size_bytes {
            if sz > 100 * 1024 * 1024 {
                return Err(SendError::Invalid {
                    diagnostic_id: "p7.4-file-too-large",
                });
            }
        }
        if self.active_count() >= MAX_ACTIVE_ATTACHMENTS {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-active-attachment-cap",
            });
        }

        let local_txn_id = self.alloc_txn_id();
        let item = OutboundAttachment {
            local_txn_id: local_txn_id.clone(),
            room_id,
            session_generation: self.session_generation,
            kind: req.kind,
            media_handle_id,
            file_name: req.file_name,
            caption: req.caption,
            mime_type: req.mime_type,
            size_bytes: req.size_bytes,
            state: LocalEchoState::Sending,
            failure_diagnostic_id: None,
        };
        self.order.push(local_txn_id.clone());
        self.items.insert(local_txn_id.clone(), item);
        Ok(self.items.get(&local_txn_id).expect("just inserted"))
    }

    pub fn get(&self, local_txn_id: &str) -> Option<&OutboundAttachment> {
        self.items.get(local_txn_id)
    }

    fn get_mut_checked(
        &mut self,
        local_txn_id: &str,
    ) -> Result<&mut OutboundAttachment, SendError> {
        let item = self
            .items
            .get_mut(local_txn_id)
            .ok_or(SendError::NotFound {
                diagnostic_id: "p7.4-send-not-found",
            })?;
        if item.session_generation != self.session_generation {
            return Err(SendError::StaleGeneration {
                diagnostic_id: "p7.4-stale-send-generation",
                expected: self.session_generation,
                observed: item.session_generation,
            });
        }
        Ok(item)
    }

    pub fn mark_sent(&mut self, local_txn_id: &str) -> Result<&OutboundAttachment, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if item.state != LocalEchoState::Sending {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-mark-sent-invalid-state",
            });
        }
        item.state = LocalEchoState::Sent;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    pub fn mark_failed(
        &mut self,
        local_txn_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<&OutboundAttachment, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if item.state != LocalEchoState::Sending {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-mark-failed-invalid-state",
            });
        }
        item.state = LocalEchoState::Failed;
        item.failure_diagnostic_id = Some(diagnostic_id);
        Ok(item)
    }

    pub fn cancel(&mut self, local_txn_id: &str) -> Result<&OutboundAttachment, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if matches!(item.state, LocalEchoState::Sent | LocalEchoState::Cancelled) {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-cancel-invalid-state",
            });
        }
        item.state = LocalEchoState::Cancelled;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    pub fn retry(&mut self, local_txn_id: &str) -> Result<&OutboundAttachment, SendError> {
        let state = self
            .items
            .get(local_txn_id)
            .ok_or(SendError::NotFound {
                diagnostic_id: "p7.4-send-not-found",
            })?
            .state;
        if state != LocalEchoState::Failed {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-retry-not-failed",
            });
        }
        if self.active_count() >= MAX_ACTIVE_ATTACHMENTS {
            return Err(SendError::Invalid {
                diagnostic_id: "p7.4-active-attachment-cap",
            });
        }
        let item = self.get_mut_checked(local_txn_id)?;
        item.state = LocalEchoState::Sending;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    pub fn list(&self) -> Vec<&OutboundAttachment> {
        self.order
            .iter()
            .filter_map(|id| self.items.get(id))
            .collect()
    }

    pub fn list_for_room(&self, room_id: &str) -> Vec<&OutboundAttachment> {
        self.list()
            .into_iter()
            .filter(|i| i.room_id == room_id)
            .collect()
    }

    pub fn prune_terminal(&mut self) -> usize {
        let before = self.items.len();
        self.order.retain(|id| {
            self.items
                .get(id)
                .map(|i| !i.is_terminal())
                .unwrap_or(false)
        });
        self.items.retain(|_, i| !i.is_terminal());
        before.saturating_sub(self.items.len())
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for item in self.items.values_mut() {
            if item.state == LocalEchoState::Sending {
                item.state = LocalEchoState::Cancelled;
                item.failure_diagnostic_id = Some("p7.4-stale-generation-cancelled");
            }
            item.session_generation = new_generation;
        }
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.items.clear();
    }
}

fn validate_handle(id: &str) -> Result<(), SendError> {
    if id.is_empty() {
        return Err(SendError::Invalid {
            diagnostic_id: "p7.4-empty-media-handle",
        });
    }
    if id.chars().count() > MAX_HANDLE_CHARS {
        return Err(SendError::Invalid {
            diagnostic_id: "p7.4-media-handle-cap",
        });
    }
    let lower = id.to_ascii_lowercase();
    if lower.starts_with("data:") || lower.starts_with("javascript:") {
        return Err(SendError::Invalid {
            diagnostic_id: "p7.4-forbidden-handle-scheme",
        });
    }
    if lower.contains("access_token") || lower.contains("refresh_token") {
        return Err(SendError::Invalid {
            diagnostic_id: "p7.4-forbidden-handle",
        });
    }
    Ok(())
}
