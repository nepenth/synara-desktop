# Matrix Rust SDK replacement test matrix and disposable Synapse topology

- Artifact: `R0.2-C-TEST-MATRIX-SYNAPSE-TOPOLOGY`
- Schema: `2`
- State: `ready_for_independent_review`
- Authoritative source: [`test-matrix-synapse-topology.json`](./test-matrix-synapse-topology.json)
- Phase 0 gate: `open`

This is the JSON artifact’s semantic twin. R0.2 defines completeness and
execution readiness only. It cannot accept P0.1–P0.7, change strict Phase 0
criteria, approve a deferral, or close Phase 0. Only an independent R0.8 review
after a full exact-head rerun may accept Phase 0.

## Program and path boundary

The shipping backend remains `matrix-js-sdk 42.0.0` until atomic cutover.
Static/unit/contract tests, the current-JS control, an isolated non-product Rust
harness, and packaged single-backend acceptance runs are permitted. A production
backend selector, dual sync owner, non-loopback destructive fixture, JS evidence
represented as Rust evidence, or an acceptance/status/gate claim is forbidden.

| Path                                                        | State / purpose / owner                                                                                                                                     |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/synapse-integration.sh`                            | Existing deterministic lifecycle and runtime-secret producer; Synapse harness owner.                                                                        |
| `synara/scripts/run-synapse-two-client-integration.mjs`     | Existing current-JS control; current desktop behavior owner. This is the corrected path; stale `scripts/run-synapse-two-client-integration.mjs` is invalid. |
| `scripts/__tests__/synapse-two-client-integration.test.mjs` | Existing safe harness unit validation; Synapse harness owner.                                                                                               |
| `integration/synapse/compose.yml`                           | Existing pinned disposable topology; Synapse harness owner.                                                                                                 |
| `integration/synapse/runtime/.gitkeep`                      | Existing ignored runtime-secret anchor; Synapse harness owner.                                                                                              |
| `docs/matrix-rust-sdk/evidence/phase0/<lane-id>.json`       | Future output of the named lane and owned by its lane owner; absence means pending. Create only in its implementing PR and bind to the full head SHA.       |
| `src-tauri/tests/matrix_rust_live_harness.rs`               | Future R0.7 isolated Rust harness output; Matrix Rust harness owner. It is not present or implied by R0.2.                                                  |

## Credentials and disposable topology

`SYNARA-DISPOSABLE-RUNTIME-SECRETS-V1` is the sole credential mechanism.
`SYNARA_PORT` is nonsecret. With umask `077`, setup generates
`integration/synapse/runtime/.env`, `homeserver.yaml`, and
`localhost.signing.key`; random account passwords and tokens remain process-only.
Stable localparts/device names are identities, not credentials. CI and evidence
must never use command arguments, commits, logs, outputs, or artifacts for
passwords, tokens, registration/database secrets, or signing keys. Evidence may
contain only safe phase/count/error/exit/cleanup data and must redact secret
values plus Matrix user/room/event/device IDs, bodies, and absolute paths.

Synapse is `matrixdotorg/synapse:v1.157.0`; PostgreSQL is
`postgres:16.9-bookworm`. Clients use
`http://127.0.0.1:${SYNARA_PORT:-8008}`. Host binding is loopback-only, the
default port is `8008`, allowed range is `1024`–`65535`, and PostgreSQL has zero
host ports.

Setup is, in order: `scripts/synapse-integration.sh reset`,
`SYNARA_PORT=18008 scripts/synapse-integration.sh up`, then
`scripts/synapse-integration.sh status`. Teardown is always/finally
`scripts/synapse-integration.sh reset`. A cleanup failure separately fails and
invalidates live evidence. Reset must leave no volume, orphan, database, media,
generated config/key/credential/runtime child except
`integration/synapse/runtime/.gitkeep`; each setup rotates credentials.

### Network/failure controls

| ID                   | Procedure                                                                                                 | Expected result                                                                  |
| -------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `NF-OFFLINE-15`      | Block client-to-loopback-Synapse for exactly 15 seconds while Synapse remains healthy.                    | Explicit offline state, no duplicate sync task, convergence after restoration.   |
| `NF-SYNAPSE-RESTART` | Restart only Synapse after a recorded sync boundary.                                                      | Bounded retry; no credential log, lost acknowledged event, or duplicate row.     |
| `NF-PROCESS-CRASH`   | Terminate after store flush but before next-token acknowledgement, then reopen the same disposable store. | Exactly one owner resumes and converges or explicitly fails without silent loss. |

