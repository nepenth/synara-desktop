# Renderer Native Client Facade — Contract & Execution Plan (Option A / D1C)

> Operator-authorized 2026-08-08: **Option A** (complete native rust-sdk, no holdover JS client),
> **D1C** (renderer cedes token custody to native entirely), **slice-by-slice** with parallel sub-agents.
> Status: **0 production importers — V-BURN complete**. Every slice landed as its own reviewed + CI-green PR.

## 1. Why this exists

The last js-sdk importer is `synara/src/client/initMatrix.ts` (still constructs a live JS client via
`createClient` + `IndexedDBStore` + `IndexedDBCryptoStore`). Native Rust (matrix-sdk 0.18) already
owns the live authenticated client + sync + **135 `matrix_*` commands**. This contract is the spec for
the renderer-facing `NativeMatrixClient` facade that replaces that JS client object with a command-
backed native proxy, letting initMatrix drop to **0 importers** and removing the JS client entirely.

## 2. The consumer surface (what the facade MUST satisfy)

Measured at tip `ac71e9bf`: **63 js-sdk client methods / 373 call sites** across the `LocalMx`/
`useMatrixClient` consumers, plus the `LocalMx[...]` typed refs (on/off/removeListener/decryptEventIfNeeded).
`Awaited<ReturnType<typeof initClient>>` is the type anchor — the facade must be assignable to it.

| Method                    |        Hits | Native backing / note                                                                              |
| ------------------------- | ----------: | -------------------------------------------------------------------------------------------------- |
| on                        |      **21** | BRIDGE: re-broadcast native Tauri events (sync_state, room_list, timeline_update, session_updated) |
| removeListener            |      **17** | BRIDGE                                                                                             |
| emit                      |      **18** | BRIDGE (internal dispatch)                                                                         |
| off                       |       **4** | BRIDGE                                                                                             |
| setMaxListeners           |       **1** | BRIDGE (no-op cap)                                                                                 |
| getSafeUserId             |      **54** | matrix_session_snapshot / restore envelope (user_id)                                               |
| getUserId                 |      **29** | matrix_session_snapshot                                                                            |
| getUser                   |       **2** | matrix_room_members_snapshot (profile)                                                             |
| getDeviceId               |       **2** | matrix_session_snapshot (device_id)                                                                |
| getThreePids              |       **2** | GAP: auth threepid surface                                                                         |
| setPusher                 |       **2** | GAP: pusher registration                                                                           |
| getPushers                |       **1** | GAP                                                                                                |
| setDisplayName            |       **2** | matrix_set_own_display_name                                                                        |
| setAvatarUrl              |       **3** | matrix_set_own_avatar                                                                              |
| getProfileInfo            |       **1** | matrix_room_members_snapshot profile / GAP                                                         |
| baseUrl                   |       **1** | build-time constant (native session client_base_config)                                            |
| getSyncState              |       **4** | matrix_sync_status + sync-updated event                                                            |
| getSyncStateData          |       **1** | matrix_sync_status extended                                                                        |
| clientRunning             |       **3** | matrix_sync_status ready/configured                                                                |
| retryImmediately          |       **2** | matrix_sync_status retry                                                                           |
| startClient               |       **1** | matrix_restore_session (native sync already running)                                               |
| stopClient                |       **4** | native: session teardown (matrix_logout path)                                                      |
| logout                    |       **1** | matrix_logout                                                                                      |
| refreshToken              |       **1** | **D1C: NEVER crosses IPC** — native refresh + session_updated event                                |
| setAccessToken            |       **1** | **D1C: renderer cedes custody — remove**                                                           |
| getAccessToken            |       **1** | **D1C: renderer cedes custody — remove/never exposed**                                             |
| initRustCrypto            |       **1** | native crypto (already live in Rust) — no-op                                                       |
| getRoom                   |      **83** | matrix_room_list_snapshot -> RoomReading facade (nativeRoom shape)                                 |
| getRooms                  |       **5** | matrix_room_list_snapshot                                                                          |
| fetchRoomEvent            |       **3** | matrix_timeline_snapshot/event readback                                                            |
| getRoomIdForAlias         |       **1** | matrix_room_directory_search resolve                                                               |
| getLocalAliases           |       **1** | GAP                                                                                                |
| createAlias               |       **1** | GAP                                                                                                |
| deleteAlias               |       **1** | GAP                                                                                                |
| \_unstable_getSharedRooms |       **1** | GAP (m.direct)                                                                                     |
| getLatestTimeline         |       **1** | matrix_timeline_snapshot                                                                           |
| relations                 |       **1** | GAP: relations (reactions aggregate)                                                               |
| setIgnoredUsers           |       **3** | GAP: ignore list                                                                                   |
| sendStateEvent            |      **20** | matrix*set_room*\* + GAP for arbitrary state event types                                           |
| sendMessage               |       **7** | matrix_send_text / matrix_send_attachment / matrix_send_poll / matrix_send_sticker                 |
| sendEvent                 |       **5** | matrix_send_text (generic) + GAP                                                                   |
| setRoomReadMarkers        |       **1** | matrix_timeline_set_read_state                                                                     |
| sendReadReceipt           |       **1** | matrix_timeline_event_readback                                                                     |
| getRoomPushRule           |       **1** | GAP: push rules                                                                                    |
| setAccountData            |      **11** | GAP: account data (currently no matrix_account_data command)                                       |
| getAccountData            |       **6** | GAP: account data                                                                                  |
| setRoomAccountData        |       **2** | GAP                                                                                                |
| uploadContent             |       **5** | matrix_upload_media (progress via token?)                                                          |
| getMediaConfig            |       **3** | matrix_call_media_config                                                                           |
| mxcUrlToHttp              |       **3** | native resolve_timeline_media / mxc -> native URI protocol                                         |
| cancelUpload              |       **1** | GAP: cancel                                                                                        |
| decryptEventIfNeeded      |       **2** | native crypto decrypt (event already decrypted natively)                                           |
| getCrypto                 |       **1** | native crypto owner stub                                                                           |
| downloadKeysForUsers      |       **1** | GAP: key download                                                                                  |
| matrixRTC                 | **removed** | V-CALL deep cut — call plugin + matrix-widget-api removed (zero-Matrix-JS)                         |
| http                      |       **4** | GAP: direct HTTP surface (native ops replace)                                                      |
| store                     |       **4** | GAP: localstore (IndexedDB gone in D1C)                                                            |
| getCapabilities           |       **1** | GAP                                                                                                |
| getAuthMetadata           |       **1** | GAP                                                                                                |
| search                    |       **1** | matrix_room_directory_search (message search GAP)                                                  |
| publicRooms               |       **1** | matrix_room_directory_search                                                                       |
| getThirdpartyProtocols    |       **1** | GAP                                                                                                |
| getOpenIdToken            |       **1** | GAP                                                                                                |

