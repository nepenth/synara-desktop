# Client polish and Synapse review — 2026-09-23

Scope: Synara Desktop on Pop!_OS 24.04 COSMIC and macOS, Synara iOS, Synapse 1.159–1.162, and matrix-sdk 0.19.1. Synapse 1.162 is **1.162.0rc1** as of this review; 1.161 is the latest stable release in the linked changelog.

## Changes prepared in this branch

| Area | Change | Device check still needed |
| --- | --- | --- |
| iOS composer | Use the same theme and color scheme as the SwiftUI composer surface for UIKit text, insertion, links, and pasted rich text. | Type and paste while changing system light/dark and Synara's theme on a physical iPhone. |
| Linux panel | Replace the tiny numeric overlay with one red dot for any unread count; leave the count in the tooltip/menu. | Check a 16–24 px panel icon on Pop!_OS COSMIC and clear it by reading all messages. |
| Linux timeline | Move inter-message spacing into the virtual row's measured box so following rows start after it; distinguish cached measurements by spacing setting. | Scroll a long room with grouped messages, media, replies and different spacing settings; resize the window and change text scaling. |
| Agent widget | Add a local, read-only priority/status widget whose data is supplied by a token-protected agent API. | Open it in Synara Desktop, confirm the Matrix widget handshake and refresh, then try an agent update. |

## COSMIC dock badge boundary

