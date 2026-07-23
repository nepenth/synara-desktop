# matrix-rust-sdk 0.18.0 compile-only probe

Isolated, non-production evidence harness for Synara Phase 0 task **P0.3a**.

## Purpose

Prove that the public API surface for these types is present and nameable against
the exact crates `matrix-sdk = 0.18.0` and `matrix-sdk-ui = 0.18.0` on Rust
`1.93`:

- `matrix_sdk::Client`
- `matrix_sdk::ClientBuilder`
- `matrix_sdk::Room`
- `matrix_sdk_ui::sync_service::SyncService`
- `matrix_sdk_ui::RoomListService`
- `matrix_sdk_ui::Timeline`

Every assertion is **compile-only API-shape** evidence. It does **not** prove
runtime behavior, network semantics, sync correctness, encryption, or
homeserver compatibility.

## Non-goals

- No homeserver connection
- No secrets, tokens, or store passphrases
- No experimental SDK features
- Not a production dependency or backend selector
- Not the full P0.3 capability dossier (later subtask)

## Dependencies

Direct requests from this probe (`default-features = false`, no explicit feature
list on either crate):

| Crate           | Pin       | Requested default-features | Requested features |
| --------------- | --------- | -------------------------- | ------------------ |
| `matrix-sdk`    | `=0.18.0` | `false`                    | `[]`               |
| `matrix-sdk-ui` | `=0.18.0` | `false`                    | `[]`               |

`sqlite` and `automatic-room-key-forwarding` are **not** requested and are **not**
required for `Client` / `ClientBuilder` / `Room` API-shape probes.

### Unified resolution note

`matrix-sdk-ui` `0.18.0` depends on `matrix-sdk` with feature `e2e-encryption`.
Cargo feature unification may therefore enable `e2e-encryption` on the resolved
`matrix-sdk` graph even though this probe does not request it directly. Record
resolved features from `cargo metadata --locked`, not from the direct request
table above. See `docs/matrix-rust-sdk/0.18.0-source-provenance.md`.

Upstream source revision for the published `0.18.0` crates:

- tag: `matrix-sdk-0.18.0`
- commit: `1c44fb66214667c6d00acaf72ab592493653708b`

## Validation

Run from this directory with an isolated Cargo target directory (use any
writable temporary directory; do not hard-code absolute temporary/cache paths in
committed artifacts):

```sh
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(mktemp -d)}"
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --check
cargo check --locked
cargo test --locked
cargo doc --locked --no-deps
cargo doc --locked -p matrix-sdk -p matrix-sdk-ui --no-deps
```

Do not commit `target/` or generated rustdoc HTML.
