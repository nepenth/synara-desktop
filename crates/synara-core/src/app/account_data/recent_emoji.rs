//! Element-compatible `io.element.recent_emoji` usage list.
//!
//! The content is `{ "recent_emoji": [[emoji, count], ...] }`, most recently
//! used first. Desktop records uses in `synara/src/app/plugins/recent-emoji.ts`
//! (`nextRecentEmoji`); this is the same rule, so iOS and desktop share one
//! list: a use moves the emoji to the front with its count plus one, and the
//! list keeps at most 100 entries. Other keys in the content are kept.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use super::RawAccountDataError;

pub const RECENT_EMOJI_EVENT_TYPE: &str = "io.element.recent_emoji";
pub const MAX_RECENT_EMOJI: usize = 100;
/// Longest emoji key accepted for a recorded use, in UTF-8 bytes. Real
/// emoji (including ZWJ sequences) are far shorter.
pub const MAX_RECENT_EMOJI_KEY_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRecentEmojiEntry {
    pub emoji: String,
    pub count: u64,
}

/// The stored list, most recently used first. Callers sort by count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRecentEmoji {
    pub session_generation: u64,
    pub entries: Vec<NativeRecentEmojiEntry>,
}

/// The well-formed `[emoji, count]` entries of `content`, in stored order.
/// Malformed entries are skipped, as desktop does.
pub fn recent_emoji_entries(content: Option<&JsonValue>) -> Vec<NativeRecentEmojiEntry> {
    let Some(list) = content
        .and_then(|content| content.get("recent_emoji"))
        .and_then(JsonValue::as_array)
    else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let emoji = pair.first()?.as_str()?;
            let count = pair.get(1).and_then(usage_count)?;
            (!emoji.is_empty()).then(|| NativeRecentEmojiEntry {
                emoji: emoji.to_owned(),
                count,
            })
        })
        .collect()
}

fn usage_count(value: &JsonValue) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|count| count.is_finite() && *count >= 0.0)
            .map(|count| count as u64)
    })
}

/// `content` after one more use of `emoji`: the emoji moves to the front
/// with its count plus one (or 1), capped at 100 entries. Other keys stay.
pub fn record_recent_emoji(
    content: Option<JsonValue>,
    emoji: &str,
) -> Result<JsonValue, RawAccountDataError> {
    if emoji.is_empty()
        || emoji.len() > MAX_RECENT_EMOJI_KEY_BYTES
        || emoji.chars().any(char::is_control)
    {
        return Err(RawAccountDataError::InvalidContent);
    }
    let mut entries = recent_emoji_entries(content.as_ref());
    let entry = match entries.iter().position(|entry| entry.emoji == emoji) {
        Some(index) => {
            let mut entry = entries.remove(index);
            entry.count = entry.count.saturating_add(1);
            entry
        }
        None => NativeRecentEmojiEntry {
            emoji: emoji.to_owned(),
            count: 1,
        },
    };
    entries.insert(0, entry);
    entries.truncate(MAX_RECENT_EMOJI);

    let mut content = match content {
        Some(JsonValue::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    content.insert(
        "recent_emoji".to_owned(),
        JsonValue::Array(
            entries
                .into_iter()
                .map(|entry| json!([entry.emoji, entry.count]))
                .collect(),
        ),
    );
    Ok(JsonValue::Object(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_a_use_moves_it_to_the_front_and_counts_it() {
        let content = json!({ "recent_emoji": [["😂", 2], ["👍", 5]], "other": true });

        let next = record_recent_emoji(Some(content), "👍").unwrap();
        assert_eq!(
            next,
            json!({ "recent_emoji": [["👍", 6], ["😂", 2]], "other": true }),
            "matches desktop nextRecentEmoji and keeps other keys"
        );

        let next = record_recent_emoji(Some(next), "🎉").unwrap();
        assert_eq!(
            recent_emoji_entries(Some(&next))
                .into_iter()
                .map(|entry| (entry.emoji, entry.count))
                .collect::<Vec<_>>(),
            vec![
                ("🎉".to_owned(), 1),
                ("👍".to_owned(), 6),
                ("😂".to_owned(), 2)
            ]
        );
    }

    #[test]
    fn recording_starts_a_list_and_caps_it_at_one_hundred() {
        let first = record_recent_emoji(None, "👍").unwrap();
        assert_eq!(first, json!({ "recent_emoji": [["👍", 1]] }));

        let full: Vec<JsonValue> = (0..MAX_RECENT_EMOJI)
            .map(|index| json!([format!("e{index}"), 1]))
            .collect();
        let next = record_recent_emoji(Some(json!({ "recent_emoji": full })), "🆕").unwrap();
        let entries = recent_emoji_entries(Some(&next));
        assert_eq!(entries.len(), MAX_RECENT_EMOJI);
        assert_eq!(entries[0].emoji, "🆕");
        assert_eq!(
            entries.last().unwrap().emoji,
            "e98",
            "the oldest entry drops off"
        );
    }

    #[test]
    fn malformed_entries_are_skipped_and_bad_keys_rejected() {
        let content =
            json!({ "recent_emoji": [["👍", 2], "x", [1, 2], ["", 3], ["😮", -1], ["🙏", 1.0]] });
        let entries = recent_emoji_entries(Some(&content));
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.emoji.as_str())
                .collect::<Vec<_>>(),
            vec!["👍", "🙏"]
        );
        assert!(recent_emoji_entries(Some(&json!({ "recent_emoji": "nope" }))).is_empty());
        assert!(recent_emoji_entries(None).is_empty());

        assert_eq!(
            record_recent_emoji(None, ""),
            Err(RawAccountDataError::InvalidContent)
        );
        assert_eq!(
            record_recent_emoji(None, &"a".repeat(MAX_RECENT_EMOJI_KEY_BYTES + 1)),
            Err(RawAccountDataError::InvalidContent)
        );
        assert_eq!(
            record_recent_emoji(None, "a\nb"),
            Err(RawAccountDataError::InvalidContent)
        );
    }
}
