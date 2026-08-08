# Desktop matrix-js-sdk usage inventory

> **Generated report.** Produced by `scripts/inventory-matrix-sdk-usage.mjs` from the machine-readable snapshot `docs/matrix-rust-sdk/desktop-sdk-usage.json`. Do not hand-edit; regenerate with `npm run inventory:matrix-sdk-usage`.

Schema version: `2`

## Analysis confidence

Method and listener findings are AST property-name candidates in files that import matrix-js-sdk. Receivers are not type-checked; counts are candidates, not verified SDK API calls.

## Repository-wide summary

Totals below count **import files** (static, `require()`, or dynamic `import()`) across all tracked TS/JS/tooling sources, split by role.

| Role       | Import files | Networking files | Networking findings |
| ---------- | -----------: | ---------------: | ------------------: |
| production |            5 |                2 |                   3 |
| test       |           10 |                8 |                  30 |
| tooling    |            3 |                2 |                   5 |
| **total**  |       **18** |                  |                     |

### Role definitions

- **production**: Non-test product runtime sources under synara/src/
- **test**: Test/mock paths (**tests**, **mocks**, _.test._, _.spec._)
- **tooling**: Scripts, integration harnesses, configs, and other non-runtime sources outside synara/src/

## Desktop runtime baseline (`synara/src/`)

This section is the plan §4 baseline: production and test import files under `synara/src/` only. Tooling outside this prefix is excluded here.

| Metric                  | Count |
| ----------------------- | ----: |
| Production import files |     5 |
| Test import files       |    10 |
| Total import files      |    15 |

### Plan comparison

Expected **220** production and **12** test import files.

- Production match: **no** (found 5)
- Test match: **no** (found 10)

### Production files by bucket (desktop runtime only)

| Bucket           | Files |
| ---------------- | ----: |
| client-lifecycle |     1 |
| feature          |     1 |
| hook             |     1 |
| plugin           |     2 |

## Aggregates: production

Scope: **production only**. Import files: 5. Files with any finding: 7.

### Imported modules

| Module path                            | Import sites | Files |
| -------------------------------------- | -----------: | ----: |
| `matrix-js-sdk`                        |            5 |     5 |
| `matrix-js-sdk/lib/http-api/interface` |            1 |     1 |

### Top imported symbols

| Symbol                     | Imports | Value | Type-only | Files |
| -------------------------- | ------: | ----: | --------: | ----: |
| `MatrixClient`             |       4 |     4 |         0 |     4 |
| `ClientEvent`              |       2 |     2 |         0 |     2 |
| `MatrixError`              |       2 |     2 |         0 |     2 |
| `MatrixEvent`              |       2 |     1 |         1 |     2 |
| `Room`                     |       2 |     2 |         0 |     2 |
| `AccessTokens`             |       1 |     0 |         1 |     1 |
| `Direction`                |       1 |     1 |         0 |     1 |
| `EventType`                |       1 |     1 |         0 |     1 |
| `IContent`                 |       1 |     0 |         1 |     1 |
| `ICreateClientOpts`        |       1 |     0 |         1 |     1 |
| `IEventWithRoomId`         |       1 |     1 |         0 |     1 |
| `IRefreshTokenResponse`    |       1 |     0 |         1 |     1 |
| `IResultContext`           |       1 |     1 |         0 |     1 |
| `ISearchRequestBody`       |       1 |     1 |         0 |     1 |
| `ISearchResponse`          |       1 |     1 |         0 |     1 |
| `ISearchResult`            |       1 |     1 |         0 |     1 |
| `IndexedDBCryptoStore`     |       1 |     1 |         0 |     1 |
| `IndexedDBStore`           |       1 |     1 |         0 |     1 |
| `KnownMembership`          |       1 |     1 |         0 |     1 |
| `MatrixEventEvent`         |       1 |     1 |         0 |     1 |
| `RoomStateEvent`           |       1 |     1 |         0 |     1 |
| `SearchOrderBy`            |       1 |     1 |         0 |     1 |
| `SendDelayedEventResponse` |       1 |     0 |         1 |     1 |
| `StateEvents`              |       1 |     0 |         1 |     1 |
| `SyncState`                |       1 |     1 |         0 |     1 |
| `TimelineEvents`           |       1 |     0 |         1 |     1 |
| `TokenRefreshFunction`     |       1 |     0 |         1 |     1 |
| `createClient`             |       1 |     1 |         0 |     1 |

