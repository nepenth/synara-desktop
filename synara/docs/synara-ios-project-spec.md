# Synara iOS Project Spec

Reviewed: 2026-08-18

Status: historical cumulative project specification. The implemented app has
advanced beyond the early phase labels retained below. Current release and
validation state is maintained in
[`synara-ios/docs/ios-validation-status.md`](../../synara-ios/docs/ios-validation-status.md)
and [`synara-ios/docs/release-checklist.md`](../../synara-ios/docs/release-checklist.md).

Related plan: [Synara iOS App Store Plan](./synara-ios-app-store-plan.md)

Pre-iOS consolidation: [Native-First Consolidation Plan](./native-first-consolidation-plan.md)

## Purpose

This document expands the App Store plan into phases, tasks, task requirements, and acceptance criteria. It is written so future autonomous work can pick up bounded tasks without re-litigating the whole architecture each time.

The target is an App Store-grade native iOS app that preserves Synara behavior across iOS, macOS, and Linux. The default implementation path is SwiftUI plus Matrix Rust SDK Swift bindings. A Tauri iOS spike is included only as a decision task.

## Working Assumptions

- The active app runtime is `synara-desktop/synara`; the standalone sibling `synara` checkout is not the target for new app work.
- The iOS app will live in `synara-ios/` inside the canonical
  `synara-desktop` monorepo per
  [ADR 0001](../../docs/adr/0001-ios-repository-layout.md).
- The iOS app will be native SwiftUI, not a shipping WebView wrapper.
- Shared compatibility will come from documented contracts and generated schemas/types, not from attempting to reuse React UI code.
- Matrix account data and Matrix events remain the source of truth for cross-device Synara state.
- Push notifications require APNs, a Matrix pusher, and a push gateway such as Sygnal or a Synara-operated equivalent.
- App Store publication under the LLC requires Apple organization enrollment, signing setup, privacy policy, and licensing review.
- The current AGPL-3.0-only licensing posture is a release gate, not a small implementation detail.
- Any production credential, APNs key, Apple signing secret, or homeserver admin token must stay out of the repository.

## Non-Goals For MVP

- Full feature parity with the desktop clients.
- Reimplementation of the desktop tray/global-shortcut model on iOS.
- Rich decrypted push previews before a completed push privacy design.
- Share extension, widgets, Live Activities, and App Intents beyond basic app-open flows.
- Multi-account support.
- Voice/video calling.
- Custom Matrix homeserver administration.
- Offline-first message queue beyond local drafts and retryable upload/send states.

## Definition Of Done

Every task that changes code should meet these baseline requirements unless explicitly waived in the task:

- Builds cleanly for simulator.
- Builds cleanly for at least one physical-device destination when the task touches signing, push, Keychain, APNs, or device-only APIs.
- Has focused unit tests for pure logic and parsing.
- Has UI tests for user-visible flows when the task changes navigation, auth, messaging, push, or settings.
- Has previews or fixture-backed sample views for new SwiftUI screens.
- Does not log tokens, secrets, device tokens, recovery keys, decrypted message bodies, or private room names.
- Uses test accounts and test rooms for screenshots, recordings, and smoke validation.
- Updates the relevant contract, spec, or release checklist when behavior changes.

## Autonomous Work Rules

Autonomous agents can safely work on:

- Documentation, schemas, mock services, previews, fixtures, unit tests, and local-only prototype code.
- Xcode project structure and Swift package scaffolding when no Apple credentials are needed.
- Simulator builds and tests.
- Tauri iOS feasibility checks that do not require production credentials.
- Test homeserver integration using non-production accounts.

Autonomous agents must stop for human approval before:

- Enrolling or changing Apple Developer Program account details.
- Creating or rotating production APNs keys.
- Uploading builds to App Store Connect.
- Changing license headers, contributor license terms, or legal notices beyond draft documents.
- Adding telemetry, crash reporting, analytics, or any third-party SDK that changes privacy disclosures.
- Connecting to production Matrix accounts, private production rooms, or real user data.
- Storing secrets in CI or a secret manager.

## Product Requirements

### Core User Flows

- First launch shows a native welcome/login path, not a marketing page.
- User can enter or select a homeserver.
- User can log in and restore the session after app restart.
- User can log out and wipe local app state.
- User can see joined rooms and DMs with unread/highlight indicators.
- User can open a room, read the timeline, paginate older events, and send a message.
- User can reply, edit, react, and redact where permissions allow.
- User can view and upload media.
- User can receive an APNs push, tap it, and land in a safe destination.
- User can see Synara agent cards and perform allowed actions.
- User can inspect app version, licenses, privacy policy, and support links.

### Cross-Platform Requirements

- iOS must read and write Synara account data according to `docs/synara-namespaces.md`.
- iOS must not create iOS-only Matrix account data for features intended to sync across devices without documenting the contract first.
- Deep links must map to the same conceptual destinations used by the desktop runtime: room, event, thread/root event, Later, notifications, settings, and agent approvals. The initial shared contract is [Synara Route Contract](./synara-route-contract.md).
- Later must use the anchor-only account-data schema in [Synara Later Account Data Contract](./synara-later-contract.md), resolving message previews locally instead of storing plaintext in account data.
- Agent actions must use typed payloads with allow-listed kinds and bounded fields.
- Notification and badge counts must be explainable from shared notification semantics.
- Media and external URLs must follow [Synara Media And External URL Policy](./synara-media-policy.md).
- Shared settings must follow [Synara Settings Compatibility Contract](./synara-settings-compatibility.md).
- Desktop and Linux must remain first-class; iOS-specific changes to shared contracts must not break existing Tauri integrations.

### iOS UX Requirements

- SwiftUI screens use native navigation and system controls where practical.
- Each tab has an independent `NavigationStack` history.
- App-wide modal presentation uses enum-driven sheets or full-screen covers.
- Async state is explicit: idle, loading, loaded, empty, failed, retrying where relevant.
- Timeline and room list rows use stable identifiers.
- UI supports light mode, dark mode, Dynamic Type, VoiceOver, Reduce Motion, and iPad split view.
- Keyboard and composer behavior must feel native on iPhone and iPad.

### Security And Privacy Requirements

- Tokens, session data, and crypto-sensitive bootstrap data use Keychain or SDK-approved secure storage.
- Local caches do not persist decrypted bodies beyond SDK-required stores without an explicit design.
- Logs must redact user IDs, room IDs, event IDs, device tokens, URLs with credentials, and access tokens when practical.
- Push payloads default to generic content.
- Safe external URL handling must reject non-HTTPS remote action URLs unless there is a specific local-system reason.
- Logout wipes Keychain entries, SDK stores, local caches, draft stores, and extension-shared data.

## Proposed iOS Project Structure

This structure is a target, not a Phase 0 prerequisite.

```text
synara-ios/
  Synara.xcodeproj
  Synara/
    App/
    Routing/
    Features/
      Auth/
      Rooms/
      Timeline/
      Composer/
      Media/
      Notifications/
      Later/
      Agents/
      Settings/
    SharedUI/
    Resources/
  SynaraCore/
    Contracts/
    Logging/
    Persistence/
    Security/
  SynaraMatrix/
    MatrixClientService.swift
    SessionStore.swift
    RoomListService.swift
    TimelineService.swift
  SynaraPush/
    APNsRegistrationService.swift
    MatrixPusherService.swift
    NotificationRouter.swift
  SynaraAgent/
    AgentAction.swift
    AgentActionValidator.swift
  SynaraTests/
  SynaraUITests/
  docs/
```

Package boundaries:

- `Synara`: app target and feature UI.
- `SynaraCore`: shared value types, schemas, logging, security helpers.
- `SynaraMatrix`: Matrix SDK wrapper and app-facing Matrix services.
- `SynaraPush`: APNs, pusher registration, notification routing.
- `SynaraAgent`: agent card/action parsing and validation.

## Phase -1: Native-First Runtime Consolidation

Status: complete for local pre-iOS prep. Packaged macOS and Linux validation is
the next gate before iOS Phase 0 starts.

Goal: make the active desktop runtime coherent before the iOS buildout starts.

Primary plan: [Native-First Consolidation Plan](./native-first-consolidation-plan.md)

Required outcomes:

- `synara-desktop/synara` is confirmed as the canonical app runtime.
- Public self-hosted web positioning is removed from active runtime docs.
- Desktop-named cross-platform APIs are routed through platform APIs.
- Notification, badge, agent-action, settings, and storage contracts are identified before iOS starts consuming them.
- The sibling `synara` checkout is archived or deleted only after unique wanted work is migrated and explicit approval is given.

Acceptance criteria:

- New task instructions point to `synara-desktop/synara`.
- The active runtime README is native-app-first.
- A final diff review exists before deleting any checkout.
- iOS Phase 0 tasks reference platform contracts rather than browser-runtime assumptions.

## Phase 0: Program, Architecture, And Contracts

Goal: make the project startable without hidden legal, account, or architecture blockers.

