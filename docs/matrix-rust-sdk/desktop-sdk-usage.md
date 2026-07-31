# Desktop matrix-js-sdk usage inventory

> **Generated report.** Produced by `scripts/inventory-matrix-sdk-usage.mjs` from the machine-readable snapshot `docs/matrix-rust-sdk/desktop-sdk-usage.json`. Do not hand-edit; regenerate with `npm run inventory:matrix-sdk-usage`.

Schema version: `2`

## Analysis confidence

Method and listener findings are AST property-name candidates in files that import matrix-js-sdk. Receivers are not type-checked; counts are candidates, not verified SDK API calls.

## Repository-wide summary

Totals below count **import files** (static, `require()`, or dynamic `import()`) across all tracked TS/JS/tooling sources, split by role.

| Role       | Import files | Networking files | Networking findings |
| ---------- | -----------: | ---------------: | ------------------: |
| production |          170 |                2 |                   3 |
| test       |           10 |                8 |                  40 |
| tooling    |            3 |                2 |                   5 |
| **total**  |      **183** |                  |                     |

### Role definitions

- **production**: Non-test product runtime sources under synara/src/
- **test**: Test/mock paths (**tests**, **mocks**, _.test._, _.spec._)
- **tooling**: Scripts, integration harnesses, configs, and other non-runtime sources outside synara/src/

## Desktop runtime baseline (`synara/src/`)

This section is the plan §4 baseline: production and test import files under `synara/src/` only. Tooling outside this prefix is excluded here.

| Metric                  | Count |
| ----------------------- | ----: |
| Production import files |   170 |
| Test import files       |    10 |
| Total import files      |   180 |

### Plan comparison

Expected **220** production and **12** test import files.

- Production match: **no** (found 170)
- Test match: **no** (found 10)

### Production files by bucket (desktop runtime only)

| Bucket           | Files |
| ---------------- | ----: |
| client-lifecycle |     2 |
| component        |    29 |
| feature          |    56 |
| hook             |    47 |
| media-boundary   |     1 |
| page             |    10 |
| plugin           |     8 |
| shared-type      |     1 |
| state            |     5 |
| utility          |    11 |

## Aggregates: production

Scope: **production only**. Import files: 170. Files with any finding: 172.

### Imported modules

| Module path                                           | Import sites | Files |
| ----------------------------------------------------- | -----------: | ----: |
| `matrix-js-sdk`                                       |          162 |   162 |
| `matrix-js-sdk/lib/types`                             |           10 |    10 |
| `matrix-js-sdk/lib/@types/event`                      |            6 |     6 |
| `matrix-js-sdk/lib/matrixrtc/CallMembership`          |            4 |     4 |
| `matrix-js-sdk/lib/models/event`                      |            4 |     4 |
| `matrix-js-sdk/lib/models/relations`                  |            4 |     4 |
| `matrix-js-sdk/lib/@types/auth`                       |            2 |     2 |
| `matrix-js-sdk/lib/@types/read_receipts`              |            2 |     2 |
| `matrix-js-sdk/lib/client`                            |            2 |     2 |
| `matrix-js-sdk/lib/models/room`                       |            2 |     2 |
| `matrix-js-sdk/lib/crypto-api`                        |            1 |     1 |
| `matrix-js-sdk/lib/http-api/interface`                |            1 |     1 |
| `matrix-js-sdk/lib/matrixrtc/MatrixRTCSession`        |            1 |     1 |
| `matrix-js-sdk/lib/matrixrtc/MatrixRTCSessionManager` |            1 |     1 |
| `matrix-js-sdk/lib/matrixrtc/membershipData`          |            1 |     1 |
| `matrix-js-sdk/lib/models/event-timeline`             |            1 |     1 |

### Top imported symbols

