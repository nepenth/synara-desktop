//! Custom Synara raw-content extraction (P5.9 harness foundation).
//!
//! Extracts **allowlisted** fields from host-provided content maps for agent /
//! custom event types. Never stores full unfiltered JSON blobs. Rejects secret
//! keys. Unknown fields may be preserved as short string values under a cap.

use std::collections::BTreeMap;

use super::error::RawContentError;

/// Soft caps.
pub const MAX_FIELDS: usize = 64;
pub const MAX_KEY_LEN: usize = 128;
pub const MAX_VALUE_LEN: usize = 4_096;
pub const MAX_UNKNOWN_FIELDS: usize = 32;

/// Event types treated as Synara agent / custom product events.
pub const SYNARA_AGENT_EVENT_PREFIX: &str = "dev.synara.";
pub const MATRIX_CUSTOM_MSGTYPE_PREFIX: &str = "dev.synara.";

/// Scalar content value (no nested objects / arrays of objects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentValue {
    Null,
    Bool(bool),
    Integer(i64),
    FloatBits(u64), // f64 bits for Eq; host maps numbers
    String(String),
}

impl ContentValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Integer(_) => "integer",
            Self::FloatBits(_) => "float",
            Self::String(_) => "string",
        }
    }
}

/// Extracted product view of custom event content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedContent {
    pub event_type: String,
    /// Allowlisted / primary fields.
    pub fields: BTreeMap<String, ContentValue>,
    /// Unknown keys preserved as short strings only (lossy).
    pub unknown: BTreeMap<String, String>,
}

impl ExtractedContent {
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.fields.get(key).and_then(|v| v.as_str())
    }

    pub fn is_agent_event(&self) -> bool {
        self.event_type.starts_with(SYNARA_AGENT_EVENT_PREFIX)
    }
}

/// Keys that must never appear in extracted content (case-insensitive match).
const FORBIDDEN_KEY_SUBSTR: &[&str] = &[
    "access_token",
    "refresh_token",
    "password",
    "private_key",
    "secret",
    "session_key",
    "recovery_key",
    "macaroon",
];

/// Default allowlist for Synara agent message-like content.
pub const DEFAULT_AGENT_ALLOWLIST: &[&str] = &[
    "body",
    "msgtype",
    "format",
    "formatted_body",
    "agent_id",
    "agent_name",
    "run_id",
    "tool_name",
    "status",
    "summary",
    "m.relates_to",
];

/// Session-generation-stamped extractor configuration + last extracts cache.
#[derive(Debug, Clone)]
pub struct RawContentExtractor {
    session_generation: u64,
    allowlist: Vec<String>,
    /// Optional last extract per (event_type) for smoke — not a full event store.
    last_by_type: BTreeMap<String, ExtractedContent>,
}

impl RawContentExtractor {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            allowlist: DEFAULT_AGENT_ALLOWLIST
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            last_by_type: BTreeMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn allowlist(&self) -> &[String] {
        &self.allowlist
    }

    pub fn set_allowlist(
        &mut self,
        keys: impl IntoIterator<Item = String>,
    ) -> Result<(), RawContentError> {
        let mut list = Vec::new();
        for k in keys {
            validate_key(&k)?;
            list.push(k);
        }
        if list.is_empty() || list.len() > MAX_FIELDS {
            return Err(RawContentError::Invalid {
                diagnostic_id: "p5.9-invalid-allowlist",
            });
        }
        self.allowlist = list;
        Ok(())
    }

    /// Extract from a flat host content map (string keys → ContentValue).
    pub fn extract(
        &mut self,
        event_type: impl Into<String>,
        content: BTreeMap<String, ContentValue>,
    ) -> Result<ExtractedContent, RawContentError> {
        let event_type = event_type.into();
        validate_event_type(&event_type)?;
        if content.len() > MAX_FIELDS * 2 {
            return Err(RawContentError::Invalid {
                diagnostic_id: "p5.9-content-too-large",
            });
        }

        let mut fields = BTreeMap::new();
        let mut unknown = BTreeMap::new();

        for (key, value) in content {
            validate_key(&key)?;
            if is_forbidden_key(&key) {
                return Err(RawContentError::ForbiddenField {
                    diagnostic_id: "p5.9-forbidden-field",
                });
            }
            if let ContentValue::String(ref s) = value {
                validate_value_str(s)?;
            }
            if self.allowlist.iter().any(|a| a == &key) {
                if fields.len() >= MAX_FIELDS {
                    return Err(RawContentError::Invalid {
                        diagnostic_id: "p5.9-field-cap",
                    });
                }
                fields.insert(key, value);
            } else if unknown.len() < MAX_UNKNOWN_FIELDS {
                // Preserve unknown as short string projection only.
                let s = match value {
                    ContentValue::String(s) => truncate_str(s, MAX_VALUE_LEN),
                    ContentValue::Bool(b) => b.to_string(),
                    ContentValue::Integer(i) => i.to_string(),
                    ContentValue::FloatBits(bits) => format!("{}", f64::from_bits(bits)),
                    ContentValue::Null => "null".to_owned(),
                };
                validate_value_str(&s)?;
                unknown.insert(key, s);
            }
        }

        let extracted = ExtractedContent {
            event_type: event_type.clone(),
            fields,
            unknown,
        };
        self.last_by_type.insert(event_type, extracted.clone());
        Ok(extracted)
    }

    /// Round-trip helper: rebuild a content map from extracted (allowlisted + unknown).
    pub fn reassemble(extracted: &ExtractedContent) -> BTreeMap<String, ContentValue> {
        let mut out = BTreeMap::new();
        for (k, v) in &extracted.fields {
            out.insert(k.clone(), v.clone());
        }
        for (k, s) in &extracted.unknown {
            out.insert(k.clone(), ContentValue::String(s.clone()));
        }
        out
    }

    pub fn last(&self, event_type: &str) -> Option<&ExtractedContent> {
        self.last_by_type.get(event_type)
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.last_by_type.clear();
    }
}

fn validate_event_type(event_type: &str) -> Result<(), RawContentError> {
    if event_type.is_empty() || event_type.len() > MAX_KEY_LEN {
        return Err(RawContentError::Invalid {
            diagnostic_id: "p5.9-invalid-event-type",
        });
    }
    if event_type.chars().any(|c| c.is_control()) {
        return Err(RawContentError::Invalid {
            diagnostic_id: "p5.9-invalid-event-type",
        });
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), RawContentError> {
    if key.is_empty() || key.len() > MAX_KEY_LEN {
        return Err(RawContentError::Invalid {
            diagnostic_id: "p5.9-invalid-key",
        });
    }
    if key.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(RawContentError::Invalid {
            diagnostic_id: "p5.9-invalid-key",
        });
    }
    Ok(())
}

fn validate_value_str(s: &str) -> Result<(), RawContentError> {
    if s.len() > MAX_VALUE_LEN {
        return Err(RawContentError::Invalid {
            diagnostic_id: "p5.9-value-too-long",
        });
    }
    let lower = s.to_ascii_lowercase();
    if lower.contains("access_token=")
        || lower.contains("refresh_token=")
        || lower.contains("-----begin")
    {
        return Err(RawContentError::ForbiddenField {
            diagnostic_id: "p5.9-forbidden-value",
        });
    }
    Ok(())
}

fn is_forbidden_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    FORBIDDEN_KEY_SUBSTR.iter().any(|f| lower.contains(f))
}

fn truncate_str(s: String, max: usize) -> String {
    if s.len() <= max {
        s
    } else {
        s.chars().take(max).collect()
    }
}