## Stable actors

| ID          | Localpart / device                            | Role and power                                               |
| ----------- | --------------------------------------------- | ------------------------------------------------------------ |
| `BOOTSTRAP` | no account/device                             | Shared-secret registration only; never a room member.        |
| `READER-A`  | `synara_reader` / `SYNARA-R0.2-READER-A`      | Primary reader; joined level 0.                              |
| `READER-B`  | same `synara_reader` / `SYNARA-R0.2-READER-B` | Second reader device; joined level 0.                        |
| `SENDER`    | `synara_sender` / `SYNARA-R0.2-SENDER`        | Creator/sender; level 100.                                   |
| `IOS`       | `synara_ios` / `SYNARA-R0.2-IOS`              | Future mixed client; joined level 0 in its future lane only. |

## Exact deterministic fixture

`FX-64-CONTROL` is `planned` with seed `FX-64-CONTROL-v2`. It has no current
producer. The existing `synara/scripts/run-synapse-two-client-integration.mjs`
implements only one room/64 messages per independently provisioned receipt-mode
scenario and must not be treated as v2 evidence. A future implementation in that
path must exercise exactly **2
rooms**, **0 spaces**, and **128 ordered message events**: 64 per room. State
events are excluded from the 128-message count.

| Room            | State graph and corpus                                                                                  | Boundary assertions                                                                                                                            |
| --------------- | ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `ROOM-CONTROL`  | Private, unencrypted; SENDER creates at level 100; READER-A/B join at 0; template `control-{000..063}`. | First 0, marker 32, latest 63. Limited initial sync excludes 0; context returns 32 once; backward pagination prepends through 0; latest is 63. |
| `ROOM-BOUNDARY` | Private, unencrypted; same membership/power; template `boundary-{000..063}`.                            | First 0, `m.read` target 31, `m.fully_read` target 32, `m.read.private` target 63, latest 63; device convergence respects receipt privacy.     |

Setup resets/starts through the named secret mechanism, creates SENDER and the
reader with new in-memory passwords, logs in both exact reader devices, creates
the rooms in the listed order, applies membership/power, sends each corpus
sequentially, holds acknowledged IDs only in memory, and waits for both devices
to see exactly 2 rooms/128 unique ordered messages. Validation rejects any
duplicate, omission, cross-room item, or ordering error and never emits IDs or
bodies. Teardown is mandatory after pass/fail/timeout. The same seed recreates
shape/identities with fresh credentials and Matrix IDs. Platforms are Ubuntu
22.04 x86_64 CI with Docker and local Docker developer hosts. Dependencies are
`scripts/synapse-integration.sh` and the corrected current-JS runner path. A
planned or produced-unreviewed fixture remains gate-open.

Other fixture rows remain gate-open:

| ID / state                     | Shape, producer/output, authority                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `FX-64-PUBLIC` / `planned`     | Seed `FX-64-PUBLIC-v1`; common reset/setup/send/wait/validate/teardown producer; exactly 1 public unencrypted room, 0 spaces, SENDER 100/READER-A 0, and 64 `public-{000..063}` messages. Validate alias join, exact unique order, cleanup. Future `docs/matrix-rust-sdk/evidence/phase0/FX-64-PUBLIC.json`; fixture owner / independent Matrix test reviewer; Ubuntu 22.04 CI/local Docker; refresh on fixture/Synapse/client/head or seven days; named secret mechanism dependency; open.                                                                                                                          |
| `FX-1000-E2EE` / `planned`     | Seed `FX-1000-E2EE-v1`; common producer; exactly 1 private encrypted room, 0 spaces, SENDER 100/READER-A/B 0, and 1,000 `e2ee-{0000..0999}` messages. Both devices decrypt/order/paginate/reopen without plaintext evidence. Future `docs/matrix-rust-sdk/evidence/phase0/FX-1000-E2EE.json`; fixture owner / independent Matrix/E2EE reviewer; macOS arm64/Linux x86_64; refresh on fixture/store/SDK/Synapse/client/head or seven days; secret mechanism dependency; open.                                                                                                                                         |
| `FX-200-ROOMS` / `planned`     | Seed `FX-200-ROOMS-v1`; common producer; exactly 1 `SPACE-LARGE` containing 200 rooms in order, every tenth encrypted (20 total), SENDER 100/READER-A 0, `ROOM-LARGE-000` has 1,000 messages and each other room 1. Validate exact graph/counts. Future `docs/matrix-rust-sdk/evidence/phase0/FX-200-ROOMS.json`; fixture owner / independent Matrix/performance reviewer; macOS/Linux; refresh on fixture/SDK/Synapse/client/head or seven days; depends on E2EE fixture/secret mechanism; open.                                                                                                                    |
| `FX-MEDIA` / `planned`         | Seed `FX-MEDIA-v1`; deterministic-byte/common producer; exactly 2 private rooms (one encrypted), 0 spaces, SENDER 100/READER-A 0; `MEDIA-SMALL` is a seeded RGBA PNG exactly 262,144 bytes and `MEDIA-MEDIUM` is a seeded binary image exactly 5,242,880 bytes. Validate upload/download/digest/cache in both without bytes/MXC/keys in evidence. Future `docs/matrix-rust-sdk/evidence/phase0/FX-MEDIA.json`; fixture owner / independent Matrix/media reviewer; macOS/Linux; refresh on generator/media/crypto/SDK/Synapse/client/head or seven days; depends on E2EE/secret mechanism; open.                      |
| `FX-GAP-RECONNECT` / `planned` | Seed `FX-GAP-RECONNECT-v1`; `NF-OFFLINE-15`/common producer; exactly 2 private rooms (one encrypted), 0 spaces, SENDER 100/READER-A/B 0; each has 32 pre-gap, 20 during exactly 15 seconds offline, 12 post-gap = 64, total 128. Index 31 precedes gap, 32–51 during, 52 first after; validate exact order/convergence/no duplicate owner/stale update. Future `docs/matrix-rust-sdk/evidence/phase0/FX-GAP-RECONNECT.json`; fixture owner / independent Matrix/reliability reviewer; macOS/Linux; refresh on fixture/control/SDK/Synapse/client/head or seven days; depends on offline/E2EE/secret mechanism; open. |

The abbreviated future-output prefix above is exactly
`docs/matrix-rust-sdk/evidence/phase0/`; it never denotes a current file.

## Evidence lifecycle

States are `planned`, `pending`, `produced_pending_review`, `accepted`,
`rejected`, `failed`, `blocked`, `deferred`, `stale`, and `expired`. Only
`accepted` can satisfy a gate. Each produced lane record contains exact
base/head, producer, result, platform, fixture/counts, sanitized artifact path or
immutable URL, owner, independent reviewer, capture/expiry, dependencies, and
exclusions. Missing, failed, blocked, rejected, deferred, stale, expired, or
produced-unreviewed evidence remains open; cleanup failure invalidates live
evidence. Live/performance expires after seven UTC days; toolchain refreshes on
toolchain/lock change; ephemeral evidence expires with its artifact; every row
refreshes on candidate, fixture, producer, SDK, Synapse, platform, or criterion
change.

## Validation lanes

Every future evidence path is owned/produced by its row and does not exist yet.

The original stable lane IDs below remain independent normative records; v2
aggregate rows do not replace them.

