//! Closed status vocabularies for write acknowledgements and similar readbacks.
//!
//! Each enum serializes to the exact wire string the desktop renderer and
//! fixtures already use, so JSON output is unchanged. Rust callers and tests
//! keep the string spelling through `as_str` and `PartialEq<&str>`.

use serde::{Deserialize, Serialize};

macro_rules! status_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $wire:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $wire)] $variant ),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$( $name::$variant ),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $wire ),+
                }
            }

            pub fn from_wire(value: &str) -> Option<Self> {
                match value {
                    $( $wire => Some($name::$variant), )+
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }
    };
}

status_enum! {
    /// A write whose only success outcome is `ok`.
    pub enum WriteAck {
        Ok => "ok",
    }
}

status_enum! {
    /// Outcome of a send: accepted by the homeserver, or still held by the
    /// SDK send queue after a recoverable failure.
    pub enum SendStatus {
        Sent => "sent",
        Queued => "queued",
    }
}

status_enum! {
    /// Outcome of an idempotent account-data or space-child mutation.
    pub enum MutationStatus {
        Updated => "updated",
        Removed => "removed",
        Skipped => "skipped",
    }
}

status_enum! {
    /// Freshness of a room-directory search page.
    pub enum DirectorySearchStatus {
        Ready => "ready",
        Stale => "stale",
        Cancelled => "cancelled",
    }
}

status_enum! {
    /// Outcome of adding an email address.
    pub enum ThreepidAddStatus {
        Ok => "ok",
        AuthenticationRequired => "authenticationRequired",
    }
}

status_enum! {
    /// State of the composer reply draft after a write or read.
    pub enum ComposerDraftStatus {
        Set => "set",
        Cleared => "cleared",
        Empty => "empty",
    }
}

status_enum! {
    /// Outcome of re-enabling a queued local echo.
    pub enum LocalEchoRetryStatus {
        Retrying => "retrying",
    }
}

status_enum! {
    /// Availability of a URL preview.
    pub enum MediaPreviewStatus {
        Ok => "ok",
        Skipped => "skipped",
        Unavailable => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_serialize_to_their_existing_wire_strings() {
        for status in WriteAck::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in SendStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in MutationStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in DirectorySearchStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in ThreepidAddStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in ComposerDraftStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in LocalEchoRetryStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        for status in MediaPreviewStatus::ALL {
            assert_eq!(serde_json::to_value(status).unwrap(), status.as_str());
        }
        assert_eq!(
            serde_json::from_str::<ThreepidAddStatus>("\"authenticationRequired\"").unwrap(),
            ThreepidAddStatus::AuthenticationRequired
        );
        assert!(serde_json::from_str::<SendStatus>("\"failed\"").is_err());
        assert_eq!(SendStatus::from_wire("queued"), Some(SendStatus::Queued));
        assert_eq!(SendStatus::from_wire("unknown"), None);
    }
}
