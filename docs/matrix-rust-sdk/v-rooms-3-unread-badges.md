# V-ROOMS.3 — native unread map for list/nav badges

| Field  | Value                                                                    |
| ------ | ------------------------------------------------------------------------ |
| Status | Implementation candidate; live authenticated runtime proof unclaimed     |
| Owner  | Managed Rust room-list snapshot → Synara unread atom                     |
| Queue  | `V-ROOMS.3`                                                              |
| Policy | Complete native replacement of list unread ownership; no JS SDK fallback |

## Retained product contract

Room-list nav badges, space parent rollup totals (via existing parent map), and
platform/tray badge unread counts must continue to reflect joined-room unread,
highlight, and marked-unread state. Muted rooms and spaces stay out of the
unread map. A native list that drops mute filtering or marked-unread presence is
not a completed V-ROOMS.3 replacement.

## Operating path

```text
Desktop session logged in
  → matrix_session_snapshot (logged_in)
  → matrix_room_list_snapshot (sole joined-room + unread owner)
  → native RoomSummary{unreadCount,highlightCount,markedUnread,notificationMode,isSpace}
  → roomToUnreadAtom RESET + parent rollup from roomToParentsAtom
  → RoomNavItem / Home / Direct / platform badge consumers
```

Disqualifying deviations: binding `matrix-js-sdk` Room Timeline/Receipt listeners
for list unread; falling back to `MatrixClient.getRooms()` for the joined-room
id list; selecting `NativeTimelinePresenter` or deleting `RoomTimeline.tsx`.

## Parity decisions

- Project `notification_mode` from the SDK cache, falling back to
  `Room::notification_mode()` so mute rooms are excluded from badges.
- Project `is_space` so space rooms are excluded, matching the previous
  `isSpaceRoom()` filter.
- Keep parent-space rollup against `roomToParentsAtom` (still JS-owned under
  **V-ROOMS.2**). Hierarchy changes re-roll unread when parents or the native
  snapshot change.
- Mark-as-read / mark-as-unread menu writes remain on later
  V-TIMELINE/V-ROOMS surfaces; this slice owns the unread **map** that drives
  badges after native counts move.

## Deletion

- Removed the JS room-list dual-backend binder from
  `synara/src/app/state/room-list/roomList.ts`.
- Removed JS MatrixClient unread listeners and imports from
  `synara/src/app/state/room/roomToUnread.ts` and its unit test.
- Dropped both paths from `p1.6-js-sdk-import-allowlist.json`.

## Inventory

From integration tip after V-ROOMS.1 (`2c48fd4`, production **194** /
repository-wide **208**):

- desktop-runtime production import files **194 → 192**
- desktop-runtime test import files **11 → 10**
- repository-wide import files **208 → 205**
- allowlist **201 → 199**

## Evidence

- `cargo test --locked room_list::live` (notification-mode mapping + counts)
- `node synara/scripts/run-modernization-tests.mjs` including native unread
  projection tests
- `npm run check:matrix-rust-guardrails`
- Regenerated `desktop-sdk-usage.{json,md}`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows nav/platform badges tracking native unread/mute/marked-unread
transitions without JS room listeners.