| Stable ID / state              | Producer/output and exact validation                                                                                                                                                                                               | Owner / authority / scope / dependencies / refresh / gate                                                                                                                                                                                                         |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `L-STATIC` / `pending`         | `npm run check:synapse-harness`; nonexistent future output `docs/matrix-rust-sdk/evidence/phase0/L-STATIC.json`; exact-head rerun checks pins, loopback, secrets, ignored state, reset; expect exit 0/all invariants.              | repository CI owner / independent R0.8 reviewer; all hosts/Ubuntu 22.04; depends on `scripts/synapse-integration.sh`, `integration/synapse/compose.yml`; refresh on candidate/harness/image; `open_until_exact_immutable_evidence_record_exists_and_is_accepted`. |
| `L-UNIT` / `pending`           | Exact Node harness test plus `cargo test --locked`; nonexistent future output `docs/matrix-rust-sdk/evidence/phase0/L-UNIT.json`; rerun checks URL/HMAC/polling/safe failures/Matrix coverage; expect 0/no skipped required cases. | repository CI owner / independent R0.8 reviewer; Node/Rust hosts; depends on `scripts/__tests__/synapse-two-client-integration.test.mjs`; refresh on candidate/toolchain/test; `open_until_exact_immutable_evidence_record_exists_and_is_accepted`.               |
| `L-CONTRACT` / `pending`       | Harness policy, IPC/DTO fixture, workflow contracts; future `.../L-CONTRACT.json`; expect exact assertions and explicit live/packaged exclusions.                                                                                  | repository CI owner / independent IPC/policy reviewer; Ubuntu 22.04; depends on `L-STATIC`, `L-UNIT`; refresh on contract/workflow/candidate; `open_until_accepted`.                                                                                              |
| `L-PROPERTY` / `planned`       | Future bounded generators for protocol/order/counters/redaction; future `.../L-PROPERTY.json`; replay seeds and inspect shrinking/bounds; expect pass or safe reproducible failure.                                                | Matrix runtime test owner / independent Matrix test reviewer; Rust/TypeScript hosts; depends on `L-CONTRACT`; refresh on generator/protocol/candidate; `open`.                                                                                                    |
| `L-INTEGRATION` / `blocked`    | After corrected v2 producer, run real disposable integration; future `.../L-INTEGRATION.json`; independent rerun expects exact 2/128, convergence, redaction, cleanup.                                                             | current desktop behavior owner / independent Matrix integration reviewer; Ubuntu Docker; depends on `FX-64-CONTROL`, `RES-SYN-FIXTURE-CORPUS`; refresh after seven days or producer/client/Synapse/candidate; `open`.                                             |
| `L-PACKAGED-MACOS` / `blocked` | Build/install/launch exact universal signed/notarized package and session/Keychain cases; future `.../L-PACKAGED-MACOS.json`; expect every case/no fallback.                                                                       | macOS platform owner / independent macOS/release reviewer; arm64/universal; depends on `RES-P05-MACOS-UNIVERSAL`, `RES-P05-MACOS-SIGN-NOTARIZE`, `RES-P05-MACOS-INSTALL-LAUNCH`; refresh after seven days or package/OS/signing/SDK/candidate; `open`.            |
| `L-PACKAGED-LINUX` / `blocked` | Install/launch exact Debian/Arch packages and session/Secret Service/portal/logout/uninstall; future `.../L-PACKAGED-LINUX.json`; expect every case/no fallback.                                                                   | Linux platform owner / independent Linux/release reviewer; Debian/Arch KDE; depends on `RES-P05-DEBIAN-INSTALL-LAUNCH`, `RES-P05-ARCH-INSTALL-LAUNCH`; refresh after seven days or package/distro/desktop/SDK/candidate; `open`.                                  |
| `L-MANUAL` / `pending`         | Execute install/launch/login/persistence/logout/media/OS/P0.6 checklist; future `.../L-MANUAL.json`; independent exact-digest repeat expects safe passing cases.                                                                   | platform operator / independent platform reviewer; macOS/Debian/Arch; depends on `L-PACKAGED-MACOS`, `L-PACKAGED-LINUX`; refresh after seven days or checklist/package/platform/candidate; `open`.                                                                |
| `L-FAULT` / `blocked`          | Execute all three NF controls plus suspend/resume/partial failure; future `.../L-FAULT.json`; expect convergence, one owner, no stale generation/loss/leak, cleanup.                                                               | Matrix lifecycle owner / independent reliability reviewer; macOS/Linux; depends on `FX-GAP-RECONNECT`, `RES-P06-RECONNECT`; refresh after seven days or control/SDK/platform/candidate; `open`.                                                                   |
| `L-SOAK` / `decision_required` | After `DEC-P06-SOAK-DURATION`, run Phase 0 soak while retaining 24-hour Phase 13; future `.../L-SOAK.json`; verify duration/platforms/fixture/thresholds/owner/convergence/resources/cleanup.                                      | reliability owner / independent reliability reviewer and program owner; macOS/Linux; depends on `DEC-P06-SOAK-DURATION`, `RES-P06-SOAK-SCOPE`, `L-FAULT`; refresh after seven days or duration/fixture/SDK/platform/candidate; `open`.                            |

