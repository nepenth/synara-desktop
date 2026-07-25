# Matrix IPC fixtures (P1.3)

Authoritative JSON fixtures for the versioned Matrix IPC envelope protocol.

| Path | Role |
|------|------|
| Design note | [`../p1.3-matrix-ipc-schemas.md`](../p1.3-matrix-ipc-schemas.md) |
| Rust types | `src-tauri/src/matrix/ipc/` |
| TypeScript types | `synara/src/app/features/matrix-ipc/` |

## Fixtures

| File | Expectation |
|------|-------------|
| `valid_hello.json` | Parse OK — version negotiation request |
| `valid_hello_ack.json` | Parse OK — version negotiation response |
| `valid_subscribe.json` | Parse OK — stream subscribe |
| `valid_snapshot.json` | Parse OK — initial stream snapshot |
| `valid_delta.json` | Parse OK — ordered stream delta |
| `valid_error_rate_limited.json` | Parse OK — privacy-safe error |
| `valid_resync_required.json` | Parse OK — gap / resubscribe signal |
| `invalid_unknown_kind.json` | Parse **fail** — unknown kind rejected |
| `invalid_missing_protocol_version.json` | Parse **fail** — required field missing |

Both Rust (`cargo test`) and TypeScript (`node:test` via modernization runner
or direct file) load these fixtures for compatibility checks.
