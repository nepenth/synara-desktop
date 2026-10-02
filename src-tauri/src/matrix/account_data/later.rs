#[cfg(test)]
pub use synara_core::app::account_data::{
    clear_completed_later_items, complete_later_item, normalize_later_content, put_later_item,
    snooze_later_item,
};

pub use synara_core::app::account_data::{NativeLaterSnapshot, SynaraLaterItem};

#[cfg(test)]
pub use synara_core::app::account_data::{SynaraLaterContent, SynaraLaterItemKind};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_drops_invalid_and_keeps_privacy_fields_only() {
        let value = json!({
            "version": 1,
            "items": {
                "good": {
                    "id": "!r:s\n$e",
                    "kind": "saved",
                    "roomId": "!r:s",
                    "eventId": "$e",
                    "createdAt": 10.0,
                    "body": "must-not-persist"
                },
                "bad": { "id": 1 }
            }
        });
        let content = normalize_later_content(Some(&value));
        assert_eq!(content.version, 1);
        assert_eq!(content.items.len(), 1);
        let item = content.items.get("good").expect("item");
        assert_eq!(item.kind, SynaraLaterItemKind::Saved);
        let serialized = serde_json::to_value(item).expect("serialize");
        assert!(serialized.get("body").is_none());
    }

    #[test]
    fn mutate_helpers_match_js_semantics() {
        let item = SynaraLaterItem {
            id: "!r:s\n$e".into(),
            kind: SynaraLaterItemKind::Saved,
            room_id: "!r:s".into(),
            event_id: "$e".into(),
            created_at: 1.0,
            due_ts: None,
            reminded_at: None,
            completed_at: None,
        };
        let content = put_later_item(SynaraLaterContent::default(), item);
        let snoozed = snooze_later_item(content.clone(), "!r:s\n$e", 99.0);
        assert_eq!(
            snoozed.items["!r:s\n$e"].kind,
            SynaraLaterItemKind::Reminder
        );
        assert_eq!(snoozed.items["!r:s\n$e"].due_ts, Some(99.0));
        let completed = complete_later_item(snoozed, "!r:s\n$e", 100.0);
        assert_eq!(completed.items["!r:s\n$e"].completed_at, Some(100.0));
        let cleared = clear_completed_later_items(completed);
        assert!(cleared.items.is_empty());
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeLaterSnapshot {
            session_generation: 3,
            content: SynaraLaterContent::default(),
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 3);
        assert_eq!(value["content"]["version"], 1);
        assert_eq!(value["content"]["items"], json!({}));
    }
}
