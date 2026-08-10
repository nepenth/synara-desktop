# Matrix Rust SDK replacement security risk register

| Field                      | Value                                                        |
| -------------------------- | ------------------------------------------------------------ |
| Artifact                   | `MRSDK-SEC-RISK-001`                                         |
| Schema                     | 1                                                            |
| As of                      | 2026-07-25                                                   |
| Status                     | `ready_for_independent_review`                               |
| Machine-readable authority | [`security-risk-register.json`](security-risk-register.json) |
| Threat model               | [`security-threat-model.md`](security-threat-model.md)       |

This is the human-readable twin of the authoritative JSON register. Its 38
risks retain all 19 risks from plan Section 16, map REV-001–REV-009, and include
risks found by the R0.2 security audit and independent follow-up review. R0.2
supplies governance and ownership; it does not close a technical risk.

## Governance

Statuses are `open`, `mitigating`, `blocked_on_decision`, `accepted`, `closed`,
and `superseded`. Critical/high technical risks remain unresolved with status
`open`, `mitigating`, or `blocked_on_decision`; they may not become `accepted`,
`closed`, or `superseded` without independently reviewed closure evidence and
documented authority. An `accepted` residual requires the approver, rationale,
date, and expiry.

A dated review finding can be remediated while the corresponding systemic risk
continues. In particular, REV-008 is accepted as an R0.1 finding disposition,
but CI, branch-policy, divergence, and agent-governance risks remain ongoing.

## Plan Section 16 risks

