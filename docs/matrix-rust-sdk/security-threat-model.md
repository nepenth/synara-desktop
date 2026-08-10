# Matrix Rust SDK replacement security threat model

| Field                      | Value                                                      |
| -------------------------- | ---------------------------------------------------------- |
| Artifact                   | `MRSDK-SEC-TM-001`                                         |
| Schema                     | 1                                                          |
| As of                      | 2026-07-25                                                 |
| Status                     | `ready_for_independent_review`                             |
| Machine-readable authority | [`security-threat-model.json`](security-threat-model.json) |

This is the security-governance baseline for the full replacement of
`matrix-js-sdk` with `matrix-sdk =0.18.0` on supported macOS and Linux desktop
builds. It records threats and ownership; it does **not** close an implementation
risk, accept P0.1–P0.7, or close a phase gate.

## Architecture states

| State                      | Allowed          | Meaning and security posture                                                                                                                                                                           |
| -------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `CURRENT_JS_ONLY`          | Yes, temporarily | The shipping product uses only `matrix-js-sdk`; the WebView and service worker still participate in token and media handling. This is pre-cutover exposure, not the target architecture.               |
| `ISOLATED_RUST_HARNESS`    | Yes, test-only   | Rust SDK foundation and disposable-Synapse probes are isolated and are not registered as a product backend. They must use isolated accounts/processes and never share a production token or device ID. |
| `TARGET_RUST_ONLY`         | Yes, required    | One Rust supervisor owns client, credentials, encrypted stores, sync, crypto, media, and lifecycle. The WebView receives bounded Synara-owned DTOs only.                                               |
| `ROLLBACK_PRIOR_BUILD`     | Yes, bounded     | Rollback means a prior product build plus intentionally retained inert legacy data. It is operationally controlled and time-bounded.                                                                   |
| `DUAL_BACKEND_OR_SELECTOR` | **No**           | Concurrent JS/Rust product clients, shared identity/session state, call-only hidden JS, or an in-app backend selector are forbidden.                                                                   |

An atomic cutover moves directly from `CURRENT_JS_ONLY` to
`TARGET_RUST_ONLY`. The harness never becomes an alternative product backend.

## Scope and trust assumptions

In scope are authentication/session material, SDK stores and native keyrings,
Tauri IPC, media, diagnostics, destructive lifecycle, filesystem confinement,
migration/rollback, widgets/calls, proxy configuration, dependencies, CI,
packaging, and supported-platform behavior.

This model does not promise confidentiality after full root/kernel compromise,
does not treat the test harness as shipping architecture, and does not design a
permanent backend abstraction. An OS credential-store compromise is a residual
platform risk; lack of a supported credential store must fail explicitly rather
than introduce a weaker fallback.

Threat actors and failure sources include malicious Matrix senders or servers,
untrusted media/widget content, compromised WebView code, same-user local
processes, symlink/TOCTOU attackers, stale asynchronous tasks, crashes and
partial disk/keyring failures, credential-bearing proxy configuration,
dependency/build compromise, and recipients of diagnostic data.

## Assets

| ID    | Classification        | Examples                                                     |
| ----- | --------------------- | ------------------------------------------------------------ |
| `A01` | Secret                | Passwords, access/refresh tokens, SSO tokens                 |
| `A02` | Secret                | Store, recovery, cross-signing, and backup keys              |
| `A03` | Highly sensitive      | Event plaintext, raw push payloads, decrypted media          |
| `A04` | Personal-sensitive    | Full user/device IDs and homeserver/proxy URLs               |
| `A05` | Personal-sensitive    | Absolute store/account/cache paths                           |
| `A06` | Integrity-critical    | Session generation, stream sequence, active-client ownership |
| `A07` | Availability-critical | Encrypted SDK stores, legacy IndexedDB, drafts, downloads    |
| `A08` | Restricted            | Health snapshots, errors, support exports, CI logs           |

“Non-secret” is not synonymous with “safe to diagnose.” Full identifiers, URLs,
and paths remain personal-sensitive and are excluded from diagnostic surfaces
unless a separately reviewed exception exists.

## Trust boundaries

