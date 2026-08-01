# V-ROOMS — room directory / Explore residual inventory

| Field            | Value                                                                          |
| ---------------- | ------------------------------------------------------------------------------ |
| Status           | **Inventory only** — no product implementation in this document                |
| Measured tip     | `8330c56bbb74b45ef01ad1f8be137b54caa2f568`                                     |
| Base             | `feature/matrix-rust-sdk-full-replacement`                                     |
| Policy           | Full vertical: UI → Tauri IPC → live Matrix Rust SDK; one owner per capability |
| Desktop fallback | **Fail closed** when native Matrix IPC is unavailable                          |
| Dual backend     | **Forbidden**                                                                  |

This is the residual map for the desktop `/explore/` route, server-scoped public
room discovery, featured-room previews, and the public-room join path. It does
not claim V-BURN completion, a cutover, or live authenticated proof.

## Current ownership

The current route has three different ownership states. The directory read is
still a JS-client path; the join mutation is already native; and the card,
summary, and navigation helpers still use JS-client room objects around those
two paths.

| Capability                                            | Current owner and evidence                                                                                                                                                                                           | Residual state                                                             | Native coverage today                                                                                                                                 |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Public-room browse, search, filtering, and pagination | `synara/src/app/pages/client/explore/Server.tsx`: `fetchPublicRooms` calls `mx.http.authedRequest` with `POST /publicRooms`; query parameters carry `server`, `limit`, `since`, `term`, `type`, and `instance`       | **Open — live JS network owner**                                           | `src-tauri/src/matrix/room_directory/` is only a pure session/projection harness; it has no live directory request, IPC command, or frontend consumer |
| Add-server Explore entry                              | `synara/src/app/pages/client/explore/Explore.tsx`: an unused `mx.publicRooms({ server, limit: 1 })` callback remains; the submit action only navigates to the server route                                           | **Open — dead JS probe / cleanup residual**                                | No native probe; the server route is the real fetch owner                                                                                             |
| Featured-room and join-preview summaries              | `synara/src/app/components/RoomSummaryLoader.tsx`: `mx.getRoomSummary(roomIdOrAlias)`; used by `explore/Featured.tsx` and `features/join-before-navigate/JoinBeforeNavigate.tsx`                                     | **Open — JS summary reader**                                               | Native space-hierarchy summary exists for a different path, but no native public-room preview/resolve owner is wired here                             |
| Directory-card room enrichment and route resolution   | `RoomCard.tsx` uses `mx.getRoom`, Matrix state-event listeners, Matrix media URL resolution, and `useJoinedRoomId`; `useRoomNavigate.ts` and `useJoinedRoomId.ts` resolve aliases/room objects through the JS client | **Open — shared UI adjacency**                                             | Directory hit data is not projected into an SDK-neutral card model; native directory results are not the source of truth for these decisions          |
| Public-room join mutation                             | `RoomCard.tsx` calls `joinRoomWithNativeOwner`; that owner checks the native session and invokes `matrix_room_join` without a JS fallback                                                                            | **Closed for mutation ownership**; live end-to-end proof remains unclaimed | Registered native command and focused owner tests are present; this row must not be reopened by the directory work                                    |

The `LocalRoomSummaryLoader` branch used by joined-room lobby items is not part
of this directory residual. The residual summary row is limited to the remote
`getRoomSummary` branch used by Explore and the join-before-navigate preview.

## Native foundation already present

`src-tauri/src/matrix/room_directory/` contains `RoomDirectorySession` and the
privacy-safe `DirectoryRoomHit` projection. The foundation currently provides:

- bounded query, server-name, alias, topic, and hit sizes;
- request-id stale-result rejection and session-generation retirement;
- result replacement, room-id deduplication, next-page state, cancellation,
  and privacy-safe diagnostic identifiers; and
- room metadata fields for id, name, topic, canonical alias, avatar URI, member
  count, world-readable, and guest-join flags.

It intentionally does **not** provide the product path. In particular, the
existing foundation has no live `matrix-sdk` directory request, no typed
Tauri command, no typed TypeScript owner, and no `/explore/` binding. The
directory hit projection also needs a room-type field (or an equivalent typed
space discriminator) before it can replace `Server.tsx`'s `room_type` routing.

## Proposed native slices

These are separate slices so a directory read does not silently absorb the
summary/card/navigation residuals, and so the already-native join writer stays
the sole mutation owner.