## 3. D1C — token custody (renderer cedes to native)

- `refreshToken`/`setAccessToken`/`getAccessToken` **never cross IPC**. Native refresh happens in Rust
  (in-place `Client::refresh_access_token` + keyring desktop-session store update), and the renderer is
  notified via a `session_updated` Tauri event (generation + readiness, **no tokens**).
- `MatrixClientSession` token copies in the renderer are removed as custody shifts native.
- This matches the existing posture: command DTOs already never carry tokens; `matrix_restore_session`
  is an availability gate, not a token handover.

## 4. Slice plan (each an independent PR; title carries the route `1 → 0`)

| #   | PR-title notation                                | Scope                                                                                                                                                                                            | Droppable |
| --- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| F0  | `1 → 0` contract & plan (this doc)               | contract spec + tracking                                                                                                                                                                         | —         |
| F1  | `1 → 0` facade core (emitter+lifecycle+identity) | NativeMatrixClient type, event bridge, session/sync proxy; initMatrix untouched, additive + unit-tested                                                                                          | —         |
| F2  | `1 → 0` room-agent + timeline                    | getRoom/getRooms/fetchRoomEvent/latestTimeline readings; nativeRoom wiring                                                                                                                       | —         |
| F3  | `1 → 0` send + state + account-data              | sendMessage/sendEvent/sendStateEvent/receipts; account-data commands (new native surface)                                                                                                        | —         |
| F4  | `1 → 0` media + profile                          | upload/media-config/mxc-resolve; displayname/avatar/threepid                                                                                                                                     | —         |
| F5  | `1 → 0` crypto + extended                        | decrypt/keys stubs (matrixRTC removed with V-CALL); http/store/search unfold                                                                                                                     | —         |
| F6  | `1 → 0` INITMATRIX re-point                      | initMatrix constructs facade; **drop importer 1→0**; dependency removal begins                                                                                                                   | **1→0**   |
| F7  | `0` lockdown + dep removal + V-BURN checklist    | **LANDED** (F6c-2c/3): allowlist 0 (full ban); `matrix-js-sdk@42.0.0` **fully removed** from package.json + lockfile (incl. devDeps; two-client harness + CI job retired); audit high-gate clean | **0**     |

## 5. Test & CI discipline

- Each slice: unit tests registered in `run-modernization-tests.mjs`; Quality gate + Desktop package gate green;
  UI/UX fidelity preserved (facade is type/behavior-identical for covered methods).
- Before any merge: `git ls-remote` feature-tip == last-recorded tip; provenance anchor `c358502c` verified.
- No secrets; D1C guarantees no tokens in renderer or IPC.

## 6. Sources

- `inventory-matrix-sdk-usage` at tip: 0 production + 0 test import files; allowlist 0 (full ban); repository-wide tooling 2 (guardrail fixtures only) — no js-sdk dynamic import remains.
- Method/hit census: `git grep -hPo '\bmx\.[a-zA-Z_][a-zA-Z0-9_]*'` across `synara/src` at `ac71e9bf`.
- Native surface: `src-tauri/src/lib.rs` registration list (135 commands).
