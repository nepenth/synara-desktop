# Synara iOS App Store Plan

> **Historical planning baseline.** Apple enrollment, signing, and internal
> TestFlight delivery are now operational. Current release gates live in
> [`synara-ios/docs/release-checklist.md`](../../synara-ios/docs/release-checklist.md)
> and [`synara-ios/docs/testflight-readiness.md`](../../synara-ios/docs/testflight-readiness.md).

Reviewed: 2026-05-24

Execution spec: [Synara iOS Project Spec](./synara-ios-project-spec.md)

Pre-iOS consolidation: [Native-First Consolidation Plan](./native-first-consolidation-plan.md)

Phase 0 repository decision:
[ADR 0001](../../docs/adr/0001-ios-repository-layout.md)

Phase 0 architecture decision:
[ADR 0002](../../docs/adr/0002-ios-architecture.md)

Release gates:
[`synara-ios/docs/release-checklist.md`](../../synara-ios/docs/release-checklist.md)

## Goal

Ship an App Store-grade Synara iOS app that feels native on iPhone and iPad while preserving the cross-platform Synara product across macOS and Linux. The iOS app should extend Synara's direction as an agentic Matrix client, not merely package the existing app runtime inside a WebView.

The product goal is one Synara identity across platforms:

- Shared Matrix account-data namespaces and compatibility semantics.
- Shared room, notification, Later, draft, folder, and agent-action behavior.
- Platform-native presentation and capabilities where the platform expects them.
- No duplicate local state that can drift from Matrix state without a sync contract.

## Strategic Decision

The App Store-grade path should treat a native SwiftUI iOS client as the target architecture, backed by Matrix Rust SDK Swift bindings, with the current Tauri/WebView implementation retained as an internal implementation detail for packaged macOS and Linux apps.

A Tauri iOS shell is still useful as a short feasibility spike because the desktop app already uses Tauri and the current app runtime already has mobile routes. It should not be the default shipping architecture unless the spike proves it can meet App Store review, performance, push, storage, crypto, accessibility, and iOS interaction quality requirements.

Reasoning:

- Apple App Review Guideline 4.2 expects apps to provide lasting utility and app-like UI beyond a repackaged website.
- Native iOS push requires APNs capability, device-token handling, server-side provider infrastructure, and Matrix pusher registration.
- Matrix on iOS has a mature native direction through Matrix Rust SDK Swift components.
- A native app gives better control over Keychain, background behavior, notification actions, share extensions, accessibility, and iPad navigation.
- Tauri remains an excellent fit for Linux and desktop shells, where tray, notification, file, and global-shortcut behavior are already implemented.

## Product Principles

1. Native where users feel it.
   Use SwiftUI, iOS navigation idioms, system sheets, native notification actions, Keychain, share sheet, photo/file pickers, Dynamic Type, VoiceOver, and iPad multitasking support.

2. Shared where correctness matters.
   Keep Matrix account data, Synara namespaces, agent-action schemas, notification semantics, Later metadata, and deep-link contracts platform-independent.

3. Privacy-first notifications.
   Default push payloads should avoid decrypted message bodies. Rich notification content can be a later opt-in only after a threat model and extension storage model are reviewed.

4. Agent workflows must be typed and bounded.
   Agent actions should remain structured events and commands, not scraped message text. iOS should mirror the desktop bridge's validation pattern with Swift-native allow-lists and payload limits.

5. App Store readiness is part of the feature.
   Signing, provisioning, privacy labels, review notes, crash reporting policy, export compliance, source-code obligations, and TestFlight hardening are first-class deliverables.

## Cross-Platform Architecture

### Platform Apps

- `synara-desktop/synara/`: canonical React/Vite app runtime used by the desktop shell.
- `synara-desktop/`: Tauri desktop wrapper for macOS, Linux, and Windows-class desktop integration.
- `synara-ios/`: proposed SwiftUI iOS app, likely a sibling repository or workspace folder after the first spike.

### Shared Contracts

Create explicit shared contracts instead of relying on UI code reuse:

- `in.synara.*` Matrix account-data namespace documentation.
- Agent action payload schema:
  [Synara Agent Action Contract](./synara-agent-action-contract.md).
