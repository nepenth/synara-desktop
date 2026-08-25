# iOS Validation Status

Reviewed: 2026-08-25

## 2026-08-25 Device Verification Validation

The current-tree signed iOS bundle completed the production shared-core SAS
route on two clean simulator device sessions authenticated as the same Matrix
account. Both roles passed without retry: the initiator sent the request, the
responder accepted it, both crossed `KeysExchanged`, the seven user-visible
emoji values matched byte-for-byte, both confirmed, both reached `Done`, and
both read the exact coordinated peer device back as `Verified` after process
termination and relaunch. No store edit, fallback trust flag, reused session,
or second verification request was used.

Three clean paired reruns reproduced the same functional result. The final run
also backgrounded and foregrounded each app before capture, proving that the
presented SAS survives the lifecycle transition. One responder PNG still
captured blank emoji glyphs even though XCTest had already read and persisted
all seven exact accessibility values, both clients completed the protocol, and
the durable SDK-backed trust readback passed. This is tracked as a concurrent
Simulator compositor capture limitation, not as missing verification state;
the earlier same-day clean proof contains painted SAS values on both roles.

## 2026-08-17 Shared-Core Validation

The release candidate was validated from branch
`release/runtime-assets-2.0.7`, based on `origin/main` at
`e3d8a45414d0a6438c216ffe4c638d4c108df928`.

- The exact unsigned CI path completed `build`, `build-for-testing`, and
  `test-without-building` against an iPhone 17 simulator on iOS 26.5.
- `SynaraTests` executed 437 tests with 435 passes, 2 intentional gated skips,
  and 0 failures.
- `SynaraUITests` executed 51 tests with 40 passes, 11 intentional live/visual
  gated skips, and 0 failures. Automatic test retries were disabled.
- A separate signed live/visual suite passed 11 of 11 scenarios without retry:
  login/send, encrypted send and relaunch restore, rich formatted send plus
  server HTML verification, room create/details/invite/leave, agent approval,
  stale-cache external-event convergence, live room/timeline/composer/
  attachment checks, live Settings traversal, and mock room/thread/agent
  visual checks.
- The signed live room-list check verified that an externally sent event
  updates the visible last-message preview through the SharedCore room
  subscription stream in approximately one second.
- Room-list consumers now share one ordered snapshot publisher instead of
  independently racing full snapshot reads after the same SDK signal. The
  default service stream also emits its initial snapshot; both behaviors have
  regression coverage.
- Settings is split into focused Account, Notifications, Appearance, Security
  & Recovery, and About screens. Signed light- and dark-mode traversals passed,
  including an explicit check that Logout is visible and tappable above the
  floating tab bar.
- After final review removed a global composer-text lookup, focused UI tests for
  plain typing/send and formatted typing/render/send both passed without retry.
- Result bundles are retained locally under `/private/tmp/synara-ios-results/`;
  final live screenshots are under
  `/private/tmp/synara-ios-final-live-proof-2.0.7/` and final dark Settings
  screenshots are under `/private/tmp/synara-ios-live-settings-dark-2.0.7/`.

The Xcode `DebuggerVersionStore.StoreError` / `no debugger version` diagnostic
still appears while launching UI tests, but the suite no longer hangs and the
diagnostic is non-fatal on this workstation. Signed simulator execution remains
required for Keychain-backed live session proof.

Status: Phase 1 shell/foundation, Phase 2 auth/session/sync/room-list/logout,
Phase 3 core messaging, Phase 5 agent workflows, Phase 6 internal
settings/hardening, Phase 6.5 iOS UI modernization, and Phase 6.6 mockup
fidelity are complete for the current native iOS MVP. Phase 6.7 is now
simulator-complete for local device-readiness: it separates actual
functionality, live/manual evidence, and visual-fidelity gaps; its matrices are
[`ios-functionality-matrix.md`](ios-functionality-matrix.md) and
[`ios-visual-fidelity-matrix.md`](ios-visual-fidelity-matrix.md). Phase 6.9
performance review is locally remediated and simulator-verified with signpost
instrumentation and fixture performance tests; physical-device traces and memory
graphs remain external release gates. Its plan is
[`phase-6-9-performance-plan.md`](phase-6-9-performance-plan.md).

Phase 7 production E2EE first slice is complete for app-level encrypted room
open/send/relaunch restore, crypto status UI, and conservative recovery
controls. Phase 8 room-management first slice is complete for service
contracts and native create/join/DM, room-details, invite, leave,
notification-mode UI surfaces, and gated live room-management smoke coverage.
Room profile editing now supports permission-aware name/topic updates through
mock and SDK-backed services, and room details expose read-only
power-level/permission context.
Deterministic simulator tests validate the mock path, and a gated live simulator
smoke validates signed session restore, live room opening, composer send, and
timeline update against disposable encrypted and unencrypted test rooms.

