# 08 — Parity Matrix (desktop vs iOS)

Legend: ✅ shipping / 🔨 in progress / ⛔ not yet

## Current bounded evidence (not parity completion)

At `main` `76f67441`
(#1006), P2 still registers one hundred eleven names—the prior one
hundred nine plus `matrix_invites_report_spam` and
`matrix_invites_block_sender`. Desktop uses that registry. iOS has
typed SharedCore wrappers through S9-31, the S11 NSE read-only
store helper (never starts sync), S10 leftover UniFFI, and
P4-S12–S37 product consume. #1006 retires desktop JS encrypt/decrypt.
Product `MatrixRustSDK` callers are
retired; leftover I/O fail-closes without a live homeserver.
Hosted iOS CI is paused (#1003). Live homeserver proof is paused.
This is not iOS-on-engine.
The previous one hundred nine remain—
`matrix_login_flows`, `matrix_register_flows`, `matrix_session_snapshot`,
`matrix_sync_status`, `matrix_crypto_status`, `matrix_media_config`,
`matrix_cross_signing_status`, `matrix_secret_storage_status`,
`matrix_typing_snapshot`, `matrix_presence_snapshot`,
`matrix_verification_list`, `matrix_device_snapshot`,
`matrix_room_join_rule_snapshot`, `matrix_get_global_image_packs`,
`matrix_get_user_image_pack`, `matrix_get_room_image_packs`, the
three image-pack writes, `matrix_typing_set`, the two presence
subscription routes, `matrix_device_rename`, device delete start/cancel,
`matrix_verification_start`, and the other verification flow routes—and all
other census names fail closed. #713/#714/#716/#717 are P1-only mechanical
extraction: they add no Core command route, UDL, or iOS behavior. The prior #708 work is
only a pure iOS room-row unread presentation from closed `Joined`/`Invited`
membership, scalar counters, and a marked-unread flag to a `u64` count plus
highlight boolean. The prior #710 work is only a pure cold-start decision from
a latest-state boolean and `{Missing, Known}` to a boolean; Swift maps
`nil`/`.distantPast` to `Missing` and a real `Date` to `Known`. Neither is Core
SDK/service ownership: product `MatrixRustSDK` callers are retired;
leftover I/O fail-closes without a live homeserver. SharedCore can
start SyncService (S12); live homeserver proof is paused. This is not
iOS-on-engine.

| Capability                                                              | Desktop (src-tauri engine)                                      | iOS today (Swift re-impl)                                                           | After P4+P5 (shared core)                                                          |
| ----------------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Sync service (sliding sync + readiness + reconnect + capability probe)  | ✅ (sync/, readiness.rs, capability.rs)                         | 🔨 (SessionCoordinator/SignedInSessionReadiness — partial)                          | ✅ single implementation                                                           |
| Room list projection (summary, filters, sort, invites, badges, counts)  | ✅ (room_list/)                                                 | 🔨 (RoomListService)                                                                | ✅                                                                                 |
| Timeline projection (live, pagination, focus, composer, registry, view) | ✅ (timeline/)                                                  | 🔨 (TimelineService, StableTimelineViewport)                                        | ✅                                                                                 |
| Send / composer / attachment + poll send                                | ✅ (send/)                                                      | 🔨 (ComposerService, MediaService)                                                  | ✅                                                                                 |
| Read receipts / typing / unread / threads / polls / spaces / search     | ✅                                                              | 🔨/⛔ partial                                                                       | ✅                                                                                 |
| Crypto: SAS in-bbox verification                                        | ✅ (verification/inbox.rs + product_commands)                   | 🔨 (SharedCore leftover/product adapters; fail-closed without a live session)       | ✅ (core supervisor)                                                               |
| Crypto: key backup + restore                                            | ✅ (backup/)                                                    | ⛔ (blocked externally per device-readiness)                                        | ✅ (shared suite gates)                                                            |
| Crypto: cross-signing / secret storage / room keys / UTD recovery       | ✅ (cross_signing/, secret_storage/, room_keys/, utd_recovery/) | ⛔ partial                                                                          | ✅                                                                                 |
| Session lifecycle: persist/restore/logout/wipe/recovery                 | ✅ (lifecycle/)                                                 | 🔨 (SessionCoordinator, SecureSessionStore, LocalWipeService)                       | ✅                                                                                 |
| Media cache/export                                                      | ✅                                                              | ⛔/🔨                                                                               | ✅ (moved, shells only add OS pickers)                                             |
| Notifications read-model                                                | ✅ (notifications/)                                             | 🔨 (PushService + NSE)                                                              | ✅ read model + NSE narrow API                                                     |
| Personal room notes (notes, ToDos, message anchors)                     | ✅ (`nativeRoomNotesOwner.ts` + React room panel)               | ✅ (`SharedCoreRoomNotesService` + SwiftUI room details; main/thread message menus) | ✅ one `room_notes_live.rs` Matrix account-data owner with platform UI projections |
| Badge / tray / global shortcuts                                         | ✅ platform                                                     | ✅ (badge)                                                                          | ✅ via sink                                                                        |
| UI                                                                      | ✅ React                                                        | ✅ SwiftUI                                                                          | UI stays platform-owned (non-goal to unify)                                        |
| Test gate                                                               | 800+ Rust + 6 Synapse proofs + boundary + audit                 | iOS sim unit tests                                                                  | ❗ ONE suite gates both                                                            |

Platform-specific behaviors intentionally never shared: credential stores,
APNs vs tray notification delivery, dialogs/file pickers, updater metadata,
tray/shortcut surfaces, app lifecycle.

### Room-notes parity evidence — 2026-08-24

The iOS projection intentionally lives under Room Details instead of adding a
fifth top-level mobile tab. Both desktop and iOS route snapshot, upsert, delete,
ToDo completion, and ToDo move through the caller-owned SharedCore and the same
`in.synara.room_notes` global Matrix account-data contract. iOS also exposes
message anchors from both the main timeline and thread message menus.

Focused simulator coverage exercises light and dark appearance, CRUD/order
semantics, room isolation, durable-event-only message anchors, and pinned-message
navigation. A gated live smoke writes a private note through an independent
Matrix client, reads it in iOS, writes a second note from iOS, and reads that
write back through the Matrix account-data API. This evidence is feature-scoped;
it does not claim whole-client parity.