- Later account-data schema:
  [Synara Later Account Data Contract](./synara-later-contract.md).
- Notification and badge count summary schema:
  [Synara Notification Contract](./synara-notification-contract.md).
- Deep-link route schema: [Synara Route Contract](./synara-route-contract.md).
- Room/event/thread anchor schema:
  [Synara Room/Event Anchor Contract](./synara-room-event-anchor-contract.md).
- Media safety and external URL policy:
  [Synara Media And External URL Policy](./synara-media-policy.md).
- Settings compatibility table:
  [Synara Settings Compatibility Contract](./synara-settings-compatibility.md).

The contracts can start as Markdown plus JSON Schema. Later, generate TypeScript and Swift types from the schemas where useful.

### Native iOS Core

Proposed iOS stack:

- SwiftUI for UI.
- Matrix Rust SDK Swift package for Matrix sync, E2EE, room state, timeline, and crypto.
- Swift Concurrency for async flows.
- Keychain for credentials and cryptographic bootstrap secrets.
- App Groups only if extensions need shared access.
- UserNotifications for local and remote notification handling.
- APNs plus a Matrix push gateway for production push.
- Universal Links and custom URL scheme for deep links.
- XCTest and XCUITest for verification.

### Desktop and Linux Core

Keep the existing Tauri desktop strategy:

- The app runtime remains the feature owner for Matrix UI behavior on desktop.
- Tauri owns native tray, native notifications, badge counts, file handoff, global shortcuts, and desktop agent bridge.
- Linux remains first-class through Tauri packaging and desktop-environment smoke tests.

## LLC and Apple Account Plan

If publishing under the LLC, enroll the LLC in the Apple Developer Program as an organization rather than as an individual account.

Needed:

- Apple Account with two-factor authentication.
- Legal entity name for the LLC.
- D-U-N-S Number for the LLC.
- Person with authority to bind the LLC to Apple agreements.
- Apple Developer Program membership.
- App Store Connect team access.
- Bundle identifiers and capabilities.
- App Store Connect API key for CI once builds are automated.

Initial bundle IDs:

- iOS: `com.whylandcreative.synara`.
- iOS notification service extension: `com.whylandcreative.synara.NotificationService`, if added.
- iOS share extension: `com.whylandcreative.synara.ShareExtension`, if added.
- Existing desktop identifier: `com.whylandcreative.synara.desktop`.

Initial iOS capabilities:

- Push Notifications.
- Associated Domains.
- Keychain Sharing, only if needed.
- App Groups, only if extensions need shared data.
- Background Modes: remote notifications only if required and justified.

## Legal and Licensing Gate

This repository is AGPL-3.0-only. App Store distribution needs legal review before submission.

Open questions:

- Can the current Synara codebase be distributed through Apple's App Store under AGPL-3.0-only without adding impermissible restrictions?
- Do we need an App Store exception, dual-license grant, contributor permission, or a clean-room native client that does not copy AGPL-covered UI code?
- What source availability, attribution, license disclosure, and offer mechanics are required inside the app and on the website?
- Does the Matrix SDK dependency graph introduce any additional distribution obligations?

Plan:

1. Inventory licenses for `synara`, `synara-desktop`, Matrix SDKs, Tauri plugins, and any iOS dependencies.
2. Create a `THIRD_PARTY_NOTICES` path for iOS.
3. Get legal review before TestFlight external distribution or App Store submission.
4. Decide whether the iOS app is covered by AGPL, dual-licensed, or separately licensed with compatible contracts.

## Push Notification Architecture

App Store-grade iOS requires real APNs-backed push, not browser notification semantics.

High-level flow:

1. iOS app requests notification permission.
2. iOS app registers with APNs and receives an app-specific device token.
3. iOS app registers a Matrix pusher with the user's homeserver.
4. Homeserver sends push events to a Synara-controlled Matrix push gateway.
5. Push gateway sends APNs payloads to Apple.
6. iOS app handles tap, badge, and action routing.

Implementation choices:

- Start with Sygnal as the reference Matrix push gateway unless we have a strong reason to write a custom service.
- Use separate sandbox and production app IDs.
- Keep production APNs keys out of the app and out of ordinary developer machines.
- Keep push payloads generic by default: "New message" or "Synara notification".
- Use opaque routing tokens or conservative deep links where possible.
- Recompute exact room/event context after opening the app and decrypting locally.