| Symbol                      | Imports | Value | Type-only | Files |
| --------------------------- | ------: | ----: | --------: | ----: |
| `Room`                      |      83 |    80 |         3 |    83 |
| `MatrixClient`              |      44 |    40 |         4 |    44 |
| `MatrixEvent`               |      30 |    25 |         5 |    30 |
| `MatrixError`               |      29 |    29 |         0 |    29 |
| `JoinRule`                  |      18 |    18 |         0 |    18 |
| `RoomMember`                |      12 |    12 |         0 |    12 |
| `RoomEvent`                 |       9 |     9 |         0 |     9 |
| `ClientEvent`               |       8 |     8 |         0 |     8 |
| `EventType`                 |       8 |     8 |         0 |     8 |
| `Direction`                 |       7 |     7 |         0 |     7 |
| `IContent`                  |       7 |     5 |         2 |     7 |
| `MsgType`                   |       7 |     7 |         0 |     7 |
| `RoomEventHandlerMap`       |       7 |     7 |         0 |     7 |
| `IPushRules`                |       6 |     6 |         0 |     6 |
| `EventTimeline`             |       5 |     3 |         2 |     5 |
| `PushRuleKind`              |       5 |     5 |         0 |     5 |
| `RelationType`              |       5 |     5 |         0 |     5 |
| `RoomJoinRulesEventContent` |       5 |     5 |         0 |     5 |
| `SyncState`                 |       5 |     4 |         1 |     5 |
| `CallMembership`            |       4 |     4 |         0 |     4 |
| `IPushRule`                 |       4 |     4 |         0 |     4 |
| `Method`                    |       4 |     4 |         0 |     4 |
| `Relations`                 |       4 |     2 |         2 |     4 |
| `Capabilities`              |       3 |     3 |         0 |     3 |
| `ConditionKind`             |       3 |     3 |         0 |     3 |
| `EventTimelineSet`          |       3 |     3 |         0 |     3 |
| `RoomStateEvent`            |       3 |     3 |         0 |     3 |
| `RuleId`                    |       3 |     3 |         0 |     3 |
| `SearchOrderBy`             |       3 |     3 |         0 |     3 |
| `Visibility`                |       3 |     3 |         0 |     3 |
| `createClient`              |       3 |     3 |         0 |     3 |
| `ClientEventHandlerMap`     |       2 |     2 |         0 |     2 |
| `HistoryVisibility`         |       2 |     2 |         0 |     2 |
| `ICreateRoomStateEvent`     |       2 |     2 |         0 |     2 |
| `IEvent`                    |       2 |     2 |         0 |     2 |
| `IEventWithRoomId`          |       2 |     2 |         0 |     2 |
| `IMentions`                 |       2 |     2 |         0 |     2 |
| `MatrixEventEvent`          |       2 |     2 |         0 |     2 |
| `Preset`                    |       2 |     2 |         0 |     2 |
| `PushRuleAction`            |       2 |     2 |         0 |     2 |

### SDK model import coupling

| Model / symbol         | Files | Import occurrences |
| ---------------------- | ----: | -----------------: |
| `CallMembership`       |     4 |                  4 |
| `EventTimeline`        |     5 |                  5 |
| `IndexedDBCryptoStore` |     1 |                  1 |
| `IndexedDBStore`       |     1 |                  1 |
| `MatrixClient`         |    44 |                 44 |
| `MatrixError`          |    29 |                 29 |
| `MatrixEvent`          |    30 |                 30 |
| `Relations`            |     4 |                  4 |
| `Room`                 |    83 |                 83 |
| `RoomMember`           |    12 |                 12 |
| `RoomState`            |     2 |                  2 |
| `createClient`         |     3 |                  3 |

### Usage categories (candidates + imports + networking)

| Category                       | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| ------------------------------ | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `client_methods`               |    98 |                 0 |                   0 |                      0 |          0 |
| `room_methods`                 |    86 |                 0 |                   0 |                      0 |          0 |
| `event_emitters_listeners`     |    22 |                 0 |                 116 |                      0 |          0 |
| `sync_lifecycle`               |    12 |                11 |                   6 |                      0 |          0 |
| `crypto_verification_recovery` |     3 |                 3 |                   0 |                      0 |          0 |
| `indexeddb_matrix_stores`      |     1 |                 0 |                   0 |                      2 |          0 |
| `authenticated_media`          |     6 |                 7 |                   0 |                      0 |          0 |
| `matrixrtc_calls`              |     5 |                 0 |                   8 |                      0 |          0 |
| `account_data`                 |     9 |                14 |                   4 |                      0 |          0 |
| `room_lists`                   |    42 |                97 |                  10 |                      0 |          0 |
| `timelines`                    |    29 |                36 |                  48 |                      0 |          0 |
| `searches`                     |     3 |                 3 |                   0 |                      0 |          0 |
| `spaces`                       |    12 |                27 |                   0 |                      0 |          0 |
| `threads`                      |     1 |                 1 |                   0 |                      0 |          0 |
| `receipts`                     |     5 |                 4 |                   4 |                      0 |          0 |
| `notifications_push_rules`     |     8 |                10 |                   0 |                      0 |          0 |
| `uia_auth`                     |    49 |               112 |                   2 |                      0 |          0 |
| `custom_raw_event_sends`       |    12 |                21 |                   0 |                      0 |          0 |
| `direct_matrix_networking`     |     2 |                 0 |                   0 |                      0 |          3 |
| `client_events`                |     5 |                 0 |                  16 |                      0 |          0 |