### IOS-0001: Choose Repository Layout

Dependencies: none.

Status: complete in [ADR 0001](../../docs/adr/0001-ios-repository-layout.md).

Requirements:

- Decide whether `synara-ios/` is a sibling folder, separate repository, or nested workspace.
- Document how `synara`, `synara-desktop`, and `synara-ios` will reference shared contract files.
- Avoid moving existing app-runtime or desktop-shell code during this task.

Deliverables:

- Architecture Decision Record for repo layout.
- Initial empty folder or placeholder README only if the chosen layout is within this workspace.

Acceptance criteria:

- The ADR names the chosen layout and two rejected alternatives.
- The ADR includes expected build commands for the app runtime, desktop shell, and iOS.
- No existing files are renamed or moved without explicit follow-up approval.

### IOS-0002: Apple Organization Enrollment Checklist

Dependencies: none.

Status: complete as an owner-action checklist in
[`synara-ios/docs/apple-developer-enrollment-checklist.md`](../../synara-ios/docs/apple-developer-enrollment-checklist.md).

Requirements:

- Create a checklist for LLC-based Apple Developer Program enrollment.
- Include D-U-N-S, authority to bind the LLC, App Store Connect access, bundle IDs, and CI signing.
- Do not submit enrollment or enter credentials.

Deliverables:

- `docs/apple-developer-enrollment-checklist.md` or equivalent under the iOS project docs.

Acceptance criteria:

- Checklist separates owner-only tasks from engineering tasks.
- Checklist identifies all planned bundle IDs and capabilities.
- Checklist references where secrets will be stored once approved.

### IOS-0003: Licensing And App Store Distribution Review

Dependencies: none.

Status: draft inventory complete in
[`synara-ios/docs/license-inventory.md`](../../synara-ios/docs/license-inventory.md);
legal review remains a release blocker.

Requirements:

- Inventory current repo licenses and likely iOS dependency licenses.
- Flag AGPL/App Store compatibility as a blocking legal review item.
- Identify whether a dual-license, exception, clean-room approach, or source distribution plan may be needed.

Deliverables:

- Draft license inventory.
- Release gate entry in the iOS release checklist.

Acceptance criteria:

- Inventory includes `synara`, `synara-desktop`, Matrix SDKs, Tauri dependencies used in the spike, and planned iOS packages.
- The spec states that external TestFlight and App Store submission cannot proceed until review is closed.
- No license text is changed by this task.

### IOS-0004: Shared Contract Inventory

Dependencies: none.

Status: locally complete in [Synara Shared Contract Inventory](./synara-contracts.md).

Requirements:

- Enumerate cross-platform contracts needed by iOS.
- Compare against `docs/synara-namespaces.md`.
- Identify schema files to create.

Deliverables:

- [Synara Shared Contract Inventory](./synara-contracts.md).
- Schema backlog in `docs/contracts/README.md` and this project spec.

Acceptance criteria:

- Inventory covers account data, agent actions, deep links, notification summaries, Later items, room/event anchors, media URL policy, and settings compatibility.
- Each contract has an owner platform and a compatibility rule.
- Unknown fields are explicitly assigned a forward-compatibility policy.

### IOS-0005: Tauri iOS Feasibility Spike

Dependencies: IOS-0001.

Status: preflight complete in
[`synara-ios/docs/tauri-ios-feasibility-spike.md`](../../synara-ios/docs/tauri-ios-feasibility-spike.md).
The generated Tauri iOS project initialized successfully after Apple helper
dependencies were installed, but simulator runtime validation was blocked by
local Xcode first-launch/simulator setup. The recommendation remains native
SwiftUI as the shipping path.

Requirements:

- Fix local Tauri CLI install if needed using a normal dependency install.
- Try a Tauri iOS initialization from `synara-desktop` or a scratch branch.
- Keep all changes isolated and reversible.
- Test the current Synara app runtime in iOS simulator if the shell launches.

Deliverables:

- Feasibility report.
- Screenshots or short recordings if the app launches.
- List of blockers, warnings, and unexpected successes.

Acceptance criteria:

- Report covers IndexedDB, WebCrypto/WASM, service worker assumptions, media auth, keyboard/composer behavior, routing, push limitations, performance feel, and App Review risk.
- Report makes a recommendation: discard, keep as internal mobile WebView shell, or revisit.
- No production app identifier or signing credential is required.

### IOS-0006: Native Matrix SDK Feasibility Spike

Dependencies: IOS-0001.

Status: package/import probe complete in
[`synara-ios/docs/matrix-sdk-feasibility-spike.md`](../../synara-ios/docs/matrix-sdk-feasibility-spike.md).
The official Swift package resolves and imports locally. Real login was
intentionally deferred until the native app shell has Keychain storage and
redacted logging.

Requirements:

- Create a small SwiftUI proof-of-concept or package spike.
- Integrate Matrix Rust SDK Swift components.
- Log in against a test homeserver and fetch a room list if SDK coverage allows.
- Do not store real user credentials.

Deliverables:

- Spike branch or scratch project.
- SDK coverage report.
- Minimal notes on build setup and SDK initialization.

Acceptance criteria:

- Report states whether login, session restore, room list, timeline, E2EE, media, and pusher APIs are available or require wrappers.
- Report identifies minimum supported iOS version implications.
- Report identifies any blocking build, binary size, or dependency issues.

### IOS-0007: Architecture Decision Record

Dependencies: IOS-0005, IOS-0006.

Status: complete in
[ADR 0002](../../docs/adr/0002-ios-architecture.md).

Requirements:

- Decide the primary implementation path.
- Record the role, if any, of Tauri iOS.
- Record the native SDK strategy and expected module boundaries.

Deliverables:

- ADR: "Synara iOS Architecture".

Acceptance criteria:

- ADR selects one primary path.
- ADR includes consequences for App Store review, push, crypto, shared contracts, CI, and maintenance.
- ADR is linked from the App Store plan and this spec.

## Phase 1: Native iOS Skeleton

Goal: create a maintainable SwiftUI app shell with routing, dependency wiring, tests, and CI-ready build commands.

### Ordered Live-Simulator Hardening Queue

This queue was created from the May 27, 2026 live iOS simulator smoke against a
dedicated private test homeserver account. These items must remain ordered
because each item increases the trustworthiness of the next validation layer.

1. Local signed simulator runtime: live auth cannot be considered validated
   until the app is launched in a signed simulator build. The unsigned CI build
   path successfully compiles the app, but it cannot validate Keychain-backed
   session persistence.
2. Keychain persistence diagnostics and restore validation: login must surface
   non-sensitive user errors and log non-sensitive Keychain status details so
   signing, entitlement, and simulator problems can be diagnosed without
   exposing tokens.
3. Live simulator automation and accessibility reliability: text fields,
   buttons, tabs, alerts, and timeline controls must be discoverable and
   operable through accessibility-driven automation, not only visual coordinate
   taps.
4. Live invite and room-list transition validation: accepting or rejecting an
   invite must refresh the room list, remove invite controls, preserve unread
   state, and allow the joined room to open without relaunching.
5. Room navigation context: opening a room from the list must carry the room
   display name into the timeline title, while deep links without cached room
   metadata may fall back to a neutral title until hydrated.
6. Timeline event visibility: room state events such as create, member,
   power-level, history-visibility, guest-access, and join-rules events must not
   pollute the chat timeline. They should either be hidden, grouped into a
   room-activity/system surface, or mapped to intentional system rows.
7. Composer live-send usability: the send control must have a reliable
   44-point hit target, work through simulator automation, clear the draft only
   after successful send, and show a non-sensitive failure state otherwise.
8. Live validation runbook: each live smoke should capture build mode, simulator
   OS, app version, homeserver, redacted logs, screenshots, mutations performed
   against the test room, and any known state changes such as invite acceptance.

### IOS-0101: Create Xcode Project And Targets

Dependencies: IOS-0007.

Status: skeleton complete. `Synara.xcodeproj`, app target, unit test target,
and UI test target exist under `synara-ios/`. Generic simulator build,
`build-for-testing`, simulator unit tests, and simulator UI tests pass; current
validation is documented in
[`synara-ios/docs/ios-validation-status.md`](../../synara-ios/docs/ios-validation-status.md).

Requirements:

- Create the iOS app project.
- Add unit and UI test targets.
- Use explicit bundle IDs from the Phase 0 decision.
- Add basic app icon placeholders and launch screen.

Deliverables:

- `synara-ios/` project.
- Documented local build command.

Acceptance criteria:

- `xcodebuild -list` shows app, unit test, and UI test schemes.
- Simulator debug build succeeds.
- Unit test target runs with at least one passing smoke test.
- App launches to an empty shell in simulator.

### IOS-0102: App Routing Shell

Dependencies: IOS-0101.

Requirements:

- Implement root app state.
- Use `TabView` with independent `NavigationStack` paths.
- Define `AppTab`, `AppRoute`, and `SheetDestination`.
- Reset navigation on logout/account change.

