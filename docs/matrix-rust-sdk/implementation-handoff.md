# Matrix Rust SDK Replacement — Execution Handoff

Last updated: 2026-07-24

Authoritative program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)

Traceability artifacts:

- [`feature-parity-traceability.json`](feature-parity-traceability.json)
- [`feature-parity-traceability.md`](feature-parity-traceability.md)

## Current state

This remains a complete replacement program: desktop production must move from
`matrix-js-sdk` to Matrix Rust SDK, not retain a selectable or permanent second
backend. No production Matrix Rust SDK migration code has been accepted yet.

### Orchestrator goal (persistent)

**Objective:** Execute the Matrix Rust SDK replacement plan using bounded native
sub-agent implementation tasks. Independently review and validate every change;
only commit, push, open PRs, or merge accepted work. Never merge to `main`
without explicit user approval.

**Host task id:** `9022c2f8-9b21-411a-acf9-a36c10515f72`
(`matrix-rust-sdk-replacement-orchestrator`, daily check-in)

**Session state (2026-07-24, P0.4 accepted — PR to integration):**

- **Integration tip (unchanged until P0.4 merge):**
  `feature/matrix-rust-sdk-full-replacement` @
  `c76f4ff7cb19e3e6d1536e9b8f7b8d269f374dcb`
  (P0.1 + P0.3 + **P0.2** merged; handoff note after P0.2)
- **Active work branch:** `matrix-rust/p0.4-swift-rust-provenance` (docs-only
  P0.4 provenance; **not merged to integration yet**)
- **P0.2 PR:** https://github.com/nepenth/synara-desktop/pull/42 — **MERGED**
  into integration (docs-only). **Never merge to `main` without explicit user
  approval.**
- **P0.2 quality audit of §7.8–7.11:** COMPLETE (source-line evidence; honest
  partial/NCE gates retained; zero shallow rows remaining in 7.8–7.11)
- **P0.4:** **accepted by independent review** (docs-only). Evidence:
  `swift-rust-version-provenance.md` + `.json`. Embedded Rust commit for
  components-swift `26.06.06` proven as
  `1c44fb66214667c6d00acaf72ab592493653708b` (same as desktop
  `matrix-sdk-0.18.0`). Alignment decision: **exact same git commit** (A); iOS
  pin and desktop crate pins **unchanged**. PR targets integration — **do not
  claim MERGED until PR lands**. **Never merge to `main` without explicit user
  approval.**
- **Optional later:** §7.1–7.7 scaffold rows may still have shallow notes if a
  full-matrix depth pass is desired beyond the handoff resume scope
- **Progress loop:** 4-minute scheduler `019f95928db7` (retarget to Phase 0
  remaining); daily durable orchestrator `9022c2f8-9b21-411a-acf9-a36c10515f72`
- **No production Matrix Rust SDK replacement code accepted yet**

Phase 0 evidence accepted:

- P0.1 SDK usage inventory — merged
- P0.3 exact Matrix Rust SDK 0.18.0 capability dossier — merged
- P0.2 feature-parity traceability (§7.8–7.11 quality audit) — **merged**
- P0.4 Swift/Rust version provenance — **accepted, awaiting integration PR merge**

**Next program work (Phase 0 remaining):**

1. **P0.4** merge accepted PR into integration  
2. **P0.5** Toolchain compatibility (Rust 1.93)  
3. **P0.6** Performance baseline  
4. **P0.7** Migration UX

Accepted notification findings that must be preserved:

- FR-7.8-001: current global/default push-rule behavior is implemented; its
  cutover test must use the named All Messages controls and typed Rust push-rule
  behavior, not capability IDs alone.
- FR-7.8-002: current per-room modes are implemented through
  `RoomNotificationModeSwitcher` and `useRoomsNotificationPreferences`; global
  controls cannot substitute.
- FR-7.8-003: status is `partial` under
  `GATE-7.8-003-INVITE-PREFERENCE`. Invite delivery is not a persistent,
  user-configurable invite-notification preference.
