# Matrix Rust SDK replacement test matrix and disposable Synapse topology

- Status: **ready_for_independent_review; implementer-authored**
- Phase 0 gate: **open**
- Artifact ID: `R0.2-C-TEST-MATRIX-SYNAPSE-TOPOLOGY`
- Authoritative source: [`test-matrix-synapse-topology.json`](./test-matrix-synapse-topology.json)

This document is the human-readable semantic twin of the JSON artifact. If the
two differ, the JSON is authoritative. Creating these artifacts does not close
Phase 0 and does not accept any evidence.

## Program boundary

The shipping desktop remains `matrix-js-sdk`-only until the atomic cutover. The
test program may use the existing current-JS control runner and a separate,
non-product Matrix Rust SDK harness, but it must not add a production backend
selector or allow both SDKs to own sync for one production session.

Permitted scopes are static/unit validation, contract/property tests, the
current-JS control, an isolated Rust SDK harness, and packaged single-backend
acceptance runs. Forbidden scopes include destructive fixtures against a
production homeserver; committed credentials, tokens, identifiers, event
bodies, raw logs, stores, or signing material; and using JS-control evidence as
Rust SDK evidence.

## Disposable Synapse topology

```text
READER-A / READER-B / SENDER / future IOS client
                     |
             http://127.0.0.1:${SYNARA_PORT:-8008}
                     |
     Synapse matrixdotorg/synapse:v1.157.0
                     |
        private Compose-only database network
                     |
           postgres:16.9-bookworm
```

- Purpose: deterministic local and CI integration only, never production.
- The default host port is `8008`; callers may choose `1024-65535`. Synapse's
  container listener is `0.0.0.0:8008`, but the only host mapping is loopback.
  PostgreSQL has no host port. Client URLs are credential-free HTTP loopback
  origins with no path, query, or fragment.
- Runtime state lives under `integration/synapse/runtime`. Everything below
  `runtime/` is ignored except `.gitkeep`. Setup uses umask `077` and generates
  the PostgreSQL password, registration shared secret, macaroon/form secrets,
  and Ed25519 signing key. Account passwords and access tokens are random,
  per-run, memory-only values.
- Lifecycle commands are
  `SYNARA_PORT=18008 scripts/synapse-integration.sh up`,
  `scripts/synapse-integration.sh status`,
  `scripts/synapse-integration.sh logs`, and
  `scripts/synapse-integration.sh down`. Every live lane must finish with
  `scripts/synapse-integration.sh reset` in always/finally semantics and report
  cleanup failure separately. Reset removes volumes, orphans, database, media,
  generated configuration, keys, credentials, and runtime children other than
  `.gitkeep`.

## Accounts and devices

| Actor       | Account/device                         | Purpose                                                                                   |
| ----------- | -------------------------------------- | ----------------------------------------------------------------------------------------- |
| `BOOTSTRAP` | No account or device                   | Uses only the loopback shared-secret registration endpoint to create fixtures.            |
| `READER-A`  | `reader`, primary device               | Normal Matrix client session.                                                             |
| `READER-B`  | Same `reader` account, second device   | Password login with an explicit disposable device display name.                           |
| `SENDER`    | `sender`, one device                   | Independent normal Matrix client session and event source.                                |
| `IOS`       | Reader or independent peer, one device | Future mixed-client participant via `matrix-rust-components-swift`; not current evidence. |

## Deterministic fixtures