Deliverables:

- Root shell, router types, destination mapping.
- Preview fixture for logged-out and logged-in shell.

Acceptance criteria:

- Tabs include Rooms, Notifications or Inbox, Later, and Settings, even if some are placeholders.
- Deep-link router can route to settings and placeholder room destinations.
- Sheets use enum-driven `.sheet(item:)` or equivalent, not multiple unrelated booleans.
- UI test can switch tabs and navigate to Settings.

### IOS-0103: Dependency Graph And Service Registry

Dependencies: IOS-0101.

Requirements:

- Add app-level dependency injection using SwiftUI environment for true app-wide services.
- Keep feature-local dependencies passed explicitly.
- Add mock services for previews and tests.

Deliverables:

- `AppEnvironment` or equivalent dependency wiring.
- Mock service implementations.

Acceptance criteria:

- Root view installs session, Matrix, push, logging, settings, and router services.
- Feature views can render in previews with mock dependencies.
- No network work starts from SwiftUI `body`.

### IOS-0104: Logging And Redaction Policy

Dependencies: IOS-0101.

Requirements:

- Add structured logging wrappers.
- Define redaction helpers for tokens, Matrix IDs, event IDs, URLs, and APNs tokens.
- Document allowed log categories.

Deliverables:

- Logging service.
- Redaction unit tests.
- Logging policy doc.

Acceptance criteria:

- Unit tests prove secrets and token-like strings are redacted.
- Release builds do not enable verbose SDK logs by default.
- No sample logs contain private production data.

### IOS-0105: Design Token Baseline

Dependencies: IOS-0101.

Requirements:

- Define color, typography, spacing, and icon rules for iOS.
- Support light/dark mode and Dynamic Type.
- Avoid copying web CSS directly.

Deliverables:

- Shared UI primitives for rows, empty states, loading states, error states, and toolbar buttons.
- Preview gallery.

Acceptance criteria:

- Preview gallery renders in light and dark mode.
- Core controls scale under large Dynamic Type without clipped text.
- VoiceOver labels exist for icon-only controls.

### IOS-0106: CI Build Skeleton

Dependencies: IOS-0101.

Requirements:

- Add local and CI-oriented build/test commands.
- Avoid requiring signing secrets for simulator tests.
- Document future App Store Connect API key needs.

Deliverables:

- Build script or README commands.
- CI notes.

Acceptance criteria:

- Simulator build and unit tests can run without Apple credentials.
- The CI note distinguishes unsigned simulator tests from signed device/archive builds.
- Build artifacts and DerivedData are ignored.

### IOS-0107: Signed Local Simulator Runtime

Dependencies: IOS-0106.

Status: complete. Local signed simulator runtime validates Keychain-backed
session restore, while the unsigned CI build path remains credential-free for
compilation and deterministic tests.

Requirements:

- Keep CI simulator compilation free of Apple credentials and signing secrets.
- Ensure local simulator runs use normal simulator code signing unless the
  developer explicitly opts into an unsigned compile-only path.
- Document that unsigned builds are not valid for live Keychain/session smoke.
- Preserve App Store signing as a future distribution track, separate from the
  local signed simulator track.

Deliverables:

- Project signing defaults suitable for local simulator execution.
- CI script overrides that keep GitHub Actions unsigned.
- README and CI notes explaining signed local runtime versus unsigned CI.

Acceptance criteria:

- Local signed simulator launch can complete login and save a Keychain session.
- CI still builds and compiles test bundles without Apple Developer credentials.
- A failed unsigned live login is diagnosed as a build-mode/signing problem, not
  a Matrix authentication failure.

## Phase 2: Auth, Session, And Matrix Sync

Goal: support secure login, restore, logout, and room list sync against a test homeserver.

### IOS-0201: Homeserver Selection

Dependencies: IOS-0102, IOS-0103.

Requirements:

- Add native homeserver entry and validation.
- Support default Synara-configured homeserver list if contract exists.
- Show clear errors for invalid URL, discovery failure, and unsupported server.

Deliverables:

- Homeserver selection screen.
- Discovery service wrapper.
- Unit tests for URL normalization.

Acceptance criteria:

- User can enter a homeserver URL and proceed to login.
- Invalid URLs fail before network calls.
- Discovery failure is displayed with retry.
- UI test covers valid and invalid URL entry using mock service.

### IOS-0202: Login Flow

Dependencies: IOS-0201, IOS-0006.

Status: complete for Phase 2. The live app environment uses a
Matrix Client-Server password auth adapter that validates username/password
input, verifies `m.login.password` support through `/_matrix/client/v3/login`,
submits password login, maps invalid credentials and unsupported flows to
non-sensitive app errors, and returns secure app sessions for persistence.
UI tests still force mock auth with `SYNARA_UI_TESTS=1`.

Requirements:

- Implement password login if supported by the SDK path.
- Leave SSO as a separate task if it needs ASWebAuthenticationSession.
- Handle loading, invalid credentials, network failure, and cancellation.

Deliverables:

- Login screen.
- Auth service.
- Mock login fixtures.

Acceptance criteria:

- Test account can log in against a test homeserver.
- Failed login does not persist partial credentials.
- UI shows non-sensitive error messages.
- Unit tests cover auth state transitions.

### IOS-0203: Secure Session Store

Dependencies: IOS-0202.

Requirements:

- Store tokens/session handles in Keychain or SDK-approved secure storage.
- Store non-sensitive UI state separately.
- Add migration hooks for future schema changes.

Deliverables:

- Session store abstraction.
- Keychain wrapper.
- Tests using mock Keychain.

Acceptance criteria:

- App restores session after force quit.
- App does not restore after logout.
- Unit tests cover save, load, delete, corrupt entry, and migration cases.
- No token appears in logs.

### IOS-0204: Matrix Client Lifecycle

Dependencies: IOS-0203.

Requirements:

- Create app-facing Matrix client service.
- Start sync after login/restore.
- Stop sync on logout.
- Surface sync status to UI.

Deliverables:

- `MatrixClientService`.
- Sync status model.
- Logged-in shell transitions.

Acceptance criteria:

- Login leads to a syncing room-list shell.
- Restart restores and resumes sync.
- Logout stops sync and clears visible data.
- Sync failure can be retried without killing the app.

### IOS-0205: Room List MVP

Dependencies: IOS-0204.

Status: complete for Phase 2. The live app environment loads
room summaries from Matrix `/sync?timeout=0`, includes joined and invited rooms,
maps room names, latest message previews, unread counts, highlight counts, and
activity timestamps, supports accept and decline actions for invites, and keeps
deterministic mock fixtures for UI tests.

Requirements:

- Render joined rooms and DMs.
- Show unread and highlight state where SDK exposes it.
- Use stable room IDs.
- Support empty, loading, failed, and loaded states.

Deliverables:

- Room list service.
- Room list screen.
- Mock data fixtures for small, large, empty, and failed lists.

Acceptance criteria:

- Test account sees joined rooms.
- Room list scrolls smoothly with 1,000 mock rooms.
- UI test opens a room placeholder from the list.
- Unit tests cover room sorting and unread mapping.

### IOS-0206: Logout And Local Wipe

Dependencies: IOS-0203, IOS-0204, IOS-0205.

Requirements:

- Implement logout from settings.
- Clear Keychain, SDK stores, local caches, drafts, and pending push registration state.
- Handle homeserver logout failure by still clearing local state after confirmation.

Deliverables:

- Logout UI.
- Wipe service.
- Wipe tests.

Acceptance criteria:

- Logout returns app to logged-out shell.
- Restart after logout does not restore account.
- Unit test verifies wipe calls all registered stores.
- No leftover local room list is shown after logout.

### IOS-0207: Live Auth Persistence Diagnostics

Dependencies: IOS-0107, IOS-0202, IOS-0203.

Status: complete. The app records non-sensitive secure-session restore and
persistence diagnostics, and signed simulator restore is live-validated without
logging credentials or tokens.

Requirements:

- Log non-sensitive secure-storage status details for failed session save/load.
- Keep user-facing errors short and non-sensitive.
- Never log access tokens, passwords, homeserver credentials, or raw session
  payloads.
- Restore an existing signed-in session after app relaunch.

Deliverables:

- Secure-session log descriptions.
- Unit tests for diagnostic descriptions and redaction behavior.
- Live validation notes covering login, relaunch, restore, logout, and failed
  persistence cases.

Acceptance criteria:

- Successful live login transitions to the signed-in shell and survives relaunch.
- Failed Keychain persistence logs a non-sensitive status code or category.
- Credential and token scans remain clean after live smoke.

### IOS-0208: Live Simulator Accessibility And Automation Reliability

Dependencies: IOS-0105, IOS-0201, IOS-0202.

