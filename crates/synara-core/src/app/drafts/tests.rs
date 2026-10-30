use super::*;

const USER: &str = "@ada:example.org";
const ROOM: &str = "!room:example.org";

fn slot(updated_at: u64, device: &str, markdown: &str) -> SynaraDraftSlot {
    SynaraDraftSlot {
        updated_at,
        device_id: device.to_owned(),
        markdown: Some(markdown.to_owned()),
        ..SynaraDraftSlot::default()
    }
}

fn payload(slots: &[(&str, SynaraDraftSlot)]) -> SynaraDraftsPayload {
    SynaraDraftsPayload {
        v: DRAFTS_PAYLOAD_VERSION,
        slots: slots
            .iter()
            .map(|(key, slot)| ((*key).to_owned(), slot.clone()))
            .collect(),
    }
}

fn sample() -> SynaraDraftsPayload {
    let mut main = slot(1_000, "DEVA", "hello **world**");
    main.reply_to = Some("$reply".into());
    main.desktop_slate = Some(serde_json::json!([{ "type": "paragraph" }]));
    payload(&[
        (MAIN_DRAFT_SLOT, main),
        ("thread:$root", slot(900, "DEVB", "in thread")),
    ])
}

#[test]
fn seal_and_open_round_trip() {
    let key = DraftsKey::generate();
    let original = sample();
    let envelope = seal_payload(&key, USER, ROOM, &original).unwrap();
    assert_eq!(envelope.v, DRAFTS_ENVELOPE_VERSION);
    assert_eq!(envelope.alg, DRAFTS_ALGORITHM);
    assert_eq!(envelope.key_id, key.key_id());
    let json = serde_json::to_string(&envelope).unwrap();
    assert!(
        !json.contains("hello"),
        "the envelope must not leak content"
    );
    let parsed = parse_envelope(&json).unwrap();
    assert_eq!(open_envelope(&key, USER, ROOM, &parsed).unwrap(), original);
}

#[test]
fn every_seal_uses_a_fresh_nonce() {
    let key = DraftsKey::generate();
    let first = seal_payload(&key, USER, ROOM, &sample()).unwrap();
    let second = seal_payload(&key, USER, ROOM, &sample()).unwrap();
    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.ciphertext, second.ciphertext);
}

#[test]
fn ciphertext_is_padded_to_a_bucket() {
    let key = DraftsKey::generate();
    let short = seal_payload(
        &key,
        USER,
        ROOM,
        &payload(&[(MAIN_DRAFT_SLOT, slot(1, "D", "a"))]),
    )
    .unwrap();
    let longer = seal_payload(
        &key,
        USER,
        ROOM,
        &payload(&[(MAIN_DRAFT_SLOT, slot(1, "D", "abcdefgh"))]),
    )
    .unwrap();
    assert_eq!(short.ciphertext.len(), longer.ciphertext.len());
}

#[test]
fn opening_fails_closed_for_wrong_key_room_user_and_tampering() {
    let key = DraftsKey::generate();
    let envelope = seal_payload(&key, USER, ROOM, &sample()).unwrap();

    let other = DraftsKey::generate();
    assert_eq!(
        open_envelope(&other, USER, ROOM, &envelope),
        Err(DraftOpenError::UnknownKey)
    );
    assert_eq!(
        open_envelope(&key, USER, "!other:example.org", &envelope),
        Err(DraftOpenError::Invalid)
    );
    assert_eq!(
        open_envelope(&key, "@eve:example.org", ROOM, &envelope),
        Err(DraftOpenError::Invalid)
    );

    let mut tampered = envelope.clone();
    let mut bytes =
        matrix_sdk::ruma::serde::Base64::<matrix_sdk::ruma::serde::base64::Standard>::parse(
            &tampered.ciphertext,
        )
        .unwrap()
        .into_inner();
    bytes[0] ^= 1;
    tampered.ciphertext =
        matrix_sdk::ruma::serde::Base64::<matrix_sdk::ruma::serde::base64::Standard>::new(bytes)
            .encode();
    assert_eq!(
        open_envelope(&key, USER, ROOM, &tampered),
        Err(DraftOpenError::Invalid)
    );

    let mut future = envelope;
    future.v = 2;
    assert_eq!(
        open_envelope(&key, USER, ROOM, &future),
        Err(DraftOpenError::Unsupported)
    );
}

#[test]
fn oversized_envelope_json_is_rejected_before_parsing() {
    let huge = format!(
        "{{\"v\":1,\"pad\":\"{}\"}}",
        "x".repeat(MAX_DRAFT_ENVELOPE_BYTES)
    );
    assert_eq!(parse_envelope(&huge), Err(DraftOpenError::Invalid));
}

