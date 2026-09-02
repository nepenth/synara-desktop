//! Live `in.synara.room_notes` Client RMW owned by the shared native core.

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde::Deserialize;
use serde_json::value::to_raw_value;
use serde_json::value::RawValue as RawJsonValue;

use super::{
    complete_room_todo_item, limit_text, move_room_todo_item, normalize_room_notes_content_checked,
    put_room_note_item, remove_room_note_item, validate_room_note_mutation_target,
    validate_room_notes_content_size, NativeRoomNotesSnapshot, RoomNoteMoveDirection,
    SynaraRoomNoteItem, SynaraRoomNoteItemKind, SynaraRoomNotesContent, MAX_MESSAGE_BODY_LENGTH,
    MAX_NOTE_BODY_LENGTH, MAX_ROOM_NOTES_CONTENT_BYTES, MAX_SENDER_LENGTH, ROOM_NOTES_EVENT_TYPE,
};

fn room_notes_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(ROOM_NOTES_EVENT_TYPE)
}

pub(super) fn parse_room_notes_content(
    raw: Option<Raw<AnyGlobalAccountDataEventContent>>,
) -> Result<SynaraRoomNotesContent, &'static str> {
    let Some(raw) = raw else {
        return Ok(SynaraRoomNotesContent::default());
    };
    // Bound the exact server/store payload before JSON parsing. Measuring a
    // reserialized Value is insufficient because whitespace and duplicate
    // keys can canonicalize to a much smaller representation.
    if raw.json().get().len() > MAX_ROOM_NOTES_CONTENT_BYTES {
        return Err("v-timeline-room-notes-payload-too-large");
    }
    let value = raw
        .deserialize_as_unchecked::<serde_json::Value>()
        .map_err(|_| "v-timeline-room-notes-deserialize-failed")?;
    normalize_room_notes_content_checked(Some(&value))
}

#[derive(Deserialize)]
struct RawRoomNotesSyncEvent {
    content: Box<RawJsonValue>,
}

pub(super) fn parse_room_notes_sync_event(
    raw_event: &RawJsonValue,
) -> Result<SynaraRoomNotesContent, &'static str> {
    // Bound the envelope before parsing it. The content is then independently
    // checked against the exact account-data limit by parse_room_notes_content.
    const MAX_SYNC_EVENT_OVERHEAD_BYTES: usize = 256;
    if raw_event.get().len()
        > MAX_ROOM_NOTES_CONTENT_BYTES.saturating_add(MAX_SYNC_EVENT_OVERHEAD_BYTES)
    {
        return Err("v-timeline-room-notes-payload-too-large");
    }
    let event: RawRoomNotesSyncEvent = serde_json::from_str(raw_event.get())
        .map_err(|_| "v-timeline-room-notes-deserialize-failed")?;
    parse_room_notes_content(Some(Raw::from_json(event.content)))
}

async fn load_cached_room_notes_content(
    client: &Client,
) -> Result<SynaraRoomNotesContent, &'static str> {
    let raw = client
        .account()
        .account_data_raw(room_notes_event_type())
        .await
        .map_err(|_| "v-timeline-room-notes-load-failed")?;
    parse_room_notes_content(raw)
}

async fn fetch_fresh_room_notes_content(
    client: &Client,
) -> Result<SynaraRoomNotesContent, &'static str> {
    let raw = client
        .account()
        // A successful set_account_data_raw does not update the SDK state
        // store. Fetch from the homeserver for every serialized RMW so an
        // immediate second mutation cannot reload a pre-write /sync snapshot.
        .fetch_account_data(room_notes_event_type())
        .await
        .map_err(|_| "v-timeline-room-notes-fetch-failed")?;
    parse_room_notes_content(raw)
}

async fn store_room_notes_content(
    client: &Client,
    content: &SynaraRoomNotesContent,
) -> Result<(), &'static str> {
    validate_room_notes_content_size(content)?;
    let raw_value = to_raw_value(content).map_err(|_| "v-timeline-room-notes-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(room_notes_event_type(), raw)
        .await
        .map_err(|_| "v-timeline-room-notes-set-failed")?;
    Ok(())
}