Status: complete for app-level automation. Deterministic XCTest coverage uses
stable accessibility identifiers for auth, room list, timeline, composer,
invites, settings, and logout. A gated live-smoke XCTest validates the signed
runtime path. XcodeBuildMCP's live `snapshot_ui` can still return an empty app
tree, which is tracked as a tooling limitation rather than an app blocker.

Requirements:

- Ensure primary screens expose stable accessibility identifiers and labels.
- Ensure homeserver, username, password, composer, invite, tab, and alert
  controls can be focused and activated reliably in simulator automation.
- Avoid relying on pixel coordinates for the canonical smoke path.
- Keep UI tests deterministic through mocks while allowing a separate live
  smoke script/runbook for test homeservers.

Deliverables:

- Accessibility audit fixes for auth, room list, timeline, and composer screens.
- A live simulator smoke checklist or script that uses accessibility targets
  where available.
- UI tests that guard important identifiers and focus order.

Acceptance criteria:

- Automation can enter homeserver, username, and password without coordinate
  fallback.
- Automation can dismiss the iOS password-save prompt when it appears.
- Automation can accept or decline an invite, open a room, and send a message
  using stable controls.

### IOS-0209: Live Invite Transition Validation

Dependencies: IOS-0205, IOS-0208.

Status: complete. Invite accept/reject endpoints, room-list refresh behavior,
and deterministic UI transitions are implemented and covered.

Requirements:

- Accept and reject invites through Matrix membership endpoints.
- Refresh the room list after membership changes.
- Remove invite controls after a successful membership change.
- Preserve unread/highlight semantics when the joined room appears.
- Show a non-sensitive retryable failure when membership changes fail.

Deliverables:

- Invite transition tests and live validation notes.
- Room-list state refresh behavior after membership updates.
- Error copy for failed accept/reject.

Acceptance criteria:

- Accepting a test invite changes the row from invited to joined without app
  relaunch.
- Rejecting a test invite removes the invite from the visible room list.
- Failed accept/reject does not leave stale loading controls behind.

## Phase 3: Core Messaging

Goal: make the app usable for daily chat in test encrypted and unencrypted rooms.

### IOS-0301: Timeline Service

Dependencies: IOS-0205.

Status: complete for Phase 3. Timeline mapping covers text, media, replies,
edits, reactions, redactions, encrypted placeholders, custom unknown events, and
Matrix pagination tokens.

Requirements:

- Provide app-facing timeline items from the Matrix SDK.
- Support pagination.
- Normalize event IDs, sender IDs, timestamps, reply metadata, redaction state, and pending local echoes.

Deliverables:

- Timeline service abstraction.
- Timeline item models.
- Fixtures for text, media, replies, edits, reactions, redactions, agent cards, and unknown events.

Acceptance criteria:

- Unit tests cover mapping for all supported event types.
- Unknown events render as safe placeholders.
- Pagination API can load older events without duplicating items.
- Timeline item identity is stable across edits and reactions.

### IOS-0302: Timeline UI

Dependencies: IOS-0301, IOS-0105.

Status: complete for Phase 3. The room timeline renders lazy
rows for mapped text, reply, edit, redaction, unknown, media placeholder, and
reaction states, exposes a native load-older control, and keeps encrypted events
as safe placeholders. Build-for-testing and simulator UI validation pass as
documented in
[`synara-ios/docs/ios-validation-status.md`](../../synara-ios/docs/ios-validation-status.md).
The live timeline service now loads Matrix room messages and maps text, reply,
media, redacted, encrypted, and unknown events into the native timeline model.

Requirements:

- Render timeline groups, sender labels, timestamps, text, replies, edits, reactions, and redactions.
- Use lazy scrolling with stable identity.
- Provide loading, empty, failed, and pagination states.

Deliverables:

- Room timeline screen.
- Timeline row components.
- Preview fixtures for common states.

Acceptance criteria:

- Opening a room shows recent events from a test homeserver.
- 10,000 synthetic events scroll without obvious hitching on simulator.
- Dynamic Type does not clip message text.
- VoiceOver can identify sender and message summary.

### IOS-0303: Composer MVP

Dependencies: IOS-0302.

Status: complete for Phase 3. Composer supports multiline
plain-text drafts, empty-message guarding, per-room draft preservation, local
echo through the message sending service, and mock UI coverage. The live
message sender now sends Matrix `m.room.message` text events with transaction
IDs and reply metadata, and a gated live-smoke UI test validates a disposable
room send.

Requirements:

- Send plain text messages.
- Support multiline input, send button state, local echo, failure retry, and draft preservation while navigating.
- Avoid sending empty or whitespace-only messages.

Deliverables:

- Composer view.
- Send service integration.
- Draft model.

Acceptance criteria:

- User can send a message in a test room.
- Failed send shows retry or failure state.
- Draft survives room navigation within the app session.
- UI test sends a message using mock service.

### IOS-0303A: Room Title Hydration

Dependencies: IOS-0205, IOS-0302.

Status: implemented. Opening a room from the list now carries the known display
name into the timeline title, and deep-link/fallback routes can still open
without cached title metadata.

Requirements:

- Carry known room display names from the room list route into the timeline.
- Fall back gracefully when a room opens from a deep link without cached
  metadata.
- Leave room ID out of the primary title unless no display name is known and a
  debug-oriented fallback is explicitly chosen.

Deliverables:

- Route payload support for optional room title.
- Timeline title rendering from route context.
- Tests for room-list route title and deep-link fallback.

Acceptance criteria:

- Tapping `Alerts` opens a timeline titled `Alerts`.
- Deep-linked rooms without cached metadata still open.
- The route remains hashable and stable for NavigationStack paths.

### IOS-0303B: Timeline Event Visibility And System Events

Dependencies: IOS-0301, IOS-0302.

Status: implemented for the primary chat timeline. Routine Matrix room state
events are filtered from visible chat rows, while custom Synara/agent unknown
events remain safe placeholders.

Requirements:

- Hide routine Matrix state events from the primary chat timeline unless mapped
  to an intentional system row.
- Keep unknown custom Synara/agent events visible as safe placeholders.
- Preserve redaction, encrypted, media, reply, edit, and plain-text handling.
- Avoid dropping actual `m.room.message` events with unsupported `msgtype`;
  these should render as safe text or media placeholders where possible.

Deliverables:

- Matrix event visibility filter.
- Unit tests proving state events are hidden and custom unknown events remain
  safe.
- Follow-up design note for a future room-activity/system-event surface.

Acceptance criteria:

- Opening a live test room shows recent chat-relevant events first, not a wall
  of room creation/member/power-level events.
- Unit tests cover hidden state events, visible text/media events, encrypted
  events, redactions, and custom unknown events.
- No visible timeline row exposes raw event IDs except where the UI explicitly
  represents reply/debug context.

### IOS-0303C: Composer Live Send Hardening

Dependencies: IOS-0303, IOS-0208.

Status: complete. Composer icon controls have larger hit targets, deterministic
UI tests cover mock sending, and the gated live-smoke UI test validates live
send through the UI in a disposable room.

Requirements:

- Give send and attachment controls at least 44x44 point hit targets.
- Ensure disabled/enabled send state tracks trimmed draft content.
- Support keyboard and accessibility activation of Send.
- Clear drafts only after Matrix send succeeds.
- Preserve failure copy and retry opportunity when send fails.

Deliverables:

- Composer control hit-target updates.
- UI tests for enabled send state and activation.
- Live smoke validation that sends a harmless test message in the test room.

Acceptance criteria:

- Live simulator can send a text message to the test room through the UI.
- Empty or whitespace-only drafts cannot be sent.
- Failed sends leave the draft available for retry.

### IOS-0304: Reply, Edit, Redact, React

Dependencies: IOS-0303.

Status: complete for Phase 3. Event action availability and
mock action application are wired through context menus and covered by unit
tests. Live Matrix adapters now send edit replacement content, redactions, and
reaction annotations, with local UI updates only after successful responses.

Requirements:

- Add event actions for reply, edit, redact, and reaction.
- Respect permission or SDK failure responses.
- Keep actions discoverable through context menus or native action sheets.

Deliverables:

- Event action UI.
- Event action service.
- Tests for action availability.

Acceptance criteria:

- Reply sends relation metadata correctly in a test room.
- Edit updates the event rendering.
- Redaction shows a redacted placeholder.
- Reactions aggregate and update without duplicate local echoes.

### IOS-0305: Media Download And Viewer

Dependencies: IOS-0302.

Status: complete for Phase 3. Authenticated media resources, safe display
descriptions, media placeholders, viewer presentation, authenticated thumbnail
requests, and media URL safety tests are in place.

Requirements:

- Display image/media placeholders and downloaded thumbnails.
- Support full-screen media viewing.
- Respect authenticated media requirements.
- Avoid leaking authenticated media URLs.

Deliverables:

- Media loader.
- Media viewer.
- Cache policy.

Acceptance criteria:

- Image messages load in a test room.
- Failed media has retry UI.
- Full-screen viewer supports dismiss and basic zoom where practical.
- Tests cover URL safety and authenticated media handling.