### Top method-name candidates (not type-proven)

| Method name                | Candidate occurrences |
| -------------------------- | --------------------: |
| `getRoom`                  |                    71 |
| `getSafeUserId`            |                    55 |
| `getUserId`                |                    44 |
| `isSpaceRoom`              |                    27 |
| `sendStateEvent`           |                    18 |
| `findEventById`            |                    13 |
| `getRooms`                 |                     8 |
| `getAccountData`           |                     7 |
| `getLiveTimeline`          |                     7 |
| `refreshToken`             |                     7 |
| `getUnfilteredTimelineSet` |                     6 |
| `joinRoom`                 |                     6 |
| `setAccountData`           |                     6 |
| `getSyncState`             |                     4 |
| `leave`                    |                     4 |
| `sendMessage`              |                     4 |
| `stopClient`               |                     4 |
| `uploadContent`            |                     4 |
| `addPushRule`              |                     3 |
| `createRoom`               |                     3 |
| `deletePushRule`           |                     3 |
| `mxcUrlToHttp`             |                     3 |
| `sendEvent`                |                     3 |
| `setPushRuleActions`       |                     3 |
| `getCapabilities`          |                     2 |
| `getCrypto`                |                     2 |
| `getDeviceId`              |                     2 |
| `getLatestTimeline`        |                     2 |
| `invite`                   |                     2 |
| `redactEvent`              |                     2 |
| `retryImmediately`         |                     2 |
| `search`                   |                     2 |
| `getEventReadUpTo`         |                     1 |
| `getEventTimeline`         |                     1 |
| `getLocalAliases`          |                     1 |
| `getRoomIdForAlias`        |                     1 |
| `getRoomPushRule`          |                     1 |
| `getThreads`               |                     1 |
| `getUsersReadUpTo`         |                     1 |
| `getVisibleRooms`          |                     1 |

### Direct Matrix networking findings

| Path                       | Line | Kind                      | Indicator                            |
| -------------------------- | ---: | ------------------------- | ------------------------------------ |
| `synara/src/app/cs-api.ts` |  117 | `matrix_cs_path_template` | `/_matrix/client/versions`           |
| `synara/src/sw.ts`         |  107 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`  |
| `synara/src/sw.ts`         |  107 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail` |

## Aggregates: test

Scope: **test only**. Import files: 10. Files with any finding: 20.

### Imported modules

| Module path                              | Import sites | Files |
| ---------------------------------------- | -----------: | ----: |
| `matrix-js-sdk`                          |           10 |    10 |
| `matrix-js-sdk/lib/@types/event`         |            1 |     1 |
| `matrix-js-sdk/lib/@types/read_receipts` |            1 |     1 |

### Top imported symbols

| Symbol                  | Imports | Value | Type-only | Files |
| ----------------------- | ------: | ----: | --------: | ----: |
| `MatrixClient`          |       3 |     0 |         3 |     3 |
| `SyncState`             |       3 |     3 |         0 |     3 |
| `Direction`             |       2 |     2 |         0 |     2 |
| `EventTimeline`         |       2 |     0 |         2 |     2 |
| `MatrixEvent`           |       2 |     0 |         2 |     2 |
| `ReceiptType`           |       2 |     2 |         0 |     2 |
| `Room`                  |       2 |     0 |         2 |     2 |
| `EventStatus`           |       1 |     1 |         0 |     1 |
| `EventType`             |       1 |     1 |         0 |     1 |
| `IRefreshTokenResponse` |       1 |     0 |         1 |     1 |
| `MatrixError`           |       1 |     1 |         0 |     1 |
| `MatrixEventEvent`      |       1 |     1 |         0 |     1 |
| `RoomEvent`             |       1 |     1 |         0 |     1 |
| `WrappedReceipt`        |       1 |     0 |         1 |     1 |

### SDK model import coupling