Phase 4 push completion is partly complete: notification permission, APNs token
capture, pusher registration flow, deep-link routing, and badge parsing are
implemented in app code and tests. Push gateway staging (`IOS-0404`) is still
blocked pending staging gateway infrastructure; app-side payload and endpoint
validation are covered by `PushServiceTests`, and configuration and smoke guidance
are documented in
[`push-gateway-staging.md`](push-gateway-staging.md).

## Project Shape

- `Synara.xcodeproj` exists under `synara-ios/`.
- `xcodegen generate` succeeds from `synara-ios/`.
- `xcodebuild -list -project Synara.xcodeproj` lists:
  - `Synara`
  - `SynaraTests`
  - `SynaraUITests`
- Shared scheme:
  - `Synara`

## Build Validation

The app target builds for a generic iOS simulator destination, and the app,
unit test, and UI test targets compile with:

```sh
scripts/ci-build.sh
```

Result: `BUILD SUCCEEDED`.

Result: `TEST BUILD SUCCEEDED`.

## Local Simulator Validation

Local simulator execution is unblocked as of 2026-05-27. Validation ran on an
iPhone 17 Pro simulator using iOS 26.5.

```sh
xcodebuild -project Synara.xcodeproj -scheme Synara -configuration Debug \
  -destination 'platform=iOS Simulator,id=<simulator-id>' \
  -derivedDataPath /private/tmp/synara-ios-mcp-derived \
  -only-testing:SynaraTests test

xcodebuild -project Synara.xcodeproj -scheme Synara -configuration Debug \
  -destination 'platform=iOS Simulator,id=<simulator-id>' \
  -derivedDataPath /private/tmp/synara-ios-mcp-derived \
  -only-testing:SynaraUITests test
```

The historical May result below is superseded by the 2026-08-17 result above.

Deterministic UI tests launch the app with `SYNARA_UI_TESTS=1`, which forces
mock services instead of live Keychain, auth, and Matrix dependencies. The
gated live-smoke UI test runs only when `SYNARA_LIVE_SMOKE=1` is supplied and
does not store homeserver credentials in the repository.

## Live Matrix Simulator Findings

Live validation on May 27, 2026 used a dedicated test account on a private test
homeserver. Credentials, homeserver details, and tokens were not written to
source files, tests, or git.

Findings are tracked in ordered implementation items in
[`synara/docs/synara-ios-project-spec.md`](../../synara/docs/synara-ios-project-spec.md):

- Unsigned simulator builds can compile the app but are not used as proof of
  Keychain-backed session behavior.
- Signed simulator login and restore are validated; the app logs only
  non-sensitive session restore status.
- Live room list loading, room title routing, timeline opening, routine state
  event filtering, and disposable-room message send are validated.
- Invite accept/reject transition behavior is covered by deterministic UI tests
  and live membership endpoints are covered by unit tests.
- XcodeBuildMCP screenshot capture works, but its live accessibility hierarchy
  snapshot can still return an empty app tree. The canonical automation path is
  XCTest accessibility, which is passing.
- Matrix Rust SDK app services now render decrypted encrypted-room messages when
  keys are available and preserve safe unavailable placeholders for UTD states.
  The app surfaces encrypted-room/session crypto status in room headers and
  Settings, including unverified device, key backup, recovery, and decryption
  issue states.
- A Matrix Rust SDK live probe validates encrypted-room login, crypto
  initialization, encrypted timeline pagination, encrypted send acceptance, and
  zero UTD callbacks in the observed window.
- The gated `testLiveEncryptedRoomSmokeWhenConfigured` simulator test validates
  app-level encrypted room open, composer send, relaunch restore, and no visible
  undecrypted placeholder on the smoke path against the disposable encrypted
  test room.
- Encrypted media is now a first-class safe-blocked state: Matrix `content.file`
  media events map to encrypted media placeholders, media loading refuses to
  fetch them until decryption support exists, and event actions are disabled.
- The same-account SAS verification and durable exact-peer trust route is now
  locally proven. Production E2EE remains blocked on full recovery/bootstrap,
  key backup restore, encrypted media decryption, broader encrypted-room
  regression coverage, and the remaining physical-device release evidence
  before an external TestFlight/App Store release can be described as complete.

The repeatable live-smoke checklist is
[`synara-ios/docs/live-simulator-smoke.md`](live-simulator-smoke.md).

