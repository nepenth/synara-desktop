# Matrix Rust SDK Alignment Audit

Date: 2026-07-21

## Summary

The iOS app should treat Matrix Rust SDK as the primary Matrix integration
surface. REST calls should be explicit exceptions used only where the SDK does
not expose the needed operation, where a Synara-specific custom event is easier
to send safely through a generic SDK send path that we have not wrapped yet, or
where Apple/APNs integration is inherently outside the SDK.

The recent room-list/timeline streaming issue exposed a broader risk: the app
still has a mix of SDK-backed services, legacy REST-backed services, and a few
view-level URLSession calls. Some of those paths are inactive test-era code, but
some are still live. This plan consolidates Matrix behavior behind SDK-first
service boundaries so the app benefits from SDK sync, cache, E2EE, local echo,
send queues, media handling, notification settings, thread support, space
support, and crypto verification.

## Current Runtime Wiring

Live SDK-backed paths:

- Auth/login and session restore: `MatrixRustSDKAuthService`.
- Room list and room-list streaming: `MatrixRustSDKRoomListService`.
- Timeline initial load and timeline streaming: `MatrixRustSDKTimelineService`.
- Text send and reply send: `MatrixRustSDKMessageSendService`.
- Room create, DM create, join, leave, invite, details, profile updates,
  public-room search, spaces metadata, notification mode: `MatrixRustSDKRoomManagementService`.
- Invite accept/reject: `MatrixRustSDKRoomMembershipService`.
- Crypto status, recovery, verification request, retry decryption:
  `MatrixRustSDKCryptoStatusService`.

Live REST/direct paths that need SDK review:

- Device display-name normalization uses direct device update endpoint.
- Room read-marker lookup uses `MatrixRoomReadMarkerService`, an isolated direct
  room account-data HTTP exception pending SDK read-marker/account-data support.

Legacy REST paths removed from production and unit-test code:

- `MatrixRoomListService`.
- `MatrixRoomMembershipService`.
- `MatrixPasswordAuthService`.
- `MatrixMessageSendService`.
- `MatrixTimelineService`.
- `MatrixAccountDataLaterService`.
- `MatrixEventActionService`.
- `MatrixAgentApprovalService`.

## SDK Capabilities Confirmed In Local Bindings

The checked-in build uses `MatrixRustSDK` from
`matrix-rust-components-swift` 26.6.6 (release tag `26.06.06`, revision
`ec3b2161ba371a13609e7181077d2f3baef188f5`). The local bindings expose the
following first-class APIs relevant to Synara:

- Client/session:
  - `ClientBuilder.sessionPaths(...)`
  - `ClientBuilder.slidingSyncVersionBuilder(...)`
  - `Client.login(...)`
  - `Client.restoreSession(...)`
  - `Client.session()`
  - `Client.syncService().finish()`
  - `SyncService.start()/stop()`
  - `Client.clearCaches(syncService:)`
- Room list and sync:
  - `SyncService.roomListService()`
  - `RoomListService.allRooms()`
  - `RoomList.entriesWithDynamicAdapters(pageSize:listener:)`
  - `RoomListEntriesListener`
  - room-list filters and loading/sync state listeners
- Room operations:
  - `Client.createRoom(...)`
  - `Client.joinRoomById(...)`
  - `Client.joinRoomByIdOrAlias(...)`
  - `Room.join()/leave()/automationt()`
  - `Room.inviteUserById(...)`
  - `Room.setName(...)`, `Room.setTopic(...)`, `Room.uploadAvatar(...)`,
    `Room.removeAvatar()`, `Room.updateCanonicalAlias(...)`
  - `Room.getPowerLevels()`
  - `Room.typingNotice(isTyping:)`
  - `Room.markAsFullyReadUnchecked(...)`
