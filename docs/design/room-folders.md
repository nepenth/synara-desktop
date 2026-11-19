# Room folders (design)

Status: **core and desktop UI implemented** (`feat/room-folders-core`,
`feat/room-folders-desktop`). iOS UI builds on the API below.

User-defined room folders are custom sidebar sections, like Slack's. A user
files rooms into named folders. Folders sync across the user's own devices
and nobody else sees them.

Product rules (fixed):

1. Folders sync across the user's devices through Matrix account data.
2. A room is in at most one folder.
3. Favorites stays a built-in section and wins. A favorite room shows in
   Favorites, not in its folder, but keeps its folder membership:
   unfavoriting it returns it to the folder.
4. Rooms only. Direct messages and spaces cannot be filed.

"Folder" is the user-facing word. Desktop's space switcher has unrelated
*space* folders (`in.synara.spaces`, `useSidebarItems.ts`). Everything here is
named `room_folders` / `RoomFolder` so the two cannot be confused.

## Event format

One global account-data event, `in.synara.room_folders`, plain JSON (not
encrypted, like `in.synara.later` and `in.synara.room_notes`):

```json
{
  "version": 1,
  "folders": [
    { "id": "Xk2pQ9…", "name": "Work", "rooms": ["!a:example.org", "!b:example.org"] },
    { "id": "Lm40Za…", "name": "Side projects", "rooms": [] }
  ]
}
```

- `folders` array order is display order.
- `id` is opaque (this client generates a random transaction-id-style string).
- `rooms` is filing order: a newly filed room is appended.

## Normalization (read side)

`normalize_room_folders_content` (pure, `app/account_data/room_folders.rs`)
never fails:

- Not an object, or no `folders` array: no folders.
- An entry is dropped when it is not an object, has no string `id`, an empty
  id, an id over 64 bytes or with whitespace, a duplicate id, or no non-empty
  `name`.
- Names are trimmed and inner whitespace runs collapse to one space. A stored
  name over 64 characters is shortened, not dropped, so its rooms stay filed.
  Duplicate names are allowed.
- Room ids must start with `!`, have no whitespace and be at most 255 bytes;
  others are dropped. A room listed in several folders stays only in the
  first one (and once in it).
- At most 100 folders are kept.
- Unknown fields, at the top level and per folder, are kept and written back,
  so a newer v1 client's additions survive this client's writes.
- `version` above 1: read best-effort (the same rules), shown, never
  written. Mutations fail with `v-room-folders.unsupported-version` and
  snapshots carry `readOnly: true`. A missing, zero or non-numeric version
  reads as 1.
- Stored payloads over 256 KiB are not parsed (`content-too-large`); the room
  list then shows no folders.

Normalization is idempotent, which the pending projection relies on (an
echo of a write equals the written document).

## Mutations

Each is a read-modify-write under one async lock per session. The read is
`GET /account_data` from the server, not the SDK store, because a PUT does
not reach the store until it syncs back.

| Command | Rule |
|---|---|
| create(name, roomId?) | Name 1–64 characters after trimming. Appends a folder with a new id. Fails at 100 folders. With `roomId`, files that room in the same write. |
| rename(folderId, name) | Same name rule. Unknown id: `folder-not-found`. |
| delete(folderId) | Its rooms become unfiled. An id that is already gone succeeds without a write. |
| move(folderId, toIndex) | Moves to that position; past the end means last. |
| setRoom(roomId, folderId?) | Files the room in that folder, removing it from any other; refiling into its current folder keeps its position. `null` unfiles. Filing rejects rooms the user has not joined (`room-not-joined`), DMs (`room-is-direct`) and spaces (`room-is-space`). Unfiling accepts any valid room id, so stale entries can be removed. |

Every write:

- prunes filed room ids the client knows the user is no longer joined to
  (left, banned, or only invited). Rooms the store does not know yet stay,
  so a write before the first sync cannot wipe folders. Pruning only rides
  along with a real change: a mutation that changes nothing writes nothing,
  and reads never write.
- checks the serialized size against the raw account-data cap (64 KiB,
  `account_data/raw.rs`) and fails with `v-room-folders.content-too-large`
  instead of writing.

Error ids (`RoomFoldersError::diagnostic_id`): `v-room-folders.invalid-name`,
`.invalid-folder`, `.folder-not-found`, `.too-many-folders`,
`.content-too-large`, `.unsupported-version`, `.invalid-room`,
`.room-not-joined`, `.room-is-direct`, `.room-is-space` (request errors), and
`.fetch-failed`, `.deserialize-failed`, `.serialize-failed`, `.set-failed`
(I/O). Without a session: `p2-room-folders-<command>-no-session`.