| Fixture            | State                       | Exact shape and purpose                                                                                                                                                                                                                                                                                                                                        |
| ------------------ | --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `FX-64-CONTROL`    | `proven_current_js_control` | One private, unencrypted joined room with 64 ordered messages. Event 0 is the first, event 32 is the marker, and event 63 is latest. Limited initial sync excludes event 0, context includes event 32, and backward pagination prepends. Covers `m.read` and `m.read.private`; “public/private” here means receipt privacy, not room visibility or encryption. |
| `FX-64-PUBLIC`     | `aspirational`              | One public, unencrypted joined room with 64 ordered messages for visibility, join, and ordering.                                                                                                                                                                                                                                                               |
| `FX-1000-E2EE`     | `aspirational`              | One private encrypted joined room with at least 1,000 ordered messages for encrypted initial open, decryption stability, backward pagination, and crash/reopen.                                                                                                                                                                                                |
| `FX-200-ROOMS`     | `aspirational`              | At least 200 joined rooms, at least one encrypted room, and at least one room with 1,000 events. Supports startup, room switching, memory, idle CPU, and disk-growth work.                                                                                                                                                                                     |
| `FX-MEDIA`         | `aspirational`              | Private unencrypted and encrypted variants with a small image (at most 262,144 bytes) and medium image (at most 5,242,880 bytes) for upload, download, cache, and first paint.                                                                                                                                                                                 |
| `FX-GAP-RECONNECT` | `aspirational`              | Private unencrypted and encrypted variants; fixed 15-second outage; 20 events while offline; markers at indexes 31 and 52 for limited gaps, reconnect, stale-generation, and ordering checks.                                                                                                                                                                  |

## Validation lanes

| ID / lane                                 | State                           | Owner and platforms                                                                | Scope and current boundary                                                                                                                                                                |
| ----------------------------------------- | ------------------------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `L-STATIC` / static                       | `proven`                        | Repository CI; all developer hosts and Ubuntu 22.04 CI                             | Image pins, loopback, generated secrets, ignored state, reset semantics; current evidence is `npm run check:synapse-harness`.                                                             |
| `L-UNIT` / unit                           | `proven`                        | Repository CI; Node and Rust test hosts                                            | URL rejection, registration HMAC, bounded polling, safe failures, Matrix units; current evidence is the Synapse runner unit test and `cargo test --locked`.                               |
| `L-CONTRACT` / contract                   | `partially_proven`              | Repository CI; Ubuntu 22.04 CI                                                     | Harness policy, IPC/DTO fixtures, workflow gates. Excludes live Rust behavior and packaged runtime.                                                                                       |
| `L-PROPERTY` / property                   | `aspirational`                  | Matrix runtime test owner; Rust and TypeScript test hosts                          | Bounded generated protocol values, ordering, counter boundaries, and redaction invariants; target R0.3-R0.6.                                                                              |
| `L-INTEGRATION` / integration             | `partially_proven`              | Repository CI; Ubuntu 22.04 CI with Docker                                         | Real Synapse/PostgreSQL, two devices, limited sync, context, pagination, receipts. Current backend is `matrix-js-sdk 42.0.0`.                                                             |
| `L-JS-CONTROL` / current-JS-control       | `proven_for_fx_64_control_only` | Current desktop behavior owner; Ubuntu 22.04 CI                                    | Existing `scripts/run-synapse-two-client-integration.mjs`, `matrix-js-sdk 42.0.0`, and only `FX-64-CONTROL`. Cannot satisfy Rust adapter, E2EE, packaged-app, or P0.6 live-UX acceptance. |
| `L-RUST-HARNESS` / isolated-Rust-harness  | `aspirational`                  | Matrix Rust harness owner; macOS arm64 and Linux x86_64                            | Non-product `matrix-sdk 0.18.0` harness for discovery, login flows, encrypted store, sync readiness, reopen, logout, and wipe; target R0.7 and never production wiring.                   |
| `L-PACKAGED-MACOS` / packaged macOS       | `build_only_partial`            | macOS platform owner; arm64 and universal arm64+x86_64                             | PR #77 proves only a thin arm64 ad-hoc app build. Universal, Developer ID, notarization, install, launch, Keychain, and live scenarios remain pending.                                    |
| `L-PACKAGED-LINUX` / packaged Debian+Arch | `build_only_partial`            | Linux platform owner; Debian- and Arch-family KDE Wayland                          | PR #77 proves only Debian amd64 and Arch x86_64 package construction. Install, launch, Secret Service, portal, uninstall, and live scenarios remain pending.                              |
| `L-MIXED-IOS` / mixed desktop-iOS         | `aspirational`                  | Cross-client owner; packaged desktop plus iOS simulator/device                     | Shared-account semantics, receipts, timeline convergence, E2EE, and device naming; target Phase 12 and final gate.                                                                        |
| `L-MANUAL` / manual                       | `pending_execution`             | Platform operator; macOS arm64, Debian-family KDE Wayland, Arch-family KDE Wayland | Install, launch, login, session persistence, logout, media, OS integration, and P0.6 measurement.                                                                                         |
| `L-FAULT` / fault-restart-reconnect       | `aspirational`                  | Matrix lifecycle owner; macOS arm64 and Linux x86_64                               | Offline/online, process crash, store reopen, suspend/resume, partial failure, and stale generation; target R0.7 and Phase 13.                                                             |
| `L-SOAK` / soak                           | `decision_required`             | Reliability owner; macOS and Linux                                                 | Single sync owner, bounded tasks, convergence, CPU, memory, and disk. Phase 0 duration needs an explicit decision (two hours recommended); Phase 13 is 24 hours.                          |

