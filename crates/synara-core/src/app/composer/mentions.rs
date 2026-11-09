//! Mentions typed out as text ("@Puck") rather than picked from the list.
//!
//! The send resolves them so the message carries a pill and `m.mentions`
//! either way. Only an unambiguous match becomes a mention; anything else
//! stays text, because mentioning the wrong person is worse than none.

use serde::{Deserialize, Serialize};

/// A room member that typed text may name.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionCandidate {
    pub user_id: String,
    pub display_name: Option<String>,
}

/// One resolved mention. `start` and `end` are UTF-16 offsets into the text,
/// covering the "@" and the name, as both shells index strings that way.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedMention {
    pub start: u32,
    pub end: u32,
    pub user_id: String,
    /// The text the range covers, as typed.
    pub text: String,
}

/// The ways one member can be named after "@": display name, full Matrix ID
/// (which already starts with "@") and localpart.
fn labels(member: &MentionCandidate) -> Vec<Vec<char>> {
    let mut labels: Vec<Vec<char>> = Vec::new();
    let mut push = |label: String| {
        let chars: Vec<char> = label.chars().collect();
        // At least one character after the "@".
        if chars.len() > 1 && !labels.contains(&chars) {
            labels.push(chars);
        }
    };
    if let Some(name) = member.display_name.as_deref().map(str::trim) {
        if !name.is_empty() {
            push(if name.starts_with('@') {
                name.to_owned()
            } else {
                format!("@{name}")
            });
        }
    }
    push(member.user_id.clone());
    if let Some(localpart) = member
        .user_id
        .strip_prefix('@')
        .and_then(|rest| rest.split(':').next())
    {
        push(format!("@{localpart}"));
    }
    labels
}

fn same_letter(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Characters that may come right before an "@" that starts a mention.
fn starts_word(previous: Option<char>) -> bool {
    previous.is_none_or(|c| c.is_whitespace() || c == '(' || c == '"')
}

/// A match must not stop inside a word: "@Al" is not "@Alex".
fn ends_word(next: Option<char>) -> bool {
    next.is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
}

/// Resolve "@name" in `text` against `members`: a display name
/// (case-insensitive, longest match, ending at a word boundary), a full
/// Matrix ID, or a Matrix ID localpart. A name more than one member answers
/// to stays text, and so does a mention of `own_user_id`.
pub fn resolve_typed_mentions(
    text: &str,
    members: &[MentionCandidate],
    own_user_id: &str,
) -> Vec<TypedMention> {
    let chars: Vec<char> = text.chars().collect();
    let member_labels: Vec<(&MentionCandidate, Vec<Vec<char>>)> = members
        .iter()
        .map(|member| (member, labels(member)))
        .collect();
    let mut utf16_at = Vec::with_capacity(chars.len() + 1);
    let mut offset = 0u32;
    for c in &chars {
        utf16_at.push(offset);
        offset += c.len_utf16() as u32;
    }
    utf16_at.push(offset);

    let mut mentions = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '@' || !starts_word(index.checked_sub(1).map(|i| chars[i])) {
            index += 1;
            continue;
        }
        // The longest label any member matches here, and who matches it.
        let mut best_len = 0;
        let mut best: Vec<&str> = Vec::new();
        for (member, labels) in &member_labels {
            for label in labels {
                let end = index + label.len();
                let matches = end <= chars.len()
                    && chars[index..end]
                        .iter()
                        .zip(label)
                        .all(|(a, b)| same_letter(*a, *b))
                    && ends_word(chars.get(end).copied());
                if !matches || label.len() < best_len {
                    continue;
                }
                if label.len() > best_len {
                    best_len = label.len();
                    best.clear();
                }
                if !best.contains(&member.user_id.as_str()) {
                    best.push(&member.user_id);
                }
            }
        }
        if best_len == 0 {
            index += 1;
            continue;
        }
        let end = index + best_len;
        if let [user_id] = best.as_slice() {
            if *user_id != own_user_id {
                mentions.push(TypedMention {
                    start: utf16_at[index],
                    end: utf16_at[end],
                    user_id: (*user_id).to_owned(),
                    text: chars[index..end].iter().collect(),
                });
            }
        }
        index = end;
    }
    mentions
}