## Read-after-write and sync echoes

`RoomFoldersState` lives on the session's `SyncServiceOwner`, shared by the
folder commands and room-list snapshots. It uses the room-notes projection
(`image_packs_live.rs`):

- After a successful write, the written document is what every reader sees
  until its sync echo arrives, for at most 30 s.
- A sync event that arrives while a write is in flight, or that does not
  equal the pending write, is ignored. So the delayed echo of an older write
  cannot flip the list back.
- After the window, the SDK store decides.
- A write wakes the room-list owner directly (`RoomFoldersState::changed`), so
  both shells re-read the list before the echo. A folder event from sync
  wakes it too.

## Conflicts between devices

Matrix account data has no conditional write, so the event is whole-document
last-writer-wins. Each write starts from a fresh server read, so ordinary use
(one device at a time, or edits seconds apart) merges correctly: device B's
write includes device A's earlier change.

Two writes inside the same round trip (both devices read the same version,
then both PUT) lose the first writer's change. What the user sees: on the
device whose change lost, the change shows for up to 30 s (the pending
window, because the winning event does not match its pending write), then the
list settles on the winning version on both devices. Nothing is corrupted;
one rename, move or filing has to be redone. The same pending window means a
legitimate change from another device that lands before this device's echo
shows only once the window ends, at the next room-list wake (at worst the
30 s safety pulse), so up to about a minute late.

## Room list

Folders are a separate presentation input, like the space graph. Snapshots
reuse cached rows; folders are applied on top of them every snapshot:

- `RoomSummary.folder_id` (`folderId` on the desktop wire, omitted when
  unfiled; `folder_id` on `RoomListRoomDto`). Set only for joined rooms that
  are neither DMs nor spaces. Favorites keep theirs.
- `RoomListPresentation.folders`: every folder in display order, empty ones
  included, with:
  - `roomIds`: rooms that show in the folder (listed, joined, not DMs or
    spaces, favorites excluded), filing order.
  - `recentOrder` / `nameOrder`: the same rooms in the existing
    `order_room_ids` orders (low priority last). Recent breaks ties in
    snapshot (SDK recency) order, as the Rooms section does.
  - `unreadHighlight`, `unreadTotal`, `unreadRoomIds`: the per-space rollup's
    attention rules (`room_attention`: muted rooms count nothing,
    marked-unread counts one) over `roomIds`, so favorites are not counted.

Filed rooms stay in `presentation.recentOrder`, `nameOrder` and `unread`, and
in the badge totals, unchanged.

### Section rule

One rule, in `app/room_list/folders.rs`, exported to Swift as
`room_section_placement(RoomSectionInput) -> RoomSectionPlacement`:

1. joined and favorite: `Favorites`;
2. else joined, not a DM, not a space, and filed: `Folder { folderId }`;
3. else `Default`: wherever the client puts the room today (Rooms, a space
   group, low priority, direct messages).

A filed low-priority room stays in its folder and sorts last there.

### Cost of a folder change

A folder change (local write or sync) re-projects no room. The global
account-data handler sends a `Folders` wake that marks nothing dirty, so the
next snapshot reuses every cached row and space graph. Its extra work is one
store read and a parse of the folders event (under 64 KiB), a hash lookup per
row for `folder_id`, and ordering the rooms of each folder. Before this
change every global account-data event (folders included) re-projected every
room; other global types still do.

## Client contract

### Desktop (Tauri)

Commands (all return the folders as written, camelCase):

| Command | Arguments | Returns |
|---|---|---|
| `matrix_room_folders_snapshot` | none | `NativeRoomFoldersSnapshot` |
| `matrix_room_folders_create` | `{ name, roomId? }` | `NativeRoomFolderCreateResult` (`folderId`, `snapshot`) |
| `matrix_room_folders_rename` | `{ folderId, name }` | `NativeRoomFoldersSnapshot` |
| `matrix_room_folders_delete` | `{ folderId }` | `NativeRoomFoldersSnapshot` |
| `matrix_room_folders_move` | `{ folderId, toIndex }` | `NativeRoomFoldersSnapshot` |
| `matrix_room_folders_set_room` | `{ roomId, folderId: string \| null }` | `NativeRoomFoldersSnapshot` |

Errors are `MatrixAuthCommandError`: `code` `Forbidden` (no session),
`InvalidRequest` (`diagnosticId` is the `v-room-folders.*` id) or `Unknown`.