| ID           | Severity / status              | Risk                                                                  | Owner; tasks; target                                                           | Closure criteria and evidence target                                                                                                                                                                                                                                                                         |
| ------------ | ------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `MRSDK-R001` | High; `mitigating`             | Rust 1.93 and cross-platform release-toolchain compatibility          | Platform/release; P0.5, P1.1, R0.2, R0.8; Phase 0                              | Immutable macOS/Linux build/package evidence covers Rust 1.93, Tauri 2, exact SDK, signing/notarization assumptions, runners, and caches; `toolchain-compatibility-report` plus green platform CI                                                                                                            |
| `MRSDK-R002` | High; `open`                   | `matrix-sdk-ui` and widget API stability                              | Calls/widgets capability; P0.3, P10.1, P10.2, P10.7; Phase 10                  | Exact pins and compile/live probes cover enabled experimental APIs with explicit approval and upgrade/exit policy; capability dossier plus Phase 10 report                                                                                                                                                   |
| `MRSDK-R003` | Critical to cutover; `open`    | Element Call and MatrixRTC parity                                     | Calls/widgets product; P10.1–P10.7; Phase 10/cutover                           | Packaged macOS/Linux parity, CSP/origin review, cleanup tests, and no hidden JS SDK exception                                                                                                                                                                                                                |
| `MRSDK-R004` | Critical data/identity; `open` | Legacy JS IndexedDB crypto store and identity cannot be reused safely | Session migration/crypto; P3.2, P3.5, P3.7, P8.5, P8.7, P11.8; cutover         | Reauthentication/new device/recovery pass; no guessed or silently copied token/device ID; ADR plus live recovery/cutover tests                                                                                                                                                                               |
| `MRSDK-R005` | High; `mitigating`             | Large `matrix-js-sdk` product dependency surface                      | Migration orchestrator; P0.1, P0.2, P11.1–P11.3, P11.9, P14.1; Phase 14        | AST inventory reaches zero product imports/ownership; inventory plus zero-usage guardrail report                                                                                                                                                                                                             |
| `MRSDK-R006` | High; `open`                   | IPC ordering and backpressure corrupt state                           | IPC contract; R0.3, P4.1–P4.3; R0.3/Phase 4                                    | Frozen contract proves checked sequence, stale rejection, bounded queues, deterministic resync, and cancellation; cross-language property/fault tests                                                                                                                                                        |
| `MRSDK-R007` | Critical; `open`               | Concurrent Matrix owners or a production backend selector             | Cutover architecture; P11.1, P11.8, P11.9, P14.1; cutover                      | Runtime proves one Rust supervisor, no shared identity/selector, and JS stops before Rust activation; guardrails plus cutover tests                                                                                                                                                                          |
| `MRSDK-R008` | High; `open`                   | Custom agent events lose required raw content                         | Agent-event/timeline; P6.4, P12.1, P12.4; Phase 12                             | Bounded owned raw-content DTO passes desktop/iOS hostile/golden fixtures without SDK objects or arbitrary JSON                                                                                                                                                                                               |
| `MRSDK-R009` | High; `open`                   | UIA/registration gaps encourage unsafe fallback                       | Authentication capability; P0.3, P3.3, P3.4; Phase 3                           | Exact SDK/upstream path passes live UIA tests with no raw REST fallback                                                                                                                                                                                                                                      |
| `MRSDK-R010` | Medium; `open`                 | Search semantics diverge                                              | Search feature; P6.8, P12.6; Phase 12                                          | Approved server/local strategy and filters/pagination/cancellation/stale-result parity tests pass                                                                                                                                                                                                            |
| `MRSDK-R011` | High; `open`                   | Media transport leaks data or exhausts memory                         | Native media; P7.1–P7.7, P11.6; Phase 7/cutover                                | Authenticated native retrieval/decryption uses bounded cache/handles, validates metadata, revokes access, and sends no JSON bytes                                                                                                                                                                            |
| `MRSDK-R012` | Critical; `open`               | Store keys, session secrets, or diagnostics leak                      | Secret-store/diagnostic privacy; R0.4, R0.6, P13.5; R0.6/final security review | Native vault, secret-aware types, producer allowlists, hostile fixtures, and scans prove prohibited material never reaches files/IPC/UI/logs                                                                                                                                                                 |
| `MRSDK-R013` | High; `mitigating`             | Long-lived integration branch diverges from main                      | Orchestrator; ongoing, R0.8, P14.6; every gate                                 | Each reconciliation gets semantic review and affected gate rerun; closes only when branch merges/retires                                                                                                                                                                                                     |
| `MRSDK-R014` | High; `mitigating`             | Implementation agent strays or masks failures                         | Orchestrator/reviewer; R0.2 and ongoing; every task PR                         | Bounded packet/template, stop conditions, independent diff review, correction loop, and reproduced validation appear in every task report                                                                                                                                                                    |
| `MRSDK-R015` | High; `mitigating`             | CI cancellation or branch policy permits unvalidated merge            | CI/repository administrator + orchestrator; R0.1, R0.8, ongoing; every merge   | PRs #77/#78, the workflow checker, and post-#77 verification prove current strict/up-to-date Quality gate + Desktop package gate requirements, administrator enforcement, conversation resolution, and disabled force-push/delete; settings can drift and each merge still needs reviewed-SHA green evidence |
| `MRSDK-R016` | Critical data loss; `open`     | Store deletion races a live client/task                               | Lifecycle supervisor; R0.5; Phase 2                                            | Ordered reject → cancel/join → close → clear key/credentials → exact wipe passes live-handle and failure tests; maps REV-001                                                                                                                                                                                 |
| `MRSDK-R017` | High; `open`                   | Store paths escape via traversal, symlinks, or races                  | Store confinement; R0.4; Phase 2                                               | Absolute canonical/no-follow confinement survives malicious roots/children/replacement races and exact-delete tests; maps REV-002                                                                                                                                                                            |
| `MRSDK-R018` | High; `open`                   | IPC integer, stream identity, and body ambiguity                      | IPC contract; R0.3; Phase 1                                                    | Rust/TS reject identical max/overflow/unsafe-integer/mismatch/unbounded/secret/media corpus; maps REV-004/005                                                                                                                                                                                                |
| `MRSDK-R019` | High; `open`                   | Migration scaffolding or JS compatibility becomes permanent           | Cutover/cleanup; P11.9, P14.1–P14.3; Phase 14                                  | Zero-use and deletion review remove SDK, selector, service-worker auth, bridges, and migration code or give a bounded approved deletion gate                                                                                                                                                                 |

## Additional review and threat-model risks