- FR-7.8-004: status `implemented`. Desktop native notification **generation**
  is owned by `ClientNonUIFeatures` (`InviteNotifications`,
  `MessageNotifications`, plus `AgentApprovalNotifications` /
  `LaterReminderNotifications` emitters) with `SystemNotification` **enablement
  only** (`showNotifications` + OS/browser permission) and platform bridge
  `normalizeSystemNotificationRequest` / `showPlatformNotification`. Push-rule
  preference UIs, badge/tray/favicon, EmailNotification pusher, SC-057 alone,
  helper/fixture-only, and raw `/_matrix/` HTTP never pass. Cutover is P9.2
  Rust-owned notification candidate stream + desktop bridge.
- FR-7.8-005: status `implemented`. Unread/badge summaries are owned by
  `roomToUnreadAtom` (Timeline/Receipt/MyMembership/MarkedUnread listeners +
  parent roll-up), `badgeSummary.summarizeNotifications` (`appBadgeCount` vs
  `inboxBadgeCount`), `RoomNavItem` `UnreadBadge` (highlight vs total), and
  `PlatformBadgeAndTrayUpdater` / `setPlatformBadgeCount`. SC-057 alone,
  push-rule preference UI, native generation path, and helper-only never pass.
  Cutover is P9.3/P4.3 Rust-owned unread/highlight + product badge/IPC DTO.
- FR-7.8-006: status `implemented`. Event resolution and deep-link routing use
  `buildDesktopNotificationRoomRoute` + `normalizeSystemNotificationRequest` /
  platform route pass-through + `navigateRoom` open (message/agent/later);
  invite notifications use distinct `getInboxInvitesPath`; inbox list Open uses
  `navigateRoom(roomId, openEventId)` with thread-root resolution;
  `timelineOpening` focuses event context. SC-032/SC-022 alone, push-rule UI,
  badge-only, generation-only, and helper-only never pass. Cutover is P9.4/P4.8
  Rust-owned event identity + Synara route DTOs.
- FR-7.8-007: status `implemented`. Focus/suppression owned by
  MessageNotifications gates (`document.hasFocus` + selected room or
  notifications inbox; SYNCING; Mute; self; showNotifications; unread delta)
  with SystemNotification/tray DND enablement; Invite has SYNCING +
  showNotifications without a focused-room gate (do not invent). Cutover P9.3
  via Rust candidate stream + product focus state; SC-057 alone never passes.
- FR-7.8-008: status `partial` under `GATE-7.8-008-ENCRYPTED-PRIVATE-MODE`.
  Message/Invite/Later OS bodies avoid event plaintext/ciphertext; AgentApproval
  may disclose `commandPreview`; `privacy:'private'` is never set by generation
  and is dropped before `desktop_notify`. Cutover must not dump decrypted content
  into OS notifications without a privacy gate.
- FR-7.8-009: status `implemented` (iOS). `SynaraPushService` +
  `MatrixRustSDKService.setPusher`/`deletePusher` (SDK-owned); resolveRoute
  including sparse event-id fallback; Settings registration UI; existing XCTest
  baseline. SC-057/SC-058 are not pusher CRUD. Desktop APNs pusher N/A.
- FR-7.9-001: status `implemented`. Ordered path: IndexedDB stores →
  `store.startup` → `initRustCrypto` → `assertCryptoStoreContinuity` → ready
  client **without** sync → product `startClient` only after crypto readiness
  (`initMatrix.ts` + `cryptoStoreContinuity.ts` + ClientRoot). Current product
  is browser IndexedDB + rust-crypto wasm; cutover is native encrypted SQLite
  under Rust. SC-061/062/083 compile-only blocked states are not runtime pass.
- FR-7.9-002: status `implemented`. Cross-signing active flag is
  `useCrossSigningActive` via `m.cross_signing.master` account-data presence
  (not JS `getCrossSigningStatus`); device `crossSigningVerified` via
  `getDeviceVerificationStatus`; Devices/UnverifiedTab/Logout gated on active;
  bootstrap/reset via `bootstrapCrossSigning`. Ceremony SAS is FR-7.9-005.
  SC-064/061/062 compile-only are not runtime pass.
- FR-7.9-003: status `implemented`. Own-account device list via
  `useDeviceList` → `mx.getDevices()`; Current vs Others split by
  `getDeviceId()` (other = other sessions of the logged-in user, not third
  parties). Devices/OtherDevices/UnverifiedTab; refresh via
  `CryptoEvent.DevicesUpdated`. SC-067 primary (compile-only blocked ≠ pass).
