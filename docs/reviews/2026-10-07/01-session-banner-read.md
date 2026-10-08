# Session gate, connection banner, and mark-read — 2026-10-07

Requirements for a later implementer. This document does not change product code.

Branch: `feature/2026-10-07-client-session-and-chrome` at `6f964d3f` (tag `v2.1.46`).
Prior contract: [Session recovery investigation — 2026-10-06](../2026-10-06-session-recovery.md). That work shipped in 2.1.46 and recorded that live installs were not validated. This install is 2.1.46 and still misbehaves.

Owner for all three bugs below: the installed native session (desktop `MatrixAuthState` plus the Core owners `wire_desktop_session` attaches; iOS `SharedCoreMatrixClientService`). Sync HTTP success, token rotation, command rejection, the top banner, and explicit mark-read are one session. They are not one signal today.

## Evidence (do not extend)

Installed package is synara 2.1.46 at `/usr/bin/synara`, upgraded 2026-10-06 14:41 EDT from 2.1.45.

Logs read for this spec:

- `~/.local/share/com.whylandcreative.synara.desktop/logs/synara-desktop.log`
- `~/.local/share/com.whylandcreative.synara.desktop/logs/matrix-session-lifecycle.log`

One native line `session-authentication-rejected` at unix ms `1791312717810` (2026-10-06 14:51:57 EDT). No later copy. No `d0.1` diagnostic was logged after it. `spawn_suspend_resume_watch` in `src-tauri/src/matrix/auth/product.rs` writes `error.diagnostic_id` only when `matrix_logout` returns `Err`. A missing Core state (`try_state` yields `None`) writes nothing.

`matrix-session-lifecycle.log` then shows only `native session-rotation-persisted`, including every ~5 minutes during the process that started 2026-10-07 08:42 (pid 9573). Zero persist failures.

The same desktop log shows many `frontend <command> failed: native command rejected` lines with no parenthetical diagnostic id, for:

- `matrix_media_preview` (majority)
- `matrix_typing_set`
- `matrix_send_text` (twice: ~Oct 5 11:19 and Oct 6 08:38)
- `matrix_timeline_paginate`
- `matrix_timeline_follow_live`
- `matrix_timeline_snapshot` (latest 2026-10-07 09:16)
- `matrix_media_config`
- `matrix_get_own_profile`
- `matrix_device_snapshot`
- `matrix_verification_list`

No `matrix_logout`, `matrix_sync`, or `matrix_room_set_read_state` / `matrix_timeline_set_read_state` lines in the last 48h.

Homeserver Synapse 1.162.0 has been up since 2026-10-05 10:53 UTC. Since then ~68640 refresh 401s (`M_UNKNOWN_TOKEN`, token not in DB) and ~68539 authenticated `/versions` 401s. A 60s sample on 2026-10-07 ~13:12 UTC showed those counters flat. Refresh 200s still trickle. One logout POST, status 401, since process start. Sync GET 200 is healthy. No 5xx.

iOS has no desktop-equivalent auto-retire watcher. The desktop mark-read path swallows errors (`markAsReadInBackground` / `markAsReadFromExplicitUserActionInBackground`). `synara/src/app/pages/client/SyncStatus.tsx` holds `RECONNECTING` for 4000ms. `ERROR` is specified to show `Connection Lost!` immediately.

## Problem

A working desktop and iOS session drops and does not come back. Quitting the app does not help. Logout often stalls.

On this 2.1.46 desktop install the drop is not a homeserver outage and not a failed token save. During pid 9573 the SDK save callback keeps persisting rotated tokens, refresh 200s still trickle, and sync GET 200 stays healthy. The historical refresh and `/versions` 401 counts are not moving in the sampled minute. User-facing commands still fail closed, and the desktop log cannot say why. The top banner can stay blank through that failure because it reads sync readiness, and readiness `running` is painted as a steady connected state with no banner.

Mark channel as read does not clear the room on other clients of the same user. Explicit desktop and iOS mark-read discard the result. A successful native call can also clear only a local unread flag when the live timeline has no receipt-capable event, which other clients do not treat as a new fully-read marker.

The single `session-authentication-rejected` line on 2026-10-06 shows the 2.1.46 latch ran once. It does not show that the generation was retired, and the next day's process kept rotating a session whose commands fail.

## Root-cause hypotheses

Ranked. Each item names the functions a reviewer can open. H1 is the explanation to implement against.

### H1 — Rotation and sync stay up while command bridges fail closed, and the banner never hears about it

