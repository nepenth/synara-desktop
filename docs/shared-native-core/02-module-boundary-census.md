# 02 — Module Boundary Census (as of tip, 2026-08)

All paths relative to the repository root. Census technique: `git ls-files`,
`grep -rl`, `grep -rn` over `src-tauri/src`, `synara-ios`, `.github/workflows`.
The current source evidence is `feature/shared-native-core`
`0935062a` (#856, after #855).

## 2.1 Desktop application-logic layer

The P0 census counted **285 `.rs` files** under `src-tauri/src/matrix/`. At the
current evidence tip, there are 100 tracked Rust files under that desktop path
and 230 under `crates/synara-core/src/app/`; the difference reflects P1 moves,
not completion. The table remains the responsibility inventory rather than a
claim that every listed domain is desktop-resident. Each domain is typically
`mod.rs` (owning types/state) + `error.rs` + `live.rs` (actor/live state) +
`product_commands.rs` (Tauri command registration anytime the UI needs it) +
`tests.rs` (or `live_synapse_proof/` for integration proofs).

At this tip, Core holds DTOs, transport/IPC, the pure task registry, and app
modules for sync, room list, pure timeline, UTD recovery, notifications, polls,
relations, threads, unread, raw content, receipts, routes, security, search,
legacy, media_cache, media_export, crypto_store, members, user_profile,
room_directory session, verification inbox, account-data index, send
queue, room-keys transfer flow, supervisor actor, diagnostics health,
store identity/paths/key-material/vault-trait, client-builder error/features/config,
lifecycle recovery-copy / remote-logout / wipe / error / session-material trait / logout,
auth device-name, auth discovery/UIA/client_config, well-known HTTP transport,
later/room-notes codecs, image-pack DTO/type-filters/write-guards, m.direct snapshot helpers,
device presentation DTOs, secret-storage presentation DTOs, backup presentation DTOs, presence DTOs, typing snapshot DTO, verification presentation DTOs, room-directory DTOs, space presentation DTOs, cross-signing presentation DTOs, room-key transfer DTOs, members presentation snapshots, room join-rule presentation DTO, timeline presentation DTOs, send/profile-write/room-create/room-profile IPC DTOs, and media upload/download/config IPC DTOs.
#713–#717 moved whole harness directories; later splits moved only harness
files and left live `product_commands.rs` / `live.rs` on desktop.
Their desktop modules are thin re-exports (plus leftover command files).
Desktop retains the remaining domains and the adapter-side command, live, and
proof surfaces; see `10-current-handoff.md` for the full residency and
nonclaim record.

| Domain dir | Responsibility (authoritative module: read the `mod.rs`) |
|---|---|
| `account_data/` | account-data-backed features: image packs, later list, room notes (Core owns codecs; desktop keeps live Client RMW + `image_packs` subscribe) |
| `auth/` | login flows, UIA, register, reset password, device name, discovery, client config, `http_transport.rs` |
| `backup/` | key backup flows (`flow.rs`, `live.rs`) |
| `client_builder/` | client construction: features, open/drop, `sdk_handle.rs`, proxy handling |
| `cross_signing/` | cross-signing identity + live state |
| `crypto_store/` | crypto store continuity + tests |
| `devices/` | device list (live + commands) |
| `diagnostics/` | health, metrics, redaction, desktop compatibility |
| `dto/` | SDK-neutral DTOs: session, room, timeline, member, receipt, relation, typing, thread, media, notification, space, search, security, ids, upload |
| `ipc/` | **transport protocol** (see §2.4) |
| `legacy/` | js-sdk-era compatibility/transition shims |
| `lifecycle/` | session material, persist, restore, logout, wipe, recovery, remote logout |
| `media/`, `media_cache/`, `media_export/` | upload/download queues, cache index, export |
| `members/` | member index projection |
| `notifications/` | notification read model (for the OS notification bridge) |
| `polls/`, `relations/`, `threads/`, `typing/`, `unread/` | event projections (poll model, relations index, thread index, typing, unread state) |
| `presence/` | presence live + commands |
| `raw_content/` | content extraction/sanitisation |
| `receipts/` | read-receipt projection |
| `room_directory/` | public room directory (session, live) |
| `room_keys/` | room keys UI/flow surface |
| `room_list/` | **room list projection** (summary, filters, sort, invites, counts, delta, badges) |
| `room_ops/` | room actions/queue |
| `room_profile/` | room profile/index |
| `routes/` | route resolution |
| `search/` | message/room search |
| `secret_storage/` | secret store bootstrap/reset/status/unlock |
| `security/` | security status |
| `send/` | message/composer send, attachment queue, poll send, `live_synapse_proof/` |
| `spaces/` | space hierarchy |
| `store/` | key material, key vault, identity, paths |
| `supervisor/` | state machine actor (session supervision) |
| `sync/` | **sync service** + readiness (`readiness.rs`), reconnect, sliding-sync capability probe |
| `tasks/` | task registry / bridge for background work |
| `timeline/` | **timeline projection** (live, pagination, composer, media, registry, view, UTD) |
| `user_profile/` | own user profile/index |
| `utd_recovery/` | unable-to-decrypt recovery workflow |
| `verification/` | SAS device verification inbox + live |

Sibling desktop modules outside `matrix/` (platform/OS layer, KEEP in src-tauri):
`desktop.rs`, `desktop_agent_actions.rs`, `desktop_file_transfer.rs`,
`desktop_integration.rs`, `desktop_logging.rs`, `desktop_notifications.rs`,
`desktop_sanitize.rs`, `desktop_secret_store.rs`, `desktop_session.rs`,
`desktop_session_store.rs`, `desktop_shortcuts.rs`, `desktop_spellcheck.rs`,
`desktop_tray.rs`, `desktop_url.rs`, `build_info.rs`, `main.rs`, `lib.rs`,
`menu.rs`, `matrix_sdk_link_smoke.rs`.

## 2.2 Tauri seam — the only Tauri coupling

Measured on tip:

- **144** `#[tauri::command]` fns inside `src-tauri/src/matrix/`
  (`grep -rho '#\[tauri::command\]' src-tauri/src/matrix | wc -l`).
- **38** non-test `AppHandle`/`Emitter::emit` references
  (`grep -rn 'AppHandle\|app\.emit\|\.emit(' src-tauri/src/matrix --include='*.rs' | grep -v test | wc -l`).

`AppHandle` references by file (the full seam, non-test):

```
12 src-tauri/src/matrix/auth/product_commands.rs
 5 src-tauri/src/matrix/timeline/live.rs
 2 src-tauri/src/matrix/timeline/product_commands.rs
 2 src-tauri/src/matrix/room_profile/live.rs
 2 src-tauri/src/matrix/presence/live.rs
 2 src-tauri/src/matrix/devices/live.rs
 2 src-tauri/src/matrix/account_data/image_packs.rs
 1 src-tauri/src/matrix/auth/product.rs
 + a few one-offs (room_ops, room_list, media, members, etc. product_commands)
```

The dominant pattern is `app.emit(<event>, <payload>)` to push
snapshot/delta/status updates to the renderer (the `ipc/` stream topics), plus
a handful of OS actions (window/tray/badge). This is the seam that becomes the
`Platform` sink (`04-platform-sinks.md`).

**Consequence:** more than 97% of the matrix layer is already pure Rust with no
Tauri types — extraction is a packaging boundary, not a rewrite.

## 2.3 Dependency pins and package topology

- The root `Cargo.toml` now defines a workspace for `crates/synara-core` and
  `crates/synara-core-bindgen`. `src-tauri` remains deliberately excluded and
  is still a standalone package with its own committed lockfile.
- `src-tauri/Cargo.toml` pins `matrix-sdk = "=0.18.0"` (exact),
  `default-features = false`, with `bundled-sqlite`, `sqlite`, `markdown`
  (ruma/markdown), `qrcode`, and e2e (via `matrix-sdk-ui` feature unification).
  `matrix-sdk-ui = "=0.18.0"` and `matrix-sdk-crypto = "=0.18.0"` remain
  exact pins.
- The workspace and current Core residency do not mean the desktop package has
  completed its P1/P3 transition.

## 2.4 The existing transport protocol (`matrix/ipc/`)

`src-tauri/src/matrix/ipc/` already formalises the native socket between the
engine and the renderer — this is the seed of the transport-agnostic API:

| File | Purpose |
|---|---|
| `envelope.rs` | versioned Matrix IPC envelope, kind-discriminated messages |
| `protocol.rs` | generation checks, sequence ordering, gap/dup detection, bounds |
| `stream.rs` | stream topics, lifecycle states, control payloads |
| `stream_body.rs` | bind snapshot/delta bodies to topic-typed DTO containers |
| `version.rs` | protocol version + hard policy constants |
| `wire_counter.rs` | wire-safe counter bounds (REV-004 / R0.3) |
| `error.rs` | stable error categories (plan §6.4) |
| `contract_tests.rs`, `tests.rs` | contract + fixture test suites |

## 2.5 iOS inventory (`synara-ios/`)

Native SwiftUI app + Notification Service Extension. Swift files (abridged):

```
Synara/App/                      app entry, routes, tabs, RootShellView
Synara/Contracts/                SynaraContracts.swift (shared contracts)
Synara/Features/                 LoginView, HomeserverSelectionView, RoomListView,
                                 RoomTimelineView, LaterListView, SettingsView, Composer/, etc.
Synara/Services/                 MatrixRustSDKService, AuthService, SessionCoordinator,
                                 SecureSessionStore, SignedInSessionReadiness, RoomListService,
                                 TimelineService, ComposerService, MediaService, EventActionService,
                                 RoomReadMarkerService, PushService, NotificationPermissionCoordinator,
                                 AgentActionService, HomeserverDiscovery, LocalWipeService,
                                 AppEnvironment, AppServices, AppLogging, PerformanceInstrumentation,
                                 BoundedLRUCache, TimelineReplyPreview ...
SynaraShared/                    SynaraNotificationPreviewSupport.swift
SynaraNotificationService/       NotificationService.swift (APNs-backed NSE)
```

`synara-ios/project.yml` pins `matrix-org/matrix-rust-components-swift`
`exactVersion: 26.06.06`, product `MatrixRustSDK`.

## 2.6 CI gates that this program must keep green

`.github/workflows/ci.yml` jobs: `changes`, `validate-rust`,
`rust-dependency-audit`, `validate-frontend`, `ios-tests`, plus six Synapse
native proofs (reactions, attachments, polls, rich-messages, threads, receipts),
and `quality-gate` (aggregates required checks). Independent workflows:
`ios-skeleton.yml` (iOS Diagnostics, macOS-26) and `desktop-package-smoke.yml`
(macOS bundle + Linux .deb + Arch). The `check-matrix-boundaries` guard
(`scripts/check-matrix-boundaries.mjs`) runs on any `src-tauri` diff (PR #642).
