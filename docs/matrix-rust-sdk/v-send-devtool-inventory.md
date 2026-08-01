# V-SEND.R-DEVTOOL — developer-tools Matrix JS SDK inventory

| Field | Value |
|-------|-------|
| Status | **Docs-only inventory** — no product code changed |
| Measured tip | `a8c5b9d1` (`feature/matrix-rust-sdk-full-replacement`) |
| Scope | `synara/src/app/features/common-settings/developer-tools/` and its SDK-bound hooks |
| Residual | **V-SEND.R-DEVTOOL** |
| Guard | Do not touch `main` or umbrella PR **#39** |

## Finding

The developer-tools feature is still a live `matrix-js-sdk` escape hatch. The
feature has three runtime files: one parent page and two editors. The parent
page does not import the npm package directly, but it passes an SDK `MatrixClient`
through `useMatrixClient()` and reads SDK-backed `Room` state and room account
data. The two editors directly import the SDK's `MatrixError` type and call
SDK client methods for arbitrary event writes.

This is not a product send path. It is a debug/developer surface that can
inspect and write custom room state, room account data, and timeline events.
It remains on the JS client until a native developer-tools bridge can provide
the same intentionally raw behavior with equivalent permission and error
semantics.

## Runtime inventory

| File | SDK-backed reads / listeners | SDK-backed writes / checks | Direct package import | Native-session status |
|------|------------------------------|---------------------------|-----------------------|-----------------------|
| `synara/src/app/features/common-settings/developer-tools/DevelopTools.tsx` | `useRoomState(room)` reads SDK `Room` state and listens for `RoomStateEvent.Events` / `CurrentStateUpdated`; `useRoomAccountData(room)` reads SDK room account data and listens for `RoomEvent.AccountData` | `mx.setRoomAccountData(room.roomId, type, content)` at lines 68–70 | None in this file; indirect through `useMatrixClient`, `useRoomState`, and `useRoomAccountData` | **Reachable** when Developer Tools is enabled; no native devtool gate or bridge |
| `synara/src/app/features/common-settings/developer-tools/SendRoomEvent.tsx` | Active SDK `Room` via `useRoom()` and SDK client via `useMatrixClient()` | `mx.sendStateEvent(...)` for custom state and `mx.sendEvent(...)` for custom timeline events at lines 53–56 | `MatrixError` at line 2 | **Reachable**; arbitrary raw event send remains JS-owned |
| `synara/src/app/features/common-settings/developer-tools/StateEventEditor.tsx` | `useStateEvent(room, ...)` returns SDK state / `MatrixEvent` data and listens through `room.client`; permission calculation uses `mx.getSafeUserId()` | `mx.sendStateEvent(...)` at line 62 | `MatrixError` at line 16 | **Reachable**; arbitrary raw state editing remains JS-owned |
| `synara/src/app/features/common-settings/developer-tools/index.ts` | None | None | None | Barrel only; not a residual owner |

The shared hook evidence is:

- `synara/src/app/hooks/useRoomState.ts:1-53` imports SDK `Room`, event
  types, and `MatrixEvent`, then subscribes directly to SDK room events.
- `synara/src/app/hooks/useRoomAccountData.ts:1-28` imports SDK `Room` and
  reads `room.accountData` while subscribing to `RoomEvent.AccountData`.
- `synara/src/app/hooks/useStateEvent.ts:1-31` imports SDK `Room`, reads SDK
  state events, and subscribes through `room.client`.

## Native boundary

The repository contains typed native account-data owners and a raw-content
extraction foundation, but those are product capabilities rather than a
generic developer-tools API. The source scan found no native command or IPC
surface that exposes arbitrary room event/state reads and writes equivalent to
the three files above. The existing native typed writers therefore do not
close **V-SEND.R-DEVTOOL**.

No implementation is proposed in this inventory. A future implementation must
add an explicit native owner and fail closed on a live native session before
removing these JS-client calls; it must not add a backend selector or a
dual-client fallback.

## Documentation reconciliation

- `docs/matrix-rust-sdk/v-send-residual-inventory.md` previously named only
  `SendRoomEvent.tsx` in the residual summary. This audit expands the runtime
  inventory to the complete developer-tools feature and its indirect SDK
  dependencies.
- `docs/matrix-rust-sdk/desktop-sdk-usage.md` already lists the two direct
  SDK-import files. It does not list `DevelopTools.tsx` because that file's
  coupling is indirect through SDK-backed hooks; this audit records that
  distinction rather than changing the generated usage inventory.
- `docs/matrix-rust-sdk/p1.6-js-sdk-import-allowlist.json` similarly captures
  direct import files, not every feature that receives an SDK client through a
  hook.

## Verification

Production source was inspected with:

```text
rg -n -i 'matrix-js-sdk|useMatrixClient|mx\.|sendEvent|sendStateEvent|setRoomAccountData|getRoomAccountData|StateEvent' \
  synara/src/app/features/common-settings/developer-tools
```

The native-side check searched `src-tauri/src` and the Matrix IPC/DTO paths for
developer-tools, arbitrary raw event/state commands, and account-data bridges.
It found typed account-data and raw-content foundations, but no generic
developer-tools command matching this surface.

**Conclusion:** the V-SEND.R-DEVTOOL inventory is complete for the current
developer-tools feature; implementation remains a separate low-priority
residual.