### IOS-0306: Media Upload

Dependencies: IOS-0303, IOS-0305.

Status: complete for Phase 3. Attachment upload presents upload progress,
sanitizes local file/path input before display, supports the native photo
library path for live builds, and live media upload sends Matrix media events
after upload.

Requirements:

- Support photo library, camera if capability is enabled, and file picker where allowed.
- Show upload progress and failure states.
- Send Matrix media events after upload.

Deliverables:

- Attachment picker.
- Upload service.
- Upload progress UI.

Acceptance criteria:

- User can send an image to a test room.
- Upload failure can be retried or removed.
- Permission denial is handled clearly.
- No local file path leaks into message bodies or logs.

### IOS-0307: Encrypted Room Validation

Dependencies: IOS-0302, IOS-0303, IOS-0305.

Status: SDK live E2EE probe and app-level live encrypted-room UI smoke complete
in
[`synara-ios/docs/e2ee-validation.md`](../../synara-ios/docs/e2ee-validation.md).
The Matrix Rust SDK probe validates disposable encrypted-room login, crypto
initialization, encrypted room state, encrypted timeline pagination, encrypted
send acceptance, and zero UTD callbacks in the observed timeline window. The
app-level smoke validates encrypted room open, send, relaunch restore, crypto
status UI, and no visible undecrypted placeholder on the smoke path. Encrypted
media is now explicitly detected and safely blocked in timeline/media/action
surfaces until decrypted media bytes are available. Production E2EE remains
blocked on complete recovery, verification/cross-signing, key backup restore,
encrypted media decryption, and broader encrypted-room regression coverage
before TestFlight or App Store release.

Requirements:

- Validate reading and sending in encrypted rooms.
- Identify key backup, verification, or cross-signing limitations.
- Document unsupported E2EE states.
- Define the SDK-backed app service integration work needed before external
  release.

Deliverables:

- E2EE validation report.
- Bug backlog for missing SDK flows.
- SDK live probe under `synara-ios/spikes/matrix-sdk-probe`.

Acceptance criteria:

- SDK live probe can authenticate, identify an encrypted room, observe timeline
  events, and send an encrypted message with disposable credentials.
- App-level encrypted room smoke can open the room, send, relaunch, restore,
  and render the sent message.
- Decryption failure renders a safe, understandable placeholder.
- The report identifies what is required for production-grade recovery and verification.

### IOS-0308: Matrix Rust SDK App E2EE Integration

Dependencies: IOS-0203, IOS-0204, IOS-0301, IOS-0303, IOS-0307.

Status: first production E2EE slice complete. The app target includes the Matrix
Rust SDK Swift package and SDK-backed auth, session restore, room list,
timeline, send, crypto status, recovery, and verification-request service
adapters. Build validation, deterministic unit/UI tests, full UI suite, and the
gated app-level live encrypted-room smoke pass. Encrypted media is a
safe-blocked state until decryption support is added. Release cannot claim
complete encrypted-room support until full recovery, verification/cross-signing,
key backup restore, encrypted media decryption, and broader encrypted
regression coverage are complete.

Requirements:

- Add the Matrix Rust SDK Swift package to the app target.
- Introduce a `SynaraMatrix` app-owned wrapper around SDK auth, session restore,
  room list, room timeline, send, media, and crypto-state APIs.
- Move password login/session restore from the REST-backed implementation to
  SDK-backed services without logging credentials or tokens.
- Store app-owned session metadata in Keychain and SDK state in SDK-approved
  persistent storage.
- Initialize SDK crypto before room sync and timeline rendering.
- Map decrypted SDK timeline events into existing `TimelineService` models.
- Preserve safe placeholders and disabled event actions for UTD or unsupported
  encrypted states.
- Detect encrypted media placeholders and block thumbnail/download/actions until
  encrypted media decryption is implemented.
- Send encrypted text messages through SDK timeline APIs.
- Add gated live simulator validation for encrypted and unencrypted rooms.
- Surface room/session crypto status in room headers and Settings.
- Provide conservative recovery UI with retry, Settings navigation, SDK-backed
  recovery-key submission, and SDK-backed device verification request.

Deliverables:

- SDK-backed Matrix service implementation.
- Unit tests for mapping decrypted, UTD, and unsupported encrypted timeline
  states.
- Gated live simulator smoke for encrypted room open and encrypted message send.
- Updated E2EE validation report with recovery, verification, key backup, and
  encrypted media status.

Acceptance criteria:

- Existing unencrypted live-smoke flows still pass.
- Encrypted-room timeline renders decrypted text events from the SDK path.
- Encrypted-room composer sends a text message that appears in the room.
- App relaunch restores the encrypted-room session and sent message without
  re-login.
- Room header and Settings expose crypto status and recovery state.
- UTD events never render as plaintext and never enable reply/edit/redact/react.
- Encrypted media does not expose authenticated media downloads or enable event
  actions before media decryption support exists.
- No credentials, access tokens, refresh tokens, recovery keys, or room keys are
  printed or committed.

Open implementation item:

- Complete production-grade recovery, verification/cross-signing, key backup
  restore, encrypted media decryption, and expanded encrypted-room regression
  coverage.

## Phase 4: Push, Badge, And Deep Links

Goal: deliver production-shaped iOS notifications with safe privacy defaults.

### IOS-0401: Notification Permission UI

Dependencies: IOS-0204.

Status: initial implementation complete for the local app shell. Settings now
shows notification authorization state through a native permission service and
can request authorization. Physical-device/APNs work remains in IOS-0402.

Requirements:

- Request notification permission at a contextually appropriate time.
- Show current permission state in settings.
- Handle denied, provisional, authorized, and unavailable states.

Deliverables:

- Notification settings screen.
- Permission service.

Acceptance criteria:

- User can request permission.
- Denied state explains how to change system settings.
- Unit tests cover permission-state mapping.

### IOS-0402: APNs Device Registration

Dependencies: IOS-0401, Apple account setup.

Status: implementation in the iOS app is complete pending physical-device validation.

Requirements:

- Register for remote notifications on a physical device.
- Capture APNs token securely.
- Redact token in logs.
- Separate sandbox and production environments.

Deliverables:

- APNs registration service.
- Device-only smoke instructions.

Acceptance criteria:

- Physical device receives an APNs token.
- Token is not printed raw.
- Simulator path fails gracefully.
- Registration state is visible in debug diagnostics without exposing secrets.

### IOS-0403: Matrix Pusher Registration

Dependencies: IOS-0402, IOS-0204.

Status: implementation in local/local-session path is complete with session/token
refresh behavior; live-device registration remains pending.

Requirements:

- Register a Matrix pusher after APNs token acquisition.
- Unregister or replace stale pushers on logout or token rotation.
- Include app ID, pushkey, gateway URL, and format required by the chosen push gateway.

Deliverables:

- Matrix pusher service.
- Tests for pusher payload generation.

Acceptance criteria:

- Test account registers a pusher against a test homeserver.
- Logout removes or invalidates app push state where possible.
- Token rotation updates the pusher.
- Unit tests cover payload shape and redaction.

### IOS-0404: Push Gateway Staging

Dependencies: IOS-0403.

Status: local staging scaffolding is in place (including payload shape assertions in tests);
production/staging gateway endpoint is still pending. iOS app code still requires
external gateway validation before this item is complete.

Requirements:

- Deploy or configure a staging push gateway.
- Keep APNs credentials in a secret manager.
- Document sandbox/production split.

Deliverables:

- Gateway deployment notes.
- Health check.
- Incident and key rotation draft.

Acceptance criteria:

- Gateway can receive a Matrix push request and send APNs sandbox notification to a test device.
- Secrets are not committed.
- Gateway logs avoid decrypted content and tokens.

### IOS-0405: Notification Routing

Dependencies: IOS-0404, IOS-0102.

Status: implemented for room/event/localized route payloads with safe fallback to
notifications tab for malformed payloads and added route coverage for `/inbox`
and `notifications`.

Requirements:

- Route notification taps to a safe destination.
- Support room/event anchors when payload allows.
- Fall back to notification inbox when exact routing is unavailable.

Deliverables:

- Notification router.
- Deep-link parser.
- Tests for route handling.

Acceptance criteria:

- Tap opens the app from terminated, background, and foreground states.
- Invalid routes fall back safely.
- Unit tests cover room, event, Later, settings, and unknown routes.

### IOS-0406: Badge Count

Dependencies: IOS-0403, IOS-0205.

Status: implemented in-app with push payload parsing and auto-clear on foreground
or tap-open navigation.

Requirements:

- Set badge count based on shared notification semantics.
- Clear or update badge after relevant room/read state changes.
- Avoid permanent stale badges.

Deliverables:

- Badge count service.
- Mapping tests.

Acceptance criteria:

- Badge updates after receiving push.
- Badge clears or decreases after opening relevant content.
- Unit tests match the documented notification-summary contract.