- FR-7.9-004: status `implemented`. Trust bit is
  `getDeviceVerificationStatus.crossSigningVerified` via `verifiedDevice` →
  VerificationStatus badges on Devices/OtherDevices/UnverifiedTab/Logout;
  refresh via DevicesUpdated. SAS ceremony is FR-7.9-005; list is FR-7.9-003.
  SC-063/064 compile-only ≠ product pass.
- FR-7.9-005: status `implemented`. SAS + request inbox:
  `verificationRequestInbox` (install before startClient) queues
  VerificationRequestReceived; ReceiveSelfDeviceVerification presents inbound;
  requestOwnUserVerification / requestDeviceVerification outbound; DeviceVerification
  SAS accept/start/verify/cancel. Trust status is 004; device list is 003.
  SC-084 + GAP-SAS compile-only ≠ product pass.
- FR-7.9-006: status `implemented`. Recovery setup via DeviceVerificationSetup
  (createRecoveryKeyFromPassphrase → bootstrapSecretStorage → resetKeyBackup);
  recovery key display/entry; BackupRestore status + restoreKeyBackup; auto-restore
  on KeyBackupDecryptionKeyCached; repair via reset re-setup. Room-key file
  import/export is FR-7.9-007. SC-065/066 compile-only ≠ product pass.
- FR-7.9-007: status `implemented` (retained UI). Settings Devices LocalBackup
  exportRoomKeysAsJson + encryptMegolmKeyFile → synara-keys.txt; import decrypt
  + importRoomKeysAsJson. Not server key backup (006). SC-061 compile-only ≠ pass.
- FR-7.9-008: status `implemented`. Automatic UTD retry via
  decryptAllTimelineEvent → attemptDecryption({isRetry:true}) on encrypted
  pagination; EncryptedContent MatrixEventEvent.Decrypted re-render; permanent
  UTD fallbacks. No dedicated Retry button.
- FR-7.9-009: status `implemented`. Key-backup state listeners in
  `useKeyBackup.ts` (KeyBackupStatus / SessionsRemaining / Failed /
  DecryptionKeyCached) drive BackupRestore Connected/Disconnected/Syncing/
  failure/trust UI. Recovery setup is 006; LocalBackup files is 007.
- FR-7.9-010: status `implemented`. Other-device multi-select delete via
  `OtherDevices` `mx.deleteMultipleDevices` + sticky Logout; 401 UIA via
  `useUIAMatrixError` → `ActionUIA` Password/SSO; success
  `refreshDeviceList`; OIDC path external `sessionEnd`. Current session uses
  `DeviceLogoutBtn` (not multi-delete). Primary Rust gaps
  GAP-DEVICE-NAMING-DELETE + GAP-UIA; SC-067 list-only secondary.
  List ownership is 003; trust 004; SAS 005; recovery UIA 006.
- FR-7.9-011: status **`partial`** under
  `GATE-7.9-011-CONCURRENT-MULTI-ACCOUNT-STORES`. Sequential single-active
  isolation only: fixed `MATRIX_LOCAL_STORE_NAMES` clear-and-replace via
  `clearMatrixStoresForIdentityChange` on fresh-login identity mismatch;
  single-slot `FALLBACK_SESSION_KEYS` / `clearSessionLocalStorage`;
  `ClientRoot` one `getActiveSession`→`initClient`. Concurrent dual clients /
  per-userId parallel stores are product non-goals (plan text “fully isolated
  multi-account stores” not met). Continuity is FR-7.9-012; logout wipe
  FR-7.1-010; crypto boot FR-7.9-001.
- FR-7.9-012: status `implemented`. Restored sessions must not wipe stores
  (`initClient` freshLogin gate); reopen fixed IndexedDB + `initRustCrypto`;
  `assertCryptoStoreContinuity` (`getCrypto`/`getOwnDeviceKeys`/
  `downloadKeysForUsers`); `stopClient` without store delete on safety fail;
  ClientRoot store-intact UI + Retry only for `server-query-incomplete`.
  Upgrades = reopen same fixed names (no separate migrator). Store-init order
  is FR-7.9-001; multi-account wipe FR-7.9-011; corruption FR-7.9-013.
  Cutover P2.2/P8.8/P13.2 SC-083+SC-061 compile-shape-only.