| ID    | Boundary                                             | Required controls                                                                                                                                                                                                                                                                                                                                                                                   |
| ----- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `B01` | WebView → Tauri IPC                                  | Purpose-specific allowlisted commands/topics; typed/bounded payloads and presentation DTOs; no long-lived session tokens, store/recovery keys, raw push payloads, or decrypted media bytes; ephemeral password/one-time-token inbound auth only, never echoed/logged/persisted; sanitized presentation plaintext only on outbound rendering topics, never diagnostics; opaque bounded media handles |
| `B02` | IPC → Rust supervisor                                | Single owner; generation checks; cancellation/join; backpressure                                                                                                                                                                                                                                                                                                                                    |
| `B03` | Rust supervisor → homeserver/proxy                   | Parsed URL/TLS policy; no raw REST fallback; redacted failures                                                                                                                                                                                                                                                                                                                                      |
| `B04` | Rust → OS credential store                           | Scoped entries; native persistence; least privilege; explicit failure                                                                                                                                                                                                                                                                                                                               |
| `B05` | Rust → filesystem/SQLite/cache                       | Canonical confinement; restrictive permissions; serialized open; exact deletion                                                                                                                                                                                                                                                                                                                     |
| `B06` | Untrusted Matrix content → media/widget/call surface | Bounds, metadata validation, origin/CSP policy, cleanup                                                                                                                                                                                                                                                                                                                                             |
| `B07` | Runtime → diagnostics/support/CI                     | Producer allowlists; central hostile fixtures; no raw SDK errors or identifiers                                                                                                                                                                                                                                                                                                                     |
| `B08` | Current JS product → Rust product/rollback build     | Atomic cutover; inert legacy data; new device; exact cleanup                                                                                                                                                                                                                                                                                                                                        |
| `B09` | Application notification pipeline → OS/lock screen   | Typed/bounded notification DTO and IPC; preserve privacy end to end; encrypted/private/lock-screen redaction; no sensitive approval command preview; packaged/negative platform fixtures                                                                                                                                                                                                            |

## Non-negotiable security invariants

1. Exactly one Rust Matrix client owner exists after cutover. No production
   backend selector or dual client is permitted.
2. No long-lived Matrix session token, store/recovery key, raw push payload, or
   decrypted media bytes cross post-cutover WebView IPC or enter logs. An
   ephemeral login password or one-time login token may cross WebView → Rust
   only through a purpose-specific, one-shot, typed/bounded authentication
   command and is never echoed, logged, or persisted. Bounded, sanitized
   event/presentation plaintext may cross Rust → WebView only on allowlisted
   rendering topics and never enters diagnostics; media uses opaque bounded
   handles rather than bytes.
3. The old token/device ID is not copied into a fresh Rust store without a
   separately approved continuity threat review.
4. An existing encrypted store with a missing key fails recoverably. It never
   causes replacement-key generation or automatic wipe.
5. Store paths are absolute, safely rooted, symlink resistant,
   collision-resistant, account isolated, and deleted only by exact idempotent
   operations.
6. New work is rejected and tasks cancel/join before clients/stores close,
   credentials clear, or files are removed.
7. IPC uses cross-language-safe counters, checked sequencing, one authoritative
   stream identity, typed/bounded bodies, bounded queues, and stale-generation
   rejection.
8. Media bytes do not travel as JSON. Authenticated retrieval, decryption,
   cache bounds, metadata checks, revocation, and persistence policy are Rust
   owned.
9. A crash or partial migration never triggers automatic wipe. Legacy data
   stays inert and recoverable until exact approved cleanup.
10. Widgets/calls cannot keep `matrix-js-sdk` as a hidden exception and require
    explicit CSP/origin and experimental-feature review.

## Threat inventory