pub async fn snapshot_room_notes(
    client: &Client,
    session_generation: u64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    Ok(NativeRoomNotesSnapshot {
        session_generation,
        content: load_cached_room_notes_content(client).await?,
    })
}

async fn mutate_room_notes<F>(
    client: &Client,
    session_generation: u64,
    mutate: F,
) -> Result<NativeRoomNotesSnapshot, &'static str>
where
    F: FnOnce(SynaraRoomNotesContent) -> SynaraRoomNotesContent,
{
    let next = mutate(fetch_fresh_room_notes_content(client).await?);
    store_room_notes_content(client, &next).await?;
    Ok(NativeRoomNotesSnapshot {
        session_generation,
        content: next,
    })
}

fn validate_note_item(item: &SynaraRoomNoteItem) -> Result<(), &'static str> {
    validate_room_note_mutation_target(&item.room_id, &item.id)?;
    if item.event_id.as_ref().is_some_and(|event_id| {
        !event_id.starts_with('$')
            || event_id.len() <= 1
            || event_id.chars().any(char::is_whitespace)
    }) || item
        .sender
        .as_ref()
        .is_some_and(|sender| sender.is_empty() || sender.chars().count() > MAX_SENDER_LENGTH)
    {
        return Err("v-timeline-room-notes-invalid-item");
    }
    if !item.created_at.is_finite() || !item.updated_at.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    match item.kind {
        SynaraRoomNoteItemKind::Note | SynaraRoomNoteItemKind::Todo => {
            if item.body.as_ref().is_none_or(|b| b.is_empty()) {
                return Err("v-timeline-room-notes-invalid-item");
            }
        }
        SynaraRoomNoteItemKind::Message => {
            if item.event_id.as_ref().is_none_or(|e| e.is_empty()) {
                return Err("v-timeline-room-notes-invalid-item");
            }
        }
    }
    Ok(())
}

pub async fn upsert_room_note_item(
    client: &Client,
    session_generation: u64,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    validate_note_item(&item)?;
    let mut item = item;
    if let Some(body) = item.body.take() {
        let capped = limit_text(
            &body,
            if item.kind == SynaraRoomNoteItemKind::Message {
                MAX_MESSAGE_BODY_LENGTH
            } else {
                MAX_NOTE_BODY_LENGTH
            },
        );
        item.body = if capped.is_empty() {
            None
        } else {
            Some(capped)
        };
    }
    validate_note_item(&item)?;
    mutate_room_notes(client, session_generation, |content| {
        put_room_note_item(content, item)
    })
    .await
}

pub async fn delete_room_note_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    validate_room_note_mutation_target(&room_id, &item_id)?;
    mutate_room_notes(client, session_generation, |content| {
        remove_room_note_item(content, &room_id, &item_id)
    })
    .await
}

pub async fn complete_room_todo_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
    completed: bool,
    now: f64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    validate_room_note_mutation_target(&room_id, &item_id)?;
    if !now.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    mutate_room_notes(client, session_generation, |content| {
        complete_room_todo_item(content, &room_id, &item_id, completed, now)
    })
    .await
}

pub async fn move_room_todo_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
    now: f64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    validate_room_note_mutation_target(&room_id, &item_id)?;
    if !now.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    mutate_room_notes(client, session_generation, |content| {
        move_room_todo_item(content, &room_id, &item_id, direction, now)
    })
    .await
}

