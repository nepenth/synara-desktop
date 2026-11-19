//! Credential-free `in.synara.room_folders` account-data codec.
//!
//! User-defined room folders (custom sidebar sections). One global
//! account-data event holds every folder in display order:
//!
//! ```json
//! { "version": 1, "folders": [ { "id": "…", "name": "…", "rooms": ["!r:s"] } ] }
//! ```
//!
//! This module is pure: parsing, normalization, the mutations and the size
//! check. The serialized read-modify-write and the pending projection that
//! hides stale sync echoes live in `room_folders_live`. Design note:
//! `docs/design/room-folders.md`.
//!
//! Unknown fields (top level and per folder) are kept through a
//! read-modify-write so a newer v1 client's additions survive this client's
//! writes. A `version` above [`ROOM_FOLDERS_ACCOUNT_DATA_VERSION`] is read
//! best-effort and never written.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::MAX_RAW_ACCOUNT_DATA_BYTES;

pub const ROOM_FOLDERS_EVENT_TYPE: &str = "in.synara.room_folders";
pub const ROOM_FOLDERS_ACCOUNT_DATA_VERSION: u32 = 1;
/// Folders kept on read and allowed on create.
pub const MAX_ROOM_FOLDERS: usize = 100;
/// Folder name length after trimming and collapsing whitespace, in characters.
pub const MAX_ROOM_FOLDER_NAME_CHARS: usize = 64;
/// Folder ids are opaque; longer ones are malformed.
pub const MAX_ROOM_FOLDER_ID_BYTES: usize = 64;
/// Largest serialized document this client writes (the raw account-data cap).
pub const MAX_ROOM_FOLDERS_CONTENT_BYTES: usize = MAX_RAW_ACCOUNT_DATA_BYTES;
/// Largest stored payload this client parses. Bigger than the write cap so a
/// slightly oversized document from elsewhere still shows; it can only be
/// written back once a mutation brings it under the write cap.
pub const MAX_ROOM_FOLDERS_READ_BYTES: usize = 4 * MAX_RAW_ACCOUNT_DATA_BYTES;
const MAX_ROOM_ID_BYTES: usize = 255;

/// Privacy-safe room-folders failure. Diagnostic ids are source constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomFoldersError {
    /// Name empty after trimming, or longer than [`MAX_ROOM_FOLDER_NAME_CHARS`].
    InvalidName,
    /// Malformed folder id in a request.
    InvalidFolder,
    /// No folder with that id (it may have been deleted on another device).
    FolderNotFound,
    /// Creating one more would exceed [`MAX_ROOM_FOLDERS`].
    TooManyFolders,
    /// The written document would exceed [`MAX_ROOM_FOLDERS_CONTENT_BYTES`],
    /// or the stored one exceeds [`MAX_ROOM_FOLDERS_READ_BYTES`].
    ContentTooLarge,
    /// The stored document has a newer `version`; this client only reads it.
    UnsupportedVersion,
    /// Malformed room id in a request.
    InvalidRoom,
    /// Only joined rooms can be filed.
    RoomNotJoined,
    /// Direct messages cannot be filed.
    RoomIsDirect,
    /// Spaces cannot be filed.
    RoomIsSpace,
    FetchFailed,
    DeserializeFailed,
    SerializeFailed,
    SetFailed,
}