| ID / state                             | Producer, output, validation / expected result                                                                                                                                                                                                                                                           | Owner / authority / scope / dependencies / refresh / gate                                                                                                                                                                                                                |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `L-STATIC-UNIT` / `pending`            | Run `npm run check:synapse-harness`, the exact Node harness test, and `cargo test --locked`; future `.../L-STATIC-UNIT.json`. R0.8 reruns; all exit 0 and cover loopback, secret/redaction/reset, URL/HMAC/polling, Matrix units.                                                                        | repository CI owner / independent R0.8 reviewer; Ubuntu 22.04, Node/Rust; depends on lifecycle script/unit test; refresh on candidate/toolchain/harness; open until accepted.                                                                                            |
| `L-JS-CONTROL` / `blocked`             | First implement v2 in `synara/scripts/run-synapse-two-client-integration.mjs`; then setup, seed v2, `npm run test:synapse-integration`, validate exact counts/assertions/redaction, reset; future `.../L-JS-CONTROL.json`. Independent exact-head rerun expects 2 rooms/128 messages and clean teardown. | current desktop behavior owner / independent R0.8 reviewer; Ubuntu 22.04 JS SDK 42.0.0 only; depends on fixture and `NF-OFFLINE-15`; refresh after seven days or candidate/harness/client/Synapse change; open and never proves Rust.                                    |
| `L-RUST-HARNESS` / `planned`           | Future R0.7 discovery, login-flow, login/device naming, encrypted store open/reopen, sync, crash, logout, wipe; future source `src-tauri/tests/matrix_rust_live_harness.rs` and `.../L-RUST-HARNESS.json`. Exact-head disposable rerun expects all cases on macOS/Linux with one non-product owner.      | Matrix Rust harness owner / independent Rust reviewer; macOS arm64, Linux x86_64, SDK 0.18.0; depends on control/E2EE fixtures; refresh after seven days or SDK/store/Synapse/platform/head; open and cannot add production backend.                                     |
| `L-PACKAGED-MACOS-LINUX` / `blocked`   | Install/launch exact-digest universal signed/notarized macOS and Debian/Arch packages; login/store/OS credential/media/logout/uninstall; future `.../L-PACKAGED-MACOS-LINUX.json`. Independent platform rerun expects all supported platforms, no fallback, cleanup.                                     | macOS/Linux owners / independent release/platform reviewer; macOS arm64+x86_64 and Debian/Arch KDE; depends on `RES-P05-MACOS-PACKAGE` and `RES-P05-LINUX-PACKAGES`; refresh after seven days or package/OS/SDK/head; open.                                              |
| `L-PERFORMANCE-SOAK-FAULT` / `blocked` | Run packaged metric matrix, approved Phase 0 soak, and all three NF controls; future `.../L-PERFORMANCE-SOAK-FAULT.json`. Validate raw-series digests, samples, statistics, thresholds, convergence/task/disk/cleanup; expect no leak/duplicate/stale/loss.                                              | performance/reliability owners / independent performance/reliability reviewer; packaged macOS/Linux; depends on performance/soak residuals and E2EE fixture; refresh after seven days or metric/fixture/hardware/toolchain/client/head; open; Phase 13 remains 24 hours. |
| `L-MIXED-IOS` / `planned`              | Future Phase 12 packaged desktop+iOS account/device/receipt/timeline/E2EE/logout scenarios; future `.../L-MIXED-IOS.json`. Independent disposable exact-head rerun expects convergence without secret/content leakage or second desktop backend.                                                         | cross-client owner / independent cross-client reviewer; packaged desktop and iOS simulator/assigned device; depends on IOS actor and Rust harness; refresh on client/SDK/wrapper/Synapse/scenario/head; open/future phase.                                               |

### Literal dependency registry

Every topology JSON dependency token appears verbatim here:
`DEC-P06-SOAK-DURATION`, `FX-1000-E2EE`, `FX-200-ROOMS`, `FX-64-CONTROL`,
`FX-GAP-RECONNECT`, `FX-MEDIA`, `IOS`, `L-CONTRACT`, `L-FAULT`,
`L-PACKAGED-LINUX`, `L-PACKAGED-MACOS`, `L-RUST-HARNESS`, `L-STATIC`, `L-UNIT`,
`NF-OFFLINE-15`, `R0.2-C-TEST-MATRIX-SYNAPSE-TOPOLOGY`,
`RES-P05-ARCH-INSTALL-LAUNCH`, `RES-P05-DEBIAN-INSTALL-LAUNCH`,
`RES-P05-LINUX-PACKAGES`, `RES-P05-MACOS-INSTALL-LAUNCH`,
`RES-P05-MACOS-PACKAGE`, `RES-P05-MACOS-SIGN-NOTARIZE`,
`RES-P05-MACOS-UNIVERSAL`, `RES-P06-LIVE-PERFORMANCE`, `RES-P06-RECONNECT`,
`RES-P06-RECONNECT-SOAK`, `RES-P06-SOAK-SCOPE`, `RES-SYN-FIXTURE-CORPUS`,
`SYNARA-DISPOSABLE-RUNTIME-SECRETS-V1`, `integration/synapse/compose.yml`,
`scripts/__tests__/synapse-two-client-integration.test.mjs`,
`scripts/synapse-integration.sh`, and
`synara/scripts/run-synapse-two-client-integration.mjs`.