Push milestones:

- Local notification permission and badge smoke test.
- APNs registration on a real device.
- Push gateway deployed in staging.
- Matrix pusher registration against a test homeserver.
- TestFlight push validation.
- Production APNs key rotation and incident procedure.

## Agentic iOS Features

Agent functionality should feel native without breaking the Matrix security model.

MVP:

- Render Hermes/agent cards in native timeline.
- Show pending approval state clearly.
- Approve, reject, copy prompt, copy markdown, copy JSON, and open safe HTTPS links.
- Persist action state through Matrix events/account data, not iOS-only storage.
- Route agent notification taps to an approval inbox or event anchor.

Post-MVP:

- Notification actions for approve/reject where payloads are safe and authentication requirements are clear.
- App Intents for opening Later, notifications, agent approvals, and room search.
- Share extension for sending links/text/files into a room.
- Widgets or Live Activities only after core notification and privacy behavior is stable.

Security requirements:

- Validate action kind, URL, prompt length, markdown length, and display text in Swift before acting.
- Never execute arbitrary agent-provided commands locally.
- Never infer actions by scraping message HTML.
- Do not leak decrypted agent output into APNs payloads by default.

## MVP Scope

The App Store MVP should be useful enough to pass review and valuable enough for daily use by a real Matrix user.

Required:

- Homeserver selection and discovery.
- Login and logout.
- Session restore.
- Secure credential storage.
- E2EE-capable sync.
- Room list with unread/highlight state.
- DMs and rooms.
- Timeline read, send, reply, edit, delete/redact where permissions allow.
- Reactions.
- Media upload and download.
- Basic room details.
- Notification permission settings.
- APNs push, app badge, and notification tap routing.
- Later inbox read support if account data is already present.
- Synara agent card rendering and basic actions.
- Settings for account, notifications, appearance, security, and about/licenses.
- Crash-free TestFlight build.

Deferred:

- Voice/video calls.
- Full room administration.
- Full space administration.
- Full registration flow.
- Rich encrypted notification previews.
- Share extension.
- Widgets.
- Multi-account.
- Offline compose queue beyond drafts.
- Full desktop feature parity.

## Quality Bar

### Security

- Tokens and secrets stored in Keychain.
- No access tokens, device tokens, decrypted messages, or recovery keys in logs.
- ATS enabled; insecure homeserver support only behind explicit development or advanced-user gates.
- Clear logout wipes local stores, Keychain entries, caches, and extension-shared data.
- Threat model for push, agent actions, media downloads, and extension data access.
- Dependency audit before release.

### Privacy

- Minimal telemetry by default.
- Privacy policy ready before TestFlight external testing.
- App Privacy labels prepared from actual data flows.
- Push payloads avoid decrypted message bodies by default.
- Screenshot and demo process uses test accounts only.

### Accessibility

- VoiceOver labels for all primary timeline, composer, room list, and settings controls.
- Dynamic Type support through at least accessibility text sizes for core flows.
- Reduce Motion respected.
- Color contrast checked in light and dark mode.
- Keyboard navigation and hardware keyboard shortcuts for iPad where reasonable.

### Performance

Initial targets, to be refined during profiling:

- Cold launch to cached room list: under 2 seconds on current supported devices.
- Warm resume to usable room list: under 700 ms.
- Room list scroll: no visible hitching on 1,000 rooms.
- Timeline scroll: stable on 10,000-event synthetic room.
- Memory: no unbounded growth after repeated room navigation.
- Battery: sync and push behavior reviewed with Instruments Energy Log.

### Reliability

- Offline and poor-network states are explicit.
- Sync errors are recoverable.
- Key backup and verification failures are understandable.
- Media upload retry behavior is clear.
- App handles homeserver downtime without corrupting local state.

## Verification Plan

### Unit Tests

- Matrix event formatting.
- Synara namespace parsing.
- Agent action validation.
- Deep-link parsing.
- Notification payload routing.
- Settings migration.
- Keychain/session wrapper behavior with mocks.

### Integration Tests