pub fn room_notes_now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use matrix_sdk::{
        authentication::matrix::MatrixSession,
        ruma::{OwnedDeviceId, UserId},
        store::RoomLoadSettings,
        Client, SessionMeta, SessionTokens,
    };
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::{
        complete_room_todo_item_live, delete_room_note_item_live, move_room_todo_item_live,
        mutate_room_notes, parse_room_notes_content, parse_room_notes_sync_event,
        put_room_note_item, upsert_room_note_item, AnyGlobalAccountDataEventContent, Raw,
        RoomNoteMoveDirection, SynaraRoomNoteItem, SynaraRoomNoteItemKind,
        MAX_ROOM_NOTES_CONTENT_BYTES,
    };
    use crate::app::account_data::{MAX_NOTE_ID_LENGTH, MAX_ROOM_ID_BYTES};

    fn raw_account_data(json: String) -> Raw<AnyGlobalAccountDataEventContent> {
        Raw::from_json(serde_json::value::RawValue::from_string(json).expect("valid raw JSON"))
    }

    #[test]
    fn rejects_oversized_raw_whitespace_before_deserializing() {
        let payload = format!(
            "{{\"version\":1,{}\"rooms\":{{}}}}",
            " ".repeat(MAX_ROOM_NOTES_CONTENT_BYTES)
        );
        assert_eq!(
            parse_room_notes_content(Some(raw_account_data(payload))),
            Err("v-timeline-room-notes-payload-too-large")
        );
    }

    #[test]
    fn rejects_oversized_duplicate_keys_before_canonicalization() {
        let duplicate = "\"padding\":0,";
        let payload = format!(
            "{{\"version\":1,{}\"rooms\":{{}}}}",
            duplicate.repeat(MAX_ROOM_NOTES_CONTENT_BYTES / duplicate.len() + 1)
        );
        assert_eq!(
            parse_room_notes_content(Some(raw_account_data(payload))),
            Err("v-timeline-room-notes-payload-too-large")
        );
    }

    #[test]
    fn synchronized_event_content_is_bounded_and_version_checked() {
        let supported = serde_json::value::RawValue::from_string(
            r#"{"type":"in.synara.room_notes","content":{"version":1,"rooms":{}}}"#.to_owned(),
        )
        .expect("valid sync event");
        assert_eq!(
            parse_room_notes_sync_event(&supported),
            Ok(super::SynaraRoomNotesContent::default())
        );

        let unknown = serde_json::value::RawValue::from_string(
            r#"{"type":"in.synara.room_notes","content":{"version":2,"rooms":{}}}"#.to_owned(),
        )
        .expect("valid sync event");
        assert_eq!(
            parse_room_notes_sync_event(&unknown),
            Err("v-timeline-room-notes-unsupported-version")
        );
    }

    async fn read_http_request(socket: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = socket.read(&mut chunk).await.expect("read request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }
        request
    }

    fn note(id: &str) -> SynaraRoomNoteItem {
        SynaraRoomNoteItem {
            id: id.to_owned(),
            kind: SynaraRoomNoteItemKind::Note,
            room_id: "!notes:example.org".to_owned(),
            created_at: 1.0,
            updated_at: 1.0,
            body: Some(id.to_owned()),
            completed_at: None,
            order: None,
            event_id: None,
            event_ts: None,
            sender: None,
        }
    }

    #[tokio::test]
    async fn every_live_mutation_rejects_invalid_targets_before_network_io() {
        let client = Client::builder()
            .homeserver_url("http://127.0.0.1:9")
            .build()
            .await
            .expect("build isolated client");
        let room_over_limit = format!("!{}", "r".repeat(MAX_ROOM_ID_BYTES));
        let item_over_limit = "n".repeat(MAX_NOTE_ID_LENGTH + 1);

        let mut invalid_upsert = note("note");
        invalid_upsert.room_id = room_over_limit.clone();
        assert_eq!(
            upsert_room_note_item(&client, 1, invalid_upsert).await,
            Err("v-timeline-room-notes-invalid-item")
        );
        assert_eq!(
            delete_room_note_item_live(&client, 1, room_over_limit, "note".to_owned()).await,
            Err("v-timeline-room-notes-invalid-item")
        );
        assert_eq!(
            complete_room_todo_item_live(
                &client,
                1,
                "!room:example.org".to_owned(),
                item_over_limit.clone(),
                true,
                1.0,
            )
            .await,
            Err("v-timeline-room-notes-invalid-item")
        );
        assert_eq!(
            move_room_todo_item_live(
                &client,
                1,
                "!room:example.org".to_owned(),
                item_over_limit,
                RoomNoteMoveDirection::Down,
                1.0,
            )
            .await,
            Err("v-timeline-room-notes-invalid-item")
        );
    }

    #[tokio::test]
    async fn immediate_mutations_refetch_server_state_before_the_next_write() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback homeserver");
        let address = listener.local_addr().expect("loopback address");
        let server_content = Arc::new(Mutex::new(json!({ "version": 1, "rooms": {} })));
        let observed_content = Arc::clone(&server_content);
        let server = tokio::spawn(async move {
            let expected_methods = ["GET", "PUT", "GET", "PUT"];
            let mut account_data_request_index = 0;
            while account_data_request_index < expected_methods.len() {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("accept account-data request");
                let request = read_http_request(&mut socket).await;
                let header_end = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|index| index + 4)
                    .expect("HTTP headers");
                let request_headers = String::from_utf8_lossy(&request[..header_end]);
                let request_line = request_headers.lines().next().expect("HTTP request line");
                if request_line.starts_with("GET /_matrix/client/versions ") {
                    let response_body = r#"{"versions":["v1.11"],"unstable_features":{}}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("write versions response");
                    continue;
                }

                // Restoring a Matrix session can lazily probe secret-storage
                // account data before the write. It is unrelated to this
                // route and must not advance the notes GET/PUT assertion.
                if !request_line.contains("/account_data/in.synara.room_notes") {
                    assert!(
                        request_line.starts_with("GET "),
                        "unexpected auxiliary request: {request_line}"
                    );
                    let response_body = r#"{"errcode":"M_NOT_FOUND","error":"Not found"}"#;
                    let response = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("write auxiliary response");
                    continue;
                }

                let expected_method = expected_methods[account_data_request_index];
                assert!(
                    request_line.starts_with(expected_method),
                    "expected {expected_method}, received {request_line}"
                );
                account_data_request_index += 1;

                let response_body = if expected_method == "GET" {
                    serde_json::to_string(&*observed_content.lock().expect("server content"))
                        .expect("encode server content")
                } else {
                    let value: serde_json::Value = serde_json::from_slice(&request[header_end..])
                        .expect("decode account-data write");
                    *observed_content.lock().expect("server content") = value;
                    "{}".to_owned()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket
                    .write_all(response.as_bytes())
                    .await
                    .expect("write account-data response");
            }
        });

        let client = Client::builder()
            .homeserver_url(format!("http://{address}"))
            .build()
            .await
            .expect("build loopback client");
        client
            .matrix_auth()
            .restore_session(
                MatrixSession {
                    meta: SessionMeta {
                        user_id: UserId::parse("@alice:example.org").expect("user id"),
                        device_id: OwnedDeviceId::from("DEVICE"),
                    },
                    tokens: SessionTokens {
                        access_token: "test-access-token".to_owned(),
                        refresh_token: None,
                    },
                },
                RoomLoadSettings::default(),
            )
            .await
            .expect("restore loopback session");

        let first = note("first");
        mutate_room_notes(&client, 1, |content| put_room_note_item(content, first))
            .await
            .expect("first mutation");
        let second = note("second");
        let result = mutate_room_notes(&client, 1, |content| put_room_note_item(content, second))
            .await
            .expect("second mutation");
        assert_eq!(result.content.rooms["!notes:example.org"].items.len(), 2);

        server.await.expect("loopback account-data server");
        let stored = server_content.lock().expect("final server content").clone();
        let items = stored["rooms"]["!notes:example.org"]["items"]
            .as_object()
            .expect("stored items");
        assert!(items.contains_key("first"));
        assert!(items.contains_key("second"));
    }
}