`install_session_rotation_callbacks` (`src-tauri/src/matrix/auth/product_commands.rs`) saves through `SessionPersistenceLease::save_credentials` and `persist_session_after_login`, then `record_session_rotation_outcome` appends `session-rotation-persisted`. That log, plus refresh 200s and sync GET 200, means the desktop SDK client in `MatrixAuthState` still has a usable token during pid 9573.

User commands do not use that health. They go through `Core::command` to an attached owner (`wire_desktop_session` attaches typing, timelines, devices, verification, sync, and the rest). Examples:

- `matrix_timeline_snapshot` → `map_timeline_snapshot_core_error` (`src-tauri/src/bridge/timeline_snapshot.rs`). `Forbidden` becomes `d0.3-timeline-requires-session` ("No native Matrix session is active."). `SdkInvariant` becomes `v-timeline-view-not-open`. Anything else becomes `v-timeline-view-snapshot-failed`.
- `matrix_media_preview` → `map_media_preview_core_error`. `Forbidden` becomes `v-send.r-media-preview-requires-session`.
- `matrix_typing_set` → `map_typing_set_core_error`. `Forbidden` becomes `v-rooms.4-typing-owner-user-missing` with the same "no session" message.
- `matrix_media_config` reuses `d0.3-timeline-requires-session`.
- `matrix_get_own_profile` → `d0.4-send-requires-session` on `Forbidden`.
- `matrix_device_snapshot` → `v-crypto.7-device-requires-session`.
- `matrix_verification_list` → `v-crypto.1-start-requires-session`.
- `matrix_send_text` → `d0.4-send-requires-session` on `Forbidden`.
- `matrix_timeline_paginate` and `matrix_timeline_follow_live` use the same three-way snapshot-style map (`d0.3-timeline-requires-session`, `v-timeline-view-not-open`, or a command-specific failure id).

`formatDesktopInvokeError` (`synara/src/app/utils/desktop.ts`) appends a parenthetical id only when `diagnosticId` is in `SAFE_NATIVE_DIAGNOSTIC_IDS` (the `d0.4-send-sdk-*` set). `MatrixAuthCommandError` serializes `diagnostic_id` as `diagnosticId` (`rename_all = "camelCase"`). Every id above is outside that set, so the log line is `native command rejected` for a missing owner, a closed timeline view, and a generic snapshot failure alike.

The banner does not read those rejections. `createNativeMatrixClient` polls `matrix_sync_status` (`readSyncStatus` → `applySyncStatus` → `readinessToSyncState`). `running` becomes `PREPARED`. `getTransientSyncStatusBannerCopy` returns null for a steady `PREPARED`. `SyncStatus` therefore renders nothing while sync readiness stays `running`. There is no `matrix_sync` failure in the last 48h, which matches a healthy sync-status poll beside failing user commands.

`readinessToSyncState` also maps `idle`, `terminated`, and `unconfigured` to `STOPPED`. `getSyncStatusBannerCopy` has no copy for `STOPPED`, so a signed-in client whose sync owner is stopped also gets a blank banner. That hole is real. It is secondary to H1 for pid 9573, because sync GET 200 was healthy in the sample.

Quitting calls `matrix_restore_session`, which reloads the credentials `record_session_rotation_outcome` just saved. A command gate that is closed on top of a still-valid token survives restart.

### H2 — The 2026-10-06 latch fired once and did not leave a checkable retirement

`SyncServiceOwner::observe` (`crates/synara-core/src/app/sync/service.rs`) sets `authentication_rejected` and forces readiness `Failed` with `p4.1-session-authentication-rejected` (`SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID` in `readiness.rs`). `spawn_suspend_resume_watch` logs `session-authentication-rejected` and calls `matrix_logout` with `Some(snapshot.session_generation)`. That call skips `client.matrix_auth().logout()` because the remote POST runs only when `expected_session_generation` is `None`.

One log line and no following `d0.1` id means `matrix_logout` did not return `Err` on that tick, or Core was not in Tauri state and the branch wrote nothing. The in-memory latch does not survive a new process. Pid 9573 then rotated successfully, so it is not sitting in that latch (the watcher would log `session-authentication-rejected` every 5 seconds while the diagnostic stayed set).