### SDK model import coupling

| Model / symbol         | Files | Import occurrences |
| ---------------------- | ----: | -----------------: |
| `IndexedDBCryptoStore` |     1 |                  1 |
| `IndexedDBStore`       |     1 |                  1 |
| `MatrixClient`         |     4 |                  4 |
| `MatrixError`          |     2 |                  2 |
| `MatrixEvent`          |     2 |                  2 |
| `Room`                 |     2 |                  2 |
| `createClient`         |     1 |                  1 |

### Usage categories (candidates + imports + networking)

| Category                       | Files | Method candidates | Listener candidates | Constructor candidates | Networking |
| ------------------------------ | ----: | ----------------: | ------------------: | ---------------------: | ---------: |
| `client_methods`               |     5 |                 0 |                   0 |                      0 |          0 |
| `room_methods`                 |     2 |                 0 |                   0 |                      0 |          0 |
| `event_emitters_listeners`     |     2 |                 0 |                  12 |                      0 |          0 |
| `sync_lifecycle`               |     2 |                 6 |                   2 |                      0 |          0 |
| `crypto_verification_recovery` |     2 |                 2 |                   0 |                      0 |          0 |
| `indexeddb_matrix_stores`      |     1 |                 0 |                   0 |                      2 |          0 |
| `authenticated_media`          |     2 |                 1 |                   0 |                      0 |          0 |
| `room_lists`                   |     2 |                 6 |                   0 |                      0 |          0 |
| `timelines`                    |     2 |                 2 |                   4 |                      0 |          0 |
| `searches`                     |     3 |                 3 |                   0 |                      0 |          0 |
| `uia_auth`                     |     3 |                13 |                   0 |                      0 |          0 |
| `custom_raw_event_sends`       |     1 |                 2 |                   0 |                      0 |          0 |
| `direct_matrix_networking`     |     2 |                 0 |                   0 |                      0 |          3 |
| `client_events`                |     1 |                 0 |                   4 |                      0 |          0 |

### Top method-name candidates (not type-proven)

| Method name           | Candidate occurrences |
| --------------------- | --------------------: |
| `refreshToken`        |                     6 |
| `getRoom`             |                     5 |
| `getSafeUserId`       |                     4 |
| `stopClient`          |                     4 |
| `getDeviceId`         |                     2 |
| `search`              |                     2 |
| `findEventById`       |                     1 |
| `getCrypto`           |                     1 |
| `getRooms`            |                     1 |
| `getSyncState`        |                     1 |
| `initRustCrypto`      |                     1 |
| `logout`              |                     1 |
| `redactEvent`         |                     1 |
| `searchUserDirectory` |                     1 |
| `sendEvent`           |                     1 |
| `sendStateEvent`      |                     1 |
| `startClient`         |                     1 |
| `uploadContent`       |                     1 |

### Direct Matrix networking findings

| Path                       | Line | Kind                      | Indicator                            |
| -------------------------- | ---: | ------------------------- | ------------------------------------ |
| `synara/src/app/cs-api.ts` |  117 | `matrix_cs_path_template` | `/_matrix/client/versions`           |
| `synara/src/sw.ts`         |  107 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`  |
| `synara/src/sw.ts`         |  107 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail` |

## Aggregates: test