| Proposed slice                                                       | UI → IPC → native path                                                                                                                                                                                                     | JS deletion / re-home                                                                                                                                                                                    | Done when                                                                                                                                                                                                                                                                             |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-ROOMS.R-DIRECTORY** — live public-room directory                 | `Server.tsx` search/filter/page actions → typed `nativeRoomDirectoryOwner` → a native room-directory command family → authenticated Matrix Rust SDK public-room directory search → bounded `RoomDirectorySession` page DTO | Remove `mx.http.authedRequest`, the `MatrixClient`/`Method` directory imports, and the raw `/publicRooms` request from `Server.tsx`; remove the unused `mx.publicRooms` probe from `Explore.tsx`         | Browse, term search, room/space filter, third-party instance filter, previous/next pagination, empty/error/loading states, stale-result rejection, and cancellation all use the native page contract; no raw JS Matrix HTTP remains                                                   |
| **V-ROOMS.R-EXPLORE-PREVIEW** — featured and join-preview resolution | Featured / join-before-navigate summary requests → typed preview owner → native room-alias/summary resolution using the live native client → SDK-neutral preview DTO                                                       | Re-home the remote `RoomSummaryLoader` branch and its `MatrixClient`-derived result type; keep the unrelated local summary branch separate                                                               | Featured cards and the pre-join card render the same bounded metadata for room ids and aliases, and fail closed if the native preview is unavailable                                                                                                                                  |
| **V-ROOMS.R-DIRECTORY-CARD** — card projection and navigation        | Native directory/preview DTO → SDK-neutral `RoomCard` model → route resolution from returned room id/type/alias; media remains an opaque native handle or approved URI                                                     | Remove directory-path dependence on `mx.getRoom`, Matrix state-event listeners, JS alias resolution, and JS Matrix media URL construction; do not broaden this slice to every shared `RoomCard` consumer | A public-room card decides joined/view versus join from native projections, routes a room versus space from typed data, and renders metadata without a JS-client room object                                                                                                          |
| **V-ROOMS.R-JOIN-PROOF** — verify the existing join vertical         | Directory-card Join → existing `joinRoomWithNativeOwner` → `matrix_room_join` → live native SDK membership mutation → native room-list projection → UI refresh                                                             | No new JS join writer; do not add a JS fallback or a second join owner                                                                                                                                   | An authenticated desktop test proves a public-directory result joins by id/alias, the card leaves the fail-closed/error states correctly, and the joined room becomes available to the selected native projection. This is proof of the existing owner, not a new join implementation |

## Proposed directory contract

The first slice should carry product-owned DTOs, not SDK or Ruma object graphs:

```text
DirectorySearchRequest {
  serverName?: string
  term?: string
  roomType?: "room" | "space"
  thirdPartyInstanceId?: string
  limit: number
  since?: string
}

DirectoryPage {
  chunk: DirectoryRoomHit[]
  prevBatch?: string
  nextBatch?: string
}
```

`DirectoryRoomHit` should retain the bounded fields already modeled by
`RoomDirectorySession`, add the product-required room/space discriminator, and
never carry access tokens, refresh tokens, event plaintext, media bytes, or
unbounded raw server content. `since`/batch values are directory pagination
state, not credentials.

The intended operating path is:

```text
Explore UI
  → nativeRoomDirectoryOwner
  → Tauri matrix_room_directory_* IPC
  → live Matrix Rust SDK directory request
  → RoomDirectorySession stale/bounds projection
  → DirectoryPage DTO
  → Explore result grid
  → existing nativeRoomJoinOwner for Join
```

The native request must use the authenticated native client and its typed
room-directory API. Retaining `mx.http.authedRequest('/publicRooms')`, adding a
JS/native selector, or keeping a desktop JS fallback fails this slice.

## Acceptance and proof queue

Implementation acceptance should remain targeted because this document is not
a V-BURN report:

1. Unit-test request bounds, room-type projection, replacement/append dedup,
   stale request ids, cancellation, and generation retirement in
   `matrix::room_directory`.
2. Add focused TypeScript tests for native directory availability, typed page
   parsing, stale-result handling, and fail-closed behavior.
3. Run the directory Rust test filter, focused TS tests, and Matrix guardrails;
   avoid `npm ci` and full workspace builds unless a later implementation
   requires them.
4. Use an authenticated disposable homeserver for the live proof: browse a
   public directory, search, change room/space filters, paginate, join a result,
   and verify the native room projection updates. Record that proof in the
   owning implementation slice; it is not claimed here.

## Explicit non-goals

- No product implementation or `product.rs` change in this inventory.
- No raw Matrix HTTP retention in the eventual native slice.
- No dual backend, SDK selector, or desktop JS fallback.
- No umbrella `#39` / `main` merge.
- No V-BURN completion claim.