- Timeline and messaging:
  - `Room.timeline()`
  - `Room.timelineWithConfiguration(...)`
  - `Timeline.addListener(...)`
  - `TimelineDiff`
  - `Timeline.paginateBackwards/Forwards(...)`
  - `Timeline.markAsRead(...)`
  - `Timeline.send(...)`
  - `Timeline.sendReply(...)`
  - `Timeline.edit(...)`
  - `Timeline.redactEvent(...)`
  - `Timeline.toggleReaction(...)`
  - `Timeline.sendReadReceipt(...)`
  - `Timeline.fetchMembers()`
  - `Timeline.loadReplyDetails(...)`
  - `Timeline.createPoll(...)`, `sendPollResponse(...)`, `endPoll(...)`
  - `Timeline.sendFile/sendImage/sendVideo/sendAudio/sendVoiceMessage(...)`
  - back-pagination status listener
- Threads:
  - `Room.threadListService()`
  - `ThreadListService`
  - `ThreadListEntriesListener`
  - `Room.fetchThreadSubscription(...)`
  - timeline focus thread modes
- Spaces:
  - `Client.spaceService()`
  - `SpaceService.topLevelJoinedSpaces()`
  - `SpaceService.spaceFilters()`
  - `SpaceService.spaceRoomList(...)`
  - `SpaceRoomList` pagination and update listeners
- Public rooms and search:
  - `Client.roomDirectorySearch()`
  - `RoomDirectorySearch.results(...)`
  - `RoomDirectorySearch.search(...)`
  - `RoomSearchIterator.nextEvents()`
  - `Client.searchUsers(...)`
- Media:
  - `Client.uploadMedia(...)`
  - `Client.getMediaFile(...)`
  - `MediaSource.fromUrl(...)` / `fromJson(...)`
  - `MediaFileHandle.persist(...)`
  - upload progress watchers and send-attachment join handles
- Notifications:
  - `Client.getNotificationSettings()`
  - `NotificationSettings.get/setRoomNotificationMode(...)`
  - default room notification modes
  - room/user mention push rules
  - `Client.setPusher(...)`
  - `Client.deletePusher(...)`
  - `Client.notificationClient(...)`
  - batch notification resolution
- Crypto:
  - `Client.encryption()`
  - backup/recovery state and listeners
  - `recover(...)`, `recoverAndFixBackup(...)`, `enableRecovery(...)`
  - verification state and listeners
  - `SessionVerificationController`
  - SAS verification flow
  - `UserIdentity`
  - UTD delegate and decryption retry support
- Profile/account:
  - `Client.getProfile(...)`
  - `Client.displayName()`
  - `Client.avatarUrl()`
  - `Client.setDisplayName(...)`
  - `Client.setAvatarUrl(...)`
  - `Client.accountData(...)`
  - `Client.setAccountData(...)`

## 26.5.13 to 26.6.6 Compatibility Review

The official Swift wrapper comparison contains one breaking signature change
used by Synara and two additive APIs:

- `Client.setPusher(...)` now requires `append: Bool`. Synara passes `false` to
  preserve the previous SDK behavior and Matrix request default: registering
  this device's APNs pusher replaces the matching registration rather than
  appending another pusher.
- `Client.tileServer()` and `TileServerInfo` were added for MSC3488 map-tile
  discovery. Synara does not currently expose location-map UI, so no runtime
  integration is required.
- `SqliteStoreBuilder.key(key:)` was added alongside passphrase configuration.
  Synara continues to use its existing SDK store setup; changing key material
  as part of a package upgrade would risk making existing stores unreadable.

No timeline, room-list, read-marker, media, authentication, crypto, or sync API
used by Synara was removed or changed in the generated Swift bindings. Package
resolution and compile/test evidence for this alignment is recorded in the
upgrade commit.

## Alignment Findings

### Finding 1: Legacy REST services have been removed

Status:

- Legacy auth, room-list, membership, composer send, timeline, Later
  account-data, event action, and agent approval REST services have been
  removed from production and test code.
- Unit tests now use protocol mocks, SDK-backed fakes, or shared contract
  fixtures instead of old HTTP clients.

Remaining risk:

- New direct Matrix HTTP could still be reintroduced without the boundary check.

Recommendation:

- Keep `npm run check:matrix-boundaries` in CI.
- Add any future REST exception only as a named service-level boundary with a
  removal condition.

### Finding 2: Timeline still has REST fallback behavior

Risk:

