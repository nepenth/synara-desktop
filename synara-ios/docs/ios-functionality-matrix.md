# iOS Functionality Matrix

Reviewed: 2026-05-28

Purpose: track what the native iOS app currently supports, how each capability
is validated, and what must be promoted from partial support to release-grade
support before external TestFlight/App Store positioning.

Status labels:

- `Complete`: implemented and covered by deterministic tests or gated live smoke.
- `Partial`: usable surface exists, but backend depth, live coverage, or UX is
  incomplete.
- `Missing`: no meaningful iOS implementation yet.
- `Deferred`: intentionally out of current parity scope.

## Phase 6.7 Validation Matrix

| Area | Capability | iOS status | Validation now | Required next validation |
| --- | --- | --- | --- | --- |
| Auth | Homeserver entry and validation | Complete | `testShellShowsHomeserverSelectionWhenSignedOut`, `testInvalidHomeserverShowsErrorBeforeNavigation` | Live smoke with bad URL and valid URL on release build |
| Auth | Password login | Complete | Auth unit tests, mock UI login, gated live login smokes | Live negative-credential smoke with redacted logs |
| Auth | Secure session restore | Complete | secure-store unit tests, live relaunch E2EE smoke | Manual simulator/keychain reset and restore checklist |
| Auth | Logout and local wipe | Complete | `testLogoutReturnsToSignedOutShell`, wipe service tests | Live logout followed by relaunch session absence |
| Rooms | Joined room list | Complete | room-list unit tests, `testRoomListShowsStableRoomRows`, live visual smoke | Screenshot baseline compared to Rooms mockup |
| Rooms | Search rooms | Complete | `testRoomSearchFiltersByName` | Live room search smoke |
| Rooms | Unread/mentions/favorites filters | Partial | deterministic filter UI exists | Live Matrix unread/highlight fixtures and screenshot states |
| Rooms | Spaces filter | Partial | `testSpaceFilterScopesRoomList`, unit mapping | Live homeserver space fixture validation |
| Rooms | Invites | Complete | invite transition UI tests, membership unit tests | Live invite accept/reject smoke with second disposable user |
| Room management | Create private encrypted room | Complete | UI mock flow, live room-management smoke | Screenshot and E2EE confirmation after create |
| Room management | Create DM | Partial | UI/service path exists | Live DM create/send smoke with disposable second user |
| Room management | Join by alias/ID | Partial | UI/service path and public-directory mock coverage | Live join/leave alias smoke |
| Room management | Public-room discovery | Partial | mock UI coverage, SDK service path | Live directory fixture smoke |
| Room settings | Details, name/topic, alias, avatar | Partial | UI tests for profile edit, service/unit coverage | Live edit permission matrix and screenshot review |
| Room settings | Notification mode | Partial | UI surface/service path | Live push-rule persistence smoke |
| Room settings | Permissions read view | Complete | UI navigation coverage | Live power-level room fixture |
| Timeline | Initial timeline load | Complete | route UI test, live smoke | Performance signpost baseline |
| Timeline | Pagination/load older | Partial | deterministic UI presence | Live pagination smoke and latency measurement |
| Timeline | Text messages | Complete | mock send, live send smoke | Visual baseline for sender/time/body spacing |
| Timeline | Rich Matrix HTML | Partial | renderer unit tests | Live formatted-message fixture screenshot |
| Timeline | Reactions | Partial | unit/service path, timeline UI rendering | Live add/remove reaction smoke and reaction detail UX |
| Timeline | Edit/redact | Partial | service/unit path | Live edit/redact UI smoke |
| Timeline | Reply | Partial | composer relation path | Live reply smoke with context preview screenshot |
| Timeline | Threads | Partial | reply-backed thread route/UI test | True Matrix `m.thread` backend and live smoke |
| Timeline | Read receipts and typing | Missing | none | SDK integration, visual states, live smoke |
| Timeline | Polls | Missing | unsupported/unknown mapping only | Read-only render first, voting later |
| Composer | Plain text send | Complete | mock and live send smokes | Latency baseline |
| Composer | Attachment sheet UI | Partial | deterministic UI test, live visual smoke | Real option-by-option backend validation |
| Composer | Photos/media upload | Partial | media upload unit tests, placeholder UI | Real Photos picker/manual simulator media smoke |
| Composer | File/camera/location/poll/code/voice/contact | Missing | visible sheet actions only | Implement or disable honestly with clear states |
| Composer | Mentions autocomplete | Missing | none | Member lookup, insertion, live send validation |
| Composer | Emoji/custom emoji/stickers/GIF | Missing | none | Native emoji first, custom media feature flags |
| Media | Authenticated media thumbnails | Partial | loader unit tests | Live image/video/file room smoke |
| Media | Full-screen viewer | Partial | placeholder viewer | Real image/video/file viewer validation |
| Media | Encrypted media | Partial | safe-blocked placeholder/unit tests | Decryption/download support after E2EE recovery work |
| Agent workflows | Card render | Complete | unit/contract/UI tests, live seed smoke | Visual baseline against agent mockup |
| Agent workflows | Approve/reject event submit | Complete | unit and live approval smoke | Failure/retry live smoke |
| Later | Later list and room/event route | Complete | deterministic UI tests | Live account-data smoke |
| Notifications | In-app tab placeholder | Partial | routing tests | Real notification center data model |
| Push | Pusher registration service | Partial | push unit tests | Physical-device APNs receive/tap/badge smoke |
| Settings | Account/security/about/licenses/support/privacy | Partial | UI tests | Final URLs/legal/signing review |
| Settings | Device verification/recovery/key backup controls | Partial | UI tests and SDK status calls | Real cross-signing/recovery live smoke |
| Platform | iPad layout | Missing | none | Split-view implementation and screenshot matrix |
| Platform | Share extension | Missing | none | Authenticated share flow tests |
| Platform | App Intents/Shortcuts | Missing | none | Shortcut/entity test plan |

## Release Gate

No capability may be marked `Complete` unless it has:

1. A deterministic mock or unit test.
2. A live smoke when it mutates Matrix state or depends on Matrix server behavior.
3. A visual screenshot baseline when it is user-facing UI.
4. A documented manual test when automation cannot reasonably prove the behavior.
