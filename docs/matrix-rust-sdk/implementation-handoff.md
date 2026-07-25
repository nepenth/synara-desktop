# Matrix Rust SDK Replacement — Execution Handoff

Last updated: 2026-07-25

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

**Session state (2026-07-25, P1.4 MERGED — next P1.5):**

- **Integration tip:** `feature/matrix-rust-sdk-full-replacement` @
  `97c30cfced3d4b9c723994cf8a6820b9d09ce3c6`
  (Phase 0 complete + **P1.1**–**P1.4** **MERGED**; tip:
  `feat(matrix): merge P1.4 Synara domain DTOs`)
- **Active work:** next **P1.5** — IPC protocol contract tests expansion
  (serialization round trips, unknown variants, invalid payloads, bounds,
  sequence gaps, stale generations, schema compatibility). **No** production
  login/sync; **no** dual-backend; **no** Matrix production Tauri commands; JS
  client remains sole runtime backend.
- **P0.2 PR:** https://github.com/nepenth/synara-desktop/pull/42 — **MERGED**
  into integration (docs-only). **Never merge to `main` without explicit user
  approval.**
- **P0.2 quality audit of §7.8–7.11:** COMPLETE (source-line evidence; honest
  partial/NCE gates retained; zero shallow rows remaining in 7.8–7.11). **Do not
  re-open FR-7.8–7.11 rows or re-promote FR-7.9-011.**