- `MatrixRustSDKTimelineService` falls back to raw timeline mapping for agent
  cards.
- This can mask SDK mapping gaps and repeat the same "REST compensates for
  incomplete SDK integration" problem.

Recommendation:

- Teach SDK timeline mapping to extract Synara agent-card custom content from
  SDK event content or event raw JSON if available.
- If custom raw event content is unavailable in SDK bindings, isolate a small
  `CustomEventResolver` exception that fetches only the specific missing event
  by ID, not a room timeline page.

### Finding 3: Media is REST-backed despite SDK media APIs

Status: live iOS wiring now uses Matrix Rust SDK media APIs for thumbnail
bytes, media upload, and media message send. `MatrixMediaLoader` calls
`Client.getMediaThumbnail`, and `MatrixMediaUploadService` calls
`Client.uploadMedia` plus SDK timeline send with typed media message content.
Remaining media work is richer download/viewer behavior, encrypted media
decrypt UX, upload progress/cancel/retry, and SDK fake coverage.

Risk:

- Direct media calls bypass SDK media cache, encrypted media handling, upload
  progress, send queues, and attachment-specific local echo.
- Current encrypted media behavior remains blocked because REST media cannot
  decrypt Matrix encrypted media payloads safely.

Recommendation:

- Move uploads to SDK `Timeline.sendImage/sendFile/sendVideo/sendAudio`.
- Move downloads/thumbnails to `Client.getMediaFile(...)` with `MediaSource`.
- Use SDK upload progress watchers and send handles for cancel/retry/progress.
- Keep REST media only as a temporary fallback behind a feature flag.

### Finding 4: Event actions bypass timeline APIs

Status: live iOS wiring now uses `MatrixRustSDKEventActionService` for reaction
and redaction operations, and `MatrixRustSDKMessageSendService` routes composer
edits through `Timeline.edit`. Legacy REST action classes have been removed.

Risk:

- Direct redaction/reaction calls bypass SDK local echo, send queue state,
  deduplication, permission handling, and encrypted-room semantics.

Recommendation:

- Move reactions to `Timeline.toggleReaction(...)`.
- Move redactions to `Timeline.redactEvent(...)`.
- Move edits to `Timeline.edit(...)`.
- Derive action availability from SDK event/timeline state and room power
  levels where possible.

### Finding 5: Account data and Later should be SDK-backed

Status: live iOS wiring now uses `MatrixRustSDKLaterService` for Later account
data reads through `Client.accountData(eventType:)`. The legacy REST-backed
`MatrixAccountDataLaterService` has been removed. Later mutations still need to
route through `Client.setAccountData`.

Risk:

- Direct account-data reads do not use SDK cache or listeners.
- Later cannot update live without explicit reload.

Recommendation:

- Replace Later reads with `Client.accountData(eventType:)`.
- Replace Later writes with `Client.setAccountData(eventType:content:)`.
- Add `AccountDataListener` if the app needs live Later updates.

### Finding 6: Avatar/profile loading is split across services and views

Status: live SDK timeline and composer send services now resolve profile avatar
metadata through SDK `senderProfile`/`Client.getProfile(userId:)`. Timeline
avatar rendering no longer performs direct profile or Matrix media HTTP; it uses
the SDK-backed media loader when an `mxc://` avatar URL is present. Read-marker
room account-data lookup has moved out of `RoomTimelineView` into
`MatrixRoomReadMarkerService`, which is the remaining direct HTTP exception.

Risk:

- View-level direct URLSession calls make caching, auth, redaction, and testing
  harder.
- Profile lookup repeats for send return values, timeline enrichment, and avatar
  rendering.

Recommendation:

- Add a `MatrixProfileMediaService` backed by `Client.getProfile(...)`,
  `Client.avatarUrl()`, `Room.avatarUrl()`, and `Client.getMediaFile(...)`.
- Remove profile/media network calls from SwiftUI views.
- Replace `MatrixRoomReadMarkerService` direct HTTP when SDK read-marker or room
  account-data support is available.
- Use a shared in-memory plus SDK media-cache aware avatar cache.