## Performance matrix

Both macOS arm64 and Linux x86_64 use release-like packaged builds and the same
fixture. The matrix state is `planned`; no end-to-end metric is accepted and the
synthetic row-map proxy cannot satisfy a row.

| Metric                      | Samples | Fixed control       |
| --------------------------- | ------: | ------------------- |
| `M-STARTUP-READY`           |      10 | 0 warmups           |
| `M-ROOM-SWITCH-STABLE`      |      20 | discard 3           |
| `M-TIMELINE-OPEN-INITIAL`   |      20 | discard 3           |
| `M-TIMELINE-OPEN-ENCRYPTED` |      20 | discard 3           |
| `M-PAGINATION-BACKWARD`     |      20 | discard 3           |
| `M-RECONNECT-SETTLE`        |      10 | offline 15 seconds  |
| `M-MEDIA-OPEN`              |      20 | discard 3           |
| `M-IDLE-CPU`                |       5 | settle 60 seconds   |
| `M-MEMORY-LARGE-ACCOUNT`    |       5 | settle 120 seconds  |
| `M-DISK-GROWTH`             |       5 | scenario 10 minutes |

Every series reports nearest-rank p50/p95, minimum, maximum, and mean.

## R0.2 readiness checks

`AC-R02C-FX64-BOUNDARY` is `blocked`; `AC-R02C-TOPOLOGY-TWIN` is `pending`.
Both review by `2026-08-02T00:00:00Z` and keep Phase 0 open.

| ID                                  | Producer/output, validation, expected result                                                                                                                                                                                     | Owner / independent authority / scope / dependencies                                                                    |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `AC-R02C-FX64-BOUNDARY` / `blocked` | After producer implementation, seed/inspect v2; current runner bytes are not a producer. Future `.../AC-R02C-FX64-BOUNDARY.json`; assert 2 rooms, 0 spaces, 128 total, 64/room, all boundaries; expect no secrets/IDs/bodies.    | fixture owner / independent R0.2-C reviewer; disposable current-JS control; depends on `FX-64-CONTROL`; readiness open. |
| `AC-R02C-TOPOLOGY-TWIN`             | Compare all normative IDs/paths/states/counts/UTC dates/owners/reviewers/authorities in JSON/Markdown; strict parse, path inventory, semantic review, Prettier, diff-check; expect no duplicate/parse/path/format/twin mismatch. | R0.2-C implementer / independent R0.2-C reviewer; four governed files; depends on this artifact; Phase 0 open.          |

## Acceptance boundary

An independently reviewed R0.2 result is only
`completeness_and_execution_readiness_only`. Formal Phase 0 acceptance belongs
to the independent R0.8 reviewer after a full exact-head rerun. Current gate is
`open`; unreviewed deferrals/residuals never count, and self-acceptance is
forbidden.

## Normative literal parity appendix

These JSON scalar values are normative and reproduced verbatim:

- `corresponding lane owner`
- `planned_or_produced_without_independent_review_remains_open`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-STATIC.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-UNIT.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-CONTRACT.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-PROPERTY.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-INTEGRATION.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-STATIC-UNIT.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-JS-CONTROL.json`
- `open_until_accepted; never proves Rust SDK behavior`
- `future output src-tauri/tests/matrix_rust_live_harness.rs and docs/matrix-rust-sdk/evidence/phase0/L-RUST-HARNESS.json`
- `independent Matrix Rust reviewer`
- `open; cannot create production alternative backend`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-PACKAGED-MACOS.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-PACKAGED-LINUX.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-PACKAGED-MACOS-LINUX.json`
- `macOS and Linux platform owners`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-PERFORMANCE-SOAK-FAULT.json`
- `performance and reliability owners`
- `open; separate Phase 13 soak remains 24 hours`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-MIXED-IOS.json`
- `open; future assigned phase only`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-MANUAL.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-FAULT.json`
- `future output docs/matrix-rust-sdk/evidence/phase0/L-SOAK.json`
- `No end-to-end metric is accepted; the synthetic timeline-map proxy cannot satisfy these rows.`
- `future output docs/matrix-rust-sdk/evidence/phase0/AC-R02C-FX64-BOUNDARY.json`
- `R0.2 readiness remains open until reviewed`
- `docs/matrix-rust-sdk/test-matrix-synapse-topology.json and docs/matrix-rust-sdk/test-matrix-synapse-topology.md`
- `Phase 0 remains open`
- `R0.8 independent reviewer after full exact-head rerun`