## Performance matrix

The macOS arm64 and Linux lanes must use release-like packaged builds and the
same fixtures. No end-to-end metric has yet been measured on either platform;
only the synthetic timeline-map proxy is proven.

| Metric                      | Minimum measured samples | Fixed setup                       |
| --------------------------- | -----------------------: | --------------------------------- |
| `M-STARTUP-READY`           |                       10 | No discarded warmups.             |
| `M-ROOM-SWITCH-STABLE`      |                       20 | Discard 3 warmups.                |
| `M-TIMELINE-OPEN-INITIAL`   |                       20 | Discard 3 warmups.                |
| `M-TIMELINE-OPEN-ENCRYPTED` |                       20 | Discard 3 warmups.                |
| `M-PAGINATION-BACKWARD`     |                       20 | Discard 3 warmups.                |
| `M-RECONNECT-SETTLE`        |                       10 | Fixed 15-second offline interval. |
| `M-MEDIA-OPEN`              |                       20 | Discard 3 warmups.                |
| `M-IDLE-CPU`                |                        5 | Settle for 60 seconds.            |
| `M-MEMORY-LARGE-ACCOUNT`    |                        5 | Settle for 120 seconds.           |
| `M-DISK-GROWTH`             |                        5 | Fixed 10-minute scenario.         |

Every timing series reports nearest-rank p50/p95, minimum, maximum, and mean.

## Evidence, failure artifacts, and privacy

Every evidence row must carry a stable ID, requirement IDs, a self-contained
immutable subject object with exact SHA coordinates,
OS/version/architecture/hardware class, client/backend, fixture and counts,
exact command, result and capture time, proof scope and explicit exclusions,
owner and reviewer, run/artifact URL and digest when available, and privacy
review.

A failure records only sanitized phase/error category, reproduction steps, exit
status, a bounded relevant log excerpt, and cleanup result. Never commit raw
homeserver logs; database/store dumps; tokens/passwords; user, room, event,
device, or media identifiers; event plaintext/ciphertext; absolute user paths;
signing certificates; or Apple credentials.

## Evidence boundary and acceptance

Currently proven: harness boundaries; the JS-only `FX-64-CONTROL`; SDK-linked
Ubuntu compile/lint/unit tests; Debian/Arch package construction; thin arm64
ad-hoc macOS construction; iOS simulator build/tests; and one synthetic
timeline-map proxy baseline.

Not proven: a live Rust SDK harness; public/E2EE/large-account/media fixtures;
universal, Developer-ID, or notarized macOS output; package
install/launch/uninstall; macOS/Linux live P0.6 metrics;
fault/restart/reconnect; mixed desktop/iOS scenarios; and either the Phase 0 or
24-hour soak.

This artifact is ready for independent review. Artifact review requires every
lane to retain a stable ID, owner, platform, state, and scope; evidence to remain
within exclusions; cleanup on every live lane; explicit separation of JS control
from Rust harness; and no production selector or dual sync ownership. Even after
artifact review, Phase 0 remains open until the residual manifest is executed or
explicitly deferred and R0.8 independently accepts every applicable row.
