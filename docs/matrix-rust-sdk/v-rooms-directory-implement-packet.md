# R-ROOM-DIRECTORY — native public-room directory implementation packet

| Field           | Value                                                                                                                          |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Status          | **Implementation packet** — this packet is docs-only; the directory product vertical is **WIP** and not implemented or accepted |
| Residual        | **V-ROOMS.R-DIRECTORY** from [#383](v-rooms-directory-residual.md)                                                             |
| Base            | `feature/matrix-rust-sdk-full-replacement` at `d82e043db25e4ec786bde103c4d457a898ef664b`                                       |
| PR shape        | Focused **draft** PR targeting `feature/matrix-rust-sdk-full-replacement`                                                      |
| Policy          | Complete UI → Tauri IPC → live `matrix-sdk` owner; superseded JS directory network is deleted in the same implementation slice |
| Desktop failure | **Fail closed** when the native Matrix session, command, response, or generation is unavailable                                |
| Prettier        | `2.8.1`                                                                                                                        |
| Guard           | Never `main`, umbrella PR **#39**, or `product.rs`; #458 is merged at the base and #461 remains a draft candidate pending rebase/acceptance; `dual_backend` and a JS fallback are forbidden |

The source of truth for the measured residual is
[v-rooms-directory-residual.md](v-rooms-directory-residual.md). This packet
turns only the live public-room directory slice into a bounded implementation
contract. It does not close the related preview, card, navigation, or join
proof slices, and it is not a V-BURN completion claim.

> **Parallel WIP note at `d82e043d`.** #446's product-command extraction, #450's
> power/creator READs, and #458's first presence slice are merged, so the
> room-directory product vertical remains module-owned without reopening the
> shared `product.rs` serial owner. Candidate #461 is currently at `5393607e`;
> its inventory/session-wire fix is recorded here as pre-acceptance evidence,
> but the candidate must rebase onto `d82e043d` before `ACCEPT`. This packet
> records no product merge, JS-owner deletion, proof, or acceptance.

## 1. Objective and completion bar

Replace the `/explore/<server>` public-room directory network owner with one
native route:

```text
Explore UI → nativeRoomDirectoryOwner → Tauri IPC → managed matrix-sdk Client → homeserver
```

The retained directory behavior is:

- browse public rooms for a selected server;
- search by the existing generic term;
- filter all rooms, ordinary rooms, or spaces;
- filter by a third-party protocol instance when the selected server exposes
  one;
- preserve the configured page limit and `since`-based previous/next
  pagination;
- show loading, empty, error, and unavailable states without claiming a
  successful result; and
- reject stale results and suppress or cancel obsolete requests when the route
  changes, the component unmounts, or the native session generation changes.

The native implementation must call the typed
`matrix_sdk::Client::public_rooms_filtered` API with a typed Ruma request. It
must not retain `mx.http.authedRequest('/publicRooms')`, use raw Matrix HTTP,
start another Matrix client, or select between native and JS implementations.
The existing high-level `RoomDirectorySearch` API is not the frozen owner for
this slice: at the pinned SDK it does not expose the current room-type,
third-party-network, or previous-page contract. Its compile probe is API-shape
evidence only, not product or network proof.

## 2. Scope and prerequisites

### In scope

The implementation slice owns these paths and boundaries:

- `synara/src/app/pages/client/explore/Server.tsx` — public-room search,
  filters, pagination, and result-page states;
- `synara/src/app/pages/client/explore/Explore.tsx` — remove the unused JS
  public-room probe and obtain the logged-in server identity through the
  existing native session snapshot;
- `synara/src/app/pages/client/explore/nativeRoomDirectoryOwner.ts` — the
  typed, fail-closed frontend owner and request lifecycle;
- `synara/src/app/features/matrix-dto/roomDirectory.ts` — Synara-owned
  directory DTO parsing and bounded validation;
- `src-tauri/src/matrix/room_directory/` — the live typed request/projection
  owner added beside the existing `RoomDirectorySession` foundation;
- `src-tauri/src/lib.rs` — registration of only the exact directory commands
  in Section 3 and their desktop capability wiring, if required by the current
  Tauri setup;
- focused frontend, DTO/IPC, Rust, source-absence, and guardrail evidence; and
- the authenticated disposable-homeserver proof described in Section 6.

The native Rust owner must be module-owned under
`src-tauri/src/matrix/room_directory/` (for example, `live.rs`). This packet
does not authorize any `src-tauri/src/matrix/auth/product.rs` edit. If the
existing managed-session access boundary cannot be reused from the room
directory module, implementation stops and reports that missing authority;
adding a second client or editing `product.rs` is not an allowed workaround.

### Out of scope

- `RoomSummaryLoader.tsx` remote summary resolution for Featured and
  JoinBeforeNavigate;
- `Featured.tsx`, `JoinBeforeNavigate`, and the shared `RoomCard` enrichment,
  Matrix state-event listener, media URL, alias-resolution, and navigation
  residuals;
- `useRoomNavigate.ts` and `useJoinedRoomId.ts` changes for the separate
  `V-ROOMS.R-DIRECTORY-CARD` slice;
- the already-native `joinRoomWithNativeOwner` / `matrix_room_join` writer;
  do not add a second join owner or a JS fallback;
- room directory visibility publishing (`useRoomDirectoryVisibility.ts`);
- Matrix user-directory search, message search, local search, or widget-only
  user search;
- V-BURN preparation or completion;
- any `product.rs` change, generated product command relocation, or broad auth
  refactor;
- any change to `main`, PR `#39`, or unrelated residuals.

Before accepting the #461 product implementation, the writer must verify:

1. the candidate is rebased onto `d82e043db25e4ec786bde103c4d457a898ef664b`
   and the PR target is `feature/matrix-rust-sdk-full-replacement`;
2. the existing managed native session is the sole authenticated Matrix SDK
   client for the desktop session;
3. the current Tauri capability mechanism permits the new module-owned
   commands without adding an alternate invocation path; and
4. the requested command, response, and session-generation contract below is
   unchanged.

A failed prerequisite blocks the slice. It must not be repaired by adding a
fallback, a runtime selector, or a `dual_backend` flag.

## 3. Frozen native IPC contract

These are the only new commands authorized for this slice. The existing
`matrix_session_snapshot` command is the required session preflight and is not
renamed or duplicated.

| Exact command                     | Request                                                                                                 | Required result                                                                                                                                                                   |
| --------------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `matrix_room_directory_protocols` | `{}`                                                                                                    | `{ sessionGeneration, instances }`, where each instance is a bounded `{ protocolId, instanceId, description }` projection; no raw SDK protocol object                             |
| `matrix_room_directory_search`    | `{ sessionGeneration, requestId, serverName?, term?, roomType?, thirdPartyInstanceId?, limit, since? }` | `{ sessionGeneration, requestId, status, page? }`; `status` is `ready`, `stale`, or `cancelled`; `page` is present only for `ready` and has the exact DTO below                   |
| `matrix_room_directory_cancel`    | `{ sessionGeneration, requestId }`                                                                      | `{ sessionGeneration, requestId, status: "cancelled" }`; idempotent for an already-obsolete request in the same generation, stale-generation input is an unavailable/error result |

`requestId` is a bounded monotonically increasing owner correlation ID. The
native session must compare it as an authority value before projecting a page;
completion timing must never decide which result wins. The frontend ignores a
`stale` result and never renders its page. `sessionGeneration` must match the
current native session before and after the SDK request.

### Directory page DTO

The wire boundary is product-owned and uses camelCase. It must not contain
`Room`, `MatrixClient`, Ruma event graphs, access tokens, refresh tokens, raw
homeserver response blobs, media bytes, or SDK error objects.

```text
DirectoryPage {
  sessionGeneration: number
  requestId: number
  chunk: DirectoryRoomHit[]
  prevBatch?: string
  nextBatch?: string
}

DirectoryRoomHit {
  roomId: string
  name?: string
  topic?: string
  canonicalAlias?: string
  avatarUrl?: string       // bounded mxc/product URI only; never bytes
  memberCount: number
  worldReadable: boolean
  guestCanJoin: boolean
  roomType: "room" | "space"
}
```

The directory page DTO remains product-owned camelCase, but the session
preflight consumed by this owner follows the live mixed-case wire: `status`,
`user_id`, `device_id`, `homeserver_url`, and `sessionGeneration`. The #461
candidate's focused tests reject the all-camelCase session shape; do not
normalize these session keys or add a compatibility fallback.

The request bounds are fixed for this slice:

- `serverName`, `term`, and `thirdPartyInstanceId` are trimmed and capped at
  the existing `MAX_TEXT_CHARS` (`256` characters), with empty optional values
  omitted;
- `since` is trimmed, non-empty when present, and capped at `512` characters;
- `limit` is an integer from `1` through `100`; the existing UI presets,
  including `96`, remain valid;
- directory hits remain capped by `MAX_DIRECTORY_HITS` (`200`) and preserve
  the existing alias, topic, room-ID, and avatar-scheme validation; and
- protocol instances, names, descriptions, and identifiers are bounded before
  serialization and are projected to the fields the selector renders.

### SDK mapping and authority rules

- Build `get_public_rooms_filtered::v3::Request` and call the managed
  `Client::public_rooms_filtered`. Set `filter.generic_search_term` from
  `term`, map `roomType: "room"` to the Matrix default-room filter and
  `roomType: "space"` to `m.space`, map `thirdPartyInstanceId` to the typed
  `RoomNetwork::ThirdParty` value, and preserve `limit`, `since`, and the
  server-name query.
- Project each typed `PublicRoomsChunk` into `DirectoryRoomHit`. The room-type
  discriminator is required; an invalid or unsupported room type is a bounded
  projection error, not an ordinary-room guess.
- Build the protocol selector from the typed Matrix third-party-protocol
  request through the managed client. Do not call a raw endpoint or return an
  arbitrary protocol map to the webview.
- The managed session and its generation are the only authority. A missing
  session, missing client, failed SDK request, failed projection, failed
  cancellation, or generation mismatch returns a safe unavailable/error
  result. There is no JS retry path.
- Native errors use the repository's privacy-safe Matrix IPC error categories
  and diagnostic IDs. Error text and logs must never contain tokens, secrets,
  credentials, arbitrary raw response content, or unbounded room metadata.

### Pinned upstream evidence

The approved SDK evidence is Matrix Rust SDK `0.18.0`, tag commit
`1c44fb66214667c6d00acaf72ab592493653708b`:

- [`Client::public_rooms_filtered`](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/client/mod.rs#L1884-L1920)
  proves the typed public-room request wrapper;
- [`get_public_rooms_filtered` request/response](https://github.com/ruma/ruma/blob/62a3c56c53c1f9d45f030e6980804691e79ebaec/crates/ruma-client-api/src/directory/get_public_rooms_filtered.rs#L28-L73)
  proves typed room filters, third-party network selection, `next_batch`, and
  `prev_batch` fields;
- [`RoomDirectorySearch`](https://github.com/matrix-org/matrix-rust-sdk/blob/1c44fb66214667c6d00acaf72ab592493653708b/crates/matrix-sdk/src/room_directory_search.rs#L114-L189)
  remains compile-probed evidence, but is explicitly not sufficient for the
  complete product contract in this packet; and
- the repository's [0.18.0 source provenance](0.18.0-source-provenance.md)
  remains the release and feature-resolution authority.

These links establish API shape only. They do not establish a live homeserver,
product, privacy, or UI result.

## 4. Physical JS-owner deletion and route boundary

The implementation PR must remove the directory-specific JS owner in the same
slice. Shared card behavior is a named later residual and must not be mixed in
as incidental cleanup.

| Path                                                              | Remove from this slice                                                                                                                                                                               | Retain or replace with                                                                                                                            |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/pages/client/explore/Server.tsx`                  | `MatrixClient`, `Method`, `RoomType` directory imports; `useMatrixClient` for directory identity/protocols; `mx.http.authedRequest`; the raw `/publicRooms` request; and `mx.getThirdpartyProtocols` | `nativeRoomDirectoryOwner`, parsed `DirectoryPage`, native protocol instances, native session identity, and the existing result-page presentation |
| `synara/src/app/pages/client/explore/Explore.tsx`                 | `useMatrixClient`, the unused `mx.publicRooms({ server, limit: 1 })` probe, and JS-derived user-server identity                                                                                      | Existing navigation/config UI backed by `matrix_session_snapshot`; no speculative native probe in the Add Server submit path                      |
| `synara/src/app/pages/client/explore/nativeRoomDirectoryOwner.ts` | Any `legacy`, `js`, `fallback`, or `isNative ? rust : js` branch                                                                                                                                     | One injected-invoke owner that preflights, validates, invokes, correlates, cancels, and rejects unavailable native results                        |
| `synara/src/app/features/matrix-dto/roomDirectory.ts`             | SDK/Ruma result typing at the UI boundary                                                                                                                                                            | Strict parsing of the frozen camelCase DTO and rejection of unknown/missing/wrong-shaped fields                                                   |

The shared `RoomCard` may continue to receive the bounded fields from this
page while `V-ROOMS.R-DIRECTORY-CARD` remains open. This packet must not add a
second directory fetch, preserve the removed directory HTTP owner through a
hidden helper, or claim that the shared card/preview residual is closed.

The implementation must include a negative source scan over the changed
Explore directory route proving absence of `matrix-js-sdk`, `useMatrixClient`,
`mx.http`, `mx.publicRooms`, `mx.getThirdpartyProtocols`,
`authedRequest`, `Method.Post`, and raw `/publicRooms` calls. The scan is
route-scoped; unrelated open Matrix JS owners elsewhere in the repository are
not silently reclassified as this residual.

## 5. Required focused tests

Test names may be expanded, but the following cases and boundaries are
required.

### Frontend owner tests

Add
`synara/src/app/pages/client/explore/__tests__/nativeRoomDirectoryOwner.test.ts`
with an injected invoke harness. Assert exact command names and arguments for:

1. logged-out, non-desktop, missing-command, and unavailable-session
   preflight states;
2. protocol loading and bounded instance projection;
3. browse, term search, all/room/space mapping, third-party instance mapping,
   custom limit, and previous/next `since` tokens;
4. request correlation, stale-result suppression, route-change cancellation,
   unmount cleanup, and generation mismatch;
5. malformed DTOs, missing `chunk`, invalid room type, invalid pagination token,
   and oversized values becoming visible unavailable/error states; and
6. every native failure making zero calls to `mx.http`, `mx.publicRooms`,
   `mx.getThirdpartyProtocols`, or any other legacy JS directory owner. The
   owner must reject or expose unavailable state, never return a `legacy`
   sentinel.

Add
`synara/src/app/pages/client/explore/__tests__/roomDirectorySourceGuard.test.ts`
or an equivalent repository guard. It must scan `Server.tsx` and `Explore.tsx`
for the route-scoped negative requirements in Section 4 and must not assert
that unrelated Matrix JS imports have reached zero.

### DTO and IPC contract tests

Extend the existing focused suites:

- `synara/src/app/features/matrix-dto/__tests__/matrixDto.test.ts` for strict
  directory page/hit/protocol parsing, room/space discriminators, bounds,
  and rejection of raw or secret-bearing fields;
- `synara/src/app/features/matrix-ipc/__tests__/matrixIpcContract.test.ts`
  for command arguments/results, generation and request IDs, stale/cancelled
  statuses, unknown fields, missing fields, and privacy-safe errors; and
- `src-tauri/src/matrix/ipc/contract_tests.rs` for the matching wire-level
  envelope validation where the shared IPC contract applies.

### Rust room-directory tests

Extend `src-tauri/src/matrix/room_directory/tests.rs` and keep the tests next
to the live module. Cover:

- typed request construction for server, term, default-room, space, and
  third-party filters;
- `next_batch`/`prev_batch` projection and replacement semantics;
- required room-type projection and bounded hit/name/topic/alias/avatar
  validation;
- request-ID stale rejection, cancellation, session-generation retirement,
  concurrent-result suppression, and idempotent cancellation;
- missing session/client, SDK error, malformed response, and unavailable
  command behavior as fail-closed errors; and
- diagnostic/error strings that contain neither credentials nor raw room
  response content.

Do not run a full workspace build merely to author or review this packet. The
future implementation PR should run the focused Rust room-directory filter,
the focused frontend owner/DTO/IPC tests, the route source guard, and the
existing Matrix guardrails. `npm ci` and a full workspace build are not
required unless a targeted failure proves one essential.

## 6. Authenticated disposable-homeserver proof

After the focused automated checks pass, record a two-client proof using the
repository's [test-matrix-synapse-topology.md](test-matrix-synapse-topology.md)
topology with a populated public directory. The proof must name the exact
native commands and remain marked **not run**, **failed**, or **passed** with
retained evidence.

1. Log in through the existing native session and open Explore for a server.
   Confirm `matrix_session_snapshot` and
   `matrix_room_directory_search` return a generation-stamped page.
2. Open the protocol selector, if the topology exposes a third-party instance.
   Confirm `matrix_room_directory_protocols` is the only protocol read and
   that the UI renders only bounded instances. If the topology has no
   third-party fixture, record that limitation; it cannot be called a pass for
   that filter.
3. Browse without a term, search for a known room, switch all/room/space
   filters, change the limit, and paginate both directions. Confirm every
   request carries the expected typed arguments and every result is rendered
   from the returned native DTO.
4. Trigger a second search before the first response completes and navigate
   away or unmount during an in-flight search. Confirm the old page is not
   rendered, cancellation is attempted when possible, and no late result
   changes the new route.
5. End the native session or make the command unavailable. Confirm the route
   shows an unavailable/error state and never calls the JS Matrix client or
   claims an empty page as success.
6. Capture the network/command trace showing no raw JS `/publicRooms` request.
   Do not use the existing native join command as proof of this read slice;
   join ownership and its live proof remain `V-ROOMS.R-JOIN-PROOF`.

This proof establishes only `V-ROOMS.R-DIRECTORY`. It does not prove preview
resolution, card projection/navigation, join mutation, V-BURN, or PR `#39`.

## 7. Ordered implementation work

1. Reconfirm the exact base SHA, integration target, managed-session authority,
   and the no-`product.rs` boundary. Stop on any mismatch.
2. Freeze the Synara-owned DTOs, request bounds, request/generation correlation,
   and exact command names in Section 3. Do not add aliases.
3. Add the module-owned live room-directory request/projection path beside the
   existing session foundation. Use the typed Matrix SDK/Ruma requests and the
   managed authenticated client; keep secrets and SDK objects inside Rust.
4. Register only the three directory commands in the existing Tauri handler and
   capability surface. Do not add a product.rs command or a second session
   accessor/client.
5. Implement the frontend owner and strict DTO parser, then rewire Server and
   Explore. Preserve the existing UI states and route parameters while making
   native failure terminal for this desktop route.
6. Physically remove the route-scoped JS directory calls and the unused Add
   Server probe. Do not modify the separate preview/card/join residuals.
7. Run the focused frontend, DTO/IPC, Rust, source-absence, Prettier `2.8.1`,
   and Matrix guardrail checks. Then run and record the disposable-homeserver
   proof.
8. Review the diff for accidental `product.rs`, `main`, `#39`, V-BURN,
   unrelated-vertical, fallback, selector, retry-as-proof, or `dual_backend`
   changes. Keep the PR draft and focused.

## 8. Acceptance statement

`V-ROOMS.R-DIRECTORY` may be marked complete only when:

- Explore public-room browse, search, room/space filtering, third-party
  instance filtering, and both pagination directions use the exact native
  command contract in Section 3;
- the native owner calls the typed Matrix SDK public-room request with the
  managed authenticated session and projects only bounded Synara DTOs;
- stale results, cancellation, generation changes, malformed responses, and
  every native availability failure are fail-closed, with no JS fallback or
  successful empty-page substitution;
- the route-scoped JS directory network owner and dead probe are physically
  absent, and the source guard proves their absence;
- focused automated checks and the authenticated proof pass, with any missing
  third-party fixture explicitly recorded rather than silently waived;
- no `product.rs` file change, second Matrix client, SDK selector, raw Matrix
  HTTP, token, credential, or unbounded room payload is introduced; and
- the residual ledger still names `V-ROOMS.R-EXPLORE-PREVIEW`,
  `V-ROOMS.R-DIRECTORY-CARD`, and `V-ROOMS.R-JOIN-PROOF` as separate work.

This statement applies only to `V-ROOMS.R-DIRECTORY`. It must not be used to
claim the entire Explore residual, V-BURN, or PR `#39` complete, and it never
authorizes a merge to `main`.
