//! Timeline visibility / kind filter (P5.11 harness foundation).
//!
//! Pure product filter over projected item kinds. No SDK timeline, no dual-backend,
//! no event bodies / tokens.

use super::error::TimelineFilterError;

/// Soft cap on selected kinds.
pub const MAX_KINDS: usize = 16;

/// Soft cap on sender filter list.
pub const MAX_SENDERS: usize = 64;

/// Soft cap on sender id length.
pub const MAX_SENDER_CHARS: usize = 255;

/// Product timeline item kind for filtering (not SDK enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineItemKind {
    Message,
    State,
    Membership,
    Reaction,
    Poll,
    Sticker,
    Encrypted,
    Unknown,
}

impl TimelineItemKind {
    pub const ALL: &'static [TimelineItemKind] = &[
        Self::Message,
        Self::State,
        Self::Membership,
        Self::Reaction,
        Self::Poll,
        Self::Sticker,
        Self::Encrypted,
        Self::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::State => "state",
            Self::Membership => "membership",
            Self::Reaction => "reaction",
            Self::Poll => "poll",
            Self::Sticker => "sticker",
            Self::Encrypted => "encrypted",
            Self::Unknown => "unknown",
        }
    }
}

/// One projected item for filter evaluation (ids only; no bodies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterableItem {
    pub event_id: Option<String>,
    pub sender: Option<String>,
    pub kind: TimelineItemKind,
    pub is_local_echo: bool,
    pub is_redacted: bool,
}

/// Product filter state for a room timeline view.
#[derive(Debug, Clone)]
pub struct TimelineFilter {
    /// Empty = allow all kinds.
    pub kinds: Vec<TimelineItemKind>,
    /// Empty = allow all senders.
    pub senders: Vec<String>,
    pub include_local_echo: bool,
    pub include_redacted: bool,
    pub include_encrypted: bool,
}

impl Default for TimelineFilter {
    fn default() -> Self {
        Self {
            kinds: Vec::new(),
            senders: Vec::new(),
            include_local_echo: true,
            include_redacted: true,
            include_encrypted: true,
        }
    }
}

impl TimelineFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_kinds(mut self, kinds: Vec<TimelineItemKind>) -> Result<Self, TimelineFilterError> {
        if kinds.len() > MAX_KINDS {
            return Err(TimelineFilterError::Invalid {
                diagnostic_id: "p5.11-kinds-cap",
            });
        }
        self.kinds = kinds;
        Ok(self)
    }

    pub fn with_senders(mut self, senders: Vec<String>) -> Result<Self, TimelineFilterError> {
        if senders.len() > MAX_SENDERS {
            return Err(TimelineFilterError::Invalid {
                diagnostic_id: "p5.11-senders-cap",
            });
        }
        for s in &senders {
            validate_sender(s)?;
        }
        self.senders = senders;
        Ok(self)
    }

    /// Whether `item` passes this filter.
    pub fn allows(&self, item: &FilterableItem) -> bool {
        if item.is_local_echo && !self.include_local_echo {
            return false;
        }
        if item.is_redacted && !self.include_redacted {
            return false;
        }
        if matches!(item.kind, TimelineItemKind::Encrypted) && !self.include_encrypted {
            return false;
        }
        if !self.kinds.is_empty() && !self.kinds.contains(&item.kind) {
            return false;
        }
        if !self.senders.is_empty() {
            match &item.sender {
                Some(s) if self.senders.iter().any(|a| a == s) => {}
                _ => return false,
            }
        }
        true
    }

    /// Filter a slice; returns indices that pass.
    pub fn select_indices(&self, items: &[FilterableItem]) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter(|(_, it)| self.allows(it))
            .map(|(i, _)| i)
            .collect()
    }
}

fn validate_sender(s: &str) -> Result<(), TimelineFilterError> {
    if s.is_empty() || !s.starts_with('@') || s.chars().count() > MAX_SENDER_CHARS {
        return Err(TimelineFilterError::Invalid {
            diagnostic_id: "p5.11-invalid-sender",
        });
    }
    let lower = s.to_ascii_lowercase();
    if lower.contains("access_token") {
        return Err(TimelineFilterError::Invalid {
            diagnostic_id: "p5.11-forbidden-sender",
        });
    }
    Ok(())
}
