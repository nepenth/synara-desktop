# V-BURN importer taxonomy — Matrix Rust full replacement

| Field                  | Value                                                                    |
| ---------------------- | ------------------------------------------------------------------------ |
| Status                 | **Docs-only taxonomy**; no product code changed                          |
| Measured tip           | `3502125ae2f529ae66c403efe9b79ae734c1c464`                               |
| Base                   | `feature/matrix-rust-sdk-full-replacement`                               |
| Scope                  | Production `matrix-js-sdk` importers under `synara/src`                  |
| Current importers      | **159**                                                                  |
| P1.6 allowlist entries | **163**                                                                  |
| Policy                 | Full replacement; `dual_backend` forbidden; fail-closed                  |
| V-BURN                 | **Not started**; `active_slice` must not be `V-BURN`                     |
| Hold                   | **#327 HOLD**; do not merge or claim V-BURN-ready; **#39 remains gated** |

This is a classification snapshot, not a cutover plan or readiness claim. An
importer can be native-first, a non-native web fallback, a shared type/model
boundary, or an indirect dependency of a named residual. Import presence alone
does not prove that the selected desktop route is currently using the JS SDK.

## Measurement and reconciliation

The counts are taken from the generated [desktop SDK usage inventory](desktop-sdk-usage.md)
and checked against a direct source import scan at the measured tip:

- 159 production importer files under `synara/src`;
- 10 test importer files under `synara/src` (not in this taxonomy);
- 163 paths in [`p1.6-js-sdk-import-allowlist.json`](p1.6-js-sdk-import-allowlist.json);
- no current production importer is outside the allowlist;
- four allowlist paths are historical and no longer import the SDK:
  `synara/src/app/pages/client/inbox/Invites.tsx`,
  `synara/src/app/state/room-list/inviteList.ts`,
  `synara/src/app/utils/later.ts`, and
  `synara/src/app/utils/roomNotes.ts`.

The generated inventory reports 161 production-role files because it also
records two production files with no SDK import. They are not counted here.

## Exhaustive primary path buckets

These buckets are mutually exclusive and sum to all 159 current production
importers. They are intentionally path-oriented; the semantic residual overlay
below records the ownership that matters for migration sequencing.

| Primary bucket     |   Count | Migration reading                                                                     |
| ------------------ | ------: | ------------------------------------------------------------------------------------- |
| `client-lifecycle` |       2 | Bootstrap, live JS client construction, and crypto-store continuity                   |
| `component`        |      26 | Renderers, room/member controls, editor and pack UI boundaries                        |
| `feature`          |      54 | Room, space, lobby, settings, search, call, developer-tool, and notification surfaces |
| `hook`             |      44 | SDK model adapters, listeners, room/space state, and client context                   |
| `media-boundary`   |       1 | Authenticated MXC/media URL and download boundary                                     |
| `page`             |       8 | Client boot/status, space, inbox, and sidebar consumers                               |
| `plugin`           |       8 | Call/widget, pack, HTML, emoji, and via-server integrations                           |
| `shared-type`      |       1 | Shared Matrix content/type constants                                                  |
| `state`            |       5 | Room-list activity, drafts, and upload state                                          |
| `utility`          |      10 | Matrix, notification, room, sync, timeline, and sorting helpers                       |
| **Total**          | **159** | **Current production importer files**                                                 |

### `client-lifecycle` — 2

```text
synara/src/client/cryptoStoreContinuity.ts
synara/src/client/initMatrix.ts
```

### `component` — 26

```text
synara/src/app/components/AccountDataEditor.tsx
synara/src/app/components/CapabilitiesLoader.tsx
synara/src/app/components/JoinRulesSwitcher.tsx
synara/src/app/components/RenderMessageContent.tsx
synara/src/app/components/RoomSummaryLoader.tsx
synara/src/app/components/ServerConfigsLoader.tsx
synara/src/app/components/create-room/CreateRoomAliasInput.tsx
synara/src/app/components/create-room/utils.ts
synara/src/app/components/editor/autocomplete/RoomMentionAutocomplete.tsx
synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx
synara/src/app/components/editor/output.ts
synara/src/app/components/emoji-board/components/Item.tsx
synara/src/app/components/event-readers/EventReaders.tsx
synara/src/app/components/image-pack-view/RoomImagePack.tsx
synara/src/app/components/invite-user-prompt/InviteUserPrompt.tsx
synara/src/app/components/leave-room-prompt/LeaveRoomPrompt.tsx
synara/src/app/components/leave-space-prompt/LeaveSpacePrompt.tsx
synara/src/app/components/member-tile/MemberTile.tsx
synara/src/app/components/message/MsgTypeRenderers.tsx
synara/src/app/components/message/Reaction.tsx
synara/src/app/components/message/Reply.tsx
synara/src/app/components/message/content/PollContent.tsx
synara/src/app/components/room-avatar/RoomAvatar.tsx
synara/src/app/components/room-card/RoomCard.tsx
synara/src/app/components/room-intro/RoomIntro.tsx
synara/src/app/components/user-profile/UserChips.tsx
```