- Agent card action handling is now covered in unit tests for safe URL, copy
  affordances, malformed payload rejection, and unknown-kind handling.
- Agent approve/reject actions now submit authenticated Matrix events carrying
  `in.synara.agent.action` payloads; unit tests validate request shape and
  signed-out errors.
- The gated `testLiveAgentApprovalSmokeWhenConfigured` simulator test seeds a
  real Matrix agent card, approves it through the app UI, and verifies the
  resulting `in.synara.agent.action` event in the room. This passed locally
  against the disposable `test-e2e-room` on `matrix.example.com`.
- The gated `testLiveRoomManagementSmokeWhenConfigured` simulator test covers
  live private encrypted room creation, room-details read, optional invite, and
  leave-room recovery. The create/details/invite/leave leg passed locally
  against disposable test accounts on May 28, 2026.
- Phase 8 public-room discovery is implemented in the join sheet through the
  Matrix Rust SDK room-directory search API, with deterministic mock UI coverage.
- Room-list space filtering is implemented with parent-space metadata from sync
  and SDK `SpaceService.joinedParentsOfChild`, with deterministic mock UI and
  unit coverage.
- Room details now supports permission-aware canonical/alternative alias edits
  and avatar upload/remove through Matrix Rust SDK room state APIs.

## Current App Surface

- Native SwiftUI app entry point.
- `TabView` root shell.
- Independent `NavigationStack` path per primary tab.
- Primary tabs:
  - Rooms
  - Notifications
  - Later
- Settings
- Enum-backed routes and sheet destinations.
- Deep-link routing for room, settings, notifications tab, and later tab destinations.
- App-level dependency registry installed through SwiftUI environment.
- Mock session, Matrix, push, logging, settings, and router services for tests
  and previews.
- Structured logging wrapper with redaction for tokens, APNs tokens, Matrix
  identifiers, event IDs, and URLs.
- iOS design-token baseline with shared empty, loading, error, and toolbar
  controls.
- Placeholder screens with iOS 16-compatible SwiftUI.
- Later tab now renders `LaterListView` wired to `in.synara.later` account-data
  loading and room/event navigation where anchors exist.
- Unit smoke tests for routing, dependency wiring, settings storage, and
  redaction.
- UI smoke tests assert primary tabs exist and Settings can be selected.
- Gated live UI smoke can be run locally with environment variables and is
  skipped in normal CI/local deterministic runs.

## Current Auth Surface

- Signed-out users land in a native homeserver selection flow.
- Homeserver addresses are normalized before discovery.
- Insecure `http://` homeserver input is rejected before discovery.
- Suggested homeservers are provided through the discovery service contract.
- Successful discovery routes to a login placeholder with the normalized
  homeserver base URL.
- Unit tests cover URL normalization, invalid input, mock discovery requests,
  and login routing.
- UI tests cover signed-out homeserver selection, invalid input, and successful
  navigation to the login form.
- Login screen accepts username and password input.
- Live auth uses `MatrixRustSDKAuthService`, which performs password login
  through the Matrix Rust SDK and maps successful SDK sessions into secure app
  sessions.
- Mock auth remains forced for UI tests through `SYNARA_UI_TESTS=1`.
- Successful mock login updates the observable session store and transitions to
  the signed-in tab shell.
- Failed login shows non-sensitive errors and does not persist credentials.
- Unit tests cover auth request validation, invalid credentials, mock auth
  fixtures, and session state transitions.
- A live homeserver flow check can be run manually, but no live test
  homeserver, username, or password is stored in the repository.
- UI tests cover missing-credential errors and successful mock login.
- Secure session storage supports save, load, delete, corrupt-entry handling,
  and legacy envelope migration through the app session store contract.
- Login saves sessions through secure storage before transitioning to the
  signed-in shell.
- Matrix lifecycle service exposes stopped, starting, syncing, and failed sync
  states, with explicit start, stop, and local-reset hooks.
- Live room list loading uses `MatrixRustSDKRoomListService` and Matrix Rust SDK
  room-list streaming with secure session restore.
- Room list service renders loading, empty, failed, and loaded states with
  stable room IDs, unread counts, highlight state, invite previews, and
  1,000-room fixtures.
- Invited rooms expose native accept and decline actions backed by Matrix Rust
  SDK room membership operations.
- Live timeline loading uses Matrix Rust SDK timelines and maps text, reply,
  media, redacted, encrypted, agent-card, and unknown events into the native
  timeline model.
- Live message sending uses Matrix Rust SDK timeline sends
  and supports reply metadata for text messages.