- Test Synapse homeserver.
- Test encrypted and unencrypted rooms.
- Login, sync, send, edit, react, redact.
- Push gateway staging path.
- Logout wipe.

### UI Tests

- First launch.
- Login.
- Room list navigation.
- Send message.
- Reply and reaction.
- Notification tap into room.
- Agent approval action.
- Settings and logout.

### Manual Release Smoke

- iPhone small screen.
- iPhone Pro Max.
- iPad split view.
- Dark/light mode.
- Dynamic Type large sizes.
- VoiceOver core flow.
- Real APNs device push.
- Airplane mode and reconnect.
- Low Power Mode.

## Delivery Phases

### Phase 0: Program and Architecture Setup

Duration: 1-2 weeks.

Deliverables:

- Apple organization enrollment started or completed.
- D-U-N-S and LLC seller-name path confirmed.
- License review issue opened.
- Architecture Decision Record for native SwiftUI target versus Tauri iOS fallback.
- Shared contract inventory.
- iOS app repo/workspace location decided.
- Initial CI feasibility checked.

Exit criteria:

- We know the legal publication path.
- We know where the iOS app will live.
- We know which SDK path is primary.

### Phase 1: Native iOS Skeleton

Duration: 2-3 weeks.

Deliverables:

- SwiftUI app shell.
- App icon and launch screen placeholders.
- Navigation structure for room list, room timeline, settings, notifications, and Later.
- Matrix Rust SDK Swift package integrated.
- Build/run on simulator and physical device.
- Unit test target and UI test target.
- Basic logging policy.

Exit criteria:

- Clean debug build.
- Clean release archive locally.
- App launches on device.
- Empty-state UI meets iOS interaction basics.

### Phase 2: Auth, Session, and Sync

Duration: 3-5 weeks.

Deliverables:

- Homeserver selection.
- Login.
- Session restore.
- Keychain storage.
- Logout wipe.
- Initial sync.
- Room list with unread state.
- Basic settings/about/license screen.

Exit criteria:

- Test account can login, quit, reopen, and see rooms.
- Logout clears local state.
- No secrets appear in logs.

### Phase 3: Core Messaging

Duration: 5-8 weeks.

Deliverables:

- Timeline rendering.
- Send text messages.
- Reply, edit, redact, react.
- Media download.
- Media upload.
- Encrypted room read/send.
- Basic room details.
- Timeline pagination.
- Error and retry states.

Exit criteria:

- Daily-use chat loop works in encrypted and unencrypted rooms.
- Timeline performance passes synthetic and real-room smoke tests.

### Phase 4: Push, Badge, and Deep Links

Duration: 3-5 weeks.

Deliverables:

- APNs registration.
- Matrix pusher registration.
- Sygnal or custom push gateway staging deployment.
- Badge count sync.
- Notification tap routing.
- Notification settings.
- Production/sandbox APNs separation.

Exit criteria:

- Real device receives pushes from a test homeserver via staging gateway.
- Taps route to the right safe destination.
- Badge count clears predictably.

### Phase 5: Synara Agent Workflows

Duration: 3-5 weeks.

Deliverables:

- Native agent card rendering.
- Agent approval inbox or filter.
- Approve/reject/copy/open actions.
- Shared schema conformance tests.
- Safe URL and payload validation.
- Notification routing for pending approvals.

Exit criteria:

- iOS can participate in the core agent workflows without desktop-only assumptions.
- Invalid action payloads are rejected locally.

### Phase 6: TestFlight Hardening

Duration: 3-6 weeks.

Deliverables:

- Internal TestFlight.
- External TestFlight if licensing/privacy gates are clear.
- Crash reporting decision and privacy disclosure.
- App privacy labels.
- Review notes and demo credentials.
- App Store screenshots.
- Release checklist.
- Security review.
- Accessibility pass.

Exit criteria:

- No known P0/P1 bugs.
- Legal, privacy, and licensing gates are closed.
- App Store Connect metadata is complete.

### Phase 7: App Store Submission

Duration: 1-2 weeks, review-dependent.

Deliverables:

- Signed App Store Connect build.
- Review notes with test account and homeserver.
- Privacy policy link.
- Support URL.
- Source/license notices.
- Review response plan.