impl RoomFoldersError {
    pub const fn diagnostic_id(self) -> &'static str {
        match self {
            Self::InvalidName => "v-room-folders.invalid-name",
            Self::InvalidFolder => "v-room-folders.invalid-folder",
            Self::FolderNotFound => "v-room-folders.folder-not-found",
            Self::TooManyFolders => "v-room-folders.too-many-folders",
            Self::ContentTooLarge => "v-room-folders.content-too-large",
            Self::UnsupportedVersion => "v-room-folders.unsupported-version",
            Self::InvalidRoom => "v-room-folders.invalid-room",
            Self::RoomNotJoined => "v-room-folders.room-not-joined",
            Self::RoomIsDirect => "v-room-folders.room-is-direct",
            Self::RoomIsSpace => "v-room-folders.room-is-space",
            Self::FetchFailed => "v-room-folders.fetch-failed",
            Self::DeserializeFailed => "v-room-folders.deserialize-failed",
            Self::SerializeFailed => "v-room-folders.serialize-failed",
            Self::SetFailed => "v-room-folders.set-failed",
        }
    }

    /// True when the request itself was wrong (as opposed to I/O or state
    /// the user cannot fix by changing the request).
    pub const fn is_invalid_request(self) -> bool {
        matches!(
            self,
            Self::InvalidName
                | Self::InvalidFolder
                | Self::FolderNotFound
                | Self::TooManyFolders
                | Self::ContentTooLarge
                | Self::UnsupportedVersion
                | Self::InvalidRoom
                | Self::RoomNotJoined
                | Self::RoomIsDirect
                | Self::RoomIsSpace
        )
    }

    /// Maps a diagnostic id back to its error (FFI and Tauri error mapping).
    pub fn from_diagnostic_id(diagnostic_id: &str) -> Option<Self> {
        [
            Self::InvalidName,
            Self::InvalidFolder,
            Self::FolderNotFound,
            Self::TooManyFolders,
            Self::ContentTooLarge,
            Self::UnsupportedVersion,
            Self::InvalidRoom,
            Self::RoomNotJoined,
            Self::RoomIsDirect,
            Self::RoomIsSpace,
            Self::FetchFailed,
            Self::DeserializeFailed,
            Self::SerializeFailed,
            Self::SetFailed,
        ]
        .into_iter()
        .find(|error| error.diagnostic_id() == diagnostic_id)
    }
}

impl std::fmt::Display for RoomFoldersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.diagnostic_id())
    }
}

impl std::error::Error for RoomFoldersError {}

/// One stored folder, normalized. `extra` keeps fields this client does not
/// know so a write does not drop them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StoredRoomFolder {
    pub id: String,
    pub name: String,
    /// Room ids in the order they were filed.
    pub rooms: Vec<String>,
    extra: Map<String, Value>,
}

impl StoredRoomFolder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            rooms: Vec::new(),
            extra: Map::new(),
        }
    }
}

/// The whole normalized `in.synara.room_folders` document.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomFoldersDocument {
    /// As stored; [`ROOM_FOLDERS_ACCOUNT_DATA_VERSION`] when absent.
    pub version: u32,
    /// Display order.
    pub folders: Vec<StoredRoomFolder>,
    extra: Map<String, Value>,
}

impl Default for RoomFoldersDocument {
    fn default() -> Self {
        Self {
            version: ROOM_FOLDERS_ACCOUNT_DATA_VERSION,
            folders: Vec::new(),
            extra: Map::new(),
        }
    }
}

impl RoomFoldersDocument {
    /// A newer client owns the schema; this one must not write it.
    pub fn is_read_only(&self) -> bool {
        self.version > ROOM_FOLDERS_ACCOUNT_DATA_VERSION
    }

    pub fn folder(&self, folder_id: &str) -> Option<&StoredRoomFolder> {
        self.folders.iter().find(|folder| folder.id == folder_id)
    }

    /// The folder each filed room is in.
    pub fn assignments(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        for folder in &self.folders {
            for room_id in &folder.rooms {
                map.entry(room_id.as_str()).or_insert(folder.id.as_str());
            }
        }
        map
    }

    /// The JSON this client writes: unknown fields first, then the known
    /// ones, which always win.
    pub fn to_json(&self) -> Value {
        let mut object = self.extra.clone();
        object.insert("version".to_owned(), Value::from(self.version));
        let folders = self
            .folders
            .iter()
            .map(|folder| {
                let mut entry = folder.extra.clone();
                entry.insert("id".to_owned(), Value::from(folder.id.clone()));
                entry.insert("name".to_owned(), Value::from(folder.name.clone()));
                entry.insert(
                    "rooms".to_owned(),
                    Value::Array(folder.rooms.iter().cloned().map(Value::from).collect()),
                );
                Value::Object(entry)
            })
            .collect();
        object.insert("folders".to_owned(), Value::Array(folders));
        Value::Object(object)
    }

    /// Serialized size of [`Self::to_json`].
    pub fn encoded_len(&self) -> usize {
        serde_json::to_vec(&self.to_json())
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX)
    }

    pub fn snapshot(&self, session_generation: u64) -> NativeRoomFoldersSnapshot {
        NativeRoomFoldersSnapshot {
            session_generation,
            version: self.version,
            read_only: self.is_read_only(),
            folders: self
                .folders
                .iter()
                .map(|folder| NativeRoomFolder {
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                    room_ids: folder.rooms.clone(),
                })
                .collect(),
        }
    }
}