The one homeserver logout POST 401 is the voluntary path (`expected_session_generation` is `None` inside `matrix_logout`). The watcher path is not allowed to send it. A 401 that returns is not, by itself, an infinite hang. The hang is available in the same function: `matrix_logout` holds `state.session` across the remote logout await and `sync.stop()`, and `finish_active_logout` waits for `begin()` before cleanup. `LogoutDialog` calls `logout().catch(() => undefined)`, so a rejection disappears and a await that never returns leaves the button spinning. `performLogout` (`synara/src/client/initMatrix.ts`) waits on `mx.logout()` → `createNativeMatrixClient.logout` → `matrix_logout`.

iOS never runs this watcher. `SharedCoreMatrixClientService.refreshLiveSyncStatus` publishes `ConnectionStatusCopy.fromReadiness` and returns on a failed `syncStatus` read. Nothing there latches `p4.1-session-authentication-rejected` or retires the generation. `SettingsView.logout` calls `environment.wipe.logoutAndWipe`. `SharedCoreLeftovers.logout` and `wipePersistedStores` are the wrong tools for an auth rejection: the first can hit the network, the second deletes stores.

### H3 — Mark-read can succeed locally, or fail invisibly, without moving other clients

Explicit desktop Mark as Read is `RoomNavItem.handleMarkAsRead` and `RoomMenu.handleMarkAsRead` (`RoomViewHeader.tsx`). Both call `markAsReadFromExplicitUserActionInBackground`, which calls `markAsRead(..., privateReceipt: true)` and then `.catch(() => undefined)`. On desktop, `markAsRead` returns through `setRoomReadStateWithNativeOwner` → `matrix_room_set_read_state` and does not use the js-sdk receipt path.

`room_set_read_state` (`src-tauri/src/bridge/room_read_state.rs`) maps `Forbidden` to `d0.3-timeline-requires-session`. `matrix_room_set_read_state` (`crates/synara-core/src/core/room_administration.rs`) requires `state.timeline_owner()` or returns `p2-room-set-read-state-no-session`. `NativeTimelineOwner::set_room_read_state` (`crates/synara-core/src/app/timeline/live.rs`) opens a live timeline and calls `mark_live_timeline_read` with `NativeTimelineReadIntent::ExplicitUser`.

`mark_live_timeline_read` is the cross-client write:

- `LiveReadTargetPlan::Send` calls `Timeline::send_multiple_receipts(exact_read_receipts(...))`. `exact_read_receipts` sets the fully-read marker and a private read receipt. Other clients of the same user learn the read from `m.fully_read`. The private receipt stays on this device.
- `LiveReadTargetPlan::ClearUnreadFlag` calls `set_unread_flag(false)` and sends no fully-read marker. That happens when explicit mark-read sees no `latest_event_id`.
- `LiveReadTargetPlan::NoOp` is the automatic-visibility path, not the explicit one.

The open-room automatic path is separate: `NativeTimelinePresenter` calls `setReadState({ action: 'mark_read', intent: 'automatic_visibility', ... })` and swallows failure. That command is `matrix_timeline_set_read_state`. It requires an open stream (`v-timeline-view-not-open`) and `position_allows_mark_read`. It is not the channel menu.

No read-state command lines in the last 48h means those invokes did not fail inside `invokeDesktopWithAvailability` during that window. Success is not logged. A throw before invoke (desktop unavailable, blank room id, `matrix_session_snapshot` not `logged_in` inside `requireLoggedIn`) is also not a `matrix_room_set_read_state` line. The background `.catch(() => undefined)` hides all of those from the menu: the menu closes immediately in `handleMarkAsRead`.

iOS channel mark-read is `RoomListView.markRoomAsRead`, which discards the result of `RoomReadMarkerServicing.markRoomAsRead`. `SharedCoreRoomReadMarkerService.markRoomAsRead` opens a live stream with `try?` and calls `SharedCoreTimelineReadState.timelineSetReadState` with `intent: "explicit_user"`, also with `try?`. Failure and a nil `acknowledgedEventId` both look like a finished tap. `markFullyRead` is the automatic path and already requires `receiptSent == true` plus a matching event id. Explicit room mark-read does not.

### H4 — Banner hold hides a transient offline blip, and on iOS it also hides a terminal loss

Desktop `SyncStatus` starts a 4000ms timer only for `RECONNECTING` (`RECONNECTING_BANNER_HOLD_MS`). `ERROR` clears that hold and `getTransientSyncStatusBannerCopy` returns `Connection Lost!` immediately. Keep that split.