| ID           | Severity / status                                 | Risk                                                                                                   | Owner; tasks; target                                                                     | Closure criteria and evidence target                                                                                                                                                                                                                |
| ------------ | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MRSDK-R020` | High; `open`                                      | FNV-1a account directory identity can collide                                                          | Store identity; R0.4                                                                     | Versioned normalized identity uses collision-resistant digest and adversarial isolation/migration tests pass; maps REV-007                                                                                                                          |
| `MRSDK-R021` | High; `open`                                      | Declared store layout differs from SDK-owned directories                                               | Store-layout; R0.4, R0.7; Phase 2                                                        | Exact SDK ownership specification matches live encrypted open/reopen on both platforms; maps REV-006                                                                                                                                                |
| `MRSDK-R022` | Critical data loss; `open`                        | Existing store plus missing key may generate a replacement key                                         | Store-key lifecycle; R0.4, R0.5; Phase 2                                                 | Key miss returns recoverable locked/corrupt state, preserves data, never generates a replacement, and passes retries                                                                                                                                |
| `MRSDK-R023` | High; `open`                                      | Store passphrase copies persist outside secret-aware memory                                            | Store-key implementation; R0.4                                                           | Exact SDK boundary is documented; supported copies zeroize; Debug/log/IPC hostile tests prove no escape                                                                                                                                             |
| `MRSDK-R024` | High; `open`                                      | Legacy WebView fallback/service worker retain credentials while platform persistence durability varies | Authentication/cutover; P3.5, P7.7, P11.1, P11.6, P14.1; cutover                         | Separate secure native restore from weaker fallback/platform durability; delete WebView/service-worker credentials; require explicit recovery when durable native storage is unavailable; pass quit/crash/OS-restart and negative IPC/storage tests |
| `MRSDK-R025` | Critical data loss/privacy; `blocked_on_decision` | Legacy cleanup, retention, and marker are incomplete or over-broad                                     | Product/migration/privacy; P3.7, P14.2; P3.7                                             | Approve retention/marker; exact idempotent cleanup preserves unrelated data and survives interruption                                                                                                                                               |
| `MRSDK-R026` | High; `open`                                      | Raw SDK errors, URLs, IDs, and paths reach diagnostics                                                 | Diagnostic privacy; R0.6                                                                 | Producer allowlists and hostile tests exclude full ID/URL/path/raw SDK/secret/content; maps REV-003                                                                                                                                                 |
| `MRSDK-R027` | Critical data loss; `open`                        | Wipe and credential deletion are not failure-atomic                                                    | Lifecycle state machine; R0.5                                                            | Verify credential-store and Matrix IndexedDB deletion fail closed; expose recoverable retry/support state; never report completion, reload, or auto-wipe on failure; inject each failure independently/together; maps REV-001                       |
| `MRSDK-R028` | Critical; `open`                                  | Rust store/session vault lacks production OS credential-store integration                              | Native secret store; R0.4, R0.7; Phase 2                                                 | Keychain/Secret Service adapters persist/reopen scoped material and fail explicitly without insecure fallback                                                                                                                                       |
| `MRSDK-R029` | High; `open`                                      | Native media security/lifecycle, including Blob URL reclamation, is unspecified                        | Native media; P7.1–P7.7; Phase 7                                                         | Exactly-once reclamation for encrypted-only thumbnail/image producers, clear/encrypted video/audio/PDF/file/file-header producers, and local upload previews across every terminal condition; retain URL/metadata/cache/persistence controls        |
| `MRSDK-R030` | High; `open`                                      | Widget/call origins/CSP, experimental APIs, or incomplete teardown expand trust and state lifetime     | Calls/widgets security; P10.2, P10.3, P10.6, P10.7, P11.7; Phase 10                      | Stable callback/disposer identities and idempotent teardown pass listener-count/no-post-dispose tests across every lifecycle; origin/CSP/capability/secret/experimental controls still pass packaged tests                                          |
| `MRSDK-R031` | Critical data/identity; `blocked_on_decision`     | Crash, incomplete migration, downgrade, or rollback reopens/destroys state                             | Migration/release; P3.7, P13.2, P13.6, P14.2; P3.7/Phase 13                              | Approved policies pass crash/prior-build/downgrade tests with inert legacy data, no concurrent clients, no auto-wipe, bounded retention                                                                                                             |
| `MRSDK-R032` | High; `open`                                      | Broad Tauri Matrix commands/capabilities expose privileged operations                                  | Tauri IPC/security; P11.1, P11.6, P13.5, R0.8; Phase 11/final                            | Only purpose-specific bounded commands exist; least-privilege capabilities and unauthorized/malformed invocation tests pass                                                                                                                         |
| `MRSDK-R033` | High; `open`                                      | Proxy URLs contain credentials, unsafe schemes, or diagnostic data                                     | Network config/privacy; R0.4, R0.6                                                       | Parsed scheme/host policy rejects credentials by default and diagnostic tests expose no raw URL                                                                                                                                                     |
| `MRSDK-R034` | High; `open`                                      | Mandatory governance/live-evidence gates were bypassed                                                 | R0 orchestrator; R0.2, R0.7, R0.8                                                        | Every Section 17 artifact is reviewed; P0.5/P0.6 residuals are owned; live Phase 2/P3.1 evidence passes; formal reports close only supported gates; maps REV-009                                                                                    |
| `MRSDK-R035` | High; `open`                                      | Personal identifiers and paths are classified as harmless/non-secret                                   | Privacy/diagnostics; R0.4, R0.6                                                          | Classification treats full IDs, URLs, and paths as personal-sensitive and hostile fixtures exclude them; maps REV-003                                                                                                                               |
| `MRSDK-R036` | Medium; `open`                                    | Forwarding accepts undecrypted or decryption-failure encrypted events                                  | Composer and forwarding owner; P6.1, P7.4, P13.5; P6.1/P7.4 and final security review    | Require successful clear content and an approved type/field allowlist; fail closed for mixed/multi-select batches; tests prove encrypted wire fields are not republished while required encrypted-attachment descriptors remain intact              |
| `MRSDK-R037` | High; `open`                                      | Authentication callback tokens remain in URL state without a complete integrity policy                 | Authentication and session owner; P3.2, P3.3, P13.5; P3.2/P3.3 and final security review | Consume once and scrub URL/history before async/UI work; prohibit referrer/history/log/diagnostic persistence; validate origin/state/nonce/PKCE and reject replay/malformed callbacks with hash/history-router hostile tests                        |
| `MRSDK-R038` | Medium; `open`                                    | OS notification privacy metadata and approval-content redaction are not enforced end to end            | Notification and privacy owner; P9.5, P13.5; P9.5 and final security review              | Preserve privacy through DTO/IPC/native/OS boundaries; apply encrypted/private/lock-screen redaction; exclude approval command previews; pass packaged platform and negative IPC/OS fixtures                                                        |

### Verified current manifestations and exact closure conditions

- `MRSDK-R024`: At integration SHA `2aa6d96f9b63aad64a14feac23df2f694857be85`,
  `createLocalStorageSessionStore` deliberately writes the access token, device
  ID, user ID, base URL, and session generation to WebView `localStorage` when
  native persistence is unavailable. Platform status separately documents
  session-scoped stores that may not survive restart and Windows builds without
  native session persistence. Secure native restore, weaker WebView fallback,
  and platform durability are distinct behaviors. The fallback makes
  credentials available to the WebView context and is weaker under XSS; it
  predates the Rust replacement, is not newly introduced by Rust, and is not
  evidence of observed disclosure. Closure removes credential-bearing WebView
  and service-worker storage after cutover and requires every supported platform
  to use approved durable native restore or fail explicitly into approved
  reauthentication/recovery. Packaged evidence covers clean quit/relaunch,
  crash/relaunch, OS restart, unavailable/locked native storage, and explicit
  unsupported Windows persistence, plus storage scans and credential-negative
  IPC tests.
- `MRSDK-R027`: At integration SHA `2aa6d96f9b63aad64a14feac23df2f694857be85`,
  `clearPersistedSessions` catches native credential-store removal and Matrix
  IndexedDB deletion failures, records completion, and returns success;
  `performLogout` then continues clearing other state and reloads. A failed
  logout or wipe can therefore leave credential or Matrix store data behind
  without a visible recoverable failure while the UI behaves as though cleanup
  completed. This is a false-completion and retained-state risk, not evidence
  of observed disclosure. Closure requires verified, fail-closed deletion,
  visible recovery and retry/support paths, no completed result or reload on
  failure, state sufficient for idempotent retry, and injected native-store and
  IndexedDB failures independently and together. Evidence belongs in the R0.5
  failure matrix plus `sessionPersistence.ts` and `initMatrix.ts` tests.
- `MRSDK-R029`: Current Matrix media consumers create Blob URLs without
  reclaiming them. `ThumbnailContent` and `ImageContent` do so only for
  encrypted media; their clear paths use resolved HTTP URLs. `VideoContent`,
  `AudioContent`, PDF/file rendering and fallback download, and `FileHeader`
  fallback download create Blob URLs for both clear and encrypted media. Local
  image/video upload-metadata extraction also creates Blob URLs independently
  of eventual room encryption. The existing `useObjectURL` cleanup hook is not
  used by these producers. Closure requires instrumented exactly-once release
  for every actual producer on replacement, retry, failure, viewer close,
  unmount, cancellation, and completed fallback save; document/process exit is
  not cleanup. Clear thumbnail/image HTTP URLs do not require Blob revocation
  but remain subject to URL/authentication lifecycle policy. URL, size,
  content/MIME, filename/disposition, encrypted metadata, bounded cache/temp,
  and no-unintended-decrypted-persistence tests remain required. This defect
  retains resources and can extend decrypted-Blob lifetime, but does not
  establish disk or network disclosure.
- `MRSDK-R030`: At integration SHA `2aa6d96f9b63aad64a14feac23df2f694857be85`,
  `CallEmbed.start` registers four Matrix listeners with fresh
  `this.method.bind(this)` callbacks, while `dispose` passes different fresh
  bound callbacks to `off`. Identity-based removal therefore fails to detach
  the registrations, potentially retaining disposed call/widget state and
  processing later Matrix events. This proves a teardown and post-dispose
  processing defect, not credential or content disclosure. Closure requires
  stable callback or stored disposer identities and exact, idempotent teardown.
  Tests must return listener counts to baseline and prove no post-dispose
  delivery across hangup/replacement, the approved room-navigation lifecycle,
  logout, and window close. Existing origin, CSP, capability, secret-handling,
  experimental-feature, and packaged acceptance controls remain required.
- `MRSDK-R036`: The current forwarding helper reads `MatrixEvent.getContent()`
  and spreads its result into a new `m.room.message` without requiring
  successful clear content and an approved clear event type. Raw, in-progress,
  or unprocessed `m.room.encrypted` events can republish wire `algorithm`,
  `ciphertext`, `sender_key`, `device_id`, and `session_id`; completed
  decryption failures can forward `m.bad.encrypted` diagnostic placeholder
  content. Successfully decrypted events forward clear content. Closure rejects
  both unsafe states, allowlists approved clear types/fields, fails closed for
  mixed/multi-select batches, and tests encrypted and unencrypted targets.
  Evidence must include the successful/raw/in-progress/failed-decryption,
  allowlist, encrypted-attachment, and mixed-batch cases in
  `synara/src/app/utils/__tests__/forward.test.ts`; single/batch integration
  tests through `Message.tsx` and `RoomTimeline.tsx`; and P13.5 proof that the
  listed wire fields are not republished.
  Standards-required encrypted-attachment `file` descriptors from successful
  clear content remain preserved, subject to unencrypted-target confirmation;
  this risk is not a claim of plaintext disclosure, Megolm-key disclosure, or
  unintended attachment-key disclosure, and does not mandate attachment
  re-upload/re-encryption.
- `MRSDK-R037`: At integration SHA `2aa6d96f9b63aad64a14feac23df2f694857be85`,
  `Login.tsx` reads `loginToken` from the browser URL, rewrites it into the login
  URL for hash-router handling, and passes it to `TokenLogin`, with no
  URL/history scrub in that flow. This extends token lifetime in
  URL/history-visible state and leaves strict callback-origin, replay,
  malformed-token, and target OAuth/OIDC state/nonce/PKCE integrity policy
  unspecified; it is not evidence of observed account compromise. Closure
  consumes once and scrubs URL/history before asynchronous work or UI rendering,
  prohibits referrer/history/log/diagnostic/storage persistence, validates
  strict callback origin and state/nonce/PKCE, and rejects replay/malformed
  callbacks. Evidence includes hash-router, history-router, and hostile callback
  tests plus P13.5 token-negative scans.
- `MRSDK-R038`: At integration SHA `2aa6d96f9b63aad64a14feac23df2f694857be85`,
  `showPlatformNotification` normalizes a privacy field but omits it from the
  desktop request, while agent approval notifications can append
  `commandPreview` to the OS-visible body. The OS boundary can lose caller
  privacy intent and receive sensitive approval command text. This is a privacy
  policy/data-minimization defect, not evidence of observed disclosure. Closure
  preserves validated privacy end to end across DTO, IPC, native adapter, and OS
  presentation; redacts encrypted-room/private-mode/lock-screen content; and
  excludes sensitive approval command previews. Packaged macOS/Linux tests and
  negative IPC/OS fixtures must prove the policy.

## Dated review-finding disposition

Finding disposition answers whether the precise 2026-07-25 observation was
remediated. It does not replace the systemic risk status above.

| Finding   | Severity | Disposition                         | Risks / owner           | Evidence or remaining work                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --------- | -------- | ----------------------------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `REV-001` | Critical | `open`                              | R016, R027 / R0.5       | Destructive ordering and failure atomicity require code/tests                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `REV-002` | High     | `open`                              | R017 / R0.4             | Canonical symlink-safe confinement remains unimplemented                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `REV-003` | High     | `open`                              | R026, R035 / R0.6       | Raw diagnostic data and classification remain technical defects                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `REV-004` | High     | `open`                              | R018 / R0.3             | Safe counters and checked overflow remain unimplemented                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `REV-005` | High     | `open`                              | R018 / R0.3             | Authoritative stream identity and bounded bodies remain unimplemented                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `REV-006` | Medium   | `open`                              | R021 / R0.4, R0.7       | Exact layout and live reopen proof remain outstanding                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `REV-007` | Medium   | `open`                              | R020 / R0.4             | Collision-resistant account identity remains unimplemented                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `REV-008` | High     | `mitigated_and_accepted_under_R0.1` | R013–R015 / ongoing     | [PR #77](https://github.com/nepenth/synara-desktop/pull/77), [PR #78](https://github.com/nepenth/synara-desktop/pull/78), merge commits `f7288b0` and `2aa6d96`, status ledger, workflow-policy checker, and independent post-#77 verification of strict/up-to-date required Quality gate + Desktop package gate checks, administrator enforcement, conversation resolution, and disabled force-push/delete. Residual: settings can drift and every future merge still requires exact-SHA non-cancelled evidence. |
| `REV-009` | High     | `open`                              | R034 / R0.2, R0.7, R0.8 | R0.2 artifacts alone do not supply live evidence or formal acceptance                                                                                                                                                                                                                                                                                                                                                                                                                                             |

## Unresolved decisions

| ID         | Status                | Subject / owner / due gate                                 | Safe default                                                              |
| ---------- | --------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------- |
| `RISK-D01` | `blocked_on_decision` | Legacy retention; Product + P3.7/P14.2; P3.7               | No automatic cleanup before later of 30 days or two releases              |
| `RISK-D02` | `blocked_on_decision` | Marker privacy/account identifier; P3.7 + R0.4/R0.6; P3.7  | No raw user ID; restrictive permissions; versioned collision-resistant ID |
| `RISK-D03` | `blocked_on_decision` | Incomplete store/missing key; R0.4/R0.5; R0.5              | Preserve data; recoverable error; no replacement key or auto-wipe         |
| `RISK-D04` | `blocked_on_decision` | Linux without Secret Service; R0.4 + Product; R0.4         | Block persistent completion; no insecure fallback                         |
| `RISK-D05` | `blocked_on_decision` | Native media/decrypted persistence; P7; P7                 | No JSON bytes or durable decrypted persistence                            |
| `RISK-D06` | `blocked_on_decision` | Proxy credentials/diagnostic IDs; R0.6 + Privacy; R0.6     | Reject credentials; no stable account identifier                          |
| `RISK-D07` | `blocked_on_decision` | Experimental widgets and rollback bounds; P10/P13/P14; P10 | Widgets disabled until approved; rollback prior-build-only and bounded    |

## R0.2 versus technical closure

R0.2 may make `MRSDK-R034` ready for later closure by delivering the missing
governance artifacts and owned residuals. It cannot close R034 because R0.7
live evidence and R0.8 formal acceptance remain required. All technical risks
owned by R0.3–R0.7 or later phases stay open/blocked/mitigating exactly as shown.

Primary sources are plan Section 16, the full finding and remediation sections
of [`review-2026-07-25.md`](review-2026-07-25.md),
[`migration-ux-decision.md`](migration-ux-decision.md), and the authoritative
[`security-threat-model.json`](security-threat-model.json). The verified
manifestations were traced to `synara/src/app/state/sessions.ts`,
`synara/src/app/platform/secrets.ts`,
`synara/src/app/state/sessionPersistence.ts`, `synara/src/client/initMatrix.ts`,
`synara/src/app/pages/auth/login/Login.tsx`,
`synara/src/app/pages/client/ClientNonUIFeatures.tsx`,
`synara/src/app/platform/notifications.ts`,
`synara/src/app/plugins/call/CallEmbed.ts`, `synara/src/app/matrix/media.ts`, the
media consumers named above, `synara/src/app/utils/forward.ts`, and its existing
`utils/__tests__/forward.test.ts` coverage.