Scope: **test only**. Import files: 10. Files with any finding: 18.

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
| `direct_matrix_networking` |     8 |                 0 |                   0 |                      0 |         30 |

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
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  343 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  361 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/thumbnail`         |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`         |  365 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  133 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  133 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  137 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  137 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  299 | `matrix_cs_path_literal`  | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  299 | `matrix_cs_path_template` | `/_matrix/client/v1/media/download`          |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  300 | `matrix_cs_path_literal`  | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  300 | `matrix_cs_path_template` | `/_matrix/client/versions`                   |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  326 | `matrix_cs_path_literal`  | `/_matrix/client/v3/sync`                    |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`        |  326 | `matrix_cs_path_template` | `/_matrix/client/v3/sync`                    |
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

| Path                                                                                                                          | Role       | Runtime | Bucket           | Import forms | Modules                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------- | ---------- | ------- | ---------------- | ------------ | ------------------------------------------------------------------------------------------- |
| `scripts/__tests__/audit-matrix-public.test.mjs`                                                                              | test       | no      | —                | —            | —                                                                                           |
| `scripts/__tests__/check-matrix-rust-sdk-guardrails.test.mjs`                                                                 | test       | no      | —                | —            | —                                                                                           |
| `scripts/__tests__/feature-parity-audit-normalization.test.mjs`                                                               | test       | no      | —                | —            | —                                                                                           |
| `scripts/__tests__/inventory-matrix-sdk-usage.test.mjs`                                                                       | test       | no      | —                | —            | —                                                                                           |
| `scripts/__tests__/matrix-rust-p1.6-guardrails.test.mjs`                                                                      | test       | no      | —                | —            | —                                                                                           |
| `scripts/__tests__/synapse-two-client-integration.test.mjs`                                                                   | test       | no      | —                | —            | —                                                                                           |
| `scripts/audit-matrix-public.mjs`                                                                                             | tooling    | no      | —                | —            | —                                                                                           |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/js-sdk-in-matrix-ipc/synara/src/app/features/matrix-ipc/leakyImport.ts`         | tooling    | no      | —                | static       | `matrix-js-sdk`                                                                             |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/js-sdk-new-file/synara/src/app/features/brand-new-migration/NewClientBridge.ts` | tooling    | no      | —                | static       | `matrix-js-sdk`                                                                             |
| `scripts/fixtures/matrix-rust-p1.6/prohibited/raw-matrix-http/synara/src/app/features/matrix-ipc/rawHttp.ts`                  | tooling    | no      | —                | —            | —                                                                                           |
| `synara/scripts/run-synapse-two-client-integration.mjs`                                                                       | tooling    | no      | —                | dynamic      | `matrix-js-sdk`                                                                             |
| `synara/src/app/cs-api.ts`                                                                                                    | production | yes     | app-other        | —            | —                                                                                           |
| `synara/src/app/features/message-search/useMessageSearch.ts`                                                                  | production | yes     | feature          | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/hooks/useCallEmbed.ts`                                                                                        | production | yes     | hook             | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/matrix/__tests__/media.test.ts`                                                                               | test       | yes     | media-boundary   | —            | —                                                                                           |
| `synara/src/app/pages/client/__tests__/syncStatusCopy.test.ts`                                                                | test       | yes     | page             | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/plugins/call/CallEmbed.ts`                                                                                    | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/plugins/call/CallWidgetDriver.ts`                                                                             | production | yes     | plugin           | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/state/__tests__/initMatrix.test.ts`                                                                           | test       | yes     | state            | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/state/__tests__/performLogout.test.ts`                                                                        | test       | yes     | state            | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/state/__tests__/tokenRefresh.test.ts`                                                                         | test       | yes     | state            | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/state/room-list/__tests__/roomActivity.test.ts`                                                               | test       | yes     | state            | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/utils/__tests__/notifications.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/utils/__tests__/remoteContent.test.ts`                                                                        | test       | yes     | utility          | —            | —                                                                                           |
| `synara/src/app/utils/__tests__/syncLifecycle.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/utils/__tests__/syncSplashRecovery.test.ts`                                                                   | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/utils/__tests__/timelineLinks.test.ts`                                                                        | test       | yes     | utility          | static       | `matrix-js-sdk`                                                                             |
| `synara/src/app/utils/__tests__/timelineOpening.test.ts`                                                                      | test       | yes     | utility          | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/@types/event`, `matrix-js-sdk/lib/@types/read_receipts` |
| `synara/src/client/initMatrix.ts`                                                                                             | production | yes     | client-lifecycle | static       | `matrix-js-sdk`, `matrix-js-sdk/lib/http-api/interface`                                     |
| `synara/src/sw.ts`                                                                                                            | production | yes     | service-worker   | —            | —                                                                                           |

## Scope notes

- Repository-wide totals include production, test, and tooling roles.
- Desktop runtime baseline counts only import files under synara/src/ and matches plan §4 (220 production / 12 test).
- Aggregates under aggregates.{production,test,tooling} never mix roles.
- Direct networking uses false-positive-resistant /\_matrix/{client,media,federation,key}/ path literals.
- Generated inventory; no wall-clock timestamps or absolute paths.
- JSON/Markdown artifacts are formatted with Prettier using config resolved from each artifact path (same as the root CLI).