/// One folder as clients see it.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomFolder {
    pub id: String,
    pub name: String,
    /// Every filed room id as stored (filing order), including rooms that
    /// are currently favorites or no longer listed. For display use the
    /// room-list presentation, which applies the section rule.
    pub room_ids: Vec<String>,
}

/// The stored folders. `read_only` is true when another client wrote a
/// newer `version`; mutations then fail with `v-room-folders.unsupported-version`.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomFoldersSnapshot {
    pub session_generation: u64,
    pub version: u32,
    pub read_only: bool,
    /// Display order.
    pub folders: Vec<NativeRoomFolder>,
}

/// Result of creating a folder: its new id and the folders as written.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomFolderCreateResult {
    pub folder_id: String,
    pub snapshot: NativeRoomFoldersSnapshot,
}

/// Trim, collapse inner whitespace runs to one space. Does not cap length.
pub fn collapse_room_folder_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A name for a create or rename: 1 to [`MAX_ROOM_FOLDER_NAME_CHARS`]
/// characters after [`collapse_room_folder_name`].
pub fn validate_room_folder_name(name: &str) -> Result<String, RoomFoldersError> {
    let name = collapse_room_folder_name(name);
    let length = name.chars().count();
    if length == 0 || length > MAX_ROOM_FOLDER_NAME_CHARS {
        return Err(RoomFoldersError::InvalidName);
    }
    Ok(name)
}

pub fn is_valid_room_folder_id(folder_id: &str) -> bool {
    !folder_id.is_empty()
        && folder_id.len() <= MAX_ROOM_FOLDER_ID_BYTES
        && folder_id
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control())
}

pub fn is_valid_room_folder_room_id(room_id: &str) -> bool {
    room_id.starts_with('!')
        && room_id.len() > 1
        && room_id.len() <= MAX_ROOM_ID_BYTES
        && room_id.chars().all(|character| !character.is_whitespace())
}

fn normalize_folder(
    entry: &Value,
    seen_ids: &mut HashSet<String>,
    seen_rooms: &mut HashSet<String>,
) -> Option<StoredRoomFolder> {
    let object = entry.as_object()?;
    let id = object.get("id")?.as_str()?;
    if !is_valid_room_folder_id(id) || seen_ids.contains(id) {
        return None;
    }
    // Another client's over-long name is shortened rather than dropped, so
    // its rooms stay filed.
    let name: String = collapse_room_folder_name(object.get("name")?.as_str()?)
        .chars()
        .take(MAX_ROOM_FOLDER_NAME_CHARS)
        .collect();
    let name = name.trim_end().to_owned();
    if name.is_empty() {
        return None;
    }
    let rooms = object
        .get("rooms")
        .and_then(Value::as_array)
        .map(|rooms| {
            rooms
                .iter()
                .filter_map(Value::as_str)
                .filter(|room_id| is_valid_room_folder_room_id(room_id))
                // A room is in at most one folder: the first one wins.
                .filter(|room_id| seen_rooms.insert((*room_id).to_owned()))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    seen_ids.insert(id.to_owned());
    let mut extra = object.clone();
    for known in ["id", "name", "rooms"] {
        extra.remove(known);
    }
    Some(StoredRoomFolder {
        id: id.to_owned(),
        name,
        rooms,
        extra,
    })
}

/// Normalize stored content. Never fails: malformed entries are dropped and
/// a non-object is an empty document. Idempotent over its own output.
pub fn normalize_room_folders_content(value: Option<&Value>) -> RoomFoldersDocument {
    let Some(object) = value.and_then(Value::as_object) else {
        return RoomFoldersDocument::default();
    };
    let version = object
        .get("version")
        .and_then(Value::as_u64)
        .filter(|version| *version > u64::from(ROOM_FOLDERS_ACCOUNT_DATA_VERSION))
        .map(|version| u32::try_from(version).unwrap_or(u32::MAX))
        .unwrap_or(ROOM_FOLDERS_ACCOUNT_DATA_VERSION);
    let mut seen_ids = HashSet::new();
    let mut seen_rooms = HashSet::new();
    let folders = object
        .get("folders")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| normalize_folder(entry, &mut seen_ids, &mut seen_rooms))
                .take(MAX_ROOM_FOLDERS)
                .collect()
        })
        .unwrap_or_default();
    let mut extra = object.clone();
    extra.remove("version");
    extra.remove("folders");
    RoomFoldersDocument {
        version,
        folders,
        extra,
    }
}