| Model / symbol  | Files | Import occurrences |
| --------------- | ----: | -----------------: |
| `EventTimeline` |     2 |                  2 |
| `MatrixClient`  |     3 |                  3 |
| `MatrixError`   |     1 |                  1 |
| `MatrixEvent`   |     2 |                  2 |
| `Room`          |     2 |                  2 |

### Usage categories (candidates + imports + networking)

| Category                   | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| -------------------------- | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `client_methods`           |     3 |                 0 |                   0 |                      0 |          0 |
| `room_methods`             |     2 |                 0 |                   0 |                      0 |          0 |
| `event_emitters_listeners` |     1 |                 0 |                   1 |                      0 |          0 |
| `sync_lifecycle`           |     3 |                 0 |                   0 |                      0 |          0 |
| `authenticated_media`      |     1 |                 0 |                   0 |                      0 |          0 |
| `timelines`                |     2 |                 0 |                   0 |                      0 |          0 |
| `receipts`                 |     2 |                 0 |                   0 |                      0 |          0 |
| `uia_auth`                 |     1 |                 3 |                   0 |                      0 |          0 |
| `direct_matrix_networking` |     8 |                 0 |                   0 |                      0 |         40 |

### Top method-name candidates (not type-proven)

| Method name    | Candidate occurrences |
| -------------- | --------------------: |
| `refreshToken` |                     3 |

### Direct Matrix networking findings

| Path                                                            | Line | Kind                      | Indicator                                    |
| --------------------------------------------------------------- | ---: | ------------------------- | -------------------------------------------- |
| `scripts/__tests__/audit-matrix-public.test.mjs`                |   45 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/audit-matrix-public.test.mjs`                |   47 | `matrix_cs_path_literal`  | `/_matrix/federation/v1/version`             |
| `scripts/__tests__/audit-matrix-public.test.mjs`                |   51 | `matrix_cs_path_literal`  | `/_matrix/key/v2/server`                     |
| `scripts/__tests__/audit-matrix-public.test.mjs`                |   55 | `matrix_cs_path_literal`  | `/_matrix/client/v3/login`                   |
| `scripts/__tests__/check-matrix-rust-sdk-guardrails.test.mjs`   |  131 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/feature-parity-audit-normalization.test.mjs` | 2076 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/feature-parity-audit-normalization.test.mjs` | 2971 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  340 | `matrix_cs_path_template` | `/_matrix/client/versions\`                  |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  340 | `matrix_cs_path_template` | `/_matrix/client/versions\`                  |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  340 | `matrix_cs_path_template` | `/_matrix/client/versions\`                  |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  343 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  343 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  343 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail`         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail`         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail`         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  365 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  365 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  365 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  131 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  131 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  134 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  134 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  284 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  284 | `matrix_cs_path_template` | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  285 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  285 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  311 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  311 | `matrix_cs_path_template` | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/synapse-two-client-integration.test.mjs`     |   40 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   14 | `matrix_cs_path_literal`  | `/_matrix/media/v3/download/example/media`   |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   27 | `matrix_cs_path_literal`  | `/_matrix/media/v3/download/example/media`   |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   48 | `matrix_cs_path_literal`  | `/_matrix/media/v3/thumbnail/example/avatar` |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   56 | `matrix_cs_path_literal`  | `/_matrix/media/v3/thumbnail/example/avatar` |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   71 | `matrix_cs_path_literal`  | `/_matrix/media/v3/download/example/media`   |
| `synara/src/app/matrix/__tests__/media.test.ts`                 |   84 | `matrix_cs_path_literal`  | `/_matrix/media/v3/download/example/media`   |
| `synara/src/app/utils/__tests__/remoteContent.test.ts`          |   37 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |

## Aggregates: tooling

Scope: **tooling only**. Import files: 3. Files with any finding: 5.

### Imported modules

| Module path     | Import sites | Files |
| --------------- | -----------: | ----: |
| `matrix-js-sdk` |            3 |     3 |

### Top imported symbols

| Symbol         | Imports | Value | Type-only | Files |
| -------------- | ------: | ----: | --------: | ----: |
| `MatrixClient` |       1 |     1 |         0 |     1 |
| `createClient` |       1 |     1 |         0 |     1 |

### SDK model import coupling

| Model / symbol | Files | Import occurrences |
| -------------- | ----: | -----------------: |
| `MatrixClient` |     1 |                  1 |
| `createClient` |     1 |                  1 |

### Usage categories (candidates + imports + networking)

| Category                   | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| -------------------------- | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `client_methods`           |     3 |                 0 |                   0 |                      0 |          0 |
| `room_methods`             |     1 |                 0 |                   0 |                      0 |          0 |
| `event_emitters_listeners` |     1 |                 0 |                   2 |                      0 |          0 |
| `sync_lifecycle`           |     1 |                 3 |                   0 |                      0 |          0 |
| `account_data`             |     1 |                 1 |                   0 |                      0 |          0 |
| `room_lists`               |     1 |                 7 |                   0 |                      0 |          0 |
| `timelines`                |     1 |                 7 |                   0 |                      0 |          0 |
| `searches`                 |     1 |                 1 |                   0 |                      0 |          0 |
| `receipts`                 |     1 |                 1 |                   0 |                      0 |          0 |
| `uia_auth`                 |     1 |                 1 |                   0 |                      0 |          0 |
| `custom_raw_event_sends`   |     1 |                 1 |                   0 |                      0 |          0 |
| `direct_matrix_networking` |     2 |                 0 |                   0 |                      0 |          5 |

### Top method-name candidates (not type-proven)

| Method name                | Candidate occurrences |
| -------------------------- | --------------------: |
| `getRoom`                  |                     5 |
| `findEventById`            |                     2 |
| `createRoom`               |                     1 |
| `getAccountData`           |                     1 |
| `getEventTimeline`         |                     1 |
| `getLatestTimeline`        |                     1 |
| `getLiveTimeline`          |                     1 |
| `getSyncState`             |                     1 |
| `getUnfilteredTimelineSet` |                     1 |
| `getUserId`                |                     1 |
| `joinRoom`                 |                     1 |
| `paginateEventTimeline`    |                     1 |
| `search`                   |                     1 |
| `sendEvent`                |                     1 |
| `setRoomReadMarkers`       |                     1 |
| `startClient`              |                     1 |
| `stopClient`               |                     1 |

### Direct Matrix networking findings

| Path                                                                                                         | Line | Kind                      | Indicator                        |
| ------------------------------------------------------------------------------------------------------------ | ---: | ------------------------- | -------------------------------- |
| `scripts/audit-matrix-public.mjs`                                                                            |   97 | `matrix_cs_path_literal`  | `/_matrix/client/versions`       |
| `scripts/audit-matrix-public.mjs`                                                                            |   98 | `matrix_cs_path_literal`  | `/_matrix/federation/v1/version` |
| `scripts/audit-matrix-public.mjs`                                                                            |   99 | `matrix_cs_path_literal`  | `/_matrix/key/v2/server`         |
| `scripts/audit-matrix-public.mjs`                                                                            |  100 | `matrix_cs_path_literal`  | `/_matrix/client/v3/login`       |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/raw-matrix-http/synara/src/app/features/matrix-ipc/rawHttp.ts` |    6 | `matrix_cs_path_template` | `/_matrix/client/versions`       |