iOS `ConnectionStatusCopy.holdsBeforeBanner` returns true for `.reconnecting`, `.disconnected`, and `.failed`. `ConnectionStatusStore.scheduleLost` waits `lostHold` (4s) before `present`. `ConnectionStatusCopyTests` currently requires the disconnected and failed holds. A terminal loss that flips back to `.connected` before 4s never paints. `fromReadiness("running")` is `.connected`, and `presentsBanner(.connected)` is false unless a recovery flash is already scheduled. The same H1 gap exists on iOS: `refreshLiveSyncStatus` publishes readiness and does not publish command rejection.

H4 does not explain pid 9573 by itself. Sync GET 200 was healthy, and desktop `ERROR` would already have been visible if readiness had been `failed`.

## In scope

1. Desktop and iOS: a dropped session must reconnect when the failure is transient, and must return to sign-in when the failure is a rejected refresh, without another remote logout and without deleting crypto.
2. Quitting must not restore a generation that was retired. Quitting may restore a generation whose tokens still rotate. In that case the UI must show the real command-gate state and logout must finish.
3. Logout must finish locally when `/logout` is 401 or the remote call does not return, including the stall behind `matrix_logout`'s session mutex.
4. Explicit Mark channel as read must publish `m.fully_read` for the current remote tail so another client of the same user clears the room. Failures of that action must be visible.
5. While the session is disconnected or the command gate is closed, the top banner must show that state. Transient offline may still wait 4000ms. Terminal loss must show `Connection Lost!` immediately on desktop and iOS.

## Out of scope

Linux window drag and the local-echo message menu are other tasks. Those changes must leave this contract intact:

- Explicit mark-read still calls `setRoomReadStateWithNativeOwner` / `RoomReadMarkerServicing.markRoomAsRead`. A local unread decoration is not a substitute for `m.fully_read`.
- Local echoes (`$local-` / `$pending-`) stay ineligible as receipt targets (`MatrixServerEventIDPolicy.canAcknowledge`).
- Dragging or resizing a window must not close the command gate, suppress the connection banner, or change logout retirement.
- Voluntary logout's existing crypto policy stays. This work does not add a store wipe and does not remove one that already exists on the explicit Sign Out path.

Room-permission denials, a closed timeline view (`v-timeline-view-not-open`), a missing preview URL, and a room the user has not joined are not connection loss.

## Requirements

### FR-1 — Command gate is a sync-status field, separate from readiness

Extend the public `matrix_sync_status` DTO (`SyncReadinessSnapshot` / `SyncStatusWireResponse` / `parseSyncStatus`) with one closed field:

- Name: `commandGate`
- Values: `open` | `closed`
- Absent on old payloads means `open`

`commandGate: closed` means the installed session cannot serve user commands: Core session/owners are missing, `SyncServiceOwner::observe` has latched `p4.1-session-authentication-rejected`, or restore installed a client that rotates tokens while `wire_desktop_session` owners are not attached.

`commandGate` stays `open` for `v-timeline-view-not-open`, ordinary room permission errors, preview URL errors, typing room-not-joined, and a send failure whose id is in the existing `d0.4-send-sdk-*` set.

`is_valid_public_sync_status` accepts the new field only with those two values. A hostile or unknown string fails closed as `closed` on the renderer (`parseSyncStatus` treats an unknown gate as `closed`) and must not be forwarded as free text.

Desktop `readinessToSyncState` maps `commandGate === 'closed'` to `ERROR` for every readiness value, including `running`. iOS `ConnectionStatusCopy.fromReadiness` maps a closed gate to `.disconnected`.

### FR-2 — Banner shows terminal loss immediately and holds only transient offline

Desktop, in `getTransientSyncStatusBannerCopy` / `SyncStatus`:

- `ERROR` shows `Connection Lost!` immediately, including when FR-1 forced `ERROR` from a closed gate while readiness is `running`.
- `RECONNECTING` stays hidden until `RECONNECTING_BANNER_HOLD_MS` (4000). A return to `PREPARED` before that cancels the banner.
- Steady `PREPARED` with `commandGate: open` stays banner-free. The short Connected flash after a banner the user actually saw stays as it is.
- `STOPPED` while a session is still signed in (`idle`, `terminated`, or `unconfigured` after the user was signed in) shows `Connection Lost!` immediately. Cold start before the first status poll may stay blank.

iOS, in `ConnectionStatusCopy.holdsBeforeBanner` and `ConnectionStatusStore`:

- `.reconnecting` keeps the 4s hold.
- `.disconnected`, `.failed`, and a closed command gate present immediately.
- `.connected` stays hidden except the existing recovery flash.
- Banner text for `.failed` stays `Connection Lost!`. The associated string is not rendered.