### Finding 7: Push pusher registration can move to SDK

Status: live iOS pusher registration now uses `Client.setPusher(...)` and
`Client.deletePusher(...)` through `MatrixRustSDKClientStore`. Old raw pusher
payload tests were removed because the app no longer builds pusher JSON
manually.

Risk:

- Direct pusher JSON may diverge from Matrix SDK behavior.
- `deletePusher`/`setPusher` exist in SDK and should own Matrix pusher
  semantics.

Recommendation:

- Keep APNs token capture in iOS code.
- Move Matrix pusher set/delete to `Client.setPusher(...)` and
  `Client.deletePusher(...)`.
- Use `NotificationClient` to resolve event-id-only push payloads when opening
  a notification.

### Finding 8: Device display-name patching is direct REST

Status: still an approved exception. Local binding inspection found pusher,
media, profile, account data, and raw room event support, but no current-device
display-name setter in the Swift bindings. The direct device-name patch remains
best-effort and non-blocking until SDK support exists.

Risk:

- Direct device update is a small but unnecessary REST exception.

Recommendation:

- Investigate whether current SDK bindings expose current-device display-name
  updates. If not, keep this as a documented REST exception until SDK support
  exists.
- Continue setting `initialDeviceName: "Synara iOS"` on login.

### Finding 9: Threads are UI-level reply-backed, not SDK thread-backed

Risk:

- Thread UI may diverge from Matrix clients that support true `m.thread`.

Recommendation:

- Use `Room.threadListService()` and `ThreadListEntriesListener`.
- Use timeline thread focus modes for thread screens.
- Use thread subscriptions if homeserver support is available.

### Finding 10: Read receipts, typing, and search should use SDK APIs

Risk:

- Desktop parity remains incomplete and ad hoc implementations would likely
  duplicate Matrix semantics poorly.

Recommendation:

- Use `Timeline.markAsRead(...)` and `Timeline.sendReadReceipt(...)`.
- Use `Room.typingNotice(isTyping:)`.
- Use room search iterators and SDK timeline event filters for search.

## Remediation Plan

### Phase SDK-1: Service Boundary Cleanup

Goal: remove ambiguity and prevent new REST-first work.

Tasks:

- Rename active Matrix services as the only production Matrix implementations.
- Keep deleted legacy REST services out of production and test code.
- Add a documented `MatrixRESTException` policy for any remaining REST call.
- Add a CI check or repository test that fails on new `/_matrix` or
  `URLSession.shared` usage outside approved exception files.

Acceptance criteria:

- Live environment wires only SDK-backed Matrix services except documented
  exceptions.
- No production SwiftUI view performs Matrix network calls directly.
- Tests use mocks or SDK-backed service fakes instead of old REST services.
- CI catches accidental new Matrix REST endpoints.

### Phase SDK-2: Timeline Actions And Local Echo

Goal: make message actions SDK-owned.

Tasks:

- Replace reaction REST with `Timeline.toggleReaction(...)`.
- Replace redaction REST with `Timeline.redactEvent(...)`.
- Implement edit through `Timeline.edit(...)`.
- Ensure action results are reflected by timeline diffs, not local manual item
  mutation.
- Wire permission-aware action availability from room power levels and SDK event
  state where available.

Acceptance criteria:

- React add/remove works in encrypted and unencrypted rooms.
- Redaction updates via SDK timeline diff.
- Edit round-trip works for own text message.
- Failed sends/actions show SDK-derived error state.
- Tests cover local echo, remote echo replacement, and failure rollback.

### Phase SDK-3: SDK Media Pipeline

Goal: use SDK media cache, upload handles, encrypted media support, and progress.

Tasks:

- Replace media upload REST with `Timeline.sendImage/sendFile/sendVideo`.
- Replace media thumbnail/download REST with `Client.getMediaFile(...)`.
- Map `mxc://` and encrypted media sources into SDK `MediaSource`.
- Add upload progress watcher and cancel/retry UI.
- Remove view-level thumbnail requests.

Acceptance criteria:

- Image/file upload creates SDK local echo and final remote event.
- Thumbnail load uses SDK media file/cache path.
- Encrypted media either decrypts through SDK or remains clearly blocked by one
  centralized policy.
- Upload progress/cancel/retry works without direct media endpoints.

### Phase SDK-4: Profile, Avatar, And Account Data

Goal: centralize profile/avatar/account data through SDK.

Tasks:

- Add `MatrixSDKProfileService`.
- Replace direct profile calls with `Client.getProfile(...)`.
- Replace self profile reads with `Client.displayName()` and `Client.avatarUrl()`.
- Replace Later account-data reads with `Client.accountData(...)`.
- Add `Client.setAccountData(...)` support for Later mutations.
- Add account-data listener if live Later updates are required.

Acceptance criteria:

- Timeline avatars render through shared SDK-backed cache.
- Room list and timeline do not issue view-level profile/media requests.
- Later loads and updates through SDK account data.
- No direct account-data URL remains in production code.

### Phase SDK-5: Push And Notification Resolution

Goal: keep iOS APNs native, but make Matrix pusher and notification content
SDK-owned.

Tasks:

- Replace pusher set/delete REST with `Client.setPusher(...)` and
  `Client.deletePusher(...)`.
- Use `NotificationSettings` for room/default/user/room mention preferences.
- Use `NotificationClient.getNotification(...)` or batch notification APIs for
  event-id-only payload resolution.
- Keep payload logging redacted.

Acceptance criteria:

- Pusher registration/unregistration passes against staging gateway.
- Test push routes through SDK-resolved room/event when possible.
- Room notification settings persist and reflect SDK state.
- No access token, APNs token, room ID, or event ID appears unredacted in logs.

### Phase SDK-6: Threads, Receipts, Typing, And Search

Goal: close high-frequency Matrix client behavior using SDK semantics.

Tasks:

- Implement true Matrix thread list with `Room.threadListService()`.
- Use thread-focused timelines for thread screens.
- Use `Timeline.markAsRead(...)` and read receipt APIs.
- Use `Room.typingNotice(isTyping:)`.
- Use SDK room/global search APIs where exposed.
- Add SDK-backed room member fetch for mentions autocomplete.

Acceptance criteria:

- Thread list and thread timeline match Matrix thread semantics.
- Read position updates consistently with desktop/other Matrix clients.
- Typing indicator sends and receives without polling.
- Message search returns SDK-backed results with event navigation.
- Mention autocomplete uses live member data.

### Phase SDK-7: Crypto And Verification Completion

Goal: move from basic crypto status to production E2EE support.

Tasks:

- Subscribe to verification, recovery, and backup state listeners.
- Implement SAS verification controller flow.
- Add recovery setup/reset UX using SDK recovery APIs.
- Use SDK encrypted media pipeline from Phase SDK-3.
- Add UTD triage and retry using SDK session IDs where exposed.

Acceptance criteria:

- Device verification can be completed from iOS.
- Recovery key restore/fix completes and updates status live.
- Encrypted media decrypts when keys are available.
- UTD state shows actionable recovery UI and never raw event dumps.

## Guardrails

- Prefer SDK state streams/listeners over polling.
- Prefer SDK send queues/local echo over manual optimistic UI.
- Prefer SDK media file/cache APIs over constructing media URLs.
- Prefer SDK room/timeline notification settings over push-rule JSON.
- REST exceptions must be named, documented, tested, and isolated from SwiftUI
  views.
- Direct Matrix HTTP must never be introduced in a view.
- Any fallback must degrade narrowly and visibly, not silently replace SDK
  semantics.

## Recommended Next Implementation Order

1. Phase SDK-1: service boundary cleanup and REST-exception guardrail.
2. Phase SDK-2: reactions/redactions/edits through timeline APIs.
3. Phase SDK-4 profile/avatar subset: remove view-level profile/media calls.
4. Phase SDK-3 media pipeline.
5. Phase SDK-5 pusher and notification resolution.
6. Phase SDK-6 thread/read/typing/search.
7. Phase SDK-7 crypto verification/recovery completion.

The first three items are the most urgent because they reduce architectural
ambiguity and directly improve daily-use correctness.