Rendering reads the room-list snapshot, not the folder snapshot: a room in
`presentation.folders[i].roomIds` renders in that folder; otherwise the
existing rules apply (favorites, then default sections). Use the folder's
`recentOrder` or `nameOrder` for the section's sort and its `unread*` fields
for the header badge. The folder snapshot is for management UI (names,
order, read-only state). Types are generated in
`synara/src/app/features/matrix-dto/generated/` (`NativeRoomFolder`,
`NativeRoomFoldersSnapshot`, `NativeRoomFolderCreateResult`,
`RoomFolderPresentation`, `RoomListPresentation.folders`).

### iOS (UniFFI)

- `SharedCore.roomFoldersSnapshot()`, `roomFoldersCreate(name:roomId:)`,
  `roomFoldersRename(folderId:name:)`, `roomFoldersDelete(folderId:)`,
  `roomFoldersMove(folderId:toIndex:)`, `roomFoldersSetRoom(roomId:folderId:)`
  return `RoomFoldersSnapshotDto` (`RoomFolderCreateDto` for create) and throw
  `RoomFoldersCommandError.Failed(code, description)` with the same codes.
- `RoomListSnapshotDto.folders: [RoomFolderSectionDto]` carries the resolved
  sections (`roomIds`, `recentOrder`, `nameOrder`, `unreadHighlight`,
  `unreadTotal`, `unreadRoomIds`); `RoomListRoomDto.folderId` the membership.
- `roomSectionPlacement(input:)` is the rule for one row, for code paths that
  build sections from rows.

## Limits

| Limit | Value |
|---|---|
| Folders | 100 |
| Name | 1–64 characters after trimming and collapsing whitespace |
| Folder id | 1–64 bytes, no whitespace or control characters |
| Written document | 64 KiB serialized (about 1,500 filed rooms) |
| Parsed document | 256 KiB |
| Pending window | 30 s |

## Desktop UI

What a desktop user sees:

- **Sections.** Home lists Favorites, then one section per folder in folder
  order, then Rooms, then Direct messages. A room in a folder is not in
  Rooms. Every folder shows on Home, also one with no rooms there, as a single
  quiet "No rooms" line, so it can be found and managed. A space lists
  Favorites, then each folder that holds rooms of that space (with only those
  rooms), then the space's own groups; a folder with none of its rooms is not
  shown there. A favorite shows in Favorites; it stays in its folder and goes
  back to it when it stops being a favorite.
- **Order.** Rooms in a folder follow the Rooms section's sort (Recent
  activity or Name; in a space, the space's sort). Folders have no sort of
  their own.
- **Collapse.** A folder collapses like any section, per device
  (`makeNavCategoryId('home', 'folder:<id>')`, or the space's id instead of
  `home`), and keeps its unread and selected rooms visible. Only while it is
  collapsed its header shows its unread: the red mention count, otherwise the
  quiet dot.
- **Managing folders.** The header's ⋮ (on hover or focus) and right-click
  offer Rename, Move up, Move down and Delete folder. Delete asks first and
  says the rooms go back to Rooms. Home options (⋮ beside the Home title)
  has New folder. There is no drag and drop.
- **Filing a room.** A room's menu has Move to folder: every folder, with a
  check on the room's current one (favorites too), New folder… (creates the
  folder with the room in it, one command) and Remove from folder. Direct
  messages and spaces do not have it.
- **Naming.** New folder and Rename share one dialog: Enter saves, Escape
  cancels, and a failed save says why under the field (too many folders,
  folders too large to save, a name that is empty or over 64 characters).
- **Read-only.** While another client's newer format is stored
  (`readOnly`), folders still show but every management action is hidden.
- **Keyboard.** Previous/next room and previous/next unread walk the sections
  in display order, folders included.
- **Unread totals.** Home, space and Direct messages totals do not change
  when a room moves into or out of a folder.

Code: `synara/src/app/features/room-folders/` (the command owner
`nativeRoomFolderOwner.ts`, the section model `roomFolderSections.ts`, the
header, room-menu submenu and dialogs); Home's sections in
`pages/client/home/homeRoomList.ts`. Every section, Favorites included, is
(on Home) its own virtual list over the pane's one scroller
(`features/room-nav/VirtualRoomSection.tsx`), so a folder of hundreds of
rooms renders only the rows on screen while every section header stays
mounted. Browser proof: `synara/e2e/room-folders.spec.ts` on the room-list
harness (`?roomFolders`).

## Not done here

- iOS UI.
- Room ids the store never learned about (rooms left long ago and forgotten)
  are kept until the user unfiles them; they are hidden from the list.
- The room-notes projection in `image_packs_live.rs` is the same algorithm
  for a different type; it was left as is rather than made generic.