- **P0.4:** **MERGED** into integration (docs-only, PR #44). Evidence:
  `swift-rust-version-provenance.md` + `.json`. Embedded Rust commit for
  components-swift `26.06.06` proven as
  `1c44fb66214667c6d00acaf72ab592493653708b` (same as desktop
  `matrix-sdk-0.18.0`). Alignment decision: **exact same git commit** (A); iOS
  pin and desktop crate pins **unchanged**.
- **P0.5:** **MERGED** into integration (docs + isolated probes only, PR #45).
  Artifacts: `toolchain-compatibility-report.md` + `.json`; coexistence probe
  `probes/tauri-matrix-sdk-compat/`. Verdict: **`pass-with-residuals`**.
  Local proofs on Rust 1.93: matrix-sdk 0.18 probe `cargo check --locked` PASS;
  production `src-tauri` `cargo check --locked` PASS (no SDK); Tauri 2.11 +
  matrix-sdk/ui 0.18 coexistence `cargo check/test --locked` PASS including
  `--target x86_64-apple-darwin`. Production `src-tauri` **not** modified; no
  production matrix-sdk deps; workflows not edited. Residuals: Linux not local;
  full universal product + notarization not re-run with SDK; permanent pin is
  **P1.1**. **Never merge to `main` without explicit user approval.**
- **P0.6:** **MERGED** into integration (docs + harness only, PR #46). Artifacts:
  `performance-baseline.md` + `.json`; aggregator
  `scripts/matrix-rust-p0.6-baseline-harness.mjs` (+ unit test). Baseline is
  current **`matrix-js-sdk` product**, not Rust SDK. Automated timeline
  row-mapping multi-iteration p50/p95 recorded on macOS arm64; live UX
  latencies / Linux live / memory-CPU-disk are **methodology-only residuals**
  (no fabricated live p50/p95). Verdict: **`pass-with-residuals`**. No production
  Matrix code changes.
- **P0.7:** **MERGED** into integration (docs only). Artifacts:
  `migration-ux-decision.md` + `.json`; optional ADR pointer
  `docs/adr/0003-matrix-rust-sdk-migration-ux.md`. Verdict:
  **`migration_ux_decided`**. Required decision IDs
  `D-LEGACY-DETECT`…`D-NO-DUAL-BACKEND` recorded; no dual-backend; no unsafe
  token/device reuse into fresh crypto store; FR-7.9-011 remains sequential
  single-active only. **No production session/migration code.**
- **P1.1:** **MERGED** into integration (PR #48). Artifacts:
  repo-root `rust-toolchain.toml` (`channel = "1.93"`, rustfmt+clippy);
  `src-tauri` `rust-version = "1.93"` (edition remains 2021); all desktop
  workflows pin `dtolnay/rust-toolchain` `toolchain: 1.93`; build-and-release
  prerequisite note. Local independent review: `cargo check/test --locked`
  PASS on rustc 1.93.1 (101 tests). Clippy `-D warnings` residual is
  pre-existing product lints (out of P1.1 scope).
- **P1.2:** **MERGED** into integration (PR #49). Exact `matrix-sdk` /
  `matrix-sdk-ui` `=0.18.0` with `default-features = false` in production
  `src-tauri`; compile-only `matrix_sdk_link_smoke` (type-path only, no Client
  session); rationale + license/security review:
  `p1.2-sdk-dependency-rationale.md` + `.json`. Independent review:
  `cargo check/test --locked` PASS (102 tests) on rustc 1.93.1; transitive
  `matrix-sdk/e2e-encryption` via `matrix-sdk-ui` documented; frontend still
  `matrix-js-sdk@42.0.0`. **No** production login/sync; **no** dual-backend.
- **P1.3:** **MERGED** into integration (PR #50). Versioned Matrix IPC
  foundation: envelope (`protocolVersion`/`sessionGeneration`/`sequence`/
  kinds), 13 control kinds, 21 error categories (§6.4), stream lifecycle +
  sequence helpers, policy constants, shared JSON fixtures, parallel Rust
  (`src-tauri/src/matrix/ipc/`) + TypeScript (`synara/.../matrix-ipc/`).
  Independent review: matrix IPC unit tests PASS; `cargo test --locked`
  matrix filter 21 ok; no `matrix_sdk` in IPC modules; no production Matrix
  Tauri commands; no dual-backend. Domain DTO bodies deferred to **P1.4**.
- **P1.4:** **MERGED** into integration (PR #52). Synara domain DTOs (15
  families: session, room summary, member, timeline item [9 kinds], relation,
  receipt, typing, upload, media, security, notification, search, space,
  thread, widget). Parallel Rust (`src-tauri/src/matrix/dto/`) + TypeScript
  (`synara/.../matrix-dto/`); shared fixtures `docs/matrix-rust-sdk/dto/`;
  design note `p1.4-domain-dtos.md` + `.json`. Independent review:
  `cargo test --locked matrix` 41 ok; TS matrix-dto 23 ok; no `matrix_sdk` in
  DTO modules; no tokens/media bytes on wire; no production Matrix Tauri
  commands; P1.3 IPC left independent. **No** production login/sync.
- **Optional later:** §7.1–7.7 scaffold rows may still have shallow notes if a
  full-matrix depth pass is desired beyond the handoff resume scope
- **Progress loop:** 4-minute scheduler `019f95928db7` (session-recurring);
  keep pipeline advancing one bounded task per fire
- **No production Matrix Rust SDK client/sync/login code accepted yet**
  (P1.2 crates + link smoke; P1.3 IPC + P1.4 DTOs contracts only; does not
  start a second client)

Phase 0 evidence accepted (complete):

- P0.1 SDK usage inventory — **merged**
- P0.3 exact Matrix Rust SDK 0.18.0 capability dossier — **merged**
- P0.2 feature-parity traceability (§7.8–7.11 quality audit) — **merged**
- P0.4 Swift/Rust version provenance — **merged**
- P0.5 Toolchain compatibility (Rust 1.93 / Tauri 2 / matrix-sdk 0.18) —
  **merged** (PR #45; `pass-with-residuals`)
- P0.6 Baseline reliability/performance evidence —
  **merged** (PR #46; `pass-with-residuals`; automated timeline mapping
  measured; live UX residuals documented)
- P0.7 Migration UX decision record —
  **merged** (`migration_ux_decided`; docs only; implementation ownership
  starts Phase 3 / P3.7)

Phase 1 progress:

- P1.1 permanent Rust 1.93 toolchain pin — **merged** (PR #48)
- P1.2 exact Matrix Rust SDK deps (`matrix-sdk` / `matrix-sdk-ui` `=0.18.0`)
  + feature rationale / license review — **merged** (PR #49); no production
  login/sync
- P1.3 versioned Matrix IPC schemas — **merged** (PR #50); contracts only;
  no production login/sync; no SDK wire types
- P1.4 Synara domain DTOs (15 families) — **merged** (PR #52); contracts
  only; no SDK object graph; no production login/sync

**Next program work:**

1. **P1.5** IPC protocol contract tests expansion (round trips, unknown
   variants, invalid payloads, bounds, sequence gaps, stale generations,
   schema compatibility)
2. **P1.6** Architectural CI guardrails
3. Phase 2 (after Phase 1 complete) — Rust client lifecycle / secure storage
   harnesses (still no dual production backend)

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
2. Complete P0.4 — **done** (merged into integration, PR #44).
3. Complete P0.5 — **done** (merged into integration @ `a2d288b`, PR #45;
   `pass-with-residuals`).
4. Complete remaining Phase 0 gate: **P0.7** migration UX (accepted; merge PR
   into integration closes Phase 0 docs). **P0.6 MERGED** @ `9e0cfca` (PR #46;
   `pass-with-residuals`).
5. Build the Phase 1 foundation: Rust 1.93, exact SDK pins, versioned
   Synara-owned IPC schemas, lifecycle/store security, and test infrastructure.
6. Implement Phases 2–11 by bounded capability task: authentication/sync,
   rooms/timelines, messaging/media, E2EE/verification/recovery, account data,
   notifications, search, spaces/threads, and calls/widgets.
7. Complete Phase 12 cutover and deletion: Rust is sole desktop Matrix owner;
   no `matrix-js-sdk`, JS sync/crypto/store, or product raw Matrix HTTP remains.
8. Complete Phases 13–14: reliability/performance/security/release validation,
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
