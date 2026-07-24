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

**Session state (2026-07-24):**

- Integration: `feature/matrix-rust-sdk-full-replacement` @ `80c8fa3` (P0.1+P0.3 merged)
- Working branch: `matrix-rust/p0.2-parity-traceability` (tracks origin; no open P0.2 PR yet)
- Ancestry: integration is ancestor of P0.2 branch
- Checkpoint commits: `06d0f86` (traceability scaffold), `9c91e0f` (handoff)
- **Accepted this session:**
  - FR-7.8-004 through FR-7.8-009 (section 7.8 complete for P0.2 audit)
  - FR-7.9-001 through FR-7.9-010
- **Progress loop:** host scheduled task every **4 minutes**
  (ID `019f95928db7`) continues bounded FR audits; also daily durable
  orchestrator `9022c2f8-9b21-411a-acf9-a36c10515f72`
- **Next writer task:** FR-7.9-011 (multiple accounts isolated stores), then
  remaining shallow 7.9–7.11 rows (~18)
- No production Matrix Rust SDK replacement code accepted
- **No open P0.2 PR yet** (open after P0.2 complete)

Phase 0 evidence accepted before this handoff:

- P0.1 SDK usage inventory is merged to the integration branch.
- P0.3 exact Matrix Rust SDK 0.18.0 capability dossier is merged to the
  integration branch.
- P0.2 branch carries accepted 7.8–7.9.010 corrections; ~18 shallow rows remain
  in 7.9–7.11.

P0.2 is not complete. Resume at FR-7.9-011, then audit remaining 7.9–7.11
before declaring P0.2 accepted.

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
- FR-7.9-010: status `implemented`. Other-device multi-select
  `deleteMultipleDevices` + ActionUIA Password/SSO; current session uses
  LogoutDialog not multi-delete; OIDC external sessionEnd branch.

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

1. Complete P0.2 and merge its reviewed PR into integration.
2. Complete remaining Phase 0 gates: P0.4 Swift provenance, P0.5 toolchain
   compatibility, P0.6 performance baseline, and P0.7 migration UX.
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