#[test]
fn key_base64_round_trip_and_debug_hides_bytes() {
    let key = DraftsKey::generate();
    let encoded = key.to_base64();
    assert_eq!(DraftsKey::from_base64(&encoded).unwrap(), key);
    assert_eq!(format!("{key:?}"), "DraftsKey(..)");
    assert!(DraftsKey::from_base64("c2hvcnQ").is_err());
    assert!(DraftsKey::from_bytes(&[0; 31]).is_err());
}

#[test]
fn normalize_drops_bad_slots_and_rejects_bad_payloads() {
    let value = serde_json::json!({
        "v": 1,
        "slots": {
            "main": { "updated_at": 5, "device_id": "D", "markdown": "ok" },
            "thread:nope": { "updated_at": 5, "device_id": "D" },
            "thread:$root": { "updated_at": 5, "device_id": "", "markdown": "x" },
            "thread:$r2": { "updated_at": 5, "device_id": "D", "reply_to": "not-an-id" },
            "thread:$r3": { "updated_at": 7, "device_id": "D", "cleared": true, "markdown": "leftover" }
        }
    });
    let payload = normalize_payload(&value).unwrap();
    assert_eq!(
        payload.slots.keys().collect::<Vec<_>>(),
        vec!["main", "thread:$r3"]
    );
    let tombstone = &payload.slots["thread:$r3"];
    assert!(tombstone.cleared);
    assert_eq!(tombstone.markdown, None, "tombstones carry no content");

    assert!(normalize_payload(&serde_json::json!({ "v": 2, "slots": {} })).is_none());
    assert!(normalize_payload(&serde_json::json!({ "v": 1 })).is_none());
    assert!(normalize_payload(&serde_json::json!([])).is_none());
}

#[test]
fn slot_keys_are_main_or_thread_roots() {
    assert!(validate_slot_key("main").is_ok());
    assert!(validate_slot_key("thread:$abc").is_ok());
    assert!(validate_slot_key("thread:").is_err());
    assert!(validate_slot_key("thread:abc").is_err());
    assert!(validate_slot_key("other").is_err());
    assert!(validate_slot_key(&format!("thread:${}", "a".repeat(300))).is_err());
}

#[test]
fn merge_is_last_writer_wins_per_slot() {
    let base = payload(&[
        (MAIN_DRAFT_SLOT, slot(10, "DEVA", "old main")),
        ("thread:$a", slot(50, "DEVA", "newer thread")),
    ]);
    let incoming = payload(&[
        (MAIN_DRAFT_SLOT, slot(20, "DEVB", "new main")),
        ("thread:$a", slot(40, "DEVB", "older thread")),
        ("thread:$b", slot(1, "DEVB", "only incoming")),
    ]);
    let merged = merge_payloads(&base, &incoming);
    assert_eq!(merged.slots["main"].markdown.as_deref(), Some("new main"));
    assert_eq!(
        merged.slots["thread:$a"].markdown.as_deref(),
        Some("newer thread")
    );
    assert_eq!(
        merged.slots["thread:$b"].markdown.as_deref(),
        Some("only incoming")
    );
    // Order-independent.
    assert_eq!(merge_payloads(&incoming, &base), merged);
}

#[test]
fn equal_timestamps_break_ties_by_device_id() {
    let a = slot(10, "DEVA", "a");
    let b = slot(10, "DEVB", "b");
    assert!(b.is_newer_than(&a));
    assert!(!a.is_newer_than(&b));
    assert!(!a.is_newer_than(&a));
}

#[test]
fn lamport_stamp_beats_a_seen_write_from_a_fast_clock() {
    assert_eq!(next_updated_at(1_000, None), 1_000);
    assert_eq!(next_updated_at(1_000, Some(500)), 1_000);
    // A peer's clock runs ahead: our next edit must still be newer.
    assert_eq!(next_updated_at(1_000, Some(9_000)), 9_001);
    assert_eq!(next_updated_at(0, Some(u64::MAX)), u64::MAX);
}

#[test]
fn old_tombstones_are_pruned_and_live_drafts_kept() {
    let now = DRAFT_TOMBSTONE_RETENTION_MS * 2;
    let mut drafts = payload(&[
        ("main", SynaraDraftSlot::tombstone(1, "D")),
        ("thread:$fresh", SynaraDraftSlot::tombstone(now - 10, "D")),
        ("thread:$old", slot(1, "D", "old but live")),
    ]);
    prune_tombstones(&mut drafts, now);
    assert_eq!(
        drafts.slots.keys().collect::<Vec<_>>(),
        vec!["thread:$fresh", "thread:$old"]
    );
}

#[test]
fn cap_leaves_a_small_payload_alone() {
    let mut drafts = sample();
    let before = drafts.clone();
    assert_eq!(cap_payload(&mut drafts, "main", 2_000), CapOutcome::Fits);
    assert_eq!(drafts, before);
}