## Phase 5: Synara Contracts And Agent Workflows

Goal: make iOS participate in Synara-specific workflows without desktop-only assumptions.

### IOS-0501: Contract Schemas

Dependencies: IOS-0004.

Status: completed locally with Swift-facing mirror work in `synara-ios/Synara/Contracts/SynaraContracts.swift`.

Requirements:

- Add JSON Schemas or equivalent machine-readable definitions for shared contracts.
- Start with Later item, agent action, deep link, notification summary, and media policy.
- Existing local pre-iOS contracts have initial schemas and fixture sets in `docs/contracts/`.

Deliverables:

- `docs/contracts/*.schema.json` or equivalent.
- Contract README.

Acceptance criteria:

- Schemas validate example payloads.
- Unknown-field policy is documented.
- Desktop-runtime and iOS owners are identified for each schema.
- Schema fixtures are validated by the desktop-runtime test suite.

### IOS-0502: Generate Or Mirror Swift Types

Dependencies: IOS-0501.

Status: implemented for notification summary, later content, agent action, and route models with decoding tests in `synara-ios/SynaraTests/SynaraContractsTests.swift`.

Requirements:

- Generate Swift types from schemas or create manually mirrored types with conformance tests.
- Avoid hand-written drift where practical.

Deliverables:

- Swift contract types.
- Schema fixture tests.

Acceptance criteria:

- Valid examples decode in Swift.
- Invalid examples fail safely.
- Type names and field names map predictably to schema names.

### IOS-0503: Later Inbox Read MVP

Dependencies: IOS-0502, IOS-0204.

Status: implemented in native service, Later tab UI, navigation handling, and UI
acceptance tests.

Requirements:

- Read `in.synara.later` account data.
- Render Later items without decrypted message bodies stored in account data.
- Navigate to room/event anchor when possible.

Implementation notes:

- `synara-ios/Synara/Services/AppEnvironment.swift` wires `MatrixAccountDataLaterService`
  in live mode and `MockLaterService` in mock mode.
- `synara-ios/Synara/App/AppTab.swift` now points the Later tab at `LaterListView`.
- `synara-ios/Synara/Features/LaterListView.swift` renders active/completed items,
  highlights reminder states, and opens room/event anchors where available.
- `synara-ios/Synara/Features/PlaceholderScreen.swift` maps the `.later` route to
  `LaterListView` for deep-link fallback.
- `synara-ios/SynaraTests/TimelineServiceTests.swift` adds sorting and decode tests
  for account-data Later loading.
- `synara-ios/SynaraTests/AppEnvironmentTests.swift` validates service wiring for
  both live and mock environments.
- `synara-ios/SynaraUITests/SynaraUITests.swift` validates active/completed
  rendering, unavailable destinations, and room/event navigation from Later rows.

Deliverables:

- Later service.
- Later list UI.

Acceptance criteria:

- Later items created on desktop appear on iOS.
- Completed and reminder states render distinctly.
- Tapping an item navigates to the best available event destination.
- Missing room/event renders a recoverable unavailable state.

### IOS-0504: Agent Card Rendering

Dependencies: IOS-0301, IOS-0502.

Status: implemented in native timeline parser and row renderer. `TimelineService` now
parses structured Hermes payloads from configured keys and explicit body markers,
maps them into `.agentCard`, and `RoomTimelineView` renders title/status/summary
with bounded action affordances. The SDK-backed live timeline path merges raw
Synara agent-card events from the Client-Server API when the Matrix Rust SDK does
not expose custom content in timeline items.

Requirements:

- Parse `in.synara.agent` and configured compatible agent keys.
- Render native agent card summary, sections, status, and actions.
- Cap displayed block sizes and action counts.

Deliverables:

- Agent card parser.
- Agent card SwiftUI components.
- Fixtures for common and malformed cards.

Acceptance criteria:

- Valid agent card fixtures render in previews.
- Oversized or malformed cards degrade safely.
- Unknown agent payloads do not crash timeline rendering.
- VoiceOver labels summarize card state and primary action.

### IOS-0505: Agent Action Validation

Dependencies: IOS-0504.

Status: implemented in native runtime and unit tests.

Requirements:

- Validate action ID, title, kind, URL, prompt, and markdown before acting.
- Allow only safe HTTPS URLs for remote opens.
- Reject unsupported kinds locally.

Deliverables:

- Agent action validator.
- Unit tests for malicious and oversized payloads.

Acceptance criteria:

- Unsafe URLs are rejected.
- Oversized fields are rejected or clamped according to schema.
- Unknown kinds render disabled or hidden based on the contract.
- Tests cover valid, invalid, oversized, and unknown action payloads.

### IOS-0506: Agent Approval Actions

Dependencies: IOS-0505, IOS-0303.

Status: complete for Phase 5. The app maps approve/reject actions to
authenticated `m.room.message` sends carrying `in.synara.agent.action`, and the
gated live simulator smoke has validated approve persistence in the dedicated
Matrix test room.

Requirements:
Note: approval persistence is intentionally limited to a controlled Matrix event
contract. The app records the explicit user decision; it does not execute local
commands or infer behavior from arbitrary agent payloads.

- Implement approve, reject, copy markdown, copy JSON, copy prompt, and open safe link where supported by contract.
- Persist resulting action state through Matrix events or account data, not local-only iOS state.

Next implementation slices:

- Define `in.synara.agent.action` contract for approve/reject and persistence target. Complete.
- Wire Matrix send path for approve/reject actions to a controlled, authenticated endpoint. Complete.
- Return explicit retryable errors for unsupported action outcomes. Complete.
- Add unit tests for approval action execution, including blocked/error states. Complete.
- Add live smoke validation in a dedicated approval test room. Complete.

Deliverables:

- Agent action UI.
- Action result handling.

Acceptance criteria:

- Approve/reject works in test agent room.
- Copy actions put expected text on clipboard without hidden fields.
- Open link uses safe external URL policy.
- Action errors are user-visible and retryable when appropriate.

## Phase 6: Settings, Accessibility, Privacy, And Hardening

Goal: make the app reviewable, testable, and usable under real iOS conditions.

### IOS-0601: Settings MVP

Dependencies: IOS-0206, IOS-0401.

Status: complete for the native MVP. Settings now expose account,
notifications, appearance, security, About, Licenses, Privacy Policy, Support,
and confirmed logout/wipe flows. UI coverage validates notification settings,
release links, About metadata, license visibility, and destructive logout
confirmation.

Requirements:

- Include account, notifications, appearance, security, about, licenses, privacy policy, support, and logout.
- Keep destructive actions confirmed.

Deliverables:

- Settings screens.
- About/license placeholders.

Acceptance criteria:

- Settings UI test covers notification settings and logout.
- About screen shows version, build, license, support, and privacy links.
- Destructive wipe requires confirmation.

### IOS-0602: Accessibility Audit Pass

Dependencies: IOS-0302, IOS-0303, IOS-0601.

Status: initial MVP pass complete. Checklist:
[`accessibility-checklist.md`](../../synara-ios/docs/accessibility-checklist.md).
Physical-device VoiceOver and screenshot capture remain release-candidate
gates before external TestFlight.

Requirements:

- Audit core flows with VoiceOver and large Dynamic Type.
- Add labels, hints, traits, and sort priorities where needed.
- Fix clipped text and tap target issues.

Deliverables:

- Accessibility checklist.
- Fixes for core flows.

Acceptance criteria:

- VoiceOver can complete login, open room, read message summaries, send message, open settings, and logout.
- Large Dynamic Type does not clip primary controls.
- Tap targets meet platform expectations.

### IOS-0603: Performance Profiling Pass

Dependencies: IOS-0302, IOS-0303, IOS-0205.

Status: initial fixture pass complete. Report:
[`performance-report.md`](../../synara-ios/docs/performance-report.md).
The app has deterministic 1,000-room and 10,000-event UI fixtures; Instruments
traces remain a release-candidate gate once signing and device testing are
available.

Requirements:

- Profile room list, timeline scroll, launch, warm resume, send, media loading, and sync.
- Use Instruments where possible.
- Create performance fixtures.

Deliverables:

- Performance report.
- Performance backlog.

Acceptance criteria:

- Room list with 1,000 mock rooms remains usable.
- Timeline with 10,000 synthetic events remains usable.
- Launch and warm resume measurements are recorded.
- Any P0/P1 performance problems are tracked before TestFlight.

### IOS-0604: Privacy Manifest And Policy Inputs

Dependencies: IOS-0404, IOS-0601.

Status: draft complete for internal readiness. Inventory:
[`privacy-data-inventory.md`](../../synara-ios/docs/privacy-data-inventory.md).
The app target includes `PrivacyInfo.xcprivacy`; final privacy policy and App
Privacy labels remain human/legal gates before external TestFlight.

Requirements:

- Identify data collected, data linked to user, tracking status, and third-party SDK data use.
- Prepare App Store privacy label inputs.
- Draft privacy policy requirements.

