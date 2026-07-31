# V-ROOMS.2b — native hierarchy-summary reads

| Field | Value |
| --- | --- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed |
| Owner | Managed Rust client → typed Matrix hierarchy request → Tauri DTO |
| Base | Integration `9579ea4462cfce5b6974ff046c547d090866fc98` |
| Residual | Local space-child graph/listener and mutations remain V-ROOMS.2c |

## Operating path

```text
Lobby or hierarchy summary requests a space level
  → matrix_session_snapshot confirms the managed logged-in owner
  → matrix_space_hierarchy_snapshot(roomId)
  → matrix-sdk Client::send(get_hierarchy::v1::Request)
  → bounded pagination (100 × 50)
  → privacy-safe NativeSpaceHierarchySnapshot
  → lobby room/space cards
```

The command accepts one room ID, rejects invalid IDs, caps pagination at 5,000
rooms, and returns only product summary fields. Raw SDK errors, events, request
tokens, and hierarchy pagination tokens do not cross IPC.

Disqualifying deviations are WebView `MatrixClient.getRoomHierarchy`, direct
`/_matrix/` HTTP, JS fallback after native ownership is selected, or exposing a
raw SDK response through IPC.

## Deletion and accounting

- Deleted both WebView `getRoomHierarchy` call sites.
- Deleted all five `matrix-js-sdk/lib/@types/spaces` imports and replaced
  `IHierarchyRoom` with the native product DTO.
- Production and repository-wide import files remain **187 / 200** because the
  touched UI files still use unrelated JS room/member owners.
- Repository-wide direct import lines decrease **249 → 244**.

## Evidence

- `cargo test --locked matrix::spaces` — 9 passed.
- `npm --prefix synara run typecheck:modernization`.
- `node synara/scripts/run-modernization-tests.mjs`.
- Native-owner tests prove desktop/session gating, command selection, argument
  shape, and DTO readback.

Live proof is **Not confirmed** until an authenticated disposable session opens
a nested public/private space lobby and reads summaries exclusively through the
native command. This does not claim V-ROOMS.2 complete: the local room-state
graph/listener, add/remove/suggest/order writers, and restricted join-rule
coordination remain V-ROOMS.2c.