Exit criteria:

- App approved and released, or review feedback converted into tracked fixes.

## Parallel Workstreams

### Platform Contracts

- Formalize `in.synara.*` behavior.
- Keep [Synara Shared Contract Inventory](./synara-contracts.md) current.
- Create schemas for any new shared account-data, route, notification, media,
  settings, or agent-action contracts before iOS consumes them.
- Add conformance tests in the desktop runtime and iOS.

### Desktop and Linux Continuity

- Keep Tauri desktop features aligned with iOS equivalents.
- Avoid adding desktop-only Matrix account data.
- Document capability differences.
- Preserve Linux validation as part of release quality.

### Push Infrastructure

- Choose Sygnal or custom gateway.
- Create staging and production environments.
- Store APNs keys in secret manager.
- Monitor gateway delivery failures.
- Document rotation and incident procedures.

### Design System

- Define Synara iOS visual language.
- Map desktop-runtime concepts to iOS components.
- Create accessibility and Dynamic Type rules.
- Produce reusable SwiftUI components for room rows, timeline events, composer, cards, and settings rows.

### Release Operations

- Apple Developer organization account.
- App Store Connect app record.
- CI signing approach.
- TestFlight groups.
- Support and privacy URLs.
- Public source/license fulfillment path.

## Decision Gates

### Gate A: Tauri iOS Spike

Run only as a feasibility check.

Questions:

- Does the current app runtime run reliably inside Tauri iOS?
- Do IndexedDB, WebCrypto/WASM, media auth, service worker assumptions, and mobile routing behave correctly?
- Can it avoid App Review 4.2 risk with enough native integration?
- Does keyboard/composer behavior feel acceptable?
- Is performance acceptable on real devices?

Likely outcome:

- Use findings to improve native-app route, composer, and desktop parity.
- Proceed with native SwiftUI for App Store target unless the spike is unexpectedly excellent.

### Gate B: Matrix SDK Path

Questions:

- Do Matrix Rust SDK Swift components cover the MVP flows?
- Are API stability and release cadence acceptable?
- Can E2EE and key backup be implemented safely within schedule?
- Do we need to wrap missing SDK capabilities?

### Gate C: Licensing

Questions:

- Is App Store distribution legally viable under the chosen license path?
- Are all notices and source obligations satisfied?
- Do dependencies permit distribution?

No App Store submission should happen before this gate is closed.

### Gate D: Push Privacy

Questions:

- What notification metadata is acceptable?
- Can notification actions be safely supported?
- Do extensions require shared crypto/session access?
- What is the default for users in encrypted rooms?

## Immediate Next Actions

1. Run macOS desktop smoke validation.
2. Run Linux desktop smoke validation.
3. Fix any platform regressions found during validation.
4. Decide repository layout for `synara-ios`.
5. Build a Matrix Rust SDK Swift spike for login, sync, room list, E2EE, and media.
6. Build a non-credentialed local Tauri iOS spike only if we still need evidence before finalizing the native path.
7. Start Apple Developer Program enrollment or access setup when ready for device push, TestFlight, and signing.

## Primary References

- Apple Developer Program enrollment: https://developer.apple.com/help/account/membership/program-enrollment
- Apple D-U-N-S Number requirements: https://developer.apple.com/support/D-U-N-S/
- Apple App Review Guidelines: https://developer.apple.com/app-store/review/guidelines/
- Apple APNs registration: https://developer.apple.com/documentation/UserNotifications/registering-your-app-with-apns
- Apple remote notification server setup: https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server
- Tauri mobile development: https://v2.tauri.app/develop/
- Tauri iOS signing: https://v2.tauri.app/distribute/sign/ios/
- Tauri App Store distribution: https://v2.tauri.app/distribute/app-store/
- Tauri notification plugin: https://v2.tauri.app/plugin/notification/
- Matrix Rust SDK: https://github.com/matrix-org/matrix-rust-sdk
- Matrix Rust SDK Swift package: https://github.com/matrix-org/matrix-rust-components-swift
- Matrix iOS SDK note on Rust SDK focus: https://github.com/matrix-org/matrix-ios-sdk
- Matrix Push Gateway API: https://spec.matrix.org/unstable/push-gateway-api/