- FR-7.9-013: status **`partial`**. Continuity anomaly detection +
  non-destructive ClientRoot guidance; no true corruption integrity scan or
  automatic non-destructive repair.
- FR-7.10-001: status `implemented`. Room-scoped `mx.search` room_events
  search_term + filter.rooms limit 20; `next_batch`/`nextToken` infinite query
  pagination; MessageSearch/useMessageSearch. Global search is 002.
- FR-7.10-002: status `implemented`. Home/Space Message Search
  `allowGlobal` + Global chip → `rooms` undefined → `mx.search` without room filter.
  SC-071 only (not SC-072 local). Room-scoped is 001.
- FR-7.10-003: status `implemented`. Two-layer Message Search filters: server
  `filter.rooms`/`filter.senders`/`search_term`/`order_by` via
  `useMessageSearch`/`mx.search`; client type + from/to via
  `filterMessageSearchGroups` / `messageSearchFilters`; SearchFilters
  multi-room/type/sender/date UI. Global is 002; room-scoped default is 001.
- FR-7.10-004: status `implemented`. Open Chip → `navigateRoom(eventId)` →
  `getRoomTimelineOpenMode` focused-event. Not Matrix `/search` event_context
  (before/after 0). JumpToTime not on Open path.
- FR-7.10-005: status **`partial`** under
  `GATE-7.10-005-USER-DIRECTORY-SEARCH`. Public rooms: Explore
  `PublicRooms` POST `/publicRooms`. Server user-directory is widget-only
  (`CallWidgetDriver`); product Invite/CreateChat are exact-ID/local only.
- FR-7.10-006: status `implemented`. Explicit decision: message search is
  server `mx.search` only (SC-071); SC-072 experimental local **not adopted**.
  `useAsyncSearch` is list filter honesty only, not message bodies.
- FR-7.10-007: status **`partial`** under
  `GATE-7.10-007-SEARCH-ABORT-SIGNAL`. Stale isolation via React Query
  `queryKey=['search', term, order, rooms, senders]`; transport cancel
  missing (`mx.search` without optional `abortSignal`; queryFn does not
  forward RQ `signal`).

- FR-7.11-001: status `implemented`. DISPLAY via useCallMembers
  (session.memberships + MembershipsChanged) → RoomNavItem/CallView/CallStatus
  Live UIs. rust_target gap presence boolean-only (not SC-082 primary).
  Cutover residual GATE-7.11-001-FULL-MEMBERSHIP-LIST-PROJECTION.


- FR-7.11-002: status `implemented`. Element Call embed: createCallEmbed /
  CallEmbed.getWidget → `/public/element-call/index.html` + iframe +
  ClientWidgetApi postMessage + CallWidgetDriver capabilities. SC-082
  experimental-widgets (not membership display 001). Widget plumbing ≠ call parity.


- FR-7.11-003: status **`partial`** under
  `GATE-7.11-003-NATIVE-OR-PRODUCT-MEMBERSHIP-WRITE`. Join via useCallStart/
  JoinCall; leave via hangup; decline capability-only (no product Decline UI);
  member status after actions (display ownership 001). Widget-mediated write.


- FR-7.11-004: status **`partial`** under
  `GATE-7.11-004-NATIVE-MATRIXRTC-KEY-SESSION`. Widget-mediated to-device
  encrypt/queue + feedToDevice + encryption_keys capabilities; no product-
  owned native MatrixRTC key-session API.


- FR-7.11-005: status **`partial`** under
  `GATE-7.11-005-LOGOUT-WINDOW-CLOSE-HANGUP-CLEANUP`. Hangup/dispose pipeline
  present; room nav retains session; logout/window-close lack explicit hangup.


- FR-7.11-006: status **`partial`** under
  `GATE-7.11-006-CSP-ORIGIN-HARDENING`. Tauri CSP + iframe sandbox + same-origin
  EC + parentUrl; residual: no HTML CSP meta, scripts+same-origin sandbox,
  broad connect-src, no strictOriginCheck.


- FR-7.11-007: status **`partial`** under
  `GATE-7.11-007-EXPERIMENTAL-WIDGETS-RISK-ACCEPTANCE`. Plan/dossier risk language
  present (SC-082 blocked, P10.1, RISK-CALLS); no formal product acceptance artifact
  for pin 0.18.0 yet.


