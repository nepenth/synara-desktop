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

## Community readiness pass

1. **Before promotion:** run the same room workflow on macOS, COSMIC Linux, and iPhone: login/restart, compose/edit/reply, media, reactions, threads, unread/read, search, notifications, and logout. Capture screenshots at light/dark, narrow/wide windows, larger text, and reduced motion. Record any platform differences as issues with a reproducible room/event fixture.
2. **Accessibility and interaction:** keyboard-only navigation and visible focus on Desktop; VoiceOver labels, Dynamic Type, tap targets, contrast and modal dismissal on iOS. Include error, empty, offline and permission-denied states, not just the successful path.
3. **Release quality:** a clean APT install and upgrade on Pop!_OS 24.04 COSMIC, macOS install/update, and iOS TestFlight install/update; confirm desktop-file identity, tray behavior, notifications, deep links, logs, privacy wording and recovery after a network interruption.
4. **iPad later:** the Xcode project already targets device families `1,2` and supports iPad orientations. This establishes installability, not a finished tablet layout. Audit split view, resizable windows, sidebar/timeline/composer proportions, hardware keyboard, pointer and multitasking before advertising iPad support.
5. **“iPhone duo” later:** keep this as a design exploration until the intended device/form factor and interaction model are specified. Start with responsive widths and two-pane navigation tests rather than a separate device promise.

Exit criterion for a public client listing: the device smoke matrix passes, known limitations (including COSMIC dock badges) are documented, support/reporting and privacy pages are current, and a fresh install can complete the core messaging path without developer setup.