Synara already broadcasts the numeric `com.canonical.Unity.LauncherEntry` signal for docks that consume it. The Pop!_OS 24.04 COSMIC app list draws a desktop file icon and running-window marker but has no unread count or LauncherEntry handling in its current source. The COSMIC app tray therefore cannot show a Synara numeric badge from this client signal alone. This is an inference from [COSMIC app-list source](https://github.com/pop-os/cosmic-applets/blob/595e49c997c16c85665ccde2a08f214a95c08768/cosmic-app-list/src/app.rs). A native COSMIC app-list badge API or upstream applet change is needed; when available, add a COSMIC publisher alongside the existing Unity publisher and test it on the user's APT package. Do not treat the panel dot as completion of the dock request.

## Synapse and SDK opportunities

Sources: [Synapse release-v1.162 changelog](https://github.com/element-hq/synapse/blob/release-v1.162/CHANGES.md) and [matrix-sdk 0.19.1 release](https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.19.1). SDK 0.19.1 itself fixes published rustdoc packaging; the client APIs below arrived in 0.19.0 and are present in 0.19.1.

| Server change | Current Synara / SDK position | Action |
| --- | --- | --- |
| 1.162 RC defaults new rooms to version 12 | Create-room code allows server default or an explicit supported version; member snapshots already recognize creators beyond versions 1–11. | Add a room-version-12 interoperability smoke case before adopting 1.162 stable: create, join, invite, encrypt, redact, and inspect room settings. Do not hard-code version 11. |
| 1.162 RC returns `allowed_room_ids` from room hierarchy | SDK 0.19.1's Ruma `JoinRuleSummary::Restricted` / `KnockRestricted` already exposes the IDs; `snapshot_space_hierarchy` drops them when mapping into `NativeSpaceHierarchyRoom`. | Carry the IDs through the Core DTO and explain restricted join eligibility in the space UI after server rollout. Useful but lower priority than the client bugs. |
| 1.162 RC limits uploaded E2EE one-time keys and rate-limits profile lookups | Crypto uploads are SDK-owned. Synara already subscribes to its own profile with a fallback fetch. | Exercise SDK behavior against a 1.162 test server; avoid extra profile polling and surface HTTP 429 gracefully if profile screens encounter it. Do not bypass SDK crypto behavior. |
| 1.161 deprecates `matrix_rtc.livekit_service_url` and supports SFU WebSocket URLs | Synara already uses SDK 0.19 `discover_rtc_transports`, which checks the authenticated endpoint and falls back to well-known. | Test transport discovery on old/new Synapse configurations. Calls UI remains a separate product maturity item. |
| 1.160 optional MSC4262 profile updates in sliding sync (local users only) and 1.159 optional MSC4429 legacy profile updates | Synara uses `subscribe_to_own_profile`; the SDK notes that useful updates require the Profiles sliding-sync extension. | Verify the homeserver feature flag and Synara sync extension configuration, then test own-avatar/name propagation across devices. The server feature is opt-in and local-user limited. |
| 1.160 fixes missing sync stream positions after cancelled writes and transparent WebP thumbnails; 1.161–1.162 fix `state_after`, leave-room, and `/relations` behavior | Existing SDK sync, media preview, and timeline paths benefit from server fixes without a new client API. Synara already uses SDK media preview, unread totals, room subscriptions, retention, and automatic back pagination. | Add regression smoke cases only where users saw corresponding symptoms; no SDK upgrade is needed just to consume these fixes. |
| 1.161 adds GET for one delayed event | SDK widget code supports delayed-event protocol work, but Synara's widget grant policy excludes delayed capabilities. | Keep the sample widget outside delayed Matrix events. Revisit when reminders need server-managed delivery and a permission model. |

### Implementation and verification on this branch

- The space hierarchy now preserves `allowed_room_ids` for restricted and
  knock-restricted join rules through the SDK mapping, native DTO, UniFFI DTO,
  desktop parser, and room list. The UI explains how membership or an invite
  grants access. Non-restricted rooms reject unexpected allowed IDs.
- Room creation still delegates the default room version to the homeserver. A
  Core test proves an explicit room version 12 request and encrypted initial
  state are accepted by the client builder. A loopback-only v12 smoke script
  now covers default and explicit room creation, invite/join, event readback,
  relations, redaction, settings, and encrypted-room initial state with two
  accounts. Its simulated-server test passes. A real 1.162 server run and
  encrypted send/readback in two Synara clients are still needed when 1.162 is
  stable; this Mac has no Docker runtime.
- The disposable integration image is pinned to the latest stable Synapse
  1.161.0. Its existing CI scenarios exercise sync, receipts, attachments,
  reactions, polls, rich messages, and threads against that server. The local
  harness pin and secret-boundary tests pass. Docker is unavailable on this
  Mac, so its live tests were not run here.
- Profile reads and writes now classify SDK `M_LIMIT_EXCEEDED` as a bounded
  diagnostic. Desktop and iOS avatar and display-name errors remain visible
  after a failed write, with specific retry guidance for rate limits. The
  one-time-key upload path remains SDK-owned; testing its 1.162 quota behavior
  requires a disposable 1.162 server with fresh devices.
- SDK UI 0.19.1's `RoomListService::new_with` already enables the Profiles
  sliding-sync extension. Synara's `SyncService` uses that builder and
  `subscribe_to_own_profile` with one fallback fetch, so no extra profile
  polling was added. Cross-device name/avatar propagation still needs a
  Synapse server with MSC4262 enabled and two client sessions.
- Synara already calls `Client::discover_rtc_transports`; SDK 0.19.1 tests
  endpoint preference and well-known fallback. The transport DTO retains the
  LiveKit authorization-service URL, which is HTTP(S); the SFU WebSocket URL
  is supplied later by that service. Old/new Synapse RTC configuration needs
  two live servers or two successive disposable configurations.
- The recent server fixes for WebP thumbnails, cancelled sync writes,
  `state_after`, leave-room, and relations require server-side regression
  runs only where the symptom is present. The client paths already use the
  SDK sync, media and timeline APIs. Matrix delayed-event capabilities stay
  outside the sample widget's grant policy.

## Community readiness pass

1. **Before promotion:** run the same room workflow on macOS, COSMIC Linux, and iPhone: login/restart, compose/edit/reply, media, reactions, threads, unread/read, search, notifications, and logout. Capture screenshots at light/dark, narrow/wide windows, larger text, and reduced motion. Record any platform differences as issues with a reproducible room/event fixture.
2. **Accessibility and interaction:** keyboard-only navigation and visible focus on Desktop; VoiceOver labels, Dynamic Type, tap targets, contrast and modal dismissal on iOS. Include error, empty, offline and permission-denied states, not just the successful path.
3. **Release quality:** a clean APT install and upgrade on Pop!_OS 24.04 COSMIC, macOS install/update, and iOS TestFlight install/update; confirm desktop-file identity, tray behavior, notifications, deep links, logs, privacy wording and recovery after a network interruption.
4. **iPad first:** the Xcode project targets device families `1,2`. The new regular-width split canvas keeps the room list and conversation visible together; compact width keeps stacked navigation. Audit resizable windows, sidebar/timeline/composer proportions, hardware keyboard, pointer and multitasking before advertising iPad support.
5. **“iPhone duo” later:** keep this as a design exploration until the intended device/form factor and interaction model are specified. Start with responsive widths and two-pane navigation tests rather than a separate device promise.

Exit criterion for a public client listing: the device smoke matrix passes, known limitations (including COSMIC dock badges) are documented, support/reporting and privacy pages are current, and a fresh install can complete the core messaging path without developer setup.

### Local device evidence

| Surface | Evidence | Remaining check |
| --- | --- | --- |
| iPhone 17 simulator, dark appearance | [Mock room screenshot](assets/2026-09-23-iphone-dark-mock-room.png) and [typed-composer screenshot](assets/2026-09-23-iphone-dark-composer-typed.png). The typed text, caret and dark keyboard are legible; the composer input and busy-timeline retention UI tests pass. | Paste rich text while changing themes, test Dynamic Type and VoiceOver, then repeat on a physical iPhone. |
| iPad Pro 11-inch (M4), iOS 26.5 and 13-inch (M5), iOS 27 simulators | [Mock room-list screenshot](assets/2026-09-23-ipad-mock-room-list.png), [selected-room portrait screenshot](assets/2026-09-23-ipad-mock-room-open.png), and [landscape screenshot](assets/2026-09-23-ipad-mock-room-landscape.png). The UI test passes with the room list, timeline, and composer visible in both orientations, including the accessibility-medium text setting. | Resize a Stage Manager window, test hardware keyboard/pointer and VoiceOver, then verify with a live account. |
| macOS desktop | Installed app received a visual read-only review; branch browser fixtures passed 42 room-list, avatar, call-indicator and approvals checks. | Run a branch build with a disposable account and complete the room workflow; the installed app is an older build. |
| Pop!_OS 24.04 COSMIC | No local device available. | Test panel dot, unread clearing, long-room overlap, APT upgrade and notification integration on the user's machine. |
