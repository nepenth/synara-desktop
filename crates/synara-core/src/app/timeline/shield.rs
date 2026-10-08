//! Per-event authenticity shields for encrypted rooms.
//!
//! The SDK owns the trust decision (`EventTimelineItem::get_shield`). Core only
//! copies its closed tone and code; the SDK's free-text message is not copied.

use matrix_sdk_ui::timeline::{TimelineEventShieldState, TimelineEventShieldStateCode};
use serde::{Deserialize, Serialize};

/// Non-strict shields, as Element X uses them. In strict mode every
/// unverified sender is flagged, which is noise for an account that has not
/// verified anyone yet. Non-strict still flags identity changes, mismatched
/// senders, unknown devices and plaintext in an encrypted room.
pub const STRICT_SHIELDS: bool = false;

/// How strongly a presenter should flag an event's authenticity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineShieldTone {
    /// Something is wrong with the sender's trust.
    Red,
    /// Authenticity is unknown or weaker than usual.
    Grey,
}

impl TimelineShieldTone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Grey => "grey",
        }
    }
}

/// Closed reason for a shield, mirroring the SDK's machine-readable code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineShieldCode {
    AuthenticityNotGuaranteed,
    UnknownDevice,
    UnsignedDevice,
    UnverifiedIdentity,
    VerificationViolation,
    MismatchedSender,
    SentInClear,
}

impl TimelineShieldCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthenticityNotGuaranteed => "authenticity_not_guaranteed",
            Self::UnknownDevice => "unknown_device",
            Self::UnsignedDevice => "unsigned_device",
            Self::UnverifiedIdentity => "unverified_identity",
            Self::VerificationViolation => "verification_violation",
            Self::MismatchedSender => "mismatched_sender",
            Self::SentInClear => "sent_in_clear",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEncryptionShield {
    pub tone: TimelineShieldTone,
    pub code: TimelineShieldCode,
}

pub fn project_encryption_shield(
    state: TimelineEventShieldState,
) -> Option<TimelineEncryptionShield> {
    let (tone, code) = match state {
        TimelineEventShieldState::None => return None,
        TimelineEventShieldState::Red { code } => (TimelineShieldTone::Red, code),
        TimelineEventShieldState::Grey { code } => (TimelineShieldTone::Grey, code),
    };
    let code = match code {
        TimelineEventShieldStateCode::AuthenticityNotGuaranteed => {
            TimelineShieldCode::AuthenticityNotGuaranteed
        }
        TimelineEventShieldStateCode::UnknownDevice => TimelineShieldCode::UnknownDevice,
        TimelineEventShieldStateCode::UnsignedDevice => TimelineShieldCode::UnsignedDevice,
        TimelineEventShieldStateCode::UnverifiedIdentity => TimelineShieldCode::UnverifiedIdentity,
        TimelineEventShieldStateCode::VerificationViolation => {
            TimelineShieldCode::VerificationViolation
        }
        TimelineEventShieldStateCode::MismatchedSender => TimelineShieldCode::MismatchedSender,
        TimelineEventShieldStateCode::SentInClear => TimelineShieldCode::SentInClear,
    };
    Some(TimelineEncryptionShield { tone, code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_events_have_no_shield() {
        assert_eq!(
            project_encryption_shield(TimelineEventShieldState::None),
            None
        );
    }

    #[test]
    fn tone_and_code_are_copied_from_the_sdk() {
        let cases = [
            (
                TimelineEventShieldState::Red {
                    code: TimelineEventShieldStateCode::VerificationViolation,
                },
                TimelineShieldTone::Red,
                TimelineShieldCode::VerificationViolation,
            ),
            (
                TimelineEventShieldState::Red {
                    code: TimelineEventShieldStateCode::MismatchedSender,
                },
                TimelineShieldTone::Red,
                TimelineShieldCode::MismatchedSender,
            ),
            (
                TimelineEventShieldState::Grey {
                    code: TimelineEventShieldStateCode::AuthenticityNotGuaranteed,
                },
                TimelineShieldTone::Grey,
                TimelineShieldCode::AuthenticityNotGuaranteed,
            ),
            (
                TimelineEventShieldState::Grey {
                    code: TimelineEventShieldStateCode::UnknownDevice,
                },
                TimelineShieldTone::Grey,
                TimelineShieldCode::UnknownDevice,
            ),
            (
                TimelineEventShieldState::Grey {
                    code: TimelineEventShieldStateCode::UnsignedDevice,
                },
                TimelineShieldTone::Grey,
                TimelineShieldCode::UnsignedDevice,
            ),
            (
                TimelineEventShieldState::Grey {
                    code: TimelineEventShieldStateCode::UnverifiedIdentity,
                },
                TimelineShieldTone::Grey,
                TimelineShieldCode::UnverifiedIdentity,
            ),
            (
                TimelineEventShieldState::Grey {
                    code: TimelineEventShieldStateCode::SentInClear,
                },
                TimelineShieldTone::Grey,
                TimelineShieldCode::SentInClear,
            ),
        ];
        for (state, tone, code) in cases {
            assert_eq!(
                project_encryption_shield(state),
                Some(TimelineEncryptionShield { tone, code })
            );
        }
    }

    #[test]
    fn shields_serialize_as_closed_snake_case_values() {
        let json = serde_json::to_value(TimelineEncryptionShield {
            tone: TimelineShieldTone::Red,
            code: TimelineShieldCode::VerificationViolation,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"tone": "red", "code": "verification_violation"})
        );
        for code in [
            TimelineShieldCode::AuthenticityNotGuaranteed,
            TimelineShieldCode::UnknownDevice,
            TimelineShieldCode::UnsignedDevice,
            TimelineShieldCode::UnverifiedIdentity,
            TimelineShieldCode::VerificationViolation,
            TimelineShieldCode::MismatchedSender,
            TimelineShieldCode::SentInClear,
        ] {
            assert_eq!(serde_json::to_value(code).unwrap(), code.as_str());
        }
    }

    // Strict mode would flag every unverified sender; keep the documented choice.
    const _: () = assert!(!STRICT_SHIELDS);
}
