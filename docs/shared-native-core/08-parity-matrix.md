# 08 — Parity Matrix (desktop vs iOS)

Legend: ✅ shipping / 🔨 in progress / ⛔ not yet

| Capability | Desktop (src-tauri engine) | iOS today (Swift re-impl) | After P4+P5 (shared core) |
|---|---|---|---|
| Sync service (sliding sync + readiness + reconnect + capability probe) | ✅ (sync/, readiness.rs, capability.rs) | 🔨 (SessionCoordinator/SignedInSessionReadiness — partial) | ✅ single implementation |
| Room list projection (summary, filters, sort, invites, badges, counts) | ✅ (room_list/) | 🔨 (RoomListService) | ✅ |
| Timeline projection (live, pagination, focus, composer, registry, view) | ✅ (timeline/) | 🔨 (TimelineService, StableTimelineViewport) | ✅ |
| Send / composer / attachment + poll send | ✅ (send/) | 🔨 (ComposerService, MediaService) | ✅ |
| Read receipts / typing / unread / threads / polls / spaces / search | ✅ | 🔨/⛔ partial | ✅ |
| Crypto: SAS in-bbox verification | ✅ (verification/inbox.rs + product_commands) | 🔨 (MatrixRustSDKService delegates, SessionVerificationControllerDelegate) | ✅ (core supervisor) |
| Crypto: key backup + restore | ✅ (backup/) | ⛔ (blocked externally per device-readiness) | ✅ (shared suite gates) |
| Crypto: cross-signing / secret storage / room keys / UTD recovery | ✅ (cross_signing/, secret_storage/, room_keys/, utd_recovery/) | ⛔ partial | ✅ |
| Session lifecycle: persist/restore/logout/wipe/recovery | ✅ (lifecycle/) | 🔨 (SessionCoordinator, SecureSessionStore, LocalWipeService) | ✅ |
| Media cache/export | ✅ | ⛔/🔨 | ✅ (moved, shells only add OS pickers) |
| Notifications read-model | ✅ (notifications/) | 🔨 (PushService + NSE) | ✅ read model + NSE narrow API |
| Badge / tray / global shortcuts | ✅ platform | ✅ (badge) | ✅ via sink |
| UI | ✅ React | ✅ SwiftUI | UI stays platform-owned (non-goal to unify) |
| Test gate | 800+ Rust + 6 Synapse proofs + boundary + audit | iOS sim unit tests | ❗ ONE suite gates both |

Platform-specific behaviors intentionally never shared: credential stores,
APNs vs tray notification delivery, dialogs/file pickers, updater metadata,
tray/shortcut surfaces, app lifecycle.