Deliverables:

- Privacy data inventory.
- Privacy label draft.

Acceptance criteria:

- Inventory covers Matrix account data, push tokens, logs, crash reports if any, media, files, contacts if ever used, and analytics if ever added.
- External TestFlight is blocked until privacy policy is ready.
- Any telemetry or crash SDK requires explicit approval.

### IOS-0605: Security Review

Dependencies: IOS-0206, IOS-0405, IOS-0506.

Status: initial MVP review complete. Report:
[`security-review.md`](../../synara-ios/docs/security-review.md). App routing
now rejects unsafe deep-link schemes/hosts, logs remain redacted by tests, and
logout wipe/agent-action security paths are covered. Production E2EE remains a
release-blocking SDK crypto integration gate.

Requirements:

- Threat model auth/session, local stores, push, agent actions, media, deep links, and logout wipe.
- Review logging and redaction.
- Review dependency versions and license/security advisories.

Deliverables:

- Security review report.
- Release-blocking issue list.

Acceptance criteria:

- No known token leaks in logs.
- Deep links reject unsafe routes.
- Agent actions reject malicious fixtures.
- Logout wipe passes local inspection.

### IOS-0606: TestFlight Readiness

Dependencies: IOS-0601, IOS-0602, IOS-0603, IOS-0604, IOS-0605.

Status: internal-readiness draft complete. Checklist:
[`testflight-readiness.md`](../../synara-ios/docs/testflight-readiness.md).
Archive/upload cannot be completed autonomously until Apple Developer
enrollment, signing, App Store Connect, privacy, and legal gates are closed.

Requirements:

- Prepare internal TestFlight build.
- Draft review notes and tester instructions.
- Use test homeserver and demo account.

Deliverables:

- TestFlight checklist.
- Internal release candidate build notes.

Acceptance criteria:

- Internal build archives successfully.
- Tester can log in with provided demo account.
- Known issues list is explicit.
- Legal and privacy gates are checked before external testers.

## Phase 6.5: iOS Product Design And UI Modernization

Goal: make the native iOS app feel like a polished, modern Matrix client with
clear continuity to the desktop Synara runtime and interaction quality in the
same class as contemporary iOS team-chat clients.

Status: complete for the current native MVP. A generated visual reference
mockup was used only for direction; no generated bitmap asset is shipped in the
app.

### IOS-0651: Design Direction And Native Component System

Dependencies: IOS-0601, IOS-0602.

Requirements:

- Align visual language with desktop Synara: sober surfaces, compact hierarchy,
  primary accent, clear warning/success/critical states, and practical density.
- Keep iOS native structure: `TabView`, per-tab `NavigationStack`, native
  search, native Forms/Lists where appropriate, and 44-point controls.
- Introduce reusable primitives for avatars, badges, status chips, cards, and
  icon controls.
- Avoid marketing-style decoration, gradient-orb backgrounds, and non-native
  controls.

Deliverables:

- Shared SwiftUI design primitives.
- Design-system preview coverage.

Acceptance criteria:

- Room, timeline, composer, agent card, auth, and settings surfaces use shared
  primitives rather than one-off styling.
- Light/dark mode remains system-driven.
- Dynamic Type and VoiceOver labels remain intact after modernization.

### IOS-0652: Room List Modernization

Dependencies: IOS-0651.

Requirements:

- Add native room search.
- Render avatars, unread badges, mention/highlight state, last activity, and
  invite treatment.
- Preserve stable identifiers and 1,000-room fixture behavior.

Deliverables:

- Modern room list row and invite card.
- UI tests for search and large-room scroll behavior.

Acceptance criteria:

- User can search by room name or preview.
- Highlighted rooms are visually distinguishable without relying only on color.
- 1,000-room fixture renders and scrolls.

### IOS-0653: Timeline, Composer, And Agent Card Modernization

Dependencies: IOS-0651.

Requirements:

- Render grouped timeline messages with sender-aware alignment, avatars,
  timestamps, reactions, reply previews, and safe encrypted/unknown states.
- Modernize composer with 44-point attachment/send controls, rounded input,
  reply/edit banners, and disabled send state.
- Make agent cards first-class: workflow header, status chip, clear approve,
  reject, open, and copy affordances.

Deliverables:

- Modern timeline rows and composer.
- Modern agent approval card layout.
- UI tests for timeline/composer/agent action smoke and 10,000-event fixture.

Acceptance criteria:

- Existing timeline, send, media, and agent approval UI tests pass.
- 10,000-event fixture renders and scrolls.
- Agent actions remain allow-listed and safely routed.

### IOS-0654: Auth, Settings, And Release Surface Polish

Dependencies: IOS-0651, IOS-0601.

Requirements:

- Make login/homeserver screens feel product-grade without introducing a
  marketing landing page.
- Keep settings native and inspectable while aligning it to shared visual
  primitives.
- Preserve About, Licenses, Privacy, Support, and confirmed logout acceptance.

Deliverables:

- Product headers for auth.
- Settings polish where it improves scanability without hiding release gates.

Acceptance criteria:

- Existing auth and settings UI tests pass.
- Destructive logout remains confirmed.
- Privacy/support/legal gates remain visible and explicit.

### IOS-0655: Visual QA And Screenshot Readiness

Dependencies: IOS-0652, IOS-0653, IOS-0654.

Requirements:

- Validate simulator build, deterministic unit tests, focused UI tests, and full
  deterministic UI suite.
- Capture or document visual reference states for room list, timeline, agent
  card, settings, and auth.
- Update validation docs with Phase 6.5 status.

Deliverables:

- Updated validation status.
- Passing test evidence.

Acceptance criteria:

- Simulator build is clean.
- Full unit test target passes.
- Full deterministic UI suite passes or completes successfully in underlying
  Xcode logs if the MCP response window times out.

## Phase 6.6: Mockup Fidelity And Desktop Parity UI Pass

Goal: close the visible distance between the native iOS app and the approved
high-fidelity iPhone mockups while preserving the desktop-derived Synara
workflow surface.

Status: complete for the current native iOS MVP. The implementation closes the
largest visible fidelity gaps in the room list, room timeline/composer, and
agent approval card while preserving the existing desktop-derived app
functions.

Requirements:

- Treat the approved three-screen mockup as the visual target for the room
  list, timeline/composer, and agent approval room.
- Preserve existing app functionality: Rooms, Notifications, Later, Settings,
  Matrix auth/session restore, secure storage, message actions, media,
  encrypted placeholders, and Synara agent approvals.
- Add room filter chips for All, Unread, Mentions, and Favorites without
  changing room-list service contracts.
- Render room/channel rows with stronger iconography, unread badges, mention
  affordances, activity timestamps, and Direct Messages grouping.
- Render timeline headers, media/file cards, reaction chips, reply affordances,
  and composer controls closer to the mockup.
- Render agent approval cards as detailed approval surfaces with status,
  changed-item/check summaries, preview links, and prominent approve/reject
  controls.

Acceptance criteria:

- The iOS room list visually exposes search, filters, room rows, DMs, unread
  state, and mention state in the same hierarchy as the mockup.
- The iOS timeline visually exposes room title, participant context, message
  grouping, attachment cards, reactions, reply/edit composer state, and send
  controls without regressing existing message actions.
- The iOS agent card surface visually supports production-style approval review
  and keeps all existing safe action routing.
- Existing deterministic unit/UI tests continue to pass or are updated only for
  intentional accessibility/visual-surface changes.

## Phase 8: Room, DM, Space, And Settings Parity

Status: implementation complete. The iOS app now has app-owned room-management
service contracts, mock services, SDK-backed adapters for create room, create
DM, join by room ID/alias, public-room discovery, leave, invite, room details,
avatar/alias profile edits, parent-space metadata, and room notification modes,
plus native UI surfaces for the same.

Implemented in this slice:

- Create room sheet with private/public visibility and encryption toggle.
- Create DM flow by Matrix ID, defaulting to encrypted.
- Join room flow by room ID or alias.
- Public room directory search in the join flow, with result cards that join by
  alias when available and room ID otherwise.
- Space filter chips in the room list, backed by Matrix parent-space metadata
  from sync and Matrix Rust SDK `SpaceService.joinedParentsOfChild`.
- Room details sheet with name, topic, aliases, encryption, member count,
  invite permission, notification mode, invite, and leave confirmation.
- Permission-aware room name/topic editing backed by Matrix Rust SDK
  `Room.setName` and `Room.setTopic` where room power levels allow those state
  events.
- Permission-aware canonical/alternative alias editing backed by Matrix Rust SDK
  `Room.updateCanonicalAlias`.
- Permission-aware room avatar upload/remove backed by Matrix Rust SDK
  `Room.uploadAvatar` and `Room.removeAvatar`.
- Read-only permissions summary showing current user level, Matrix threshold
  levels, and allowed/restricted state for message, invite, room profile, and
  moderation actions.
