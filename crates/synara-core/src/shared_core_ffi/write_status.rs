//! Swift-facing forms of Core's closed write-acknowledgement statuses.
//!
//! Each enum mirrors one `crate::dto` status exhaustively, so a new Core
//! variant fails to compile here instead of reaching Swift as free text.

use super::wire_enum::{wire_enum, wire_enum_from};

wire_enum! {
    /// A write whose only success outcome is `ok`.
    pub enum WriteAckDto {
        Ok => "ok",
    }
}
wire_enum_from!(crate::dto::WriteAck => WriteAckDto { Ok });

wire_enum! {
    /// A send accepted by the homeserver, or still held by the send queue.
    pub enum SendStatusDto {
        Sent => "sent",
        Queued => "queued",
    }
}
wire_enum_from!(crate::dto::SendStatus => SendStatusDto { Sent, Queued });

wire_enum! {
    /// Outcome of an idempotent account-data or space-child mutation.
    pub enum MutationStatusDto {
        Updated => "updated",
        Removed => "removed",
        Skipped => "skipped",
    }
}
wire_enum_from!(crate::dto::MutationStatus => MutationStatusDto { Updated, Removed, Skipped });

wire_enum! {
    /// Freshness of a room-directory search page.
    pub enum DirectorySearchStatusDto {
        Ready => "ready",
        Stale => "stale",
        Cancelled => "cancelled",
    }
}
wire_enum_from!(crate::dto::DirectorySearchStatus => DirectorySearchStatusDto { Ready, Stale, Cancelled });

wire_enum! {
    /// Outcome of adding an email address.
    pub enum ThreepidAddStatusDto {
        Ok => "ok",
        AuthenticationRequired => "authenticationRequired",
    }
}
wire_enum_from!(crate::dto::ThreepidAddStatus => ThreepidAddStatusDto { Ok, AuthenticationRequired });

wire_enum! {
    /// State of the composer reply draft after a write or read.
    pub enum ComposerDraftStatusDto {
        Set => "set",
        Cleared => "cleared",
        Empty => "empty",
    }
}
wire_enum_from!(crate::dto::ComposerDraftStatus => ComposerDraftStatusDto { Set, Cleared, Empty });