### `feature` — 54

```text
synara/src/app/features/add-existing/AddExisting.tsx
synara/src/app/features/call-status/CallRoomName.tsx
synara/src/app/features/call-status/LiveChip.tsx
synara/src/app/features/call-status/MemberGlance.tsx
synara/src/app/features/call-status/MemberSpeaking.tsx
synara/src/app/features/call/CallMemberCard.tsx
synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx
synara/src/app/features/common-settings/developer-tools/StateEventEditor.tsx
synara/src/app/features/common-settings/emojis-stickers/RoomPacks.tsx
synara/src/app/features/common-settings/general/RoomAddress.tsx
synara/src/app/features/common-settings/general/RoomEncryption.tsx
synara/src/app/features/common-settings/general/RoomHistoryVisibility.tsx
synara/src/app/features/common-settings/general/RoomJoinRules.tsx
synara/src/app/features/common-settings/general/RoomProfile.tsx
synara/src/app/features/common-settings/general/RoomPublish.tsx
synara/src/app/features/common-settings/general/RoomUpgrade.tsx
synara/src/app/features/common-settings/members/Members.tsx
synara/src/app/features/create-chat/CreateChat.tsx
synara/src/app/features/create-room/CreateRoom.tsx
synara/src/app/features/create-space/CreateSpace.tsx
synara/src/app/features/lobby/Lobby.tsx
synara/src/app/features/lobby/RoomItem.tsx
synara/src/app/features/lobby/SpaceHierarchy.tsx
synara/src/app/features/lobby/SpaceItem.tsx
synara/src/app/features/message-search/MessageSearch.tsx
synara/src/app/features/message-search/SearchFilters.tsx
synara/src/app/features/message-search/SearchResultGroup.tsx
synara/src/app/features/message-search/useMessageSearch.ts
synara/src/app/features/room-nav/RoomNavItem.tsx
synara/src/app/features/room-settings/RoomSettings.tsx
synara/src/app/features/room/CommandAutocomplete.tsx
synara/src/app/features/room/MembersDrawer.tsx
synara/src/app/features/room/RoomInput.tsx
synara/src/app/features/room/RoomSidePanel.tsx
synara/src/app/features/room/RoomView.tsx
synara/src/app/features/room/RoomViewFollowing.tsx
synara/src/app/features/room/RoomViewHeader.tsx
synara/src/app/features/room/RoomViewTyping.tsx
synara/src/app/features/room/jump-to-time/JumpToTime.tsx
synara/src/app/features/room/message/Message.tsx
synara/src/app/features/room/message/MessageEditor.tsx
synara/src/app/features/room/message/NativeEventContent.tsx
synara/src/app/features/room/message/Reactions.tsx
synara/src/app/features/room/msgContent.ts
synara/src/app/features/room/reaction-viewer/ReactionViewer.tsx
synara/src/app/features/room/room-notes/RoomNotesPanel.tsx
synara/src/app/features/room/room-pin-menu/RoomPinMenu.tsx
synara/src/app/features/search/Search.tsx
synara/src/app/features/settings/notifications/AllMessages.tsx
synara/src/app/features/settings/notifications/KeywordMessages.tsx
synara/src/app/features/settings/notifications/NotificationModeSwitcher.tsx
synara/src/app/features/settings/notifications/SpecialMessages.tsx
synara/src/app/features/settings/notifications/SystemNotification.tsx
synara/src/app/features/space-settings/SpaceSettings.tsx
```

### `hook` — 44