Copy is the existing product strings. No tokens, user ids, homeserver paths, or SDK error text.

### FR-3 — Rejected refresh retires that generation locally, on desktop and iOS

When `observe()` reports `p4.1-session-authentication-rejected`:

- Desktop `spawn_suspend_resume_watch` retires that `session_generation` through `matrix_logout(..., Some(generation))`.
- The call does not POST `/logout` and does not refresh.
- Crypto SQLite and its keys stay. Credentials and the active identity for that generation are cleared. `matrix_restore_session` then fails closed with the existing missing-session result (`d0.1-active-session-missing` or the current restore failure id). `record_session_rotation_outcome` does not append another `session-rotation-persisted` for the retired generation.
- One retirement attempt is in flight per generation. A later tick may retry local cleanup. It must not call `client.matrix_auth().logout()` and must not send the refresh token.
- If `matrix_logout` returns `Err`, the watcher logs that `diagnostic_id` only when it is already a static `d0.1-*` or `p4.1-*` id.
- If Core is not in Tauri state, the watcher logs the new static id `d0.1-session-rejection-no-core` once for that generation. It must not stay silent.

iOS `SharedCoreMatrixClientService.refreshLiveSyncStatus` performs the same local retirement when the sync DTO carries `p4.1-session-authentication-rejected`. It must not call `SharedCoreLeftovers.logout`, `wipePersistedStores`, or `logoutAndWipe` for this path. After retirement the UI is signed out with the existing expiry explanation desktop already stores for this diagnostic. There is no iOS watcher today; add the behavior on this status poll rather than a second sync loop.

A generation that is still rotating (`session-rotation-persisted`, refresh 200) stays signed in. FR-1 and FR-2 apply. Do not retire it just because a timeline view is closed.

### FR-4 — Logout finishes when the remote call is 401 or does not return

`matrix_logout` must drop `MatrixAuthState.session` before `client.matrix_auth().logout()` and before `SyncServiceOwner::stop`. Re-acquire the mutex only to clear the slot, and only when the generation still matches.

Voluntary logout (`expected_session_generation == None`) may still attempt one remote `/logout`. That attempt is bounded at 15 seconds. A 401, a timeout, or a transport error continues local cleanup. The function still returns `MatrixSessionSnapshot::LoggedOut` after local credential cleanup succeeds. It returns a closed `d0.1-*` error when local cleanup fails, and the renderer keeps the signed-in screen so the user can retry (`performLogout` already does this when native logout does not report `logged_out`).

`LogoutDialog` must surface that closed failure on the dialog. `logout().catch(() => undefined)` is not the completion path.

The auth-rejection path (FR-3) still skips the remote POST entirely.

iOS explicit Sign Out must leave the spinner within the same 15 second bound when the remote logout does not return. It must not call `wipePersistedStores` as a new step. Auth-rejection retirement stays the FR-3 path.

### FR-5 — Explicit mark-read publishes `m.fully_read` and reports failure

Desktop explicit Mark as Read (`handleMarkAsRead` in `RoomNavItem.tsx` and `RoomViewHeader.tsx`) awaits `markAsReadFromExplicitUserAction`. The menu stays open until the call settles. On failure it shows a fixed string: `Couldn't mark this channel as read.` The room's unread badge stays. `markAsReadFromExplicitUserActionInBackground` and `markAsReadInBackground` remain for non-explicit callers only.

The native result of `matrix_room_set_read_state` is no longer `()`. Return a closed readback:

- `receiptSent: true` and `acknowledgedEventId` set when `mark_live_timeline_read` took `LiveReadTargetPlan::Send`
- `receiptSent: false` and `unreadFlagCleared: true` when the room has no receipt-capable remote event and `set_unread_flag(false)` succeeded
- error otherwise (`d0.3-timeline-requires-session`, `v-rooms-room-read-state-room-not-found`, `v-rooms-room-read-state-mark-read-failed`, or the existing invalid-room id)

`Send` must keep using `exact_read_receipts`: fully-read marker plus private read receipt, via `send_multiple_receipts`. Do not send a public `m.read` for this action. Cross-client success is the fully-read marker. The private receipt is not that signal.

`ClearUnreadFlag` is success only when `plan_live_read_target` sees no latest event id. A timeline that failed to load is an error, not a silent flag clear.

iOS `RoomListView.markRoomAsRead` treats a nil return from `markRoomAsRead` as failure and leaves the row unread. `SharedCoreRoomReadMarkerService.markRoomAsRead` returns the acknowledged event id only when `receiptSent == true`, matching `markFullyRead`. `try?` is not the success test. Opening the live stream may stay an implementation detail; a failed open is a failed mark-read.