- FR-7.11-008: status **`not-currently-exposed`** under
  `GATE-7.11-008-DOCUMENTED-CONTINGENCY-ARTIFACT`. Plan §7.11 + P10.7 contingency
  language present; formal contingency decision artifact not delivered. Must not
  reintroduce permanent dual-backend without new user decision.



## Branch and PR contract

1. Start each task branch from the current
   `feature/matrix-rust-sdk-full-replacement` integration branch:
   `matrix-rust/<task-id>-<short-slug>`.
2. Task PRs target the integration branch only. Do not target or merge `main`.
3. Keep each PR to one bounded task with production code, tests, fixtures, and
   necessary documentation in the same reviewable change.
4. Do not mix refactors, formatting sweeps, dependency upgrades, unrelated bug
   fixes, or changes from another task.
5. Rebase/reconcile with integration only after reviewing conflict semantics and
   rerunning the affected task gate.
6. Commit messages must name the task ID. Generated lockfiles/schema changes
   belong with the change that requires them.
7. The final integration-to-`main` PR needs every Section 14 final gate, green
   checks, independent review, and explicit user approval. It is never an
   automatic merge.

## Writer-harness contract

The implementation harness may write only its explicitly allowed task scope. It
must not commit, push, rebase, switch branches, open/merge PRs, delete unrelated
files, or alter this program plan unless the task explicitly authorizes a
documentation update.

Every task prompt must supply:

- task ID, exact branch/base commit, allowed paths, and prohibited paths;
- relevant plan sections and pinned upstream evidence;
- concrete behavior, non-goals, deletion/convergence target, and failure modes;
- exact commands/tests plus live Synapse, platform, or fixture requirements;
- prohibition on `matrix-js-sdk` additions, runtime raw `/_matrix/` HTTP,
  backend selectors, dual clients, weak/fixture-only substitutions, and
  suppressed errors;
- a stop condition when typed SDK support is absent or experimental beyond an
  approved gate.

Use a fresh writer session for each task. Keep prompts narrow enough that a
reviewer can compare the entire diff to the task acceptance criteria.

## Required reviewer evidence

Before accepting a task or merging its PR, the reviewer must independently:

1. Inspect branch/base, `git status`, complete diff, and changed-file scope.
2. Check the exact pinned Matrix Rust SDK API/source—not moving upstream `main`.
3. Reproduce all stated tests and inspect that they exercise the required
   behavior rather than helpers, mocks, renamed controls, or compile shape only.
4. Audit for dual clients, raw Matrix runtime HTTP, SDK-shaped IPC leakage,
   insecure token/store/media handling, lifecycle races, and unremoved legacy
   paths.
5. Check desktop and iOS impacts where the task touches shared contracts.
6. Reject defects with an evidence-backed correction request; review the full
   resulting diff after every correction.
7. Confirm CI is green and the task's plan/documentation/fixtures remain
   synchronized.

## Remaining program sequence

1. Complete P0.2 — **done** (merged into integration).
2. Complete remaining Phase 0 gates: **P0.4** Swift provenance (accepted;
   merge PR into integration), then **P0.5** toolchain compatibility,
   **P0.6** performance baseline, and **P0.7** migration UX.
3. Build the Phase 1 foundation: Rust 1.93, exact SDK pins, versioned
   Synara-owned IPC schemas, lifecycle/store security, and test infrastructure.
4. Implement Phases 2–11 by bounded capability task: authentication/sync,
   rooms/timelines, messaging/media, E2EE/verification/recovery, account data,
   notifications, search, spaces/threads, and calls/widgets.
5. Complete Phase 12 cutover and deletion: Rust is sole desktop Matrix owner;
   no `matrix-js-sdk`, JS sync/crypto/store, or product raw Matrix HTTP remains.
6. Complete Phases 13–14: reliability/performance/security/release validation,
   final deletion audit, integration review, and an explicitly approved final PR
   to `main`.

## Handoff acceptance checklist

- [ ] Task branch is based on the latest integration branch.
- [ ] PR targets integration, not `main`.
- [ ] Diff is bounded and contains no unapproved dependency/version drift.
- [ ] Required tests are present and independently reproduced.
- [ ] Capability/traceability artifacts are updated when behavior evidence
      changes.
- [ ] Reviewer findings are resolved and the complete final diff is re-reviewed.
- [ ] CI is green before merge.
