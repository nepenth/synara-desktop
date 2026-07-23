# matrix-rust-sdk 0.18.0 compile-only probe

Isolated, non-production evidence harness for Synara Phase 0 tasks **P0.3a** and
**P0.3b**.

## Purpose

Prove that stable public API surfaces for `matrix-sdk = 0.18.0` and
`matrix-sdk-ui = 0.18.0` are present and nameable on Rust `1.93` under the locked
feature resolution.

Every assertion is **compile-only API-shape** evidence. It does **not** prove
runtime behavior, network semantics, sync correctness, encryption correctness, or
homeserver compatibility.

## Probe organization

| Module             | Focus                                          |
| ------------------ | ---------------------------------------------- |
| `p0_3a.rs`         | Preserved P0.3a foundation probes              |
| `auth.rs`          | Discovery, login, session restore, logout      |
| `sync_rooms.rs`    | Sync service, room list, room lookup/state     |
| `timeline.rs`      | Timeline subscribe/pagination/focus            |
| `messaging.rs`     | Send, state, redact, receipts, typing          |
| `media.rs`         | Media type, upload, retrieval                  |
| `room_ops.rs`      | Create/join/leave/invite/members/power/profile |
| `account_data.rs`  | Account data raw get/set                       |
| `notifications.rs` | Notification settings and push rules           |
| `e2ee.rs`          | Encryption, devices, recovery, backups         |
| `search.rs`        | User search and room directory search          |
| `spaces.rs`        | SpaceService and hierarchy list                |
| `threads.rs`       | Relations, thread list, threaded timeline      |

Stable probe IDs are listed in `PROBE_IDS` in `src/lib.rs` and mirrored by
`docs/matrix-rust-sdk/0.18.0-stable-capabilities.{json,md}`.

## Non-goals

- No homeserver connection
- No secrets, tokens, or store passphrases
- No experimental SDK features
- Not a production dependency or backend selector
- Not the final P0.3 capability dossier (later subtask)
- Not live Synapse or UI acceptance evidence

## Dependencies

Direct requests from this probe:

| Crate           | Pin       | Requested default-features | Requested features |
| --------------- | --------- | -------------------------- | ------------------ |
| `matrix-sdk`    | `=0.18.0` | `false`                    | `[]`               |
| `matrix-sdk-ui` | `=0.18.0` | `false`                    | `[]`               |

Direct dependencies are exactly the P0.3a set (only the two Matrix crates).
`Media::upload` is probed by taking the method as a value so the compiler checks
its public signature without naming `mime::Mime` or adding a direct `mime` pin.

`sqlite` and `automatic-room-key-forwarding` are **not** requested and are **not**
enabled.

### Unified resolution note

`matrix-sdk-ui` `0.18.0` depends on `matrix-sdk` with feature `e2e-encryption`.
Cargo feature unification therefore enables `e2e-encryption` on the resolved
`matrix-sdk` graph even though this probe does not request it directly. Record
resolved features from `cargo metadata --locked`, not from the direct request
table above. See `docs/matrix-rust-sdk/0.18.0-source-provenance.md` and
`docs/matrix-rust-sdk/0.18.0-stable-capabilities.md`.

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