- Timeline pagination uses Matrix pagination tokens and exposes a native
  `Load Older` control.
- Live edit sending uses Matrix Rust SDK `Timeline.edit`.
- Live redaction and reaction actions use Matrix Rust SDK timeline APIs and
  update through SDK timeline state.
- Settings exposes logout through a local wipe service that stops sync, clears
  cached rooms, clears push registration state, deletes the secure session, and
  returns to the signed-out shell.
- Settings exposes account, notifications, appearance, security, About,
  Licenses, Privacy Policy, Support, and confirmed destructive logout flows.
- Settings exposes crypto verification, recovery, key backup, decryption issue
  status, SDK-backed device verification request, and recovery-key submission
  controls.
- Room management exposes native create-room, create-DM, and join-room sheet
  flows. Private rooms and DMs default to encryption.
- Room details exposes live room metadata, encryption/member context, invite
  entry, leave confirmation, and per-room notification mode controls.
- Room details exposes permission-aware room name/topic editing and saves
  changes through Matrix Rust SDK room state APIs where allowed.
- Room details exposes read-only power-level thresholds and allowed/restricted
  status for message, invite, name/topic/avatar, and moderation permissions.
- Daily messaging parity has started with safe Matrix HTML rendering for the
  shared-core timeline mapper. Supported formatting includes emphasis, strong
  text, heading hierarchy, superscript/subscript, strict Matrix colors, inline
  and fenced code with exact whitespace and language metadata, quoted or
  unquoted allowlisted absolute links, quotes, nested lists with ordered starts,
  tables, recursively structured collapsed details, concealed/revealable
  inline and block spoilers, math/image textual fallbacks, and stripped
  executable content with bounded body fallback. The native sanitizer also
  strips legacy `mx-reply` content, caps source size at 256 KiB, and caps
  emitted HTML nesting at 100 levels.
- Phase 6.5 UI modernization adds shared native design primitives, product
  auth headers, modern room avatars/badges/search, grouped timeline message
  bubbles, a stronger composer, and first-class agent action cards.
- Phase 6.6 mockup fidelity adds room filter chips, channel/direct-message
  grouping, stronger room iconography, a custom timeline header, file-style
  media cards, reaction affordances, composer tool controls, detailed agent
  approval review rows, preview links, and prominent approve/reject actions.
- Phase 6.7 adds functionality and visual-fidelity matrices plus deterministic
  and live screenshot capture paths for rooms, timeline, composer, attachment
  sheet, thread, and agent approval screens. Core mockup screens are
  release-prep accepted for local device testing; exact final polish waits for
  physical-device product review.
- Phase 6.9 adds stable performance signposts for app/session lifecycle,
  room-list loading, timeline loading/pagination, thread loading, message send,
  media upload, room open, and encrypted room open. Simulator fixture tests and
  signpost log validation are passing; exportable Time Profiler summaries,
  memory graphs, and physical-device profiling are still required before
  external TestFlight.
- Phase 6 hardening artifacts now include accessibility, performance, privacy,
  security, and TestFlight readiness documents under `synara-ios/docs/`.
- The app target includes a privacy manifest at
  `Synara/Resources/PrivacyInfo.xcprivacy`.
- Deep-link routing accepts only `synara://` routes and
  `https://synara.app/r/...` universal links; unsafe schemes and hosts are
  rejected by unit tests.
- Timeline service scaffolding normalizes raw events into stable timeline item
  models for text, media placeholders, redactions, unknown events, replies, and
  edits.
- Room timeline screen renders lazy rows with sender labels, reply/edit states,
  redactions, unknown event placeholders, media placeholders, and reaction
  summaries.
- Room timeline headers render encrypted-room crypto status, and encrypted
  timelines show conservative recovery UI with retry and Settings navigation
  when keys or recovery need attention.
- Composer MVP supports multiline text input, empty-message guarding, local
  echo, send failure messaging, and per-room draft preservation.
- Event action service and context menu support reply, edit, redact, and react
  availability against the mock service layer.
- Media service supports authenticated media resources, safe media descriptions,
  viewer presentation, upload progress state, sanitized upload display names,
  authenticated thumbnail requests, Matrix media uploads, and media-message
  sends after upload.
- Unit tests cover secure session storage, Matrix lifecycle, room sorting and
  unread mapping, local wipe behavior, timeline mapping, composer/draft behavior,
  event action behavior, and media URL/path safety.
- UI tests cover opening a room from the room list, sending a mock message,
  loading older messages, accepting/rejecting invites, adding a mock media
  attachment, and logout return to the homeserver selection shell.