```text
synara/src/app/hooks/types.ts
synara/src/app/hooks/useAccountDataCallback.ts
synara/src/app/hooks/useAuthMetadata.ts
synara/src/app/hooks/useCall.ts
synara/src/app/hooks/useCallEmbed.ts
synara/src/app/hooks/useCapabilities.ts
synara/src/app/hooks/useCommands.ts
synara/src/app/hooks/useGetRoom.ts
synara/src/app/hooks/useImagePacks.ts
synara/src/app/hooks/useLocalRoomSummary.ts
synara/src/app/hooks/useMatrixClient.ts
synara/src/app/hooks/useMemberEventParser.tsx
synara/src/app/hooks/useMemberFilter.ts
synara/src/app/hooks/useMemberPowerTag.ts
synara/src/app/hooks/useMemberSort.ts
synara/src/app/hooks/useMembership.ts
synara/src/app/hooks/useNotificationMode.ts
synara/src/app/hooks/usePowerLevelTags.ts
synara/src/app/hooks/usePowerLevels.ts
synara/src/app/hooks/usePushRule.ts
synara/src/app/hooks/useRecentEmoji.ts
synara/src/app/hooks/useRelations.ts
synara/src/app/hooks/useRoom.ts
synara/src/app/hooks/useRoomAccountData.ts
synara/src/app/hooks/useRoomActivity.ts
synara/src/app/hooks/useRoomAliases.ts
synara/src/app/hooks/useRoomCreators.ts
synara/src/app/hooks/useRoomDirectoryVisibility.ts
synara/src/app/hooks/useRoomEvent.ts
synara/src/app/hooks/useRoomEventReaders.ts
synara/src/app/hooks/useRoomLatestRenderedEvent.ts
synara/src/app/hooks/useRoomMembers.ts
synara/src/app/hooks/useRoomMeta.ts
synara/src/app/hooks/useRoomPinnedEvents.ts
synara/src/app/hooks/useRoomState.ts
synara/src/app/hooks/useRoomsNotificationPreferences.ts
synara/src/app/hooks/useSidebarItems.ts
synara/src/app/hooks/useSpace.ts
synara/src/app/hooks/useSpaceHierarchy.ts
synara/src/app/hooks/useStateEvent.ts
synara/src/app/hooks/useStateEventCallback.ts
synara/src/app/hooks/useSyncState.ts
synara/src/app/hooks/useUserPresence.ts
synara/src/app/hooks/useUserProfile.ts
```

### Remaining primary buckets

```text
media-boundary (1): synara/src/app/matrix/media.ts

page (8):
synara/src/app/pages/client/ClientNonUIFeatures.tsx
synara/src/app/pages/client/ClientRoot.tsx
synara/src/app/pages/client/SyncStatus.tsx
synara/src/app/pages/client/explore/Server.tsx
synara/src/app/pages/client/inbox/Notifications.tsx
synara/src/app/pages/client/sidebar/SpaceTabs.tsx
synara/src/app/pages/client/space/Space.tsx
synara/src/app/pages/client/syncStatusCopy.ts

plugin (8):
synara/src/app/plugins/call/CallEmbed.ts
synara/src/app/plugins/call/CallWidgetDriver.ts
synara/src/app/plugins/call/utils.ts
synara/src/app/plugins/custom-emoji/ImagePack.ts
synara/src/app/plugins/custom-emoji/utils.ts
synara/src/app/plugins/react-custom-html-parser.tsx
synara/src/app/plugins/recent-emoji.ts
synara/src/app/plugins/via-servers.ts

shared-type (1): synara/src/types/matrix/common.ts

state (5):
synara/src/app/state/hooks/roomList.ts
synara/src/app/state/room-list/roomActivity.ts
synara/src/app/state/room-list/utils.ts
synara/src/app/state/room/roomInputDrafts.ts
synara/src/app/state/upload.ts

utility (10):
synara/src/app/utils/matrix.ts
synara/src/app/utils/notifications.ts
synara/src/app/utils/polls.ts
synara/src/app/utils/room.ts
synara/src/app/utils/sort.ts
synara/src/app/utils/syncLifecycle.ts
synara/src/app/utils/syncSplashRecovery.ts
synara/src/app/utils/timelineLifecycle.ts
synara/src/app/utils/timelineLinks.ts
synara/src/app/utils/timelineOpening.ts
```

## Semantic residual overlay

The following categories explain what the path inventory means operationally.
They are an overlay, so one source file may appear in more than one row when it
bridges a shared SDK model and a named feature. The exhaustive, non-overlapping
accounting is the primary path taxonomy above.