/// Parse the exact stored bytes, bounding them before JSON parsing.
pub fn parse_room_folders_json(json: &str) -> Result<RoomFoldersDocument, RoomFoldersError> {
    if json.len() > MAX_ROOM_FOLDERS_READ_BYTES {
        return Err(RoomFoldersError::ContentTooLarge);
    }
    let value: Value =
        serde_json::from_str(json).map_err(|_| RoomFoldersError::DeserializeFailed)?;
    Ok(normalize_room_folders_content(Some(&value)))
}

/// The written document must fit the raw account-data cap.
pub fn validate_room_folders_size(document: &RoomFoldersDocument) -> Result<(), RoomFoldersError> {
    if document.encoded_len() > MAX_ROOM_FOLDERS_CONTENT_BYTES {
        return Err(RoomFoldersError::ContentTooLarge);
    }
    Ok(())
}

fn writable(document: RoomFoldersDocument) -> Result<RoomFoldersDocument, RoomFoldersError> {
    if document.is_read_only() {
        return Err(RoomFoldersError::UnsupportedVersion);
    }
    Ok(document)
}

fn folder_index(
    document: &RoomFoldersDocument,
    folder_id: &str,
) -> Result<usize, RoomFoldersError> {
    if !is_valid_room_folder_id(folder_id) {
        return Err(RoomFoldersError::InvalidFolder);
    }
    document
        .folders
        .iter()
        .position(|folder| folder.id == folder_id)
        .ok_or(RoomFoldersError::FolderNotFound)
}

/// Append a new folder `folder_id` named `name`.
pub fn create_room_folder(
    document: RoomFoldersDocument,
    folder_id: &str,
    name: &str,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    let mut next = writable(document)?;
    let name = validate_room_folder_name(name)?;
    if !is_valid_room_folder_id(folder_id) || next.folder(folder_id).is_some() {
        return Err(RoomFoldersError::InvalidFolder);
    }
    if next.folders.len() >= MAX_ROOM_FOLDERS {
        return Err(RoomFoldersError::TooManyFolders);
    }
    next.folders.push(StoredRoomFolder::new(folder_id, name));
    Ok(next)
}

pub fn rename_room_folder(
    document: RoomFoldersDocument,
    folder_id: &str,
    name: &str,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    let mut next = writable(document)?;
    let name = validate_room_folder_name(name)?;
    let index = folder_index(&next, folder_id)?;
    next.folders[index].name = name;
    Ok(next)
}

/// Remove a folder; its rooms become unfiled. Deleting a folder that is
/// already gone (another device deleted it) succeeds without a change.
pub fn delete_room_folder(
    document: RoomFoldersDocument,
    folder_id: &str,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    let mut next = writable(document)?;
    match folder_index(&next, folder_id) {
        Ok(index) => {
            next.folders.remove(index);
            Ok(next)
        }
        Err(RoomFoldersError::FolderNotFound) => Ok(next),
        Err(error) => Err(error),
    }
}

/// Move a folder to `to_index` in display order (clamped to the last slot).
pub fn move_room_folder(
    document: RoomFoldersDocument,
    folder_id: &str,
    to_index: usize,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    let mut next = writable(document)?;
    let from = folder_index(&next, folder_id)?;
    let folder = next.folders.remove(from);
    let to = to_index.min(next.folders.len());
    next.folders.insert(to, folder);
    Ok(next)
}

/// File `room_id` in `folder_id`, or unfile it with `None`. The room leaves
/// any other folder; refiling into its current folder keeps its position.
/// Eligibility (joined, not a DM, not a space) is the live owner's check.
pub fn set_room_folder(
    document: RoomFoldersDocument,
    room_id: &str,
    folder_id: Option<&str>,
) -> Result<RoomFoldersDocument, RoomFoldersError> {
    let mut next = writable(document)?;
    if !is_valid_room_folder_room_id(room_id) {
        return Err(RoomFoldersError::InvalidRoom);
    }
    let target = folder_id
        .map(|folder_id| folder_index(&next, folder_id))
        .transpose()?;
    if let Some(index) = target {
        if next.folders[index].rooms.iter().any(|id| id == room_id) {
            return Ok(next);
        }
    }
    for folder in &mut next.folders {
        folder.rooms.retain(|id| id != room_id);
    }
    if let Some(index) = target {
        next.folders[index].rooms.push(room_id.to_owned());
    }
    Ok(next)
}