## Files (import and networking inventory)

| Path                                                                                                                          | Role       | Runtime | Bucket           | Import forms | Modules                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------- | ---------- | ------- | ---------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/__tests__/audit-matrix-public.test.mjs`                                                                              | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/check-matrix-rust-sdk-guardrails.test.mjs`                                                                 | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/feature-parity-audit-normalization.test.mjs`                                                               | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                                                                       | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                                                                       | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                                                                       | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`                                                                      | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/__tests__/synapse-two-client-integration.test.mjs`                                                                   | test       | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/audit-matrix-public.mjs`                                                                                             | tooling    | no      | —                | —            | —                                                                                                                                                                         |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/js-sdk-in-matrix-ipc/synara/src/app/features/matrix-ipc/leakyImport.ts`         | tooling    | no      | —                | static       | `matrix-js-sdk`                                                                                                                                                           |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/js-sdk-new-file/synara/src/app/features/brand-new-migration/NewClientBridge.ts` | tooling    | no      | —                | static       | `matrix-js-sdk`                                                                                                                                                           |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/raw-matrix-http/synara/src/app/features/matrix-ipc/rawHttp.ts`                  | tooling    | no      | —                | —            | —                                                                                                                                                                         |
| `synara/scripts/run-synapse-two-client-integration.mjs`                                                                       | tooling    | no      | —                | dynamic      | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/AccountDataEditor.tsx`                                                                             | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/AuthFlowsLoader.tsx`                                                                               | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/CapabilitiesLoader.tsx`                                                                            | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/JoinRulesSwitcher.tsx`                                                                             | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/RenderMessageContent.tsx`                                                                          | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/RoomSummaryLoader.tsx`                                                                             | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/ServerConfigsLoader.tsx`                                                                           | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/create-room/CreateRoomAliasInput.tsx`                                                              | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/create-room/utils.ts`                                                                              | production | yes     | component        | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/components/editor/autocomplete/EmoticonAutocomplete.tsx`                                                      | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/editor/autocomplete/RoomMentionAutocomplete.tsx`                                                   | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx`                                                   | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/editor/output.ts`                                                                                  | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/emoji-board/EmojiBoard.tsx`                                                                        | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/emoji-board/components/Item.tsx`                                                                   | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/event-readers/EventReaders.tsx`                                                                    | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/image-pack-view/RoomImagePack.tsx`                                                                 | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/invite-user-prompt/InviteUserPrompt.tsx`                                                           | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/leave-room-prompt/LeaveRoomPrompt.tsx`                                                             | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/leave-space-prompt/LeaveSpacePrompt.tsx`                                                           | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/member-tile/MemberTile.tsx`                                                                        | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/message/MsgTypeRenderers.tsx`                                                                      | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/message/Reaction.tsx`                                                                              | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/message/Reply.tsx`                                                                                 | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/message/content/PollContent.tsx`                                                                   | production | yes     | component        | static       | `matrix-js-sdk/lib/models/event`                                                                                                                                          |
| `synara/src/app/components/room-avatar/RoomAvatar.tsx`                                                                        | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/room-card/RoomCard.tsx`                                                                            | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/room-intro/RoomIntro.tsx`                                                                          | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/components/user-profile/UserChips.tsx`                                                                        | production | yes     | component        | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/cs-api.ts`                                                                                                    | production | yes     | app-other        | —            | —                                                                                                                                                                         |
| `synara/src/app/features/add-existing/AddExisting.tsx`                                                                        | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/call-status/CallRoomName.tsx`                                                                        | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/call-status/LiveChip.tsx`                                                                            | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/matrixrtc/CallMembership`                                                                                                             |
| `synara/src/app/features/call-status/MemberGlance.tsx`                                                                        | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/matrixrtc/CallMembership`                                                                                                             |
| `synara/src/app/features/call-status/MemberSpeaking.tsx`                                                                      | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/call/CallMemberCard.tsx`                                                                             | production | yes     | feature          | static       | `matrix-js-sdk/lib/matrixrtc/CallMembership`                                                                                                                              |
| `synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx`                                                   | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/developer-tools/StateEventEditor.tsx`                                                | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/emojis-stickers/RoomPacks.tsx`                                                       | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/general/RoomAddress.tsx`                                                             | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/general/RoomEncryption.tsx`                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/general/RoomHistoryVisibility.tsx`                                                   | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/features/common-settings/general/RoomJoinRules.tsx`                                                           | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/features/common-settings/general/RoomProfile.tsx`                                                             | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/common-settings/general/RoomPublish.tsx`                                                             | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/features/common-settings/general/RoomUpgrade.tsx`                                                             | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/features/common-settings/members/Members.tsx`                                                                 | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/create-chat/CreateChat.tsx`                                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/create-room/CreateRoom.tsx`                                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/create-space/CreateSpace.tsx`                                                                        | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/lobby/Lobby.tsx`                                                                                     | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/lobby/RoomItem.tsx`                                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/lobby/SpaceHierarchy.tsx`                                                                            | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/lobby/SpaceItem.tsx`                                                                                 | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/message-search/MessageSearch.tsx`                                                                    | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/message-search/SearchFilters.tsx`                                                                    | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/message-search/SearchResultGroup.tsx`                                                                | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/message-search/useMessageSearch.ts`                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room-nav/RoomNavItem.tsx`                                                                            | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room-settings/RoomSettings.tsx`                                                                      | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/CommandAutocomplete.tsx`                                                                        | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/MembersDrawer.tsx`                                                                              | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/RoomInput.tsx`                                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`                                                                                                                         |
| `synara/src/app/features/room/RoomSidePanel.tsx`                                                                              | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/RoomTimeline.tsx`                                                                               | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/matrixrtc/membershipData`                                                                           |
| `synara/src/app/features/room/RoomView.tsx`                                                                                   | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/RoomViewFollowing.tsx`                                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/RoomViewHeader.tsx`                                                                             | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/RoomViewTyping.tsx`                                                                             | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/jump-to-time/JumpToTime.tsx`                                                                    | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/message/Message.tsx`                                                                            | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/models/relations`                                                                                   |
| `synara/src/app/features/room/message/MessageEditor.tsx`                                                                      | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/message/NativeEventContent.tsx`                                                                 | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/message/Reactions.tsx`                                                                          | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/models/relations`                                                                                                                     |
| `synara/src/app/features/room/msgContent.ts`                                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/reaction-viewer/ReactionViewer.tsx`                                                             | production | yes     | feature          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/models/relations`                                                                                                                     |
| `synara/src/app/features/room/room-notes/RoomNotesPanel.tsx`                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/room/room-pin-menu/RoomPinMenu.tsx`                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/search/Search.tsx`                                                                                   | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/emojis-stickers/GlobalPacks.tsx`                                                            | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/notifications/AllMessages.tsx`                                                              | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/notifications/KeywordMessages.tsx`                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/notifications/NotificationModeSwitcher.tsx`                                                 | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/notifications/SpecialMessages.tsx`                                                          | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/settings/notifications/SystemNotification.tsx`                                                       | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/features/space-settings/SpaceSettings.tsx`                                                                    | production | yes     | feature          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/types.ts`                                                                                               | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useAccountDataCallback.ts`                                                                              | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useAuthFlows.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/auth`                                                                                                                          |
| `synara/src/app/hooks/useAuthMetadata.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useCall.ts`                                                                                             | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/matrixrtc/CallMembership`, `matrix-js-sdk/lib/matrixrtc/MatrixRTCSession`, `matrix-js-sdk/lib/matrixrtc/MatrixRTCSessionManager`      |
| `synara/src/app/hooks/useCallEmbed.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useCapabilities.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useCommands.ts`                                                                                         | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/hooks/useGetRoom.ts`                                                                                          | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useImagePackRooms.ts`                                                                                   | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useImagePacks.ts`                                                                                       | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useLocalRoomSummary.ts`                                                                                 | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useMatrixClient.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk/lib/client`                                                                                                                                                |
| `synara/src/app/hooks/useMemberEventParser.tsx`                                                                               | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useMemberFilter.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useMemberPowerTag.ts`                                                                                   | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useMemberSort.ts`                                                                                       | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useMembership.ts`                                                                                       | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useNotificationMode.ts`                                                                                 | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useParsedLoginFlows.ts`                                                                                 | production | yes     | hook             | static       | `matrix-js-sdk/lib/@types/auth`                                                                                                                                           |
| `synara/src/app/hooks/usePowerLevelTags.ts`                                                                                   | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/usePowerLevels.ts`                                                                                      | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/usePushRule.ts`                                                                                         | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRecentEmoji.ts`                                                                                      | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRelations.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk/lib/models/relations`                                                                                                                                      |
| `synara/src/app/hooks/useRoom.ts`                                                                                             | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomAccountData.ts`                                                                                  | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomActivity.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomAliases.ts`                                                                                      | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/hooks/useRoomCreators.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomDirectoryVisibility.ts`                                                                          | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomEvent.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomEventReaders.ts`                                                                                 | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomLatestRenderedEvent.ts`                                                                          | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomMembers.ts`                                                                                      | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomMeta.ts`                                                                                         | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/hooks/useRoomPinnedEvents.ts`                                                                                 | production | yes     | hook             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/hooks/useRoomState.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useRoomsNotificationPreferences.ts`                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useSidebarItems.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useSpace.ts`                                                                                            | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useSpaceHierarchy.ts`                                                                                   | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useStateEvent.ts`                                                                                       | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useStateEventCallback.ts`                                                                               | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useSyncState.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useUserPresence.ts`                                                                                     | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/hooks/useUserProfile.ts`                                                                                      | production | yes     | hook             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/matrix/__tests__/media.test.ts`                                                                               | test       | yes     | media-boundary   | —            | —                                                                                                                                                                         |
| `synara/src/app/matrix/media.ts`                                                                                              | production | yes     | media-boundary   | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/auth/login/PasswordLoginForm.tsx`                                                                       | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/auth/login/loginUtil.ts`                                                                                | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/ClientNonUIFeatures.tsx`                                                                         | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/ClientRoot.tsx`                                                                                  | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/SyncStatus.tsx`                                                                                  | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/__tests__/syncStatusCopy.test.ts`                                                                | test       | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/explore/Server.tsx`                                                                              | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/inbox/Notifications.tsx`                                                                         | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/sidebar/SpaceTabs.tsx`                                                                           | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/pages/client/space/Space.tsx`                                                                                 | production | yes     | page             | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/types`                                                                                                                                |
| `synara/src/app/pages/client/syncStatusCopy.ts`                                                                               | production | yes     | page             | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/call/CallEmbed.ts`                                                                                    | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/call/CallWidgetDriver.ts`                                                                             | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/call/utils.ts`                                                                                        | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/custom-emoji/ImagePack.ts`                                                                            | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/custom-emoji/utils.ts`                                                                                | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/react-custom-html-parser.tsx`                                                                         | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/recent-emoji.ts`                                                                                      | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/plugins/via-servers.ts`                                                                                       | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/__tests__/initMatrix.test.ts`                                                                           | test       | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/__tests__/performLogout.test.ts`                                                                        | test       | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/__tests__/tokenRefresh.test.ts`                                                                         | test       | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/hooks/roomList.ts`                                                                                      | production | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/room-list/__tests__/roomActivity.test.ts`                                                               | test       | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/room-list/roomActivity.ts`                                                                              | production | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/room-list/utils.ts`                                                                                     | production | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/room/roomInputDrafts.ts`                                                                                | production | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/state/upload.ts`                                                                                              | production | yes     | state            | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/__tests__/notifications.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/__tests__/remoteContent.test.ts`                                                                        | test       | yes     | utility          | —            | —                                                                                                                                                                         |
| `synara/src/app/utils/__tests__/syncLifecycle.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/__tests__/syncSplashRecovery.test.ts`                                                                   | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/__tests__/timelineLinks.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/__tests__/timelineOpening.test.ts`                                                                      | test       | yes     | utility          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/@types/read_receipts`                                                                               |
| `synara/src/app/utils/forward.ts`                                                                                             | production | yes     | utility          | static       | `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/models/event`, `matrix-js-sdk/lib/models/room`                                                                       |
| `synara/src/app/utils/matrix.ts`                                                                                              | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/notifications.ts`                                                                                       | production | yes     | utility          | static       | `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/@types/read_receipts`, `matrix-js-sdk/lib/client`, `matrix-js-sdk/lib/models/event`, `matrix-js-sdk/lib/models/room` |
| `synara/src/app/utils/polls.ts`                                                                                               | production | yes     | utility          | static       | `matrix-js-sdk/lib/models/event`                                                                                                                                          |
| `synara/src/app/utils/room.ts`                                                                                                | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/sort.ts`                                                                                                | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/syncLifecycle.ts`                                                                                       | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/syncSplashRecovery.ts`                                                                                  | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/timelineLifecycle.ts`                                                                                   | production | yes     | utility          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/models/event-timeline`                                                                                                                |
| `synara/src/app/utils/timelineLinks.ts`                                                                                       | production | yes     | utility          | static       | `matrix-js-sdk`                                                                                                                                                           |
| `synara/src/app/utils/timelineOpening.ts`                                                                                     | production | yes     | utility          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/@types/read_receipts`                                                                               |
| `synara/src/client/cryptoStoreContinuity.ts`                                                                                  | production | yes     | client-lifecycle | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/crypto-api`                                                                                                                           |
| `synara/src/client/initMatrix.ts`                                                                                             | production | yes     | client-lifecycle | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/http-api/interface`                                                                                                                   |
| `synara/src/sw.ts`                                                                                                            | production | yes     | service-worker   | —            | —                                                                                                                                                                         |
| `synara/src/types/matrix/common.ts`                                                                                           | production | yes     | shared-type      | static       | `matrix-js-sdk`                                                                                                                                                           |

## Scope notes

- Repository-wide totals include production, test, and tooling roles.
- Desktop runtime baseline counts only import files under synara/src/ and matches plan §4 (220 production / 12 test).
- Aggregates under aggregates.{production,test,tooling} never mix roles.
- Direct networking uses false-positive-resistant /\_matrix/{client,media,federation,key}/ path literals.
- Generated inventory; no wall-clock timestamps or absolute paths.
- JSON/Markdown artifacts are formatted with Prettier using config resolved from each artifact path (same as the root CLI).