Automatic visibility (`NativeTimelinePresenter` `setReadState` with `automatic_visibility`, and iOS `markFullyRead`) may keep swallowing a superseded navigation. It must not be the implementation of the channel menu.

### FR-6 — Session-scoped command failures are visible in the desktop log

Add the static ids below to the allowlist `formatDesktopInvokeError` consults. Unknown strings still log as `native command rejected` with no suffix and no other fields. Do not log `code`, `message`, URLs, or SDK text.

Session-scoped ids that must appear as `native command rejected (<id>)`:

- `d0.3-timeline-requires-session`
- `v-timeline-view-not-open`
- `v-timeline-view-snapshot-failed`
- `v-timeline-view-read-state-failed`
- `v-rooms-room-read-state-mark-read-failed`
- `v-rooms-room-read-state-room-not-found`
- `v-send.r-media-preview-requires-session`
- `v-rooms.4-typing-owner-user-missing`
- `v-rooms.4-typing-notice-failed`
- `d0.4-send-requires-session`
- `v-crypto.7-device-requires-session`
- `v-crypto.1-start-requires-session`
- `p4.1-session-authentication-rejected`
- `d0.1-session-rejection-no-core`
- `d0.1-session-rejection-stale`

`v-timeline-view-not-open` is logged and does not close `commandGate` (FR-1). The point of logging it is so the next capture can tell a closed view from a missing session. Today those are the same line.

## Acceptance criteria

A reviewer can check these without asking the author. "Pass" means the named test or the named source assertion holds on the change.

1. `parseSyncStatus` / `readinessToSyncState`: a payload with `readiness: "running"` and `commandGate: "closed"` yields sync state `ERROR`. The same payload with `commandGate: "open"` yields `PREPARED`. A missing `commandGate` yields the old mapping.
2. `getTransientSyncStatusBannerCopy('ERROR', false, false)` is `Connection Lost!`. `getTransientSyncStatusBannerCopy('RECONNECTING', false, false)` is null. The reconnecting string appears only once the hold flag is true. `STOPPED` while the facade still has a logged-in session snapshot shows `Connection Lost!`.
3. iOS `holdsBeforeBanner(.disconnected)` and `holdsBeforeBanner(.failed)` are false. `holdsBeforeBanner(.reconnecting)` stays true. A store updated to `.disconnected` sets `isBannerVisible` without waiting `lostHold`.
4. A unit or integration fixture where the sync owner observes `Running` and `timeline_owner()` is `None` produces `commandGate: closed` on `matrix_sync_status` and `d0.3-timeline-requires-session` from `matrix_timeline_snapshot`. The lifecycle log is not required to change in that fixture.
5. A fixture that latches `p4.1-session-authentication-rejected` records one `session-authentication-rejected` line, calls logout with the generation, does not invoke the remote logout client, leaves the crypto store file in place, and makes a following restore return the missing-session diagnostic. A second watcher tick does not POST `/logout`.
6. A fixture whose Core handle is absent logs `d0.1-session-rejection-no-core` and does not claim retirement succeeded.
7. `matrix_logout` with a remote logout stub that waits longer than 15 seconds, or returns 401, still returns `logged_out` when local cleanup succeeds, and does not hold the session mutex during the wait. A test can take the mutex from another task within the bound.
8. `LogoutDialog` renders the fixed retry copy when `matrix_logout` rejects. It does not use an empty catch as the only result handling.
9. Desktop `handleMarkAsRead` source awaits the explicit mark-read function. A rejected `matrix_room_set_read_state` leaves a test-visible error string and does not call `requestClose` first.
10. `mark_live_timeline_read` for `ExplicitUser` with a latest event id submits `exact_read_receipts` (fully-read marker and private receipt). With no latest event id it clears the unread flag and sets `receiptSent` false. A forced `send_multiple_receipts` error surfaces `v-rooms-room-read-state-mark-read-failed` to the explicit caller.
11. iOS `markRoomAsRead` returns nil when `timelineSetReadState` throws or `receiptSent` is false. `RoomListView` does not clear unread in that case.
12. `formatDesktopInvokeError({ diagnosticId: 'd0.3-timeline-requires-session' })` is `native command rejected (d0.3-timeline-requires-session)`. `formatDesktopInvokeError({ diagnosticId: 'not-a-real-id' })` is `native command rejected`. A value that looks like a token or a URL is still `native command rejected`.
13. iOS auth-rejection fixture: a sync DTO with `failureDiagnosticId == p4.1-session-authentication-rejected` ends signed out, does not call `wipePersistedStores`, and does not call the remote logout entry point.