/// Drop filed room ids `keep` rejects (rooms the user is no longer in).
pub fn prune_room_folders(
    document: RoomFoldersDocument,
    keep: impl Fn(&str) -> bool,
) -> RoomFoldersDocument {
    let mut next = document;
    for folder in &mut next.folders {
        folder.rooms.retain(|room_id| keep(room_id));
    }
    next
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn doc(value: Value) -> RoomFoldersDocument {
        normalize_room_folders_content(Some(&value))
    }

    fn ids(document: &RoomFoldersDocument) -> Vec<&str> {
        document.folders.iter().map(|f| f.id.as_str()).collect()
    }

    #[test]
    fn missing_or_non_object_content_is_an_empty_v1_document() {
        for value in [None, Some(json!([])), Some(json!("x")), Some(json!({}))] {
            let document = normalize_room_folders_content(value.as_ref());
            assert_eq!(document.version, 1);
            assert!(document.folders.is_empty());
            assert!(!document.is_read_only());
        }
    }

    #[test]
    fn normalizer_drops_malformed_entries_and_trims_names() {
        let document = doc(json!({
            "version": 1,
            "folders": [
                { "id": "a", "name": "  Work   stuff \n", "rooms": ["!r1:s", 7, "bad", "!r2:s", "! space:s"] },
                { "id": "a", "name": "Duplicate id", "rooms": [] },
                { "id": "", "name": "Empty id" },
                { "id": "b", "name": "   " },
                { "id": "c" },
                "not an object",
                { "id": "d", "name": "No rooms field" },
                { "id": "e", "name": "Work stuff", "rooms": "nope" }
            ]
        }));
        assert_eq!(ids(&document), ["a", "d", "e"]);
        assert_eq!(document.folders[0].name, "Work stuff");
        assert_eq!(document.folders[0].rooms, ["!r1:s", "!r2:s"]);
        // Duplicate names are allowed; ids are unique.
        assert_eq!(document.folders[2].name, "Work stuff");
        assert!(document.folders[1].rooms.is_empty());
    }

    #[test]
    fn a_room_in_several_folders_stays_only_in_the_first() {
        let document = doc(json!({
            "version": 1,
            "folders": [
                { "id": "a", "name": "A", "rooms": ["!x:s", "!x:s", "!y:s"] },
                { "id": "b", "name": "B", "rooms": ["!y:s", "!z:s", "!x:s"] }
            ]
        }));
        assert_eq!(document.folders[0].rooms, ["!x:s", "!y:s"]);
        assert_eq!(document.folders[1].rooms, ["!z:s"]);
        let assignments = document.assignments();
        assert_eq!(assignments["!x:s"], "a");
        assert_eq!(assignments["!z:s"], "b");
    }

    #[test]
    fn long_stored_names_are_shortened_and_folders_capped() {
        let long = "n".repeat(MAX_ROOM_FOLDER_NAME_CHARS + 10);
        let folders: Vec<Value> = (0..MAX_ROOM_FOLDERS + 5)
            .map(|index| json!({ "id": format!("f{index}"), "name": long, "rooms": [] }))
            .collect();
        let document = doc(json!({ "version": 1, "folders": folders }));
        assert_eq!(document.folders.len(), MAX_ROOM_FOLDERS);
        assert_eq!(
            document.folders[0].name.chars().count(),
            MAX_ROOM_FOLDER_NAME_CHARS
        );
        assert_eq!(document.folders[MAX_ROOM_FOLDERS - 1].id, "f99");
    }

    #[test]
    fn unknown_fields_survive_a_round_trip_and_normalization_is_idempotent() {
        let document = doc(json!({
            "version": 1,
            "collapsed": ["a"],
            "folders": [{ "id": "a", "name": "A", "rooms": ["!r:s"], "emoji": "📁" }]
        }));
        let written = document.to_json();
        assert_eq!(written["collapsed"], json!(["a"]));
        assert_eq!(written["folders"][0]["emoji"], json!("📁"));
        assert_eq!(doc(written.clone()), document);
        // Known fields win over a stale copy in `extra`.
        let renamed = rename_room_folder(document, "a", "B").unwrap();
        assert_eq!(renamed.to_json()["folders"][0]["name"], json!("B"));
        assert_eq!(
            parse_room_folders_json(&written.to_string()).unwrap(),
            doc(written)
        );
    }

    #[test]
    fn newer_versions_are_read_but_never_written() {
        let document = doc(json!({
            "version": 2,
            "folders": [{ "id": "a", "name": "A", "rooms": ["!r:s"] }]
        }));
        assert!(document.is_read_only());
        assert_eq!(document.version, 2);
        assert_eq!(document.folders.len(), 1);
        assert!(document.snapshot(3).read_only);
        let unsupported = Err(RoomFoldersError::UnsupportedVersion);
        assert_eq!(create_room_folder(document.clone(), "b", "B"), unsupported);
        assert_eq!(rename_room_folder(document.clone(), "a", "B"), unsupported);
        assert_eq!(delete_room_folder(document.clone(), "a"), unsupported);
        assert_eq!(move_room_folder(document.clone(), "a", 0), unsupported);
        assert_eq!(set_room_folder(document, "!r:s", None), unsupported);
        // Version 0, strings and missing versions read as v1.
        for version in [json!(0), json!("2"), Value::Null] {
            assert!(!doc(json!({ "version": version, "folders": [] })).is_read_only());
        }
    }

    #[test]
    fn create_validates_names_ids_and_the_folder_cap() {
        let document =
            create_room_folder(RoomFoldersDocument::default(), "a", "  My   room ").unwrap();
        assert_eq!(document.folders[0].name, "My room");
        assert_eq!(
            create_room_folder(document.clone(), "a", "Again"),
            Err(RoomFoldersError::InvalidFolder)
        );
        assert_eq!(
            create_room_folder(document.clone(), "b", " \t "),
            Err(RoomFoldersError::InvalidName)
        );
        let too_long = "x".repeat(MAX_ROOM_FOLDER_NAME_CHARS + 1);
        assert_eq!(
            create_room_folder(document.clone(), "b", &too_long),
            Err(RoomFoldersError::InvalidName)
        );
        let at_limit = "é".repeat(MAX_ROOM_FOLDER_NAME_CHARS);
        assert!(create_room_folder(document.clone(), "b", &at_limit).is_ok());
        let duplicate_name = create_room_folder(document, "b", "My room").unwrap();
        assert_eq!(ids(&duplicate_name), ["a", "b"]);

        let mut full = RoomFoldersDocument::default();
        for index in 0..MAX_ROOM_FOLDERS {
            full = create_room_folder(full, &format!("f{index}"), "F").unwrap();
        }
        assert_eq!(
            create_room_folder(full, "extra", "F"),
            Err(RoomFoldersError::TooManyFolders)
        );
    }

    fn three() -> RoomFoldersDocument {
        doc(json!({
            "version": 1,
            "folders": [
                { "id": "a", "name": "A", "rooms": ["!1:s", "!2:s"] },
                { "id": "b", "name": "B", "rooms": ["!3:s"] },
                { "id": "c", "name": "C", "rooms": [] }
            ]
        }))
    }

    #[test]
    fn rename_and_delete() {
        let renamed = rename_room_folder(three(), "b", " Bee ").unwrap();
        assert_eq!(renamed.folders[1].name, "Bee");
        assert_eq!(
            rename_room_folder(three(), "zzz", "x"),
            Err(RoomFoldersError::FolderNotFound)
        );
        assert_eq!(
            rename_room_folder(three(), "b", ""),
            Err(RoomFoldersError::InvalidName)
        );
        let deleted = delete_room_folder(three(), "a").unwrap();
        assert_eq!(ids(&deleted), ["b", "c"]);
        // Its rooms are unfiled.
        assert!(!deleted.assignments().contains_key("!1:s"));
        // Already gone: no change, no error.
        assert_eq!(delete_room_folder(three(), "gone").unwrap(), three());
        assert_eq!(
            delete_room_folder(three(), ""),
            Err(RoomFoldersError::InvalidFolder)
        );
    }

    #[test]
    fn move_puts_a_folder_at_the_index_and_clamps() {
        assert_eq!(
            ids(&move_room_folder(three(), "a", 2).unwrap()),
            ["b", "c", "a"]
        );
        assert_eq!(
            ids(&move_room_folder(three(), "c", 0).unwrap()),
            ["c", "a", "b"]
        );
        assert_eq!(
            ids(&move_room_folder(three(), "a", 1).unwrap()),
            ["b", "a", "c"]
        );
        assert_eq!(
            ids(&move_room_folder(three(), "a", 99).unwrap()),
            ["b", "c", "a"]
        );
        assert_eq!(
            ids(&move_room_folder(three(), "b", 1).unwrap()),
            ["a", "b", "c"]
        );
        assert_eq!(
            move_room_folder(three(), "x", 0),
            Err(RoomFoldersError::FolderNotFound)
        );
    }

    #[test]
    fn set_room_keeps_one_folder_per_room() {
        let moved = set_room_folder(three(), "!1:s", Some("b")).unwrap();
        assert_eq!(moved.folders[0].rooms, ["!2:s"]);
        assert_eq!(moved.folders[1].rooms, ["!3:s", "!1:s"]);
        // Refiling into the same folder keeps the position.
        let same = set_room_folder(three(), "!1:s", Some("a")).unwrap();
        assert_eq!(same, three());
        let unfiled = set_room_folder(three(), "!3:s", None).unwrap();
        assert!(unfiled.folders[1].rooms.is_empty());
        // Unfiling an unfiled room is a no-op.
        assert_eq!(set_room_folder(three(), "!9:s", None).unwrap(), three());
        let new_room = set_room_folder(three(), "!9:s", Some("c")).unwrap();
        assert_eq!(new_room.folders[2].rooms, ["!9:s"]);
        assert_eq!(
            set_room_folder(three(), "!1:s", Some("gone")),
            Err(RoomFoldersError::FolderNotFound)
        );
        assert_eq!(
            set_room_folder(three(), "room", Some("a")),
            Err(RoomFoldersError::InvalidRoom)
        );
        for document in [moved, unfiled, new_room] {
            let mut seen = HashSet::new();
            for folder in &document.folders {
                for room in &folder.rooms {
                    assert!(seen.insert(room.clone()), "{room} filed twice");
                }
            }
        }
    }

    #[test]
    fn prune_drops_rooms_the_predicate_rejects() {
        let pruned = prune_room_folders(three(), |room_id| room_id != "!2:s");
        assert_eq!(pruned.folders[0].rooms, ["!1:s"]);
        assert_eq!(pruned.folders.len(), 3);
    }

    #[test]
    fn size_cap_is_the_raw_account_data_cap() {
        let mut document = create_room_folder(RoomFoldersDocument::default(), "a", "A").unwrap();
        assert!(validate_room_folders_size(&document).is_ok());
        let mut index = 0;
        while document.encoded_len() <= MAX_ROOM_FOLDERS_CONTENT_BYTES {
            document.folders[0]
                .rooms
                .push(format!("!{index:0>40}:example.org"));
            index += 1;
        }
        assert_eq!(
            validate_room_folders_size(&document),
            Err(RoomFoldersError::ContentTooLarge)
        );
        document.folders[0].rooms.pop();
        assert!(validate_room_folders_size(&document).is_ok());
    }

    #[test]
    fn parse_bounds_raw_bytes_before_parsing() {
        let padded = format!(
            "{{\"version\":1,{}\"folders\":[]}}",
            " ".repeat(MAX_ROOM_FOLDERS_READ_BYTES)
        );
        assert_eq!(
            parse_room_folders_json(&padded),
            Err(RoomFoldersError::ContentTooLarge)
        );
        assert_eq!(
            parse_room_folders_json("{not json"),
            Err(RoomFoldersError::DeserializeFailed)
        );
    }

    #[test]
    fn diagnostic_ids_round_trip() {
        for error in [
            RoomFoldersError::InvalidName,
            RoomFoldersError::RoomIsDirect,
            RoomFoldersError::SetFailed,
        ] {
            assert_eq!(
                RoomFoldersError::from_diagnostic_id(error.diagnostic_id()),
                Some(error)
            );
        }
        assert_eq!(RoomFoldersError::from_diagnostic_id("other"), None);
        assert!(RoomFoldersError::RoomNotJoined.is_invalid_request());
        assert!(!RoomFoldersError::FetchFailed.is_invalid_request());
    }
}
