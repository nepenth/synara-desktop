# Retired SDK experiment

Retired: 2026-09-30.

This directory is a provenance record. Its executable sources, package
manifests, and lockfiles have been removed now that the product uses the
project-owned Rust Core and Matrix Rust SDK 0.19.1. It is not an active build,
test, dependency-update, or supported Matrix client path.

The original experiment can be inspected at repository commit `0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2`:

```sh
git show 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2:probes/matrix-rust-sdk-0.18-feature-probes/README.md
git ls-tree -r 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2 -- probes/matrix-rust-sdk-0.18-feature-probes
```

Historical measurements and capability reports retain their original version,
source paths, and conclusions; they do not establish current runtime behavior.
Current architecture is [ADR 0003](../../docs/adr/0003-shared-native-rust-core.md)
and [ADR 0004](../../docs/adr/0004-rust-language-boundaries.md).

---

## Original experiment record

The following README is retained as historical evidence. Its commands refer
to files at the commit above and cannot be run from this retired directory.

# matrix-rust-sdk-0.18-feature-probes (P0.3c)

Isolated, non-production compile-only probes for Matrix Rust SDK **0.18.0**
feature gates, typed residual requests, experimental surfaces, and plan-critical
API gaps.

Upstream pin: tag `matrix-sdk-0.18.0`, commit
`1c44fb66214667c6d00acaf72ab592493653708b`.

## Constraints

- Does **not** connect to a homeserver, open a store, or handle secrets.
- Does **not** enable `automatic-room-key-forwarding`.
- Does **not** modify the accepted P0.3a/P0.3b probe crate.
- Experimental profiles must be compiled **independently** (never combined).
- `e2e-encryption` may appear only via `matrix-sdk-ui` dependency unification
  (or as a transitive of `experimental-widgets` →
  `experimental-send-custom-to-device` → `e2e-encryption`).

## Profiles

| Feature flag | Direct matrix-sdk feature request | Purpose |
| --- | --- | --- |
| `profile-stable-typed` | _(none)_ | Typed `Client::send` residual paths + stable gap APIs |
| `profile-experimental-search` | `experimental-search` | Local index / message search experimental surface |
| `profile-experimental-widgets` | `experimental-widgets` | Widget driver / Element Call settings surface |
| `profile-sqlite` | `sqlite` | SQLite store builder surface |

Validate each profile separately, for example:

```bash
# Use an isolated temporary Cargo target directory per profile (do not commit it).
export CARGO_TARGET_DIR="${TMPDIR:-/tmp}/p03c-stable-typed"
cargo metadata --locked --format-version 1 --features profile-stable-typed
cargo fmt --check
cargo check --locked --features profile-stable-typed
cargo test --locked --features profile-stable-typed
cargo doc --locked --no-deps --features profile-stable-typed
```

Repeat with `profile-experimental-search`, `profile-experimental-widgets`, and
`profile-sqlite` using distinct temporary target directories.

## Evidence

Canonical artifacts:

- `docs/matrix-rust-sdk/0.18.0-feature-and-gap-analysis.json`
- `docs/matrix-rust-sdk/0.18.0-feature-and-gap-analysis.md`

Evidence statuses are intermediate only (for example
`stable-typed-request-probed`, `experimental-api-probed`). Final Section 5
states such as `confirmed-stable` are **not** claimed here.