Live homeserver checks (not a unit-test pass): after a build is installed, a rejected refresh produces the retirement log and a signed-out UI on desktop and iOS; a still-valid rotating session that fails a session-scoped command shows `Connection Lost!` while `matrix-session-lifecycle.log` continues to show only `session-rotation-persisted`; Mark as Read on one client moves the fully-read marker observed by a second client of the same user. Those runs use redacted logs. They are not required for the unit suite.

## Testing requirements

Add or extend tests. No test may read a secret, a token, an mxid from the developer machine, or a live homeserver.

Extend:

- `synara/src/app/utils/__tests__/desktop.test.ts` — FR-6 allowlist and the unknown-id fallback.
- `synara/src/app/pages/client/__tests__/syncStatusCopy.test.ts` and `synara/src/app/features/native-client/__tests__/nativeClientFacade.test.ts` — FR-1 and FR-2 desktop mapping.
- `synara/src/app/utils/__tests__/notifications.test.ts` and `synara/src/app/features/room-nav/__tests__/roomNavItemReadState.test.ts` — explicit mark-read awaits and surfaces failure. The existing test that background mark-read swallows UI-only failures stays true for `markAsReadInBackground`, and becomes false for the explicit menu path.
- `src-tauri/src/matrix/auth/product_tests.rs` — watcher logging, generation-fenced retirement, mutex not held across logout, 15s bound.
- `crates/synara-core/src/app/timeline/live` receipt tests (the `exact_read_receipts` / `send_multiple_receipts` assertions already in `live.rs` tests) — FR-5 readback shape.
- `synara-ios/SynaraTests/ConnectionStatusCopyTests.swift` — terminal states are not held. Update the assertions that currently require `holdsBeforeBanner(.disconnected)` and `holdsBeforeBanner(.failed)`.
- An iOS test beside `SharedCoreRoomReadMarkerService` / `SynaraCoreBindingsTests` mark-read cases — nil on `try` failure and on `receiptSent != true`.

New fixture, no network: sync owner state `Running` combined with a missing timeline owner yields `commandGate: closed` and a timeline snapshot `Forbidden` mapped to `d0.3-timeline-requires-session`.

Cannot be proven without a live homeserver, and must not be faked with a secret-bearing fixture:

- That pid 9573's real token will keep receiving refresh 200s after the patch.
- That a second logged-in client observes `m.fully_read` for a production room.
- That the historical ~68640 refresh 401s stop increasing because of this change. They were already flat in the 60s sample. Do not add a test that replays a rejected refresh token against Synapse.

A mocked SDK client that records receipt type and logout HTTP is the stand-in for those behaviors in CI.

## Allowed paths

The implementer may change only what FR-1–FR-6 require, plus the tests named above:

