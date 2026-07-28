# Client feature / functionality graph

| Field | Value |
| --- | --- |
| Status | **Living map** (migration + post-cutover re-review) |
| Date | 2026-07-28 |
| Machine twin | [`client-feature-graph.json`](client-feature-graph.json) |
| Related | [`desktop-sdk-usage.md`](desktop-sdk-usage.md), [`feature-parity-traceability.md`](feature-parity-traceability.md), [`0.18.0-feature-and-gap-analysis.md`](0.18.0-feature-and-gap-analysis.md), [`cutover-operating-model.md`](cutover-operating-model.md), [`program-status.md`](program-status.md) |

## Why this exists

We already have:

- **JS usage inventory** (which files/symbols import `matrix-js-sdk`)
- **FR parity matrix** (requirements → evidence)
- **Rust SDK 0.18 feature/gap dossier** (upstream capabilities)

What was missing is a **client-perspective feature graph**: user-visible capabilities → current JS surface → intended Rust owner → plan tasks → hard problems (scroll, read state, sync, etc.).

Use this:

1. **During migration** — pick slices and keep implementation aligned with how matrix-rust-sdk expects clients to work.
2. **After cutover** — re-review each node for “are we fighting the SDK or following it?”
3. **Parallel text work** — MiniMax-M3 can expand surfaces, FR links, and checklists while Grok implements (see [`minimax-parallel-work.md`](minimax-parallel-work.md)).

This graph does **not** claim product cutover. Runtime remains **js-sdk only** until atomic sole-owner cutover.

## Delivery status vocabulary

| Status | Meaning |
| --- | --- |
| `not_started` | No harness/product work yet |
| `harness_foundation` | Rust foundation on integration branch; product still js-sdk |
| `harness_live_partial` | Some live SDK wiring in harness |
| `product_cutover` | UI → IPC only; Rust sole owner for this capability |
| `burned_down` | js-sdk usage removed for this capability |

## Capability map (summary)

| Node | Client feature | Plan tasks | Status | Hard problems |
| --- | --- | --- | --- | --- |
| `session.discovery_login` | Discovery + login | P3.1–P3.2 | harness_foundation | — |
| `session.persist_restore` | Persist + restore | P3.5–P3.6, P2.2 | harness_foundation | crypto_identity, multi_device |
| `session.logout_legacy` | Logout / legacy | P2.6, P3.7–P3.8 | harness_foundation | crypto_identity |
| `sync.readiness` | Sync / reconnect | P4.1, P2.1, P2.4 | harness_foundation | sync_freshness |
| `rooms.list` | Room list / tags / spaces / members | P4.2–P4.6 | harness_foundation | sync_freshness |
| `timeline.read_window` | Timeline window / pagination / focus | P5.1–P5.5 | harness_foundation | **scroll_position, large_room, streaming_append, sync_freshness** |
| `timeline.relations_threads` | Reactions / threads | P5.6–P5.9 | harness_foundation | streaming_append |
| `composer.send_echo` | Send + local echo | P6.1, P6.5 | harness_foundation | streaming_append, sync_freshness |
| `timeline.read_state` | Receipts / unread | P6.2, P4.3, P7.1 | harness_foundation | **read_state, multi_device, sync_freshness** |
| `presence.typing` | Typing | P6.3 | harness_foundation | — |
| `media.upload_download` | Media | P6.4, P7.2–P7.7 | harness_foundation | media_lifecycle |
| `search.room_messages` | Search | P6.8, P12.6 | harness_foundation | — |
| `crypto.devices_verification` | Devices / SAS / cross-signing / backup | P8.1–P8.8 | harness_foundation | crypto_identity, multi_device |
| `calls.widgets` | Widgets / Element Call | P9.1, P10.* | harness_foundation | — |
| `nav.routes` | Deep links | P4.8 | harness_foundation | scroll_position |
| `cutover.sole_owner` | Sole-owner cutover + burn-down | P11.*, P14.1 | **not_started** | crypto_identity, sync_freshness |

Full field-level detail (JS symbols, Rust surfaces, modules, notes) lives in the JSON twin.

## Hard problems (client pain → SDK alignment)

These are areas where past client bugs clustered. When implementing or re-reviewing, prefer **SDK-native models** over re-creating js-sdk TimelineWindow / dual-store patterns.

### `scroll_position` / `large_room` / `streaming_append`

- **Pain:** large rooms, keep position while paginating, live append + local echo, jump-to-latest, focused-event open.
- **Align with:** `matrix-sdk-ui` Timeline item identity, back-pagination status, and diff streams — UI owns viewport math only.
- **Nodes:** `timeline.read_window`, `composer.send_echo`, `nav.routes`.

### `read_state` / multi-device receipts

- **Pain:** read markers and unread badges drift across devices/restarts.
- **Align with:** SDK receipts + RoomInfo unread; single write path for markers.
- **Nodes:** `timeline.read_state`, `rooms.list`.

### `sync_freshness`

- **Pain:** two owners of “what’s live” race (interactive sync vs background).
- **Align with:** one Rust `SyncService` / room-list owner; no concurrent JS client.
- **Nodes:** `sync.readiness`, `rooms.list`, `timeline.read_window`.

### `crypto_identity`

- **Pain:** device/crypto store continuity; verification/backup UX.
- **Align with:** encryption store only in Rust; clean-break re-login (no JS IndexedDB crypto reuse).
- **Nodes:** session + crypto + cutover.

### `media_lifecycle`

- **Pain:** encrypted media, cache bounds, URL/Blob reclamation.
- **Align with:** native media pipeline; no durable decrypted JSON.

## Post-cutover re-review checklist (per node)

After sole-owner cutover, for each node in JSON:

1. Is product UI free of direct `matrix-js-sdk` for this capability?
2. Does host ownership match `rust_sdk_surface` (not a parallel reimplementation)?
3. Are hard_problem tests green (pagination anchor, receipt monotonicity, single sync owner)?
4. Any residual experimental SDK flags documented with exit criteria?
5. Update `delivery_status` → `product_cutover` / `burned_down` only with evidence.

## Maintenance

- **On product task merge:** if the task maps to a node, update that node’s `delivery_status` / notes in the JSON and refresh this summary table if status changed.
- **Machine inventory** remains [`program-status.json`](program-status.json) — this graph is capability-oriented, not a second 112-task ledger.
- MiniMax expansion jobs are listed under `minimax_jobs` in the JSON.