#[test]
fn cap_drops_rich_text_before_cutting_markdown() {
    let mut main = slot(10, "D", "short text");
    main.desktop_slate = Some(serde_json::json!("x".repeat(MAX_DRAFT_PAYLOAD_BYTES)));
    let mut drafts = payload(&[("main", main)]);
    assert_eq!(
        cap_payload(&mut drafts, "main", 20),
        CapOutcome::DroppedRichText
    );
    let main = &drafts.slots["main"];
    assert_eq!(main.desktop_slate, None);
    assert_eq!(main.markdown.as_deref(), Some("short text"));
    assert!(!main.truncated);
}

#[test]
fn cap_truncates_markdown_on_a_char_boundary_and_marks_it() {
    let text = "é\"\n".repeat(MAX_DRAFT_PAYLOAD_BYTES / 2);
    let other = slot(5, "PEER", "peer thread draft");
    let mut drafts = payload(&[
        ("main", slot(10, "D", &text)),
        ("thread:$peer", other.clone()),
    ]);
    assert_eq!(cap_payload(&mut drafts, "main", 20), CapOutcome::Truncated);
    assert!(serde_json::to_vec(&drafts).unwrap().len() <= MAX_DRAFT_PAYLOAD_BYTES);
    let main = &drafts.slots["main"];
    assert!(main.truncated);
    let cut = main.markdown.as_deref().unwrap();
    assert!(!cut.is_empty() && text.starts_with(cut));
    assert_eq!(
        drafts.slots["thread:$peer"], other,
        "other slots are never cut"
    );
    let key = DraftsKey::generate();
    let envelope = seal_payload(&key, USER, ROOM, &drafts).unwrap();
    assert!(serde_json::to_vec(&envelope).unwrap().len() <= MAX_DRAFT_ENVELOPE_BYTES);
}

#[test]
fn seal_refuses_an_uncapped_payload() {
    let key = DraftsKey::generate();
    let drafts = payload(&[("main", slot(1, "D", &"x".repeat(MAX_DRAFT_PAYLOAD_BYTES)))]);
    assert_eq!(
        seal_payload(&key, USER, ROOM, &drafts),
        Err("drafts-payload-too-large")
    );
}

fn local(updated_at: Option<u64>, dirty: bool, markdown: &str) -> LocalDraftState {
    LocalDraftState {
        updated_at,
        dirty,
        markdown: Some(markdown.to_owned()),
    }
}

#[test]
fn own_echo_and_older_remote_are_kept() {
    let remote = slot(100, "ME", "mine");
    assert_eq!(
        decide_remote(&local(None, true, "x"), &remote, "ME"),
        DraftDecision::Keep
    );
    let remote = slot(100, "PEER", "theirs");
    assert_eq!(
        decide_remote(&local(Some(100), false, ""), &remote, "ME"),
        DraftDecision::Keep
    );
}

#[test]
fn idle_composer_takes_the_newer_remote_draft() {
    let remote = slot(100, "PEER", "from phone");
    assert_eq!(
        decide_remote(&local(Some(50), false, "older"), &remote, "ME"),
        DraftDecision::Apply
    );
    assert_eq!(
        decide_remote(&LocalDraftState::default(), &remote, "ME"),
        DraftDecision::Apply
    );
}

#[test]
fn typing_composer_is_prompted_not_overwritten() {
    let remote = slot(100, "PEER", "from phone");
    assert_eq!(
        decide_remote(&local(Some(50), true, "typing here"), &remote, "ME"),
        DraftDecision::Prompt
    );
}

#[test]
fn local_text_of_unknown_age_is_prompted_not_replaced() {
    let remote = slot(100, "PEER", "from desktop");
    assert_eq!(
        decide_remote(&local(None, false, "typed offline"), &remote, "ME"),
        DraftDecision::Prompt
    );
}

#[test]
fn identical_text_needs_no_prompt() {
    let remote = slot(100, "PEER", "same");
    assert_eq!(
        decide_remote(&local(Some(50), true, " same "), &remote, "ME"),
        DraftDecision::Keep
    );
}

#[test]
fn a_send_elsewhere_clears_an_idle_composer_but_not_new_typing() {
    let tombstone = SynaraDraftSlot::tombstone(100, "PEER");
    assert_eq!(
        decide_remote(&local(Some(50), false, "stale"), &tombstone, "ME"),
        DraftDecision::Apply
    );
    assert_eq!(
        decide_remote(&local(Some(50), true, "something new"), &tombstone, "ME"),
        DraftDecision::Keep
    );
    assert_eq!(
        decide_remote(&local(None, false, ""), &tombstone, "ME"),
        DraftDecision::Keep
    );
    assert_eq!(
        decide_remote(&local(None, false, "unknown age"), &tombstone, "ME"),
        DraftDecision::Keep
    );
}