- Deterministic unit/UI tests for mock create-room validation, Matrix ID
  validation, room-management sheet, public directory search, space filters,
  room profile editing, alias/avatar edits, room permissions, room details, and
  leave navigation.
- Gated live room-management smoke for private encrypted room creation, room
  details read, optional invite, leave, and room-list recovery.

Remaining Phase 8 work:

- Power-level editing only after safe write semantics are specified; read-only
  power-level visibility is implemented.

## Phase 9: Daily Messaging Parity

Status: first implementation slice complete. The REST timeline mapper now
preserves Matrix `org.matrix.custom.html` formatted bodies separately from the
plain fallback body, renders a safe formatted `AttributedString` in the native
timeline, strips script/style/unsupported tags, allows only `http`, `https`,
and `matrix` links, and keeps the fallback body for editing/accessibility.
Encrypted media handling from Phase 7 also closes a daily-use safety gap by
making encrypted media visibly blocked instead of presenting it as a normal file
download.

Goal: make the iOS timeline and composer feel like a full Matrix client.

Requirements:

- Render safe Matrix HTML for bold, italic, links, inline code, quotes, lists,
  spoilers as safe text, and plain-text fallback.
- Add rich composer controls with valid outbound Matrix HTML and plain-text
  fallback.
- Add mentions autocomplete for room members and the current user display name.
- Add thread affordances, thread timeline navigation, and root-event context.
- Add read receipts and typing indicators from live Matrix data.
- Expand reply/edit UI with full preview, edit banner, cancel affordance, and
  permission-aware controls.
- Add reaction picker, add/remove own reaction, and reaction details.
- Add message actions for copy text, copy permalink/deep link, and iOS share.
- Render polls read-only before voting support is enabled.

Deliverables:

- Matrix HTML rendering and sanitizer tests.
- Composer rich-text send/edit tests.
- Mention insertion tests.
- Thread navigation tests.
- Reaction and message-action tests.
- Desktop contract tests for shared Matrix HTML/event semantics.

Acceptance criteria:

- Formatted messages render safely and never execute unsafe links or HTML.
- Sending rich text produces valid Matrix HTML plus a readable fallback body.
- Mention autocomplete inserts Matrix-valid mentions and degrades safely when
  member data is unavailable.
- Threads are navigable from timeline and deep links.
- Typing and read receipts render without timeline layout jumps.
- Reaction add/remove works live and updates the local timeline only after
  server success.
- UI tests cover rich send/edit, mention insertion, thread navigation,
  reactions, copy/share, and poll rendering.

Remaining Phase 9 work:

- Extend formatted-body support through the Matrix Rust SDK timeline path once
  the Swift SDK exposes formatted content metadata in the event timeline item.
- Implement rich outbound composer formatting instead of sending only the
  current markdown/plain-text content.
- Implement mentions autocomplete, threads, read receipts, typing indicators,
  richer reaction controls, message share/copy/permalink actions, and poll
  rendering.

## Final Release Phase: App Store Submission

Status: deferred until the desktop-parity release-gate track is complete:
Phase 7 production E2EE, Phase 8 room/DM/space/settings parity, Phase 9 daily
messaging parity, Phase 10 push/device/TestFlight readiness, and Phase 11
media/search/iOS platform polish.

Goal: submit a legally cleared, privacy-reviewed, production-signed app.

### IOS-0701: App Store Connect Metadata

Dependencies: IOS-0606, legal gate.

Requirements:

- Prepare app name, subtitle, description, keywords, category, age rating, support URL, privacy URL, screenshots, and review notes.
- Include demo credentials and homeserver instructions.

Deliverables:

- Metadata checklist.
- Screenshot set.

Acceptance criteria:

- App Store Connect record is complete.
- Screenshots use test data only.
- Review notes explain Matrix login, push behavior, and test account.

### IOS-0702: Production Signing And Archive

Dependencies: IOS-0701.

Requirements:

- Use production bundle ID and distribution signing.
- Archive and upload through approved process.
- Do not expose signing credentials in logs or repo.

Deliverables:

- App Store archive.
- Build number and commit reference.

Acceptance criteria:

- Uploaded build appears in App Store Connect.
- Build uses production APNs environment.
- Entitlements match approved capabilities.

### IOS-0703: Review Response And Release

Dependencies: IOS-0702.

Requirements:

- Track App Review feedback.
- Convert rejection or metadata questions into actionable fixes.
- Release only after approval and final smoke test.

Deliverables:

- Review feedback log.
- Release notes.

Acceptance criteria:

- Approved build is released manually or scheduled.
- Support and privacy URLs are live.
- Post-release monitoring plan exists.

## Cross-Cutting Task Backlog

### CONTRACT-0001: Deep Link Contract

Status: initial contract drafted in [Synara Route Contract](./synara-route-contract.md),
with canonical route schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Covers room, event, thread/root event, Later, notifications, settings, and agent approval destinations.
- Has examples for app-runtime, desktop, and iOS transports.
- Invalid routes fail closed.

### CONTRACT-0002: Notification Summary Contract

Status: initial contract drafted in
[Synara Notification Contract](./synara-notification-contract.md), with
canonical summary schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Defines unread, highlight, Later, invite, and agent approval counts.
- Explains badge count derivation.
- Has fixtures that match desktop-runtime behavior.

### CONTRACT-0003: Agent Action Contract

Status: initial contract drafted in
[Synara Agent Action Contract](./synara-agent-action-contract.md), with
canonical action schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Defines action IDs, titles, kinds, prompt, markdown, URL, limits, and unknown-kind behavior.
- Includes malicious fixture examples.
- Has TypeScript and Swift decoding tests or equivalent conformance tests.

### CONTRACT-0004: Later Account Data Contract

Status: initial contract drafted in
[Synara Later Account Data Contract](./synara-later-contract.md), with
canonical writer schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Defines `in.synara.later`, item IDs, kinds, room/event anchors, timestamps, and privacy rules.
- Includes fixtures for saved, reminder, completed, legacy plaintext-field stripping, and malformed items.
- Has TypeScript and Swift decoding tests or equivalent conformance tests.

### CONTRACT-0005: Room/Event Anchor Contract

Status: initial contract drafted in
[Synara Room/Event Anchor Contract](./synara-room-event-anchor-contract.md), with
canonical anchor schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Defines room, event, thread root, and parent-space anchor fields.
- Excludes decrypted previews, display names, tokens, and media URLs.
- Has TypeScript and Swift decoding tests or equivalent conformance tests.

### CONTRACT-0006: Media URL Policy Contract

Status: initial contract drafted in
[Synara Media And External URL Policy](./synara-media-policy.md), with canonical
safe remote URL schema and fixtures under `docs/contracts/`.

Acceptance criteria:

- Defines public HTTPS-only URL policy and local/private target rejection.
- Documents Matrix media and authenticated media handling.
- Has TypeScript and Swift decoding tests or equivalent conformance tests.

### CONTRACT-0007: Settings Compatibility Contract

Status: initial contract drafted in
[Synara Settings Compatibility Contract](./synara-settings-compatibility.md), with
shared and desktop-platform settings schemas and fixtures under
`docs/contracts/`.

Acceptance criteria:

- Defines shared versus platform settings boundaries.
- Preserves desktop legacy merged settings migration behavior.
- Has TypeScript and Swift decoding tests or equivalent conformance tests.

### INFRA-0001: Test Homeserver

Acceptance criteria:

- Provides test users, encrypted rooms, unencrypted rooms, media room, agent workflow room, and notification room.
- Can be reset or recreated.
- Contains no private production data.

### INFRA-0002: Push Gateway Staging

Acceptance criteria:

- Receives push from test homeserver.
- Sends sandbox APNs to a real test device.
- Logs are redacted.
- Key rotation path is documented.

### RELEASE-0001: iOS Release Checklist

Acceptance criteria:

- Includes signing, capabilities, legal, privacy, support, screenshots, demo credentials, App Review notes, push smoke, accessibility smoke, and rollback plan.
- Separates internal TestFlight, external TestFlight, and App Store release criteria.

## Suggested First Sprint

The local pre-iOS contract sprint is complete. The next sprint should validate
the current native desktop channels before opening iOS implementation work.

Recommended tasks:

1. Run macOS desktop smoke validation.
2. Run Linux desktop smoke validation.
3. Fix any regressions found in session restore, notifications, Later, routes,
   settings split, or agent actions.
4. IOS-0001: Choose Repository Layout.
5. IOS-0006: Native Matrix SDK Feasibility Spike.
6. IOS-0005: Tauri iOS Feasibility Spike only if still needed for evidence.
7. RELEASE-0001: iOS Release Checklist draft.

Expected sprint output:

- Validated macOS and Linux desktop builds against the pre-iOS changes.
- Architecture facts instead of guesses for the iOS implementation path.
- A clear native-vs-Tauri decision path if the optional Tauri spike is run.
- A startable iOS project backlog with release gates visible from day one.