| Residual category                                           | Importer surface at this tip                                                                                                                                                                                                                                                      | Current disposition and gate                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bootstrap / `initMatrix` / live client construction         | `synara/src/client/initMatrix.ts`; `synara/src/client/cryptoStoreContinuity.ts`                                                                                                                                                                                                   | Hard migration owner. `initMatrix.ts` constructs the live JS client and IndexedDB/crypto continuity; the continuity file is type/compatibility coupling. No V-BURN work is implied.                                                                                                   |
| Web-only / legacy fallback: composer upload and pack `get*` | `features/room/RoomInput.tsx`, `features/room/msgContent.ts`, `state/upload.ts`, `utils/matrix.ts`, `hooks/useImagePacks.ts`, and `plugins/custom-emoji/utils.ts`                                                                                                                 | Desktop native paths are fail-closed where already landed; JS callbacks remain for non-native web/legacy operation. The `custom-emoji/utils.ts` `getGlobalImagePacks` / `getRoomImagePack(s)` / `getUserImagePack` helpers are the physical-delete residual and are **V-BURN-gated**. |
| Authenticated media / C4 boundary                           | `synara/src/app/matrix/media.ts` plus the shared media functions in `synara/src/app/utils/matrix.ts`                                                                                                                                                                              | JS MXC conversion/download remains a media boundary for the current renderer and web path. **C4 is Not confirmed**; this taxonomy does not claim media cutover.                                                                                                                       |
| CallWidget-adjacent, post-#349                              | `features/call-status/**` (4), `features/call/CallMemberCard.tsx`, `plugins/call/**` (3), `hooks/useCall.ts`, and `hooks/useCallEmbed.ts` — 10 importers                                                                                                                          | #328 closes the native upload owner. The remaining `CallWidgetDriver` `getMediaConfig`, `downloadFile`, and `getKnownRooms` surfaces remain JS residuals; see [the #349 inventory](v-send-call-widget-residual.md).                                                                   |
| Developer-tools / **V-SEND.R-DEVTOOL**                      | Direct importers `features/common-settings/developer-tools/SendRoomEvent.tsx` and `StateEventEditor.tsx`; `DevelopTools.tsx` is an indirect SDK-backed consumer                                                                                                                   | Raw room state/account-data and event writes remain a deliberate developer surface. The implementation gate is after **C3–C5 live proofs**, not now; see [the developer-tools inventory](v-send-devtool-inventory.md).                                                                |
| Pack read / **V-SEND.R-PACK-READ**                          | `hooks/useImagePacks.ts`, `plugins/custom-emoji/ImagePack.ts`, `plugins/custom-emoji/utils.ts`, `components/image-pack-view/RoomImagePack.tsx`, `features/common-settings/emojis-stickers/RoomPacks.tsx`, and pack consumers such as `components/emoji-board/components/Item.tsx` | Native snapshot and subscription work is landed; the remaining JS read-helper deletion is gated on retiring the non-native web fallback. Keep pack model/render code until its consumers leave the JS owner. See [the pack-read residual](v-send-pack-read-residual.md).              |
| Pack write/upload / named send residuals                    | `features/common-settings/emojis-stickers/RoomPacks.tsx`, `components/image-pack-view/RoomImagePack.tsx`, `state/upload.ts`, `utils/matrix.ts`, and their pack/media consumers                                                                                                    | Native desktop write/upload owners are fail-closed where landed; legacy web branches remain. This is not permission to delete shared upload/model code or to start V-BURN.                                                                                                            |
| Timeline / **V-TIMELINE.C3–C5**                             | Room feature/message paths, `components/message/**`, timeline/room hooks, `state/room-list/**`, and `utils/timeline*.ts`                                                                                                                                                          | These importers include the active JS model/listener boundary and media/action consumers. C3, C4, and C5 remain **Not confirmed**; do not infer V-BURN readiness from the native presenter work.                                                                                      |
| Room, space, lobby, membership, search, and notifications   | `features/lobby/**`, create/space/room settings, room/member hooks and components, message search, and `features/settings/notifications/**`                                                                                                                                       | These are remaining SDK model/read/listener consumers or shared adapters without a single new residual claim in this document. The generated inventory's category counts are candidate usage classifications, not proof of native ownership.                                          |
| Types / unused-looking residual                             | Type-only or representation-heavy coupling includes `client/cryptoStoreContinuity.ts`, `app/hooks/types.ts`, `app/hooks/useMatrixClient.ts`, `app/matrix/media.ts`, `app/utils/polls.ts`, `app/components/message/content/PollContent.tsx`, and `types/matrix/common.ts`          | Do not equate type-only imports with safe deletion: `useMatrixClient`, `PollContent`, and shared content constants still participate in runtime contracts. Remove only with a complete owning-path proof.                                                                             |

## Queue and operator constraints

At this snapshot the residual queue is unchanged:

1. Pack `get*` helper deletion is **V-BURN-gated**.
2. **C3–C5 are Not confirmed**.
3. **R-DEVTOOL starts only after C3–C5**, per its implementation gate.
4. **#327 is HOLD**. Do not merge, claim V-BURN-ready, set
   `active_slice=V-BURN`, or introduce a dual backend.
5. **#39 is gated** and `main` is out of scope.

This document is preparation only. It does not alter the scoreboard's tip SHA,
start a burn slice, or authorize product work while the product is idle.

## Verification commands

The source inventory can be rechecked without changing product code:

```text
rg -l --glob 'synara/src/**/*.{ts,tsx,js,jsx,mjs,cjs}' \
  "from ['\"]matrix-js-sdk(?:/|['\"])" synara/src

jq -r '.files[]
  | select(.role == "production" and (.imports | length) > 0)
  | .path' docs/matrix-rust-sdk/desktop-sdk-usage.json
```

The generated report remains the source for AST candidate counts; its method
and listener categories are not type-checked and must not be read as proof that
every candidate is an SDK call.