| ID       | Domain                | Scenario                                                                                                                                                                                                                                                           | Risks                        | Required controls                                                                                                                                                                                                                  |
| -------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TM-T01` | Tokens                | Credentials remain in WebView/localStorage/service-worker memory, cross IPC, or enter Rust without continuity review; fallback/session-scoped stores blur durability; callback tokens remain URL/history-visible or lack origin/replay/state/nonce/PKCE integrity. | R004, R012, R024, R028, R037 | Native vault/Rust custody; no WebView fallback; explicit platform recovery; consume-once/pre-async URL scrub; strict origin/replay/malformed rejection; state/nonce/PKCE; quit/crash/restart and hash/history-router hostile tests |
| `TM-T02` | Store/keyring         | A missing, replaced, leaked, or partially deleted key strands or exposes an encrypted store; swallowed credential-store/IndexedDB deletion failures retain state while logout appears complete.                                                                    | R012, R022, R023, R027, R028 | Native keyring; existing-store key-miss error; zeroization; failure-atomic lifecycle; verified fail-closed credential/store deletion                                                                                               |
| `TM-T03` | Filesystem            | Traversal, symlink substitution, collision, or TOCTOU redirects creation/deletion.                                                                                                                                                                                 | R017, R020, R021             | Canonical/no-follow confinement; versioned collision-resistant identity; adversarial tests                                                                                                                                         |
| `TM-T04` | IPC                   | Unsafe numbers, overflow, stream mismatch, unbounded JSON, or stale tasks corrupt or disclose state.                                                                                                                                                               | R006, R011, R018             | Safe counters; checked sequence; one stream ID; typed/bounded corpus                                                                                                                                                               |
| `TM-T05` | Media                 | Untrusted media metadata/content or decrypted handles cause disclosure, exhaustion, unsafe files, or persistence.                                                                                                                                                  | R011, R029                   | Native bounded cache/stream; metadata validation; TTL/revocation; no JSON bytes                                                                                                                                                    |
| `TM-T06` | Diagnostics           | Raw errors, URLs, IDs, paths, content, or secrets reach health/error/support surfaces.                                                                                                                                                                             | R012, R026, R035             | Producer allowlists; stable categories; hostile fixtures; no raw SDK strings                                                                                                                                                       |
| `TM-T07` | Lifecycle             | Logout/wipe destroys keys/files while work is live or leaves indeterminate partial state; swallowed deletion errors falsely complete and reload without visible recovery.                                                                                          | R016, R027                   | Close barrier; transactional states; ordered actions; failure injection/retry; verified deletion; no completion/reload on failure; visible recovery                                                                                |
| `TM-T08` | Migration/cutover     | Concurrent owners, reused identity, premature cleanup, or marker ambiguity loses data or duplicates actions.                                                                                                                                                       | R004, R007, R019, R025, R031 | Reauth/new device; atomic marker; inert retention; exact cleanup                                                                                                                                                                   |
| `TM-T09` | Rollback              | A selector or uncontrolled downgrade reopens legacy state while Rust is active.                                                                                                                                                                                    | R007, R019, R024, R031       | Prior-build-only rollback; time bound; no shared identity; runbook                                                                                                                                                                 |
| `TM-T10` | Widgets/calls         | Origins, CSP, experimental APIs, or hidden JS SDK can expose content/credentials; mismatched listener identities retain disposed call/widget state and process later events.                                                                                       | R002, R003, R030             | Origin allowlist; least privilege CSP; explicit feature approval; Phase 10 gate; stable callback/disposer identities; idempotent teardown; listener/no-delivery tests                                                              |
| `TM-T11` | Proxy/network         | Credential-bearing or malformed URLs leak or broaden network access.                                                                                                                                                                                               | R026, R033                   | Parsed URL policy; reject embedded credentials; redacted categories                                                                                                                                                                |
| `TM-T12` | Supply chain/platform | Toolchain drift, unstable features, missing keyring, or unvalidated packaging invalidates assumptions.                                                                                                                                                             | R001, R002, R015, R028       | Exact pins; cross-platform CI; dependency review; reviewed-SHA checks                                                                                                                                                              |
| `TM-T13` | Untrusted events      | Malformed/custom events lose required content or inject unsafe/unbounded content; forwarding republishes encrypted wire metadata or decryption-failure placeholders without a successful-clear-content gate.                                                       | R008, R036                   | Bounded raw-content DTO; hostile/golden fixtures; safe rendering; forwarding clear-state/approved-type gate; fail-closed mixed/multi-select forwarding                                                                             |
| `TM-T14` | Product parity        | Auth/search/migration gaps produce raw REST fallback or concealed JS retention.                                                                                                                                                                                    | R005, R009, R010, R019       | Owned parity rows; no raw fallback; zero-usage cutover gate                                                                                                                                                                        |
| `TM-T15` | Tauri capabilities    | Broad commands expose lifecycle, filesystem, or secret operations to the WebView.                                                                                                                                                                                  | R032                         | Purpose-specific commands; least privilege; negative invocation tests                                                                                                                                                              |
| `TM-T16` | Governance            | Stale plans, agent drift, divergence, or artifact-only claims bypass security gates.                                                                                                                                                                               | R013, R014, R015, R034       | Owned register; bounded packets; independent review; R0.8 reports                                                                                                                                                                  |
| `TM-T17` | Notification privacy  | Privacy metadata is lost before the OS boundary, or encrypted/private content and approval command previews reach OS/lock-screen surfaces.                                                                                                                         | R038                         | Preserve privacy end to end; encrypted/private/lock-screen redaction; no approval command preview; bounded DTO/IPC; packaged platform and negative IPC/OS fixtures                                                                 |

Risk IDs in this table expand to `MRSDK-Rnnn` in the authoritative
[`security-risk-register.json`](security-risk-register.json).

## Open decisions and safe defaults

| ID       | Decision / owner / due gate                                    | Safe default                                                                                 |
| -------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `TM-D01` | Final legacy retention; Product + P3.7/P14.2; due P3.7         | Retain inert data until the later of 30 days or two releases; no premature automatic cleanup |
| `TM-D02` | Cutover marker schema/privacy; P3.7 + R0.4/R0.6; due P3.7      | No raw user ID; restrictive permissions; versioned collision-resistant account ID            |
| `TM-D03` | Incomplete Rust store repair/discard; R0.4/R0.5/P3.7; due R0.5 | Stop recoverably, preserve all data, never auto-wipe                                         |
| `TM-D04` | Linux without Secret Service; R0.4 + Product; due R0.4         | Block persistent completion; no localStorage/file/keyutils fallback                          |
| `TM-D05` | Native media delivery/decrypted persistence; P7; due P7        | No byte JSON and no durable decrypted persistence                                            |
| `TM-D06` | Pseudonymous diagnostic identifiers; R0.6 + Privacy; due R0.6  | No stable account identifier                                                                 |
| `TM-D07` | Proxy credential support; R0.4/R0.6; due R0.6                  | Reject credential-bearing proxy URLs                                                         |
| `TM-D08` | `experimental-widgets`; P10; due P10                           | Disabled in production until explicit risk approval                                          |
| `TM-D09` | Rollback/downgrade bounds; P13.6/P14; due P13                  | Prior-build-only, time-bounded, no concurrently active Rust session                          |

## Validation and ownership

R0.2 makes these threats and owners explicit. It closes no technical finding.
R0.3 owns IPC correctness; R0.4 owns store confinement, identity, layout, and
keyring; R0.5 owns destructive lifecycle; R0.6 owns diagnostic privacy; R0.7
owns isolated live adapters/evidence; R0.8 owns formal acceptance. Later P3–P14
tasks own product capabilities and cutover.

No critical/high risk may cross its target gate without linked evidence. An
accepted residual requires an authorized approver, rationale, date, and expiry.
The current, target, and rollback states must be reassessed after material
architecture, dependency, platform, or data-flow changes.

## Primary source references

- [`matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md), especially Sections 3, 8, 11, 16, and 17
- [`review-2026-07-25.md`](review-2026-07-25.md), REV-001–REV-009
- [`migration-ux-decision.md`](migration-ux-decision.md)
- [`p1.6-architectural-guardrails.md`](p1.6-architectural-guardrails.md)
- [`desktop-secure-secret-storage-plan.md`](../desktop-secure-secret-storage-plan.md)
- `src-tauri/src/matrix/store/{identity,paths,key_vault}.rs`
- `src-tauri/src/matrix/client_builder/{config,open}.rs`
- `src-tauri/src/matrix/lifecycle/{logout,wipe}.rs`
- `src-tauri/src/matrix/ipc/{protocol,stream}.rs`
- `src-tauri/src/matrix/diagnostics/redact.rs`
- `synara/src/app/features/matrix-ipc/envelope.ts`
- `synara/src/app/pages/auth/login/Login.tsx`
- `synara/src/app/pages/client/ClientNonUIFeatures.tsx`
- `synara/src/app/plugins/call/CallEmbed.ts`
- `synara/src/app/platform/notifications.ts`
- `synara/src/app/platform/secrets.ts`
- `synara/src/app/state/sessions.ts`
- `synara/src/app/state/sessionPersistence.ts`
- `synara/src/{sw.ts,client/initMatrix.ts,app/matrix/media.ts}`
- `synara/src/app/utils/forward.ts`
