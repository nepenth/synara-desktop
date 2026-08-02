# V-ROOMS.R-POWERS-BULK — native permissions implementation packet

| Field    | Value                                                                                                                                  |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Status   | **Implementation packet** — this PR is docs-only and does not claim the vertical is implemented                                        |
| Residual | **V-ROOMS.R-POWERS-BULK** from **#377**                                                                                                |
| Base     | `52953091` on `feature/matrix-rust-sdk-full-replacement` (current feature tip after #424)                                    |
| PR shape | Focused **draft** PR targeting `feature/matrix-rust-sdk-full-replacement`                                                              |
| Policy   | [full-vertical-policy.md](full-vertical-policy.md): native UI → Tauri IPC → live `matrix-sdk`, physical JS-owner deletion, fail-closed |
| Guard    | Never `main`, umbrella **#39**, or V-BURN/#327; **CallWidget #407 owns `product.rs` until its parent merge**; `dual_backend` is forbidden; powers-bulk is next after #407 |

The source inventory is
[v-rooms-power-levels-residual.md](v-rooms-power-levels-residual.md). This
packet freezes the implementation boundary for the two settings-editor writes
left by #377: custom power-level tags and the complete `m.room.power_levels`
state event. It is not product implementation and does not claim powers-bulk
implemented. CallWidget **#407** still owns the serial
`src-tauri/src/matrix/auth/product.rs` until its parent merge lands. Powers-bulk
is the next product slice after #407 and has not started at this tip. This docs
PR does not edit `src-tauri/src/matrix/auth/product.rs`.

> **Serial ownership note at `52953091`.** This packet is a docs-only draft
> handoff. Do not start powers-bulk or edit `product.rs` until the parent
> CallWidget merge releases #407's serial ownership; then revalidate the
> managed session boundary before starting product implementation.

## 1. Objective and completion bar

Replace both common Permissions write owners with one native-only operating
path:

```text
Room/Space Permissions UI
  → nativeRoomPowerLevelsOwner
  → exact Tauri IPC below
  → the managed live matrix-sdk client
  → state-event send
  → native typed readback/ack
  → editor success and retained UI state
```

The implementation slice must preserve:

- custom tag create, edit, delete, name, color, and icon association;
- room and space permission groups using one complete `m.room.power_levels`
  replacement per Apply action;
- the pending multi-change and Reset/Apply behavior in `PermissionGroups`;
- all power-level fields and map entries retained by the current event content;
- native session and command failures as visible unavailable/error states; and
- no JS SDK write or desktop JS fallback when the native session is live.

This packet closes the **write** residual only. Merged **#450** now provides
native power-level/creator read projection and native permission-gate ownership
for the migrated paths. The custom power-level-tag read and direct helper/plugin
readers remain the named dependency of V-ROOMS.MEMBERS-READ. Do not delete
shared read hooks or claim the whole V-ROOMS power vertical closed from this
write packet alone.

## 2. Frozen scope and prerequisites

### In scope

- `PowersEditor` writes of the complete
  `in.synara.room.power_level_tags` content.
- `PermissionGroups` writes of the complete `m.room.power_levels` content.
- Both `room-settings/permissions/Permissions.tsx` and
  `space-settings/permissions/Permissions.tsx` through their common editor
  components.
- A typed TypeScript native owner, exact command argument/result validation,
  native readback checks, source-absence tests, and focused IPC contract tests.

### Required preflight

Before product implementation starts, the writer must verify:

1. `HEAD` is exactly `52953091` or the
   approved integration tip that explicitly includes it.
2. The PR target is
   `feature/matrix-rust-sdk-full-replacement`, never `main` or #39.
3. The managed native session exposes one live `matrix-sdk` client for the
   current session; no second Matrix client or selector is introduced.
4. The adjacent #375 single-user command remains separate. Neither bulk
   editor may be implemented as repeated `matrix_room_set_power_level` calls.
5. `matrix_upload_media` remains the owner of tag-icon upload transport. This
   packet owns the tag-state association, not a second media uploader.

If a prerequisite is false, stop and escalate. Do not add a fallback, rename
the commands, or widen this packet.

### Explicit non-goals

- `PowerChip` single-user writes, invite/kick/ban/unban, or member-list reads;
- native member/power-level snapshots, creator projection, or a new permission
  calculation API owned by this packet;
- generic developer `StateEventEditor` writes;
- tag-icon upload transport or authenticated media URL construction;
- changes to `main`, #39, V-BURN/#327, or `dual_backend`; and
- any edit to `src-tauri/src/matrix/auth/product.rs` in this docs-only PR.

## 3. Exact IPC contract

These command names and wire shapes are frozen. Do not add aliases, a generic
state-event command, an `eventType`/`stateKey` escape hatch, or a patch-command
alternative.

| Exact command                      | Request               | Required result                                                                                      |
| ---------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------- |
| `matrix_room_set_power_levels`     | `{ roomId, content }` | `NativePowerLevelWriteResult` with `eventType: "m.room.power_levels"` and `stateKey: ""`             |
| `matrix_room_set_power_level_tags` | `{ roomId, content }` | `NativePowerLevelWriteResult` with `eventType: "in.synara.room.power_level_tags"` and `stateKey: ""` |

The frontend wire field names are camelCase. `roomId` must be a non-empty
Matrix room ID. The command fixes the event type and empty state key from its
name; callers cannot select another state event.

### 3.1 Common result and failure contract

Every successful command returns this exact semantic shape:

```ts
type NativePowerLevelWriteResult<TContent> = {
  status: "ok";
  roomId: string;
  eventType: "m.room.power_levels" | "in.synara.room.power_level_tags";
  stateKey: "";
  sessionGeneration: number;
  content: TContent;
};
```

The returned `roomId`, fixed `eventType`, empty `stateKey`, and
`sessionGeneration` must match the command and active native session. `content`
is a native readback of the state after the send, not an echo produced before
the homeserver/native room state accepts the event. The owner reports success
only after all fields match the submitted content under the canonical JSON
comparison used by the IPC contract tests.

Native errors use the repository's safe Matrix IPC error envelope with a stable
category and diagnostic ID. They do not include access tokens, refresh tokens,
passwords, crypto material, raw SDK error strings, or arbitrary state content in
logs. Unavailable session, logged-out session, invalid content, permission denial,
send failure, stale generation, missing readback, mismatched room/event/state
key, or malformed result is terminal for the operation.

### 3.2 `m.room.power_levels` content

`matrix_room_set_power_levels` receives the complete state content, not a
single-user patch:

```ts
type PowerLevel = number; // finite, integral, JSON-safe Matrix integer
type PowerLevelMap = Record<string, PowerLevel>;

type RoomPowerLevelsContent = {
  ban?: PowerLevel;
  events?: PowerLevelMap;
  events_default?: PowerLevel;
  historical?: PowerLevel;
  invite?: PowerLevel;
  kick?: PowerLevel;
  notifications?: PowerLevelMap;
  redact?: PowerLevel;
  state_default?: PowerLevel;
  users?: PowerLevelMap;
  users_default?: PowerLevel;
  // Other existing JSON members are preserved and validated as JSON values;
  // the full event must not be narrowed to only the editor's visible fields.
};
```

The native DTO may represent the final comment as a serde-flattened validated
map, but it must preserve unknown existing top-level members when the editor
passes them through. Numeric power values are finite safe integers. `users`,
`events`, and `notifications` preserve every existing key/value not edited by
the UI. The command sends exactly one complete `m.room.power_levels` state
event for one Apply action.

The implementation must not reconstruct the content from defaults, discard
`historical`, replace maps wholesale, or turn the request into repeated
`matrix_room_set_power_level` calls.

### 3.3 `in.synara.room.power_level_tags` content

`matrix_room_set_power_level_tags` receives the complete custom-tag map:

```ts
type PowerLevelTagIcon = {
  key?: string;
  info?: {
    w?: number;
    h?: number;
    mimetype?: string;
    size?: number;
    "xyz.amorgan.blurhash"?: string;
  };
};

type PowerLevelTag = {
  name: string;
  color?: string;
  icon?: PowerLevelTagIcon;
};

// Numeric Matrix power levels are object keys on the JSON wire.
type PowerLevelTags = Record<string, PowerLevelTag>;
```

Create and edit add or replace one numeric key in the complete map. Delete
omits that key from the complete map. An empty map is valid and represents an
empty tag state; deletion is not encoded as an unrelated event type or a
special sentinel. The native validator rejects non-object content, non-numeric
power keys, empty tag names, non-finite metadata, invalid icon shapes, and
unbounded values according to the repository's existing IPC payload limits.

The `icon.key`/`icon.info` association is preserved exactly. Uploaded icon
bytes still use the existing `matrix_upload_media` owner and are never sent in
the power-level IPC payload.

### 3.4 Native owner semantics

Add one SDK-neutral owner at:

`synara/src/app/features/common-settings/permissions/nativeRoomPowerLevelsOwner.ts`

The owner must:

1. require the desktop/native environment and a `logged_in`
   `matrix_session_snapshot` before either write;
2. validate `roomId` and the complete content before invoking Tauri;
3. invoke only the exact command associated with the editor;
4. reject unavailable or malformed results, including a missing or stale
   readback; and
5. return a typed native result or throw a safe unavailable/error result.

It must not return a `legacy` sentinel, inspect an `isNative ? rust : js`
selector, call `mx.sendStateEvent`, call `mx.setStateEvent`, or retry through a
second backend. On every native failure the owner makes zero calls to a legacy
JS writer.

The common editors keep their current local draft behavior. The native owner
is called only at Apply/Save; local Reset and Cancel remain local operations.
`PermissionGroups` sends the current full `editedPowerLevels` once. `PowersEditor`
sends the current full tag map once. A successful native result is the only
condition that clears the pending draft.

## 4. Physical deletion list

The following JS ownership is removed in the same product implementation
slice. “Delete from path” means the live writer/import/compatibility branch is
physically absent; the component itself remains where it still provides
SDK-neutral presentation or the separately tracked read behavior.

| Path                                                                                | Delete from path                                                                                                                                              | Retain or replace with                                                                                                                              |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/features/common-settings/permissions/PowersEditor.tsx`              | `mx.sendStateEvent(room.roomId, StateEvent.PowerLevelTags, content)` and the write-only `StateEvent.PowerLevelTags` import/use                                | `setRoomPowerLevelTagsWithNativeOwner(room.roomId, content, ...)`; retain tag editing, local drafts, and the existing native media-upload transport |
| `synara/src/app/features/common-settings/permissions/PermissionGroups.tsx`          | `mx.sendStateEvent(room.roomId, StateEvent.RoomPowerLevels, editedPowerLevels)`, the write-only `useMatrixClient`, and the write-only `StateEvent` import/use | `setRoomPowerLevelsWithNativeOwner(room.roomId, editedPowerLevels, ...)`; retain staged multi-change calculation and Reset/Apply UI                 |
| `synara/src/app/features/room-settings/permissions/Permissions.tsx`                 | No direct writer remains after rewiring; remove only imports/branches made obsolete by the native write owner                                                 | The route continues to mount the common native-owned editors; its read/permission dependency remains with V-ROOMS.MEMBERS-READ                      |
| `synara/src/app/features/space-settings/permissions/Permissions.tsx`                | No direct writer remains after rewiring; remove only imports/branches made obsolete by the native write owner                                                 | The route continues to mount the common native-owned editors; its read/permission dependency remains with V-ROOMS.MEMBERS-READ                      |
| `synara/src/app/features/common-settings/permissions/nativeRoomPowerLevelsOwner.ts` | New file in the implementation PR; no JS SDK imports or fallback branch                                                                                       | The sole desktop write owner for both exact commands                                                                                                |

Do **not** delete in this packet:

- `usePowerLevels.ts`, `usePowerLevelTags.ts`, `useRoomPermissions.ts`, or
  `useRoomCreators.ts`; they have read consumers and are covered by the named
  V-ROOMS.MEMBERS-READ dependency.
- `PowerChip.tsx` or the #375 moderation owner;
- `createUploadAtom`, `CompactUploadCardRenderer`, or `matrix_upload_media`;
- a hidden `LegacyPowersEditor`, compatibility callback, or desktop selector;
- any test that proves SDK-neutral retained behavior; or
- `src-tauri/src/matrix/auth/product.rs` in this docs-only PR.

The implementation PR must include a negative source scan proving that the two
common editor runtime files contain none of `sendStateEvent`, `setStateEvent`,
or a JS fallback for these two state writes. Repository-wide JS Matrix imports
may remain nonzero because the read residual and unrelated verticals are still
open; that is not permission to retain either bulk writer.

## 5. Required focused tests

### 5.1 Native owner tests

Add:

`synara/src/app/features/common-settings/permissions/__tests__/nativeRoomPowerLevelsOwner.test.ts`

Use an injected invoke harness and assert exact command names, arguments, and
result validation for:

1. logged-in native preflight followed by
   `matrix_room_set_power_levels` with one complete room policy;
2. logged-in native preflight followed by
   `matrix_room_set_power_level_tags` with a complete tag map;
3. room and space route calls using the same room-scoped owner;
4. preservation of `users`, `events`, `notifications`, `historical`, unknown
   retained policy members, tag metadata, icon key, and icon info;
5. create/edit/delete tag semantics, including delete-by-omission and an empty
   tag map;
6. required native readback/ack before the owner resolves success, including
   matching `roomId`, fixed event type, empty state key, content, and session
   generation;
7. desktop unavailable, logged-out, missing command, invoke rejection,
   malformed result, failed send, missing readback, mismatched readback, and
   stale-generation cases becoming visible unavailable/error results; and
8. every native failure making zero calls to any legacy JS writer. The owner
   must reject rather than return a `legacy` sentinel.

### 5.2 Source-absence tests

Add:

`synara/src/app/features/common-settings/permissions/__tests__/powerLevelsSourceGuard.test.ts`

The guard reads `PowersEditor.tsx`, `PermissionGroups.tsx`, and both Permissions
routes and asserts:

- no `mx.sendStateEvent`/`sendStateEvent` or `mx.setStateEvent` call remains for
  either power-level state event;
- no `matrix_room_set_power_level` call is used to implement bulk editing;
- no `Legacy*` component, `legacy` return sentinel, or
  `isNative ? rust : js` writer selector is introduced; and
- the shared read-hook imports are not falsely claimed deleted by this write
  packet.

The guard must not assert repository-wide `matrix-js-sdk` usage is zero.

### 5.3 Rust/IPC contract tests

The implementation PR must add focused contract coverage outside the product
command body for:

- camelCase request serialization for both exact command names;
- fixed event type and empty state key per command;
- typed result serialization with `status`, `roomId`, `sessionGeneration`, and
  canonical readback content;
- complete policy-map and tag-map round trips, including unknown retained policy
  members and icon metadata;
- rejection of non-object content, non-integral power values, invalid tag keys,
  empty tag names, malformed icon metadata, wrong room/event/state key, and
  stale session generation; and
- safe error categories with no raw SDK error or state content leakage.

The contract tests must not depend on a live homeserver. An authenticated
disposable Synapse proof belongs to the eventual product implementation PR and
must cover one room Permissions Apply and one space Permissions Apply; this
docs packet does not claim that proof.

## 6. Acceptance checklist for the eventual implementation PR

- [ ] Exact commands are registered and permissioned without aliases or a
      generic state-event escape hatch.
- [ ] Both commands use the one managed live native Matrix client.
- [ ] `PowersEditor` and `PermissionGroups` have no JS state-event write path.
- [ ] Full policy and full tag content are sent once per Apply/Save and are
      validated by typed native readback.
- [ ] Native failures are terminal and visibly reported; no JS fallback or
      `dual_backend` selector exists.
- [ ] Physical deletion and source-absence tests pass.
- [ ] Focused TypeScript and Rust/IPC contract tests pass.
- [ ] Prettier **2.8.1**, repository Matrix guardrails, and relevant lint/type
      checks pass.
- [ ] Live proof is recorded separately; this packet is not a V-BURN or full
      V-ROOMS completion claim.

The docs PR itself is complete when this packet is reviewed, linked from the
#377 residual, and contains no product implementation or `product.rs` change.