- `src-tauri/src/matrix/auth/product.rs` — `spawn_suspend_resume_watch`
- `src-tauri/src/matrix/auth/product_commands.rs` — `matrix_logout`, `finish_active_logout`, `install_session_rotation_callbacks`, `record_session_rotation_outcome`, `wire_desktop_session`
- `src-tauri/src/matrix/auth/product_tests.rs`
- `src-tauri/src/bridge/session_lifecycle.rs` — sync status wire DTO
- `src-tauri/src/bridge/timeline_snapshot.rs`
- `src-tauri/src/bridge/timeline_paginate.rs`
- `src-tauri/src/bridge/timeline_follow_live.rs`
- `src-tauri/src/bridge/timeline_set_read_state.rs`
- `src-tauri/src/bridge/room_read_state.rs`
- `src-tauri/src/bridge/media_preview.rs`
- `src-tauri/src/bridge/media_config.rs`
- `src-tauri/src/bridge/typing_set.rs`
- `src-tauri/src/bridge/own_profile.rs`
- `src-tauri/src/bridge/device_snapshot.rs`
- `src-tauri/src/bridge/verification_list.rs`
- `src-tauri/src/bridge/send_text.rs`
- `crates/synara-core/src/app/sync/readiness.rs`
- `crates/synara-core/src/app/sync/service.rs` — `observe` latch only; do not change `is_terminal_auth_error`'s match set
- `crates/synara-core/src/app/timeline/live.rs` — `set_room_read_state`, `mark_live_timeline_read`, `exact_read_receipts`, `plan_live_read_target`
- `crates/synara-core/src/core/room_administration.rs` — `matrix_room_set_read_state` response shape
- `crates/synara-core/src/core/session_crypto.rs` only if the sync status projection is built there
- `synara/src/app/utils/desktop.ts`
- `synara/src/app/utils/notifications.ts`
- `synara/src/app/utils/nativeRoomReadStateOwner.ts`
- `synara/src/app/features/native-client/nativeClientFacade.ts`
- `synara/src/app/pages/client/SyncStatus.tsx`
- `synara/src/app/pages/client/syncStatusCopy.ts`
- `synara/src/app/features/room-nav/RoomNavItem.tsx`
- `synara/src/app/features/room/RoomViewHeader.tsx`
- `synara/src/app/components/LogoutDialog.tsx`
- `synara/src/client/initMatrix.ts` — `performLogout` error surface only
- `synara-ios/Synara/Services/ConnectionStatus.swift`
- `synara-ios/Synara/SharedUI/ConnectionStatusBanner.swift` — only if immediate present needs a visibility tweak
- `synara-ios/Synara/Services/SharedCoreProductServices.swift` — `SharedCoreMatrixClientService.refreshLiveSyncStatus` and `SharedCoreRoomReadMarkerService.markRoomAsRead`
- `synara-ios/Synara/Features/RoomListView.swift` — `markRoomAsRead`
- `synara-ios/Synara/Features/SettingsView.swift` — Sign Out completion bound only
- Matching tests under those trees

Do not edit `docs/reviews/2026-10-07/02-*` or `03-*`. Do not widen command behavior for Linux drag or the local-echo menu.

## Non-goals

- Do not delete crypto stores, store keys, or call `wipePersistedStores` from the auth-rejection path.
- Do not log tokens, mxids, raw SDK errors, HTTP bodies, or homeserver URLs. Lifecycle lines stay `session-rotation-persisted`, `session-rotation-persist-failed`, `session-authentication-rejected`, and the static diagnostic ids in FR-6.
- Do not retry a rejected refresh token. Do not POST `/logout` with the rejected generation's credentials. The voluntary logout POST is one bounded attempt and is skipped entirely after FR-3 has latched that generation.
- Do not treat the historical refresh 401 storm as a reason to replay tokens or to probe `/versions` with an Authorization header. The 2026-10-06 contract (anonymous capability probe, no offline-mode versions loop) stays.
- Do not clear unread on other devices by writing a public `m.read` receipt.

## Addendum (post-implementation review)

A matrix-sdk 0.19.1 alignment review after implementation corrected two premises above. The requirements stay; their rationale and one acceptance check change.

- **H1 is not supported by the evidence.** Desktop calls `core.close()` only after the session slot is emptied. Login, register, and restore hold the session mutex through `wire_desktop_session`, and `sync_status_snapshot` takes the same mutex, so the status poll cannot report `running` while the owners are missing. `matrix_timeline_open` never logged a failure in the captured window. The pid 9573 rejections were mostly a chronic `matrix_media_preview` failure plus view-level `follow_live` / `paginate` / `snapshot` errors, which FR-1 must not count as connection loss. The 2026-10-06 sequence (fast `media_config` / `get_own_profile` / `device_snapshot` failures, the latch, then fresh rotations about five minutes later) fits a dead refresh token that was retired and followed by a new sign-in. FR-6 diagnostics are the main value for the next capture. The logout teardown race closed in this change was the only path found where the gate could read closed on a live session.
- **Mark-read cause.** The live timeline hides threaded events (`HIDE_THREADED_EVENTS` in `crates/synara-core/src/app/timeline/live.rs`) without enabling SDK threading support. `latest_event_id()` then skips in-thread events, while unread counting (SDK and Synapse) still counts them after an unthreaded receipt. A room whose newest activity is thread replies therefore stays unread on every client after Mark as read. The explicit read target has to include threaded events.
- **FR-5 receipt semantics.** The SDK computes unread state from `m.read.private` and `m.read` (`ALL_RECEIPT_TYPES` in matrix-sdk `event_cache/caches/read_receipts.rs`). The private receipt is delivered to all of the user's own devices, so it is the cross-device unread signal. `m.fully_read` only moves the read-marker line. `exact_read_receipts` (fully-read marker plus private receipt) remains correct. Acceptance 10 and the live check should assert the private receipt and the unread-count drop on a second client, not only `m.fully_read`.
