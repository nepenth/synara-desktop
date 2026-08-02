# V-ROOMS — room directory / Explore residual inventory

| Field            | Value                                                                          |
| ---------------- | ------------------------------------------------------------------------------ |
| Status           | **First directory slice merged (#461); docs-only residual inventory remains open** |
| Measured tip     | `c1e9c3be2b8ff13da42853913b30493cb030e6ec`                                     |
| Base             | `feature/matrix-rust-sdk-full-replacement`                                     |
| Policy           | Full vertical: UI → Tauri IPC → live Matrix Rust SDK; one owner per capability |
| Desktop fallback | **Fail closed** when native Matrix IPC is unavailable                          |
| Dual backend     | **Forbidden**                                                                  |
| V-BURN           | **HOLD** — no preparation or completion claim                                  |

This is the residual map for the desktop `/explore/` route, server-scoped public
room discovery, featured-room previews, and the public-room join path. It does
not claim V-BURN completion, a cutover, or live authenticated proof.

> **Post-merge note at `c1e9c3be`.** #461 is merged. Its first slice lands the
> native public-room browse/search/filter/pagination route, bounded protocol and
> page DTOs, route-scoped JS-owner deletion, focused evidence, and green CI.
> Authenticated live proof and independent acceptance remain **Not confirmed**;
> this residual remains open for those gates and for the separate preview/card/
> join residuals.

## Current ownership

The current route has three different ownership states. The directory read is
now native-owned after #461; the join mutation was already native; and the card,
summary, and navigation helpers still use JS-client room objects around those
two paths.

| Capability                                            | Current owner and evidence                                                                                                                                                                                           | Residual state                                                             | Native coverage today                                                                                                                                 |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Public-room browse, search, filtering, and pagination | `Server.tsx` → `nativeRoomDirectoryOwner` → `matrix_room_directory_search` → typed Matrix SDK `public_rooms_filtered` → bounded `DirectoryPage`; `matrix_room_directory_protocols` owns the third-party selector | **Implementation merged in #461; live proof and acceptance remain Not confirmed** | Native owner now carries server/term/room-space/third-party filters, both pagination tokens, request/session correlation, stale suppression, cancellation, and fail-closed errors; no route-scoped JS Matrix network owner remains |
| Add-server Explore entry                              | `Explore.tsx` reads `matrix_session_snapshot` through `readNativeRoomDirectorySession`; the former unused `mx.publicRooms({ server, limit: 1 })` probe is deleted and Add Server only navigates to the server route | **Closed in #461; no separate directory probe**                                | Native session identity supplies the signed-in server for the protocol selector; unavailable state remains fail-closed                                                                                             |
| Featured-room and join-preview summaries              | `synara/src/app/components/RoomSummaryLoader.tsx`: `mx.getRoomSummary(roomIdOrAlias)`; used by `explore/Featured.tsx` and `features/join-before-navigate/JoinBeforeNavigate.tsx`                                     | **Open — JS summary reader**                                               | Native space-hierarchy summary exists for a different path, but no native public-room preview/resolve owner is wired here                             |
| Directory-card room enrichment and route resolution   | `RoomCard.tsx` still uses `mx.getRoom`, Matrix state-event listeners, Matrix media URL resolution, and `useJoinedRoomId`; `useRoomNavigate.ts` and `useJoinedRoomId.ts` resolve aliases/room objects through the JS client | **Open — shared UI adjacency**                                             | #461 supplies bounded native hit fields and room/space routing to the existing card; shared card enrichment, joined-room resolution, and media/navigation ownership remain separate residual work |
| Public-room join mutation                             | `RoomCard.tsx` calls `joinRoomWithNativeOwner`; that owner checks the native session and invokes `matrix_room_join` without a JS fallback                                                                            | **Closed for mutation ownership**; live end-to-end proof remains unclaimed | Registered native command and focused owner tests are present; this row must not be reopened by the directory work                                    |

The `LocalRoomSummaryLoader` branch used by joined-room lobby items is not part
of this directory residual. The residual summary row is limited to the remote
`getRoomSummary` branch used by Explore and the join-before-navigate preview.

## Native foundation already present

`src-tauri/src/matrix/room_directory/` now contains the landed
`RoomDirectorySession`, live typed request owner, and privacy-safe
`DirectoryRoomHit` projection. The #461 implementation provides:

- bounded query, server-name, alias, topic, and hit sizes;
- request-id stale-result rejection and session-generation retirement;
- result replacement, room-id deduplication, next-page state, cancellation,
  and privacy-safe diagnostic identifiers; and
- room metadata fields for id, name, topic, canonical alias, avatar URI, member
  count, world-readable, guest-join flags, and the required room/space
  discriminator; and
- the exact native command family, typed TypeScript owner, strict DTO parser,
  `/explore/` binding, route-scoped source guard, and fail-closed availability
  boundary.

The landed slice intentionally does **not** absorb the preview, shared-card,
navigation, or join-proof residuals. Those remain separate ownership and proof
boundaries.

## Landed slice and remaining native slices

These are separate slices so a directory read does not silently absorb the
summary/card/navigation residuals, and so the already-native join writer stays
the sole mutation owner.

| Proposed slice                                                       | UI → IPC → native path                                                                                                                                                                                                     | JS deletion / re-home                                                                                                                                                                                    | Done when                                                                                                                                                                                                                                                                             |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-ROOMS.R-DIRECTORY** — live public-room directory                 | `Server.tsx` search/filter/page actions → typed `nativeRoomDirectoryOwner` → `matrix_room_directory_*` → authenticated Matrix Rust SDK public-room directory search → bounded `RoomDirectorySession` page DTO | #461 removes `mx.http.authedRequest`, the `MatrixClient`/`Method` directory imports, raw `/publicRooms`, and the unused `mx.publicRooms` probe from the route | **Implemented and merged in #461**: browse, term search, room/space filter, third-party instance filter, previous/next pagination, empty/error/loading states, stale-result rejection, and cancellation use the native page contract; live proof and independent acceptance remain **Not confirmed** |
| **V-ROOMS.R-EXPLORE-PREVIEW** — featured and join-preview resolution | Featured / join-before-navigate summary requests → typed preview owner → native room-alias/summary resolution using the live native client → SDK-neutral preview DTO                                                       | Re-home the remote `RoomSummaryLoader` branch and its `MatrixClient`-derived result type; keep the unrelated local summary branch separate                                                               | Featured cards and the pre-join card render the same bounded metadata for room ids and aliases, and fail closed if the native preview is unavailable                                                                                                                                  |
| **V-ROOMS.R-DIRECTORY-CARD** — card projection and navigation        | Native directory/preview DTO → SDK-neutral `RoomCard` model → route resolution from returned room id/type/alias; media remains an opaque native handle or approved URI                                                     | Remove directory-path dependence on `mx.getRoom`, Matrix state-event listeners, JS alias resolution, and JS Matrix media URL construction; do not broaden this slice to every shared `RoomCard` consumer | A public-room card decides joined/view versus join from native projections, routes a room versus space from typed data, and renders metadata without a JS-client room object                                                                                                          |
| **V-ROOMS.R-JOIN-PROOF** — verify the existing join vertical         | Directory-card Join → existing `joinRoomWithNativeOwner` → `matrix_room_join` → live native SDK membership mutation → native room-list projection → UI refresh                                                             | No new JS join writer; do not add a JS fallback or a second join owner                                                                                                                                   | An authenticated desktop test proves a public-directory result joins by id/alias, the card leaves the fail-closed/error states correctly, and the joined room becomes available to the selected native projection. This is proof of the existing owner, not a new join implementation |

## Proposed directory contract

The landed first slice carries product-owned DTOs, not SDK or Ruma object graphs:

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

Implementation acceptance remains targeted because this document is not a
V-BURN report. #461 is merged at `c1e9c3be`; the checks and route source guard
are implementation evidence, not a substitute for authenticated product proof:

1. Retain the landed Rust, TypeScript, DTO, source-absence, and Matrix
   guardrail evidence; #461's CI was green before merge.
2. Use an authenticated disposable homeserver for the directory live proof:
   browse a public directory, search, change room/space filters, paginate in
   both directions, and verify the native page projection. Record exact command
   and result readback; this proof is **Not confirmed** here.
3. Keep preview resolution, shared card/navigation behavior, and the existing
   native join mutation as separate residual/proof slices. Do not relabel their
   closure from the directory merge.

## Explicit non-goals

- No further product implementation or `product.rs` change is made by this
  inventory; #461's merged implementation is recorded, not changed here.
- No claim that focused automated checks substitute for authenticated live
  directory proof or independent acceptance.
- No raw Matrix HTTP retention in the eventual native slice.
- No claim that #461's merge closes live proof, independent acceptance, preview,
  card/navigation, or join proof.
- No dual backend, SDK selector, or desktop JS fallback.
- No umbrella `#39` / `main` merge.
- No V-BURN completion claim.
