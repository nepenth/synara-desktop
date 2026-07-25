# Matrix domain DTO fixtures (P1.4)

Authoritative JSON fixtures for Synara-owned Matrix domain DTOs used over IPC.

| Path | Role |
|------|------|
| Design note | [`../p1.4-domain-dtos.md`](../p1.4-domain-dtos.md) |
| Machine twin | [`../p1.4-domain-dtos.json`](../p1.4-domain-dtos.json) |
| Rust types | `src-tauri/src/matrix/dto/` |
| TypeScript types | `synara/src/app/features/matrix-dto/` |
| Transport (P1.3) | `docs/matrix-rust-sdk/ipc/` (envelopes only) |

DTO domain is **distinct from transport**: `matrix-dto` is a sibling of
`matrix-ipc`. Snapshot/delta envelope bodies may remain opaque JSON until later
phases compose these DTOs into stream payloads.

## Constraints

- Product meaning first — **not** a `matrix-js-sdk` / Ruma object-graph clone
- No `matrix_sdk` / Ruma types in DTO modules
- Wire JSON: **camelCase** fields; enum discriminators **snake_case**
- No `accessToken` / `refreshToken` on session (or any) wire DTO
- No large media byte arrays — handles / mxc URIs / paths only
- No production login/sync/Client session; no Matrix product Tauri commands

## Fixtures

| File | Expectation |
|------|-------------|
| `valid_session.json` | `SessionSnapshot` |
| `valid_room_summary.json` | `RoomSummary` |
| `valid_member.json` | `RoomMember` |
| `valid_timeline_item_message.json` | `TimelineItem` kind `message` |
| `valid_timeline_item_state.json` | `TimelineItem` kind `state` |
| `valid_relation_reaction.json` | `RelationRef` (annotation) |
| `valid_receipt.json` | `Receipt` |
| `valid_typing.json` | `TypingSnapshot` |
| `valid_upload.json` | `UploadJob` (no bytes) |
| `valid_media_handle.json` | `MediaHandle` (no bytes) |
| `valid_security_status.json` | `SecurityStatus` |
| `valid_notification_candidate.json` | `NotificationCandidate` |
| `valid_search_result.json` | `SearchResult` |
| `valid_space_summary.json` | `SpaceSummary` |
| `valid_thread_summary.json` | `ThreadSummary` |
| `valid_widget_session.json` | `WidgetSession` |

Both Rust (`cargo test --locked matrix`) and TypeScript (`esbuild` +
`node --test` on `matrix-dto/__tests__/matrixDto.test.ts`) load these fixtures.
