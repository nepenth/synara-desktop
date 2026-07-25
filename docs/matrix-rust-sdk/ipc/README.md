# Matrix IPC fixtures (P1.3 + P1.5)

Authoritative JSON fixtures and schema catalog for the versioned Matrix IPC
envelope protocol.

| Path | Role |
|------|------|
| Design note (foundation) | [`../p1.3-matrix-ipc-schemas.md`](../p1.3-matrix-ipc-schemas.md) |
| Design note (contract tests) | [`../p1.5-ipc-contract-tests.md`](../p1.5-ipc-contract-tests.md) |
| Schema catalog (v1) | [`schema_catalog_v1.json`](schema_catalog_v1.json) |
| Rust types | `src-tauri/src/matrix/ipc/` |
| TypeScript types | `synara/src/app/features/matrix-ipc/` |
| Rust contract tests | `src-tauri/src/matrix/ipc/contract_tests.rs` (+ `tests.rs`) |
| TS contract tests | `synara/.../matrix-ipc/__tests__/matrixIpcContract.test.ts` (+ `matrixIpc.test.ts`) |

## Schema catalog

`schema_catalog_v1.json` is the **compatibility oracle** for protocol version 1:

- exhaustive `kinds` (13)
- exhaustive `errorCategories` (21)
- `streamTopics`, `resyncReasons`, `cancelReasons`
- policy `bounds` (payload size, queue depth, coalesce window, open streams)

Rust and TypeScript contract tests assert their constants match this catalog.

## Valid fixtures (parse OK)

| File | Role |
|------|------|
| `valid_hello.json` | Version negotiation request |
| `valid_hello_ack.json` | Version negotiation response |
| `valid_subscribe.json` | Stream subscribe |
| `valid_unsubscribe.json` | Stream unsubscribe |
| `valid_subscribed.json` | Subscribe ack |
| `valid_unsubscribed.json` | Unsubscribe ack + resources released |
| `valid_snapshot.json` | Initial stream snapshot |
| `valid_snapshot_with_room_summary_body.json` | Snapshot + P1.4 DTO-shaped body |
| `valid_delta.json` | Ordered stream delta |
| `valid_error_rate_limited.json` | Privacy-safe rate-limit error |
| `valid_error_stale_session.json` | Stale-generation error |
| `valid_resync_required.json` | Sequence-gap resync signal |
| `valid_resync_stale_generation.json` | Stale-generation resync signal |
| `valid_cancel.json` | Cancellation token |
| `valid_ping.json` / `valid_pong.json` | Liveness |

## Invalid fixtures (parse **fail** / reject-at-boundary)

| File | Expectation |
|------|-------------|
| `invalid_unknown_kind.json` | Unknown kind rejected |
| `invalid_missing_protocol_version.json` | Required field missing |
| `invalid_missing_session_generation.json` | Required field missing |
| `invalid_missing_sequence.json` | Required field missing |
| `invalid_missing_kind.json` | Missing kind |
| `invalid_missing_payload.json` | Missing payload |
| `invalid_wrong_type_protocol_version.json` | Wrong type (string vs number) |
| `invalid_wrong_type_sequence.json` | Wrong type (string vs number) |
| `invalid_unknown_topic.json` | Unknown stream topic |
| `invalid_unknown_error_category.json` | Unknown error category |
| `invalid_unknown_resync_reason.json` | Unknown resync reason |
| `invalid_error_with_secret_field.json` | Secret-looking field rejected |
| `invalid_hello_missing_client_protocol_version.json` | Incomplete hello payload |
| `invalid_subscribe_missing_stream_id.json` | Incomplete subscribe payload |

Both Rust (`cargo test --locked matrix::ipc`) and TypeScript (`node:test` via
modernization runner or direct esbuild bundle) load these fixtures. P1.5 covers
serialization round trips, bounds, sequence gaps, stale generations, schema
catalog compatibility, and privacy rejection.
