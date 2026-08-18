# Matrix Rust SDK Full Replacement Plan

Date: 2026-07-25

> **Historical migration plan.** The replacement has since landed on `main`.
> The branch, runtime, queue, and acceptance statements below describe the
> migration program at its recorded snapshots, not the current product. See
> [the codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md) and
> [the 2026-08-17 local proof](shared-native-core/15-2026-08-17-local-proof.md)
> for the current architecture and latest consolidated evidence.

Status: approved direction; current delivery, acceptance, and runtime state are
recorded only in the canonical status ledger below

Integration branch: `feature/matrix-rust-sdk-full-replacement`

Current execution record: [`docs/matrix-rust-sdk/README.md`](matrix-rust-sdk/README.md)

<!-- matrix-rust-program-status-link -->

Canonical current status: [`docs/matrix-rust-sdk/program-status.md`](matrix-rust-sdk/program-status.md)
(generated from [`program-status.json`](matrix-rust-sdk/program-status.json)).
Task evidence remains historical; only this ledger records current delivery and
strict-acceptance state.

Independent review and rebaseline:
[`docs/matrix-rust-sdk/review-2026-07-25.md`](matrix-rust-sdk/review-2026-07-25.md)

Target upstream release: `matrix-sdk-0.18.0`

Upstream source commit: `1c44fb66214667c6d00acaf72ab592493653708b`

## 2026-07-25 audited execution baseline

The independent review of integration commit `edfefee` supersedes earlier
handoff claims that Phases 0–2 and P3.1 were complete.

This table is the immutable audit baseline, not a live PR ledger. For progress
after `edfefee`, use the canonical status ledger and the current live snapshot in
[`README.md`](matrix-rust-sdk/README.md).

| Band        | Artifact inventory                 | Acceptance state                                                                                                                     |
| ----------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Phase 0     | P0.1–P0.7 landed                   | **Gate open** — cross-platform/live evidence, full traceability, and mandatory planning artifacts remain incomplete                  |
| Phase 1     | P1.1–P1.6 landed                   | **Gate open** — fmt/lint/clippy/CI fail and IPC correctness issues are unresolved                                                    |
| Phase 2     | P2.1–P2.6 harness landed           | **Gate open** — native keyring/live lifecycle evidence is absent; critical/high lifecycle, path, and privacy findings are unresolved |
| Phase 3     | P3.1 domain/mock foundation landed | **P3.1 acceptance open** — live SDK adapter and disposable-homeserver proof remain                                                   |
| Phases 4–14 | No named tasks landed              | **Not started**                                                                                                                      |

“N/112” is only an inventory of named task artifacts, not validated completion.
Zero of 15 strict phase gates are closed. Remediation tasks R0.1–R0.8 from the
review report are additions to the original 112. **Product-first policy
(2026-07-27):** residual unaccepted formal R0 gates do **not** hard-block
capability slices such as P3.2+ on the integration branch; real safety findings
still must be fixed. Inventory growth measures Rust owner capability landing,
not dual-SDK product mode.

The status ledger must be updated as part of each accepted remediation PR;
changing artifact or merge state never closes a strict acceptance or phase gate
by implication.

The shipping desktop product still uses only `matrix-js-sdk`. No Rust production
login/sync backend, selector, dual-client runtime, or cutover has been accepted.

## 1. Decision

Replace `matrix-js-sdk` completely in the macOS/Linux desktop client and make
Matrix Rust SDK the only Matrix client SDK used by every Synara product client.
The iOS client already uses Matrix Rust SDK through
`matrix-rust-components-swift`; its remaining direct Matrix HTTP exceptions and
version alignment are included in this program.

This is a replacement, not a permanent backend abstraction and not an A/B
backend project. The production application must never offer a JavaScript-versus-
Rust SDK selector. The final runtime must never create both SDK clients for the
same signed-in session.

Implementation will happen on the integration branch and task branches derived
from it. Incomplete migration work must not merge to `main`. The final merge to
`main` occurs only after product confidence (CI + client testing) and the
completion gates in this document — never via a dual-SDK interim release.

**Canonical execution model (2026-07-27):** build the Rust Matrix owner as
**capability vertical slices** on the integration branch; keep product on
`matrix-js-sdk` only as temporary branch scaffolding until an **atomic sole-owner
cutover**; then burn down and remove `matrix-js-sdk`. Do **not** replace usage
file-by-file as an in-process package swap (frontend JS vs host Rust are
different boundaries — UI talks IPC/DTOs to the Rust owner). Clean-break
re-login / wipe of local Matrix state is acceptable; elaborate JS→Rust
session/token migration is out of scope. Full write-up:
[`docs/matrix-rust-sdk/cutover-operating-model.md`](matrix-rust-sdk/cutover-operating-model.md).

**Full-vertical deletion clarification (2026-07-28):** physical deletion now
happens in the owning capability vertical. Native wiring beside a retained JS
implementation is “wired,” not “done.” Each completed slice deletes the
superseded JS implementation/imports and records its count delta. Phase 11 is
therefore the final repository-wide convergence, cross-cutting cleanup, and
dependency-removal gate—not the primary backlog for capability deletion. See
[`full-vertical-policy.md`](matrix-rust-sdk/full-vertical-policy.md).

## 2. Outcomes

The completed system will have:

- one Matrix client owner per desktop process, hosted in the Tauri Rust process;
- native encrypted SQLite-backed Matrix state, event-cache, media-cache, and
  crypto storage;
- no browser-owned Matrix sync, crypto, session, or room state;
- no `matrix-js-sdk` package, import, type, model, store, event listener, or
  runtime initialization;
- a versioned Tauri IPC contract that sends Synara-owned DTOs rather than Rust
  SDK objects or JavaScript SDK-shaped objects;
- React views that render state and issue intentions without owning Matrix
  protocol behavior;
- Matrix Rust SDK-backed authentication, sync, room lists, timelines, media,
  E2EE, verification, recovery, room management, account data, search, spaces,
  threads, notifications, and Element Call/widget integration;
- iOS and desktop behavior checked against the same shared event/account-data
  fixtures and the same semantic acceptance tests;
- an explicit user-safe transition from legacy desktop sessions and crypto
  storage;
- no undocumented raw Matrix Client-Server requests in product runtime code.

## 3. Non-negotiable constraints

### 3.1 No dual production backend

- Do not add a user-visible or persistent SDK selection setting.
- Do not run `matrix-js-sdk` and Matrix Rust SDK simultaneously for one app
  session.
- Do not reuse one Matrix device ID concurrently from two SDK stores.
- Rust-core work may be exercised in tests and harnesses before cutover, but it
  must not become a second production sync loop.
- The desktop bootstrap cutover must be atomic: Rust becomes the sole Matrix
  owner, JavaScript initialization stops, and obsolete JavaScript runtime paths
  are deleted in the same cutover phase.
- **Transitional monorepo coexistence is allowed only on the integration
  branch:** product may still boot `matrix-js-sdk` while Rust harness code grows.
  That is scaffolding, not a dual-backend product. After sole-owner cutover,
  js-sdk importers and dependencies only decrease (burn-down), never grow.
- **Execution order:** capability vertical slices (auth → session → sync →
  timeline → crypto → …) with host ownership + IPC contracts first; UI rewires
  consume Synara DTOs only and delete their superseded JS owner in the same
  vertical. The ~220 baseline production `matrix-js-sdk` import sites are a
  capability-owned convergence checklist, not a random file-by-file rewrite
  order or a deferred bulk-deletion excuse. See
  [`cutover-operating-model.md`](matrix-rust-sdk/cutover-operating-model.md).

### 3.2 No compatibility clone of `matrix-js-sdk`

- Do not recreate `MatrixClient`, `Room`, `MatrixEvent`, `RoomMember`, or their
  event-emitter APIs in TypeScript.
- Do not expose Rust SDK or Ruma structures directly over IPC.
- Define minimal Synara-owned DTOs around product needs.
- DTO names and shapes must describe product meaning, not preserve accidental
  JavaScript SDK structure.

### 3.3 No silent fallback to raw REST

- Product runtime Matrix traffic must go through Matrix Rust SDK.
- Typed `Client::send` requests through the SDK are allowed when a higher-level
  API does not exist; direct `reqwest`, `URLSession`, `fetch`, XHR, or service-
  worker calls to `/_matrix/` are not.
- Operational audit scripts, test homeserver setup, and integration harness
  fixtures may use documented network exceptions outside product runtime code.
- An SDK capability gap is a planning blocker requiring an upstream issue,
  contribution, or consciously approved scope decision. It is not permission to
  add an untracked fallback.

### 3.4 Exact dependency pinning

- Desktop starts from exact crates `matrix-sdk = 0.18.0` and
  `matrix-sdk-ui = 0.18.0`.
- The upstream release requires Rust `1.93`; the current desktop manifest claims
  Rust `1.77.2`, so toolchain modernization is a mandatory prerequisite.
- Cargo dependencies and the Rust toolchain must be pinned and locked.
- Experimental features are disabled unless a named requirement cannot be met
  without them and a dedicated risk gate approves their use.
- `experimental-widgets` is expected to be required for Element Call and must
  receive its own exit/rollback criteria.
- No implementation task may upgrade the SDK, Rust toolchain, Tauri, or Swift
  bindings opportunistically.

### 3.5 No feature regression hidden as migration

- Existing supported behavior is the parity baseline.
- Missing behavior must be tracked explicitly; it may not be deleted because it
  is difficult to port.
- Intentional product changes require a separate decision and changelog entry.
- macOS and Linux remain supported. Windows remains outside the supported
  release matrix unless separately authorized.

### 3.6 One writer and explicit lifecycle

- Exactly one Rust actor owns each active Matrix `Client`, sync service, send
  queues, and store handles.
- Login, restore, logout, local wipe, app shutdown, crash recovery, network loss,
  and account switching must have explicit state transitions.
- UI unmounts and window recreation must not start or stop the SDK implicitly.
- Background tasks must be cancellable, joined, monitored, and unable to publish
  into a superseded session generation.

### 3.7 UI/UX fidelity and public-repo hygiene

- **UI/UX fidelity (high-fidelity mandate).** Replacing a capability must not
  change the app's look and feel. The native rewiring renders the same UI from
  the same Synara-owned DTOs; no visual redesign, layout/UX/copy change, or
  rendering-altering component swap is acceptable. A visual difference
  introduced by a slice is a defect, not decoration: file a named residual and
  fix forward. A deliberately requested design change is a separate task and is
  never smuggled into a migration slice (see
  `docs/matrix-rust-sdk/operating-instructions.md` §3 and
  `docs/matrix-rust-sdk/full-vertical-policy.md`).
- **Public-repo hygiene.** This repository is public. No tokens, keys,
  credentials, session/recovery material, private endpoints, or personal data
  may ever appear in committed artifacts, fixtures, examples, or logs. Public
  examples use placeholders (see
  `docs/matrix-rust-sdk/operating-instructions.md` §1).

## 4. Current baseline

The baseline must be regenerated in Phase 0 and committed as machine-readable
evidence. Current observations as of this plan:

- Desktop uses `matrix-js-sdk` `42.0.0`.
- Its JavaScript dependency graph includes
  `@matrix-org/matrix-sdk-crypto-wasm` `18.3.1`.
- 220 production files under `synara/src` directly import `matrix-js-sdk` or one
  of its internal modules; 12 test files do the same.
- Direct production imports are spread across:
  - 59 feature files;
  - 56 hook files;
  - 43 component files;
  - 19 page files;
  - 16 utility files;
  - 14 state files;
  - 8 plugin files;
  - 3 client-lifecycle files;
  - 1 media-boundary file;
  - 1 shared type file.
- The most common imported models are `Room` (88 files), `MatrixClient` (61),
  `MatrixError` (45), and `MatrixEvent` (35).
- Calls depend on internal `matrix-js-sdk/lib/matrixrtc/*` modules and on the
  embedded Element Call/widget bridge.
- Browser IndexedDB owns Matrix state and crypto stores.
- The service worker participates in authenticated Matrix media access.
- iOS uses `matrix-rust-components-swift` `26.06.06`, resolved at revision
  `ec3b2161ba371a13609e7181077d2f3baef188f5`.
- iOS still has documented direct Matrix HTTP exceptions for current-device
  naming and room read-marker/account-data lookup.

Baseline source documents:

- `docs/native-first-architecture-spike.md`
- `docs/matrix-sdk-alignment-audit.md`
- `docs/desktop-matrix-sdk-boundaries.md`
- `synara-ios/docs/matrix-sdk-alignment-audit.md`
- `docs/timeline-room-state-reliability-contract.md`

## 5. Authoritative upstream evidence

Planning and review must use the exact release rather than the moving `main`
branch.

Required evidence set:

- release: <https://github.com/matrix-org/matrix-rust-sdk/releases/tag/matrix-sdk-0.18.0>
- source commit: `1c44fb66214667c6d00acaf72ab592493653708b`
- core API docs: <https://docs.rs/matrix-sdk/0.18.0/matrix_sdk/>
- UI API docs: <https://docs.rs/matrix-sdk-ui/0.18.0/matrix_sdk_ui/>
- exact source for `crates/matrix-sdk`, `crates/matrix-sdk-ui`, and the FFI
  bindings at the pinned tag;
- exact Swift package source revision used by the iOS build;
- generated local `cargo doc` output from the locked desktop dependency graph;
- compile-time API probes in this repository;
- live two-client Synapse capability tests.

Documentation alone does not establish support. A capability is “confirmed”
only when all applicable evidence exists:

1. a stable or consciously accepted experimental API is present at the pinned
   source revision;
2. a minimal repository-owned probe compiles against the locked dependency;
3. a unit or integration test demonstrates required semantics;
4. a live Synapse test demonstrates network behavior when the capability is
   server-dependent;
5. UI acceptance demonstrates the end-to-end product behavior.

Every capability-matrix row must be marked one of:

- `confirmed-stable`;
- `confirmed-experimental`;
- `typed-sdk-request-required`;
- `upstream-change-required`;
- `not-required`;
- `blocked`.

No row may be left as “probably supported.”

## 6. Target architecture

### 6.1 Rust ownership

Create a focused Rust Matrix domain under `src-tauri/src/matrix/` (or a dedicated
workspace crate if Phase 1 proves that materially improves testability). It owns:

- client construction and homeserver discovery;
- encrypted store configuration and store-key retrieval;
- authentication and session lifecycle;
- sync service and room-list service;
- timeline creation, pagination, subscriptions, and destruction;
- send queues and upload handles;
- E2EE, recovery, backup, verification, and device state;
- media cache, downloads, uploads, and persistence;
- account data and custom Synara event encoding;
- room, member, profile, space, thread, search, notification, and widget APIs;
- translation from SDK/Ruma types to bounded Synara DTOs;
- error classification and privacy-safe diagnostics.

### 6.2 React ownership

React owns:

- presentation state and view composition;
- timeline virtualization and viewport policy;
- composer editing state and local drafts;
- commands representing user intent;
- platform-neutral route and selection state;
- view-only formatting derived from Synara DTOs;
- native bridge invocation through a single Matrix transport module.

React does not own:

- sync loops;
- Matrix event emitters;
- room or member model identity;
- crypto state machines;
- access/refresh tokens;
- Matrix storage;
- media authentication or decryption;
- receipt, relation, thread, or push-rule protocol semantics.

### 6.3 IPC protocol

The IPC protocol must be versioned from its first commit.

Required envelope fields:

- `protocolVersion`;
- `sessionGeneration`;
- `streamId` where applicable;
- monotonically increasing `sequence` per stream;
- `kind` discriminator;
- bounded payload;
- optional correlation/request ID;
- timestamp only where product semantics require it.

Required behaviors:

- snapshot followed by ordered deltas;
- stale-generation rejection;
- duplicate-delta idempotence;
- gap detection and explicit snapshot resubscription;
- deterministic unsubscribe and Rust resource cleanup;
- bounded queues and documented coalescing for high-frequency updates;
- cancellation for pagination, search, uploads, and long-running crypto work;
- no unbounded event history in WebView memory;
- no large media byte arrays serialized through JSON IPC;
- exhaustive TypeScript and Rust discriminated-union handling;
- schema/fixture compatibility tests on both sides.

### 6.4 Error model

Define stable Synara error categories rather than passing SDK strings:

- authentication rejected;
- user deactivated;
- interactive authentication required;
- forbidden/insufficient power;
- rate limited, including retry time;
- connectivity/offline;
- homeserver unavailable;
- unsupported homeserver capability;
- store locked/corrupt/unavailable;
- crypto/recovery/verification failure;
- media too large/unsupported/decrypt failed;
- cancellation;
- stale session generation;
- SDK invariant/internal failure;
- unknown, with a privacy-safe diagnostic identifier.

Tokens, credentials, recovery keys, event plaintext, raw push payloads, and
decrypted media must never appear in logs or UI error details.

## 7. Functional requirements and parity inventory

Phase 0 must turn this section into a row-by-row traceability matrix linking the
current implementation, Rust API, implementation task, automated test, and UI
acceptance case.

### 7.1 Authentication and account lifecycle

- homeserver/server-name discovery;
- password login;
- token login if still supported by product UI;
- SSO and OAuth callback handling;
- login-flow discovery;
- UIA stages used by login, registration, password reset, email, terms,
  registration token, dummy, and reCAPTCHA flows;
- refresh-token rotation and persistence;
- session restore after normal quit, crash, and OS restart;
- account switching without state crossover;
- logout and local wipe;
- device creation/naming as `Synara macOS`, `Synara Linux`, or `Synara iOS`;
- deactivated-account and invalid-token handling;
- server capability and version discovery.

### 7.2 Sync, room list, navigation, and state

- sync startup/readiness and progress reporting;
- offline and reconnect behavior;
- joined, invited, knocked, and left room states used by the UI;
- deterministic room-list ordering and activity updates;
- unread/highlight counts and marked-unread state;
- DM identification and `m.direct` maintenance;
- room summaries and heroes;
- favorites, low priority, folders, recent-room organization, and custom filters;
- spaces, hierarchy, parents, children, and space-filter semantics;
- room replacement/upgrades and tombstones;
- deep links to room, event, thread, Later, and notification anchors;
- route stability while room snapshots/deltas arrive;
- presence and typing state used by the product.

### 7.3 Timeline and event presentation

- initial focused timeline load;
- live timeline deltas;
- backward and forward pagination;
- event-focused opening and context loading;
- unread/read-marker positioning;
- stable anchor restoration with variable-height items;
- local echo, remote echo reconciliation, failure, retry, and cancellation;
- edits, redactions, reactions, replies, relations, and replacement events;
- encrypted, undecryptable, redacted, malformed, and unsupported events;
- membership and state events rendered by Synara;
- polls: creation, response, end, and result aggregation;
- threads: list, focused timeline, subscription, reply counts, and navigation;
- read receipts and event readers;
- custom Synara agent approval, room note, Later, and related event content;
- safe access to raw custom content without REST timeline fallback;
- date dividers, sender profiles, power-level-derived action availability, and
  room member hydration;
- no duplicate or reordered events after reconnect, pagination overlap, or
  local/remote echo transition.

### 7.4 Composer and message actions

- plain and formatted text;
- mentions and room mentions;
- replies and edits;
- reactions and redactions;
- file, image, video, audio, and voice messages;
- polls and custom agent events;
- send queue status, retry, cancel, and progress;
- typing notices;
- drafts remain local and survive the migration;
- slash/command behaviors preserve existing semantics;
- forwarding preserves compatible content and does not leak encrypted payloads.

### 7.5 Media

- authenticated thumbnail and original retrieval;
- encrypted attachment download and decryption;
- upload, thumbnail generation, progress, cancellation, retry, and local echo;
- cache policy and bounded disk usage;
- image/video/audio/PDF presentation;
- save, share, drag/drop, and file-open platform bridges;
- avatar and room-image loading;
- MIME type, filename, size, and content-disposition safety;
- object/local URLs revoked deterministically;
- no service-worker Matrix authentication after cutover;
- no decrypted media persisted unintentionally.

### 7.6 Room, profile, member, and permission management

- create room, create DM, join by ID/alias, leave, forget, invite, kick, ban,
  unban, and knock flows used by the product;
- name, topic, avatar, canonical alias, directory visibility, join rules,
  history visibility, guest access, and encryption enablement;
- room upgrades and predecessor/successor navigation;
- member profiles, display names, avatars, mentions, and membership;
- power-level reads, permission decisions, and updates;
- public-room search and user search;
- user profile reads and updates;
- ignore/unignore behavior.

### 7.7 Account data and custom contracts

- Later;
- room notes;
- unread anchors;
- event/room opening anchors;
- custom emoji and image packs;
- recent emoji;
- favorites, folders, and shared settings;
- any custom Synara account-data or message-like event;
- schema versioning, unknown-field preservation where required, and fixture
  compatibility with iOS.

### 7.8 Notifications

- push-rule reads and updates;
- per-room notification modes;
- keyword, mention, invite, and all-message preferences;
- desktop native notification generation;
- correct unread/badge summaries;
- event resolution and deep-link routing;
- notification suppression for active/focused contexts;
- privacy-safe encrypted notification behavior;
- iOS pusher registration/deletion and notification resolution remain SDK-owned.

### 7.9 Devices, E2EE, verification, and recovery

- encrypted store initialization before sync;
- cross-signing status;
- own-device and other-device lists;
- device trust and verification state;
- SAS verification and request inbox behavior;
- recovery setup, recovery key, backup enablement, restore, and repair;
- room-key import/export if retained by product UI;
- undecryptable-event retry;
- key-backup state listeners;
- device deletion and UIA;
- multiple accounts with fully isolated stores and keys;
- store continuity across upgrades and crashes;
- store corruption detection and non-destructive recovery guidance.

### 7.10 Search

- room message search and pagination;
- global message search if currently exposed;
- sender, room, date, and content filters used by the UI;
- event-context navigation from results;
- user and public-room search;
- explicit decision on server search versus experimental local search;
- search cancellation and stale-result rejection.

### 7.11 Calls and widgets

- current MatrixRTC membership state displayed by Synara;
- embedded Element Call startup, widget URL generation, capabilities, and
  postMessage transport;
- call join/leave/decline and member status;
- encryption-key to-device/event flow required by Element Call;
- session cleanup on room change, logout, and window close;
- CSP and origin restrictions;
- explicit risk acceptance for `experimental-widgets` at `0.18.0`;
- a documented contingency if upstream widget behavior cannot meet current call
  parity without reintroducing `matrix-js-sdk`.

The contingency may delay the final cutover or require an upstream patch. It may
not retain `matrix-js-sdk` as a hidden permanent call-only dependency without a
new user decision.

## 8. Session, store, and user-data transition

### 8.1 Safety position

The JavaScript IndexedDB state/crypto stores and native Rust SQLite stores are
not assumed compatible. Reusing an access token/device ID with a fresh crypto
store can break identity and decryption continuity. No implementation may copy
token/device identifiers into the Rust store until a written threat and
continuity review proves it safe.

### 8.2 Required transition design

- Detect a legacy desktop session without starting `matrix-js-sdk`.
- Preserve local non-Matrix user data such as drafts and platform settings.
- Explain that a one-time sign-in and key recovery may be required.
- Prefer creating a new, clearly named Matrix device through normal login/SSO.
- Use cross-signing/recovery/key backup to restore trust and history.
- Confirm the Rust store is usable and sync has reached readiness before marking
  transition complete.
- Retain legacy IndexedDB data inertly for a bounded rollback window defined by
  product policy; never reopen it in the new release.
- Offer explicit cleanup only after successful transition and confirmation.
- Make cleanup idempotent and scoped to the exact legacy account stores.
- Do not delete drafts, downloads, user configuration, or unrelated browser
  storage.
- Account switching must produce separate store directories, store keys, and
  session generations.

### 8.3 Store requirements

- Use Matrix Rust SDK SQLite state/event-cache/crypto storage.
- Store directories derive from a non-secret stable account identifier and are
  protected against traversal/collision.
- Store encryption keys are generated with a CSPRNG and kept in the native
  secret store, not config files or logs.
- File permissions are least privilege.
- Store opening is serialized.
- Crash recovery does not wipe automatically.
- Logout versus local wipe semantics are distinct and tested.
- Schema migrations are backed up or otherwise recoverable before mutation.
- Failed migrations leave the previous store intact.

## 9. Delivery model and branch policy

### 9.1 Branches

- Integration branch: `feature/matrix-rust-sdk-full-replacement`.
- Each implementation unit uses a short-lived task branch named
  `matrix-rust/<task-id>-<slug>` from the current integration branch.
- Task PRs target the integration branch, never `main`.
- The final PR targets `main` only after all program gates pass.
- Mainline changes are incorporated into the integration branch only by the
  orchestrator after reviewing conflict semantics and rerunning the phase gate.

### 9.2 Commit rules

- One task should normally produce one reviewable commit or a small ordered
  series.
- Generated lockfile/schema changes remain in the same commit as their source
  change.
- No mixed formatting, unrelated cleanup, version bumps, or feature work.
- Every commit message references its task ID.
- Implementation agents do not commit, push, rebase, switch branches, create
  PRs, or merge unless a task explicitly delegates a bounded Git operation.
- The orchestrator controls all Git and GitHub state.

### 9.3 Task-size limits

Default task limits:

- one coherent behavior;
- at most 8 production files unless the task is a mechanical type migration;
- target at most 800 net new/changed production lines;
- tests delivered with production code;
- one clearly stated deletion or convergence target;
- no unbounded “continue migrating” tasks.

Tasks exceeding a limit require the plan to be split before implementation.

## 10. Native orchestrator and implementation-agent protocol

The program now runs in one agent harness. The primary agent is the orchestrator
and independent reviewer. It may delegate bounded implementation units to native
sub-agents when the task can be isolated safely. External CLI model sessions are
not part of the required workflow.

The active execution model is Codex `gpt-5.6-sol` with medium reasoning for the
primary orchestrator and delegated implementation/review agents. MiniMax-M3 may
assist with non-authoritative text or research work, but acceptance, Git/PR
state, and merges remain owned by the primary Codex orchestrator. Grok is not in
the active execution path while its usage allocation is unavailable.

### 10.1 Orchestrator responsibilities

Before delegation, the orchestrator must:

- fetch and verify the integration tip, worktree state, open PRs, and active CI;
- select one planned task or remediation item whose prerequisites are closed;
- create a short-lived branch from the current integration tip;
- resolve material architecture or SDK-capability questions from the exact
  pinned source and committed plan rather than leaving them to the implementer;
- define the exact file scope, acceptance cases, required tests, forbidden
  changes, and stop conditions;
- retain ownership of Git history, PR creation, review decisions, and merges.

### 10.2 Required implementation task packet

Every delegated task must include:

1. task ID and title;
2. the authoritative plan path and exact sections to follow;
3. current branch and base commit;
4. exact goal and non-goals;
5. allowed production files/directories;
6. files/directories that must not change;
7. required upstream SDK version and relevant source/doc links;
8. required implementation invariants;
9. required tests and commands;
10. explicit deletion/convergence target;
11. instruction not to commit, push, rebase, switch branches, create/merge a PR,
    or edit the plan unless explicitly delegated;
12. instruction not to add `matrix-js-sdk`, raw Matrix HTTP, a backend selector,
    a fallback path, or suppressed errors;
13. instruction to stop and report if an SDK capability is absent or experimental
    beyond the task's approved scope;
14. instruction to report files changed, design decisions, tests run, failures,
    residual risks, and anything not completed.

### 10.3 Implementation completion is not acceptance

After an implementation agent stops, the orchestrator must independently:

- inspect `git status` and the complete diff;
- verify file scope;
- inspect relevant upstream API source;
- run or reproduce required tests;
- check architecture, concurrency, lifecycle, privacy, security, and cleanup;
- search for prohibited fallback patterns;
- confirm documentation and fixtures match behavior;
- classify findings by severity;
- reject the task if any acceptance criterion lacks evidence.

### 10.4 Correction loop

If review finds a defect:

- send the responsible implementation agent the exact finding, evidence,
  required behavior, and affected acceptance case;
- require the agent to fix the defect and rerun affected tests on the same task
  branch;
- review the entire new diff, not only the latest hunk;
- repeat until no blocking finding remains;
- reject and re-scope the task if correction reveals a material unanswered
  architecture question or unsafe scope expansion.

If an implementation agent disputes a finding, the orchestrator resolves it
from source, tests, and this plan. Unsupported assertions are not accepted.

### 10.5 Automatic rejection conditions

Reject implementation output immediately if it:

- changes the plan or broadens scope without approval;
- adds a runtime backend flag or dual-client path;
- introduces raw Matrix HTTP in product code;
- recreates JavaScript SDK classes or emitter semantics;
- uses an unpinned or different SDK version;
- enables an experimental feature without the named gate;
- stores tokens, keys, plaintext events, or decrypted media insecurely;
- deletes tests to get green;
- weakens assertions, linting, CI, CSP, permissions, or error handling;
- adds production `unwrap`, `expect`, `panic!`, swallowed errors, or detached
  tasks without an approved invariant;
- changes unrelated files;
- claims tests passed without command output;
- leaves TODOs in place of acceptance requirements;
- implements only the happy path for lifecycle or concurrency-sensitive work.

### 10.6 PR and merge discipline

- The reviewed SHA must have a green, non-cancelled required CI run. Success on
  an older SHA or a cancelled run is not evidence.
- The umbrella PR to `main` must not be allowed to cancel or substitute for task
  validation on the integration-targeted PR.
- The orchestrator reviews the complete diff against the current integration
  base and records any accepted residual explicitly.
- No critical/high security, privacy, data-loss, crypto, lifecycle, or protocol
  finding may be deferred across a phase gate.
- Task PRs may merge only into the integration branch. The final PR to `main`
  requires every program gate plus explicit user approval.

## 11. Review checklist for every task

### 11.1 Correctness

- Behavior matches the task and parity matrix.
- SDK API exists at the pinned source revision.
- Error and cancellation paths are covered.
- Reconnect, stale generation, duplicate events, and ordering are considered.
- State has one clear owner.
- No old path remains reachable for the migrated behavior.

### 11.2 Rust

- No production panic path without a documented impossible invariant.
- Async tasks have cancellation and shutdown behavior.
- Locks are not held across network awaits unless proven safe.
- Lock order is documented where multiple locks exist.
- Channels are bounded or intentionally watch/coalescing channels.
- Large SDK objects and byte buffers are not cloned casually.
- DTO conversion validates size, URLs, identifiers, and untrusted content.
- Secrets use zeroizing/secret-aware handling where applicable.
- Logs use structured redacted fields.
- Tests cover concurrency and task teardown where relevant.

### 11.3 TypeScript/React

- No SDK imports or SDK-shaped compatibility types are added.
- Discriminated unions are handled exhaustively.
- Subscriptions unsubscribe on teardown.
- Stale session generations and late promises cannot update current UI.
- No large media buffers cross JSON IPC.
- Components remain presentation-focused.
- Timeline virtualization and anchor behavior remain deterministic.

### 11.4 Security/privacy

- Tokens, keys, recovery secrets, event bodies, user IDs where unnecessary,
  media bytes, and push payloads are absent from diagnostics.
- File paths and URLs are validated.
- Native commands have least-privilege Tauri capabilities.
- CSP changes are justified and tested.
- Store deletion targets are resolved exactly and are recoverable where required.
- No new broad network allowlist is introduced.

### 11.5 Tests and evidence

- Tests fail before the fix when practical.
- Tests assert semantics rather than implementation trivia.
- Mocks do not hide SDK lifecycle behavior that requires integration coverage.
- Fixtures stay compatible across desktop and iOS.
- Command output and environment are recorded.
- `git diff --check` is clean.

## 12. Phased implementation plan

Each phase is a hard gate. Later phases may be researched early, but code may
not rely on an unaccepted earlier phase.

### Mandatory rebaseline gate — R0 remediation

The 2026-07-25 independent review found that implementation advanced past open
gates. Before P3.2 or any later original task begins, complete:

- **R0.1 — Restore truthful quality and CI gates:** fix introduced fmt/lint and
  whitespace failures, baseline only proven pre-existing debt, require green
  non-cancelled task CI, and reconcile stale task metadata.
- **R0.2 — Complete missing governance and Phase 0 evidence:** threat model,
  test/Synapse topology, native-agent review template, owned risk register,
  full traceability, and the outstanding cross-platform/live evidence.
- **R0.3 — Repair and freeze IPC v1:** safe cross-language counter semantics,
  checked sequencing, one authoritative stream identity, typed/bounded topic
  payloads, and adversarial Rust/TypeScript contract tests.
- **R0.4 — Harden storage:** canonical/symlink-safe confinement,
  collision-resistant versioned account identity, exact SDK store layout, and
  supported macOS/Linux native secret-store integration.
- **R0.5 — Make shutdown/logout/wipe transactional:** stop and join work and
  close client/store handles before destructive operations; specify and test
  every partial failure.
- **R0.6 — Enforce diagnostic privacy:** eliminate full URLs, absolute paths,
  raw SDK errors, identifiers, secrets, and content from diagnostic surfaces.
- **R0.7 — Close Phase 2 and P3.1 with live adapters:** exercise discovery,
  login-flow retrieval, encrypted store open/reopen, sync, crash, logout, and
  wipe against disposable Synapse without production dual-client wiring.
- **R0.8 — Issue formal acceptance reports:** rerun the full gate and change a
  phase status only through evidence reviewed on a green PR.

#### R0.2/R0.8 acceptance authority

R0.2 accepts only the completeness, traceability, ownership, and review
readiness of the Phase 0 evidence package. Its signed report is named
`r0.2-phase-0-evidence-readiness-report.{md,json}` and must state that it does
not accept P0.1–P0.7, change Phase 0 strict acceptance, or close the Phase 0
gate. After R0.2 is accepted, Phase 0 remains `open` and blocked by R0.8.

R0.8 alone issues `phase-0-formal-acceptance-report.{md,json}` after rerunning
the complete gate on a green, reviewed commit. Only that formal report may
accept P0.1–P0.7 and change Phase 0 from `open` to `accepted`/`closed`. A signed
report means an explicit reviewer attestation bound to an immutable commit SHA,
UTC time, decision, and PR/review reference; it does not imply a cryptographic
signature unless the project adopts that separate requirement.

The detailed finding-to-acceptance mapping is authoritative in
[`docs/matrix-rust-sdk/review-2026-07-25.md`](matrix-rust-sdk/review-2026-07-25.md).
R0 tasks are corrective additions and do not change the original 112-task
denominator.

### Phase 0 — Freeze scope, evidence, and baselines

Goal: establish an exhaustive, reproducible definition of current behavior and
upstream support before implementation choices become expensive.

Tasks:

- **P0.1 — Reproducible SDK usage inventory**
  - Add an AST-based inventory command for production and tests.
  - Record imports, deep/internal imports, SDK models, event listeners, client
    calls, direct Matrix networking, IndexedDB/store use, and call/widget use.
  - Produce machine-readable JSON plus a reviewed Markdown summary.
- **P0.2 — Feature parity traceability matrix**
  - Map every current Matrix-facing feature to current files, UI entry points,
    state owner, Rust API, task ID, automated test, and manual acceptance case.
  - Mark unused/dead paths only after call-site proof.
- **P0.3 — Exact 0.18.0 capability dossier**
  - Clone the tag, record commit and release metadata, generate local rustdoc,
    and create compile probes.
  - Classify every feature using the evidence states in Section 5.
  - Record experimental features and open upstream gaps.
- **P0.4 — Swift/Rust version provenance**
  - Determine the Matrix Rust SDK source revision embedded in
    `matrix-rust-components-swift` `26.06.06`.
  - Decide and document the desktop/iOS version-alignment target.
- **P0.5 — Toolchain compatibility spike**
  - Prove Rust `1.93`, edition 2024 dependencies, Tauri 2, macOS universal
    builds, Linux builds, signing, and notarization can coexist.
  - Identify CI runner and cache changes.
- **P0.6 — Baseline reliability/performance evidence**
  - Measure startup-to-ready, room switch, initial timeline, pagination,
    reconnect, encrypted-room open, media, memory, CPU, and disk growth.
  - Record p50/p95 and exact scenarios on macOS and Linux.
- **P0.7 — Migration UX decision record**
  - Specify reauthentication, new-device creation, key recovery, legacy-data
    retention, cleanup, rollback, and user copy.

Acceptance criteria:

- Every production `matrix-js-sdk` use is classified.
- Every currently supported user behavior has an owner and acceptance case.
- No Rust capability is described without source/probe evidence.
- Toolchain builds a minimal Tauri + Matrix SDK application on macOS and Linux.
- Calls/widgets, UIA, delayed events, custom raw events, read markers, and device
  naming have explicit answers or blocking upstream issues.
- Baseline evidence is committed and repeatable.
- The migration UX is approved before session code is written.

Validation:

- inventory self-tests and checked-in snapshot comparison;
- `cargo check` against exact `0.18.0` with Rust `1.93`;
- macOS and Linux smoke builds;
- generated rustdoc inspection;
- live disposable-Synapse API probes;
- review of every `blocked` or experimental matrix row.

### Phase 1 — Toolchain and contract foundation

Goal: establish the final build/toolchain and Synara-owned contracts without
starting a second production Matrix client.

Tasks:

- **P1.1 — Pin Rust toolchain 1.93**
  - Add repository toolchain configuration and update CI/build documentation.
  - Verify every existing desktop unit/package/release build.
- **P1.2 — Add exact SDK dependencies**
  - Add `matrix-sdk` and `matrix-sdk-ui` `=0.18.0` with the minimum approved
    features.
  - Document feature rationale and transitive security/licensing review.
- **P1.3 — Define Matrix IPC schemas**
  - Define versioned commands, responses, snapshots, deltas, errors, and stream
    lifecycle.
  - Add generated or fixture-checked TypeScript/Rust representations.
- **P1.4 — Define Synara domain DTOs**
  - Session, room summary, member, timeline item, relation, receipt, typing,
    upload, media, security, notification, search, space, thread, and widget
    DTOs.
- **P1.5 — Add IPC protocol contract tests**
  - Serialization round trips, unknown variants, invalid payloads, bounds,
    sequence gaps, stale generations, and schema compatibility.
- **P1.6 — Add architectural guardrails**
  - CI checks prohibit new JS SDK imports, deepening direct-client access, raw
    Matrix runtime HTTP, unversioned Matrix IPC, and SDK types in DTO modules.

Acceptance criteria:

- Existing product behavior still uses only the existing JS client.
- Rust SDK is present but no production sync/login is started.
- Contracts contain no SDK-specific object graph.
- Rust 1.93 builds and tests on supported platforms.
- New guardrails fail on representative prohibited fixtures.

Validation:

- full pre-existing CI suite;
- Rust fmt/check/test and Clippy with warnings denied;
- schema round-trip and negative tests;
- macOS/Linux package smoke;
- license and vulnerability review.

### Phase 2 — Rust client lifecycle and secure storage

Goal: implement the final single-owner Rust foundation, exercised only by unit
and dedicated integration harnesses until atomic cutover.

Tasks:

- **P2.1 — Matrix supervisor actor**
  - State machine for empty, opening, authenticating, restoring, syncing, ready,
    stopping, logged out, failed, and wiping.
- **P2.2 — Store paths and encryption keys**
  - Per-account path derivation, native keyring integration, permissions,
    serialization, and collision tests.
- **P2.3 — SDK client builder**
  - Homeserver, proxy/network policy, user agent, stores, crypto, timeouts, and
    approved feature configuration.
- **P2.4 — Task supervision and cancellation**
  - Track sync/listener/upload/search tasks; guarantee shutdown and stale-
    generation isolation.
- **P2.5 — Diagnostics and health model**
  - Privacy-filtered lifecycle, sync, queue, store, and error metrics compatible
    with desktop diagnostics.
- **P2.6 — Destructive lifecycle operations**
  - Logout, local wipe, failed-store recovery, and exact-target safeguards.

Acceptance criteria:

- One actor is the only construction path for a Matrix client.
- Repeated open/close/logout/wipe cycles leak no tasks or handles.
- Two accounts cannot share a store path or key.
- Store failures do not trigger automatic deletion.
- Diagnostic fixtures prove secret redaction.

Validation:

- deterministic actor-state unit tests;
- cancellation and race tests;
- temporary-directory store integration tests;
- keyring abstraction tests;
- crash/reopen tests;
- live login/sync/logout loop in the isolated harness.

### Phase 3 — Authentication and legacy-session transition

Goal: complete all authentication and account lifecycle behavior in Rust.

Tasks:

- **P3.1 — Discovery and login-flow service**
- **P3.2 — Password/token login and device naming**
- **P3.3 — SSO/OAuth callback lifecycle**
- **P3.4 — UIA/registration/password-reset capability completion**
- **P3.5 — Refresh-token persistence and rotation**
- **P3.6 — Session restore and account switching**
- **P3.7 — Legacy-session detection and transition coordinator**
- **P3.8 — Logout, remote logout, local wipe, and recovery copy**

Acceptance criteria:

- All supported login and UIA flows have parity evidence.
- No access/refresh token enters WebView storage or IPC after Rust login.
- Restart restores the correct account and store.
- Legacy transition never starts the JavaScript client.
- Failed transition preserves legacy data and offers retry.
- New devices have correct platform names.

Validation:

- unit tests for every state/error;
- two-account switching tests;
- SSO callback and cancellation tests;
- invalid/expired/refresh-token tests;
- crash during transition tests;
- live Synapse login, restore, logout, and recovery scenarios.

### Phase 4 — Sync, room list, spaces, and navigation state

Goal: reproduce all non-timeline reactive Matrix state in the Rust core.

Tasks:

- **P4.1 — Sync service readiness/reconnect model**
- **P4.2 — Room-list snapshot and delta stream**
- **P4.3 — Invites, membership, unread/highlight, and marked-unread state**
- **P4.4 — DM, favorite, low-priority, folder, and recent-room semantics**
- **P4.5 — Space service, hierarchy, filters, and parents**
- **P4.6 — Room summary/profile/member/power-level projection**
- **P4.7 — Typing and presence streams**
- **P4.8 — Route/deep-link resolution service**

Acceptance criteria:

- Snapshots/deltas reconstruct deterministic state under randomized tests.
- Reconnect does not duplicate or regress rooms/counts.
- Existing shared fixture results match desktop and iOS.
- Room-list and navigation performance stays within Phase 0 budgets.
- No Rust SDK object crosses IPC.

Validation:

- property tests for delta application;
- sequence-gap/resnapshot tests;
- large-account fixture tests;
- live two-client invite/join/leave/space/unread scenarios;
- CPU/memory/backpressure measurements.

### Phase 5 — Timeline, threads, receipts, and event mapping

Goal: implement the complete Rust-owned read side of rooms.

Tasks:

- **P5.1 — Timeline registry and lifecycle**
- **P5.2 — Snapshot/diff mapping and event identity**
- **P5.3 — Back/forward pagination and overlap deduplication**
- **P5.4 — Event-focused/context opening**
- **P5.5 — Read markers, receipts, typing, and unread positioning**
- **P5.6 — Relations, edits, redactions, reactions, and reply details**
- **P5.7 — Poll and state/membership event projection**
- **P5.8 — Thread lists, focused timelines, and subscriptions**
- **P5.9 — Custom Synara raw-content extraction**
- **P5.10 — UTD/decryption update propagation**

Acceptance criteria:

- Timeline diffs preserve order and identity through all fixture sequences.
- No REST timeline fallback exists.
- Pagination overlap, reconnect, local/remote echoes, edits, and redactions do not
  duplicate UI items.
- Event-focused opens and unread anchors meet current reliability contracts.
- Custom agent events survive unknown-field round trips as required.

Validation:

- golden timeline fixtures shared with iOS;
- randomized diff/reducer tests;
- two-client live encrypted and unencrypted timelines;
- thread and pagination integration tests;
- existing timeline viewport E2E and performance harnesses adapted to DTOs.

### Phase 6 — Sending, room operations, account data, and search

Goal: implement every Matrix mutation and query outside media/crypto/calls.

Tasks:

- **P6.1 — Text/formatted send, reply, edit, redact, and reaction**
- **P6.2 — Polls, custom agent events, and delayed events**
- **P6.3 — Send queue status, retry, cancel, and echo reconciliation**
- **P6.4 — Room/DM create, join, leave, invite, kick, ban, and forget**
- **P6.5 — Room profile, alias, directory, join/history rule, and upgrade operations**
- **P6.6 — User profile, ignore, device management, and power levels**
- **P6.7 — Account-data service and shared Synara codecs**
- **P6.8 — Message, user, and room-directory search**

Acceptance criteria:

- Cross-client event-content fixtures are byte/semantic compatible where
  canonicalization permits.
- Send queues preserve pending work across network interruption and restart as
  supported by the SDK.
- Permission and rate-limit errors map correctly.
- No UI code constructs Matrix event JSON except within approved shared custom-
  event codecs.
- Search cancellation and stale-result protection work.

Validation:

- unit/fixture tests for every mutation;
- live two-client operations suite;
- offline send/retry/restart tests;
- power-level negative cases;
- account-data concurrent update tests;
- search pagination/context tests.

### Phase 7 — Media pipeline

Goal: move all Matrix media authentication, encryption, cache, and transfer work
to Rust SDK/native boundaries.

Tasks:

- **P7.1 — Media source and safe local delivery protocol**
- **P7.2 — Thumbnails, originals, avatars, and encrypted downloads**
- **P7.3 — Upload preparation, thumbnails, progress, cancel, and retry**
- **P7.4 — File/image/video/audio/voice timeline sends**
- **P7.5 — Save/share/open/drag integration**
- **P7.6 — Cache retention, cleanup, and privacy policy**
- **P7.7 — Remove Matrix media service-worker responsibilities**

Acceptance criteria:

- No Matrix media request or decryption occurs in the WebView/service worker.
- Large media does not traverse JSON IPC.
- Encrypted media works after restart and in offline cache scenarios.
- Cancellation stops work and cleans partial files.
- Cache limits and cleanup are deterministic.
- Path traversal, MIME confusion, malicious filename, and oversized media tests
  pass.

Validation:

- unit tests with malformed and adversarial media metadata;
- live encrypted/unencrypted upload/download tests;
- progress/cancel/retry tests;
- large-file memory measurements;
- platform save/share smoke tests;
- service-worker boundary audit.

### Phase 8 — E2EE, devices, verification, backup, and recovery

Goal: reach release-grade security parity before any desktop cutover.

Tasks:

- **P8.1 — Crypto readiness and security-state projection**
- **P8.2 — Device list/trust and deletion**
- **P8.3 — Verification request inbox and SAS flow**
- **P8.4 — Cross-signing and identity state**
- **P8.5 — Key backup and recovery setup/restore/repair**
- **P8.6 — Room-key import/export if retained**
- **P8.7 — UTD retry and encrypted-history recovery**
- **P8.8 — Crypto-store continuity and corruption handling**

Acceptance criteria:

- Encrypted rooms decrypt before and after restart.
- New device becomes trusted through supported verification/recovery flows.
- Backup/recovery state remains correct through network and process failures.
- Verification requests are not lost during UI navigation.
- No crypto secret enters diagnostics or persistent WebView state.
- Recovery failure never wipes a usable store.

Validation:

- multi-device live Synapse scenarios;
- restart/crash/store-lock tests;
- wrong/missing recovery-key tests;
- key-backup loss and repair tests;
- log/diagnostic secret scanning;
- manual SAS verification acceptance on macOS and Linux.

### Phase 9 — Notifications, push semantics, and deep links

Goal: make Rust SDK state the source for notification decisions while preserving
platform-native delivery.

Tasks:

- **P9.1 — Notification settings and push-rule service**
- **P9.2 — Desktop notification candidate stream**
- **P9.3 — Focus/suppression and badge semantics**
- **P9.4 — Room/event/thread notification routing**
- **P9.5 — Encrypted notification privacy behavior**
- **P9.6 — iOS pusher/notification parity verification**

Acceptance criteria:

- Notification preferences round-trip through SDK APIs.
- Duplicate notifications are prevented across reconnect/restart.
- Active-room suppression and badge counts match current contracts.
- Notification clicks land on the correct event/thread.
- Encrypted content is not exposed before decryption policy permits it.

Validation:

- push-rule fixture tests;
- focus/reconnect/restart notification tests;
- native macOS/Linux notification smoke;
- cross-platform route fixtures;
- iOS push regression suite.

### Phase 10 — Element Call and widget completion

Goal: eliminate the final Matrix JS/MatrixRTC dependency without reducing call
functionality.

Tasks:

- **P10.1 — Exact experimental-widget risk dossier**
- **P10.2 — Rust widget driver and capability policy**
- **P10.3 — Element Call postMessage transport bridge**
- **P10.4 — MatrixRTC membership/call-state projection**
- **P10.5 — Join, leave, decline, encryption keys, and cleanup**
- **P10.6 — CSP/origin/permission hardening**
- **P10.7 — Failure and upstream-gap decision gate**

Acceptance criteria:

- Existing one-to-one and room call scenarios pass.
- Member speaking/status UI receives equivalent state.
- No `matrix-js-sdk/lib/matrixrtc/*` imports remain.
- Widget capabilities are least privilege.
- Call teardown leaves no SDK tasks, widget sessions, or leaked permissions.
- Experimental API risk is explicitly accepted with a pinned version and tests.

Validation:

- widget protocol unit tests;
- two-client Element Call smoke;
- encrypted call setup;
- reconnect/room-switch/logout teardown;
- CSP and untrusted postMessage tests;
- manual macOS/Linux audio/video acceptance.

If this phase fails, the migration remains blocked on the feature branch. Do not
cut over with a permanent JavaScript SDK exception.

### Phase 11 — Final React convergence and atomic dependency removal

Goal: verify the serial full verticals have changed the actual desktop product
to the Rust-owned Matrix client, remove cross-cutting leftovers and the npm
dependency exactly once, and never maintain selectable backends.

Tasks:

- **P11.1 — Replace SDK model types in shared utilities and state**
- **P11.2 — Convert hooks to Synara snapshot/delta selectors**
- **P11.3 — Convert components/features/pages by domain**
- **P11.4 — Replace auth/client bootstrap with Rust lifecycle**
- **P11.5 — Wire all commands and subscriptions**
- **P11.6 — Delete JavaScript sync/client/crypto/store initialization**
- **P11.7 — Delete obsolete Matrix media/service-worker paths**
- **P11.8 — Remove `matrix-js-sdk` and crypto-WASM npm dependencies**
- **P11.9 — Remove obsolete CSP exceptions and generated assets where possible**
- **P11.10 — Add zero-usage enforcement**

Execution rule:

- Owning verticals must already have deleted their superseded JS implementations
  and imports; Phase 11 must not become a bulk capability rewrite.
- Individual cross-cutting cleanup tasks may land on the integration branch
  while preserving buildability, but the branch is not releasable during final
  convergence.
- There is no runtime toggle. The final convergence commit proves JS client
  startup is impossible and removes the dependency, stores, assets, and
  remaining scaffolding before the phase is accepted.

Acceptance criteria:

- `matrix-js-sdk` does not appear in production/test imports, package manifests,
  or lockfiles.
- No Matrix JS client can be constructed.
- No Matrix IndexedDB/crypto store code remains.
- No production `/_matrix/` networking bypasses the SDK.
- All 220 baseline production imports are burned down to zero.
- React receives only Synara DTOs.
- One and only one Matrix sync owner exists.
- Full desktop feature parity suite passes.

Validation:

- AST zero-usage audit;
- package-lock transitive audit;
- raw-network boundary audit;
- all TypeScript tests/typechecks/lint;
- all Rust tests/checks/Clippy;
- browser timeline tests adapted to mocked IPC fixtures;
- live two-client Synapse suite using the packaged desktop runtime;
- macOS/Linux package smoke;
- manual exploratory matrix covering every Section 7 category.

### Phase 12 — iOS completion and cross-client alignment

Goal: ensure “across the board” means one SDK family and shared semantics, not
desktop-only replacement.

Tasks:

- **P12.1 — Align iOS Swift bindings to the approved upstream Rust revision**
- **P12.2 — Remove device-name direct Matrix HTTP**
- **P12.3 — Remove read-marker/account-data direct Matrix HTTP**
- **P12.4 — Eliminate any remaining production `URLSession` Matrix endpoint use**
- **P12.5 — Complete thread/search/media/crypto gaps required by parity matrix**
- **P12.6 — Run shared DTO/event/account-data fixtures on both clients**
- **P12.7 — Decide whether a shared Rust domain crate is justified**

The shared-crate decision is not permission to start another rewrite. Adopt it
only if it demonstrably removes duplicate protocol/domain logic without
replacing stable Swift UI service boundaries or creating a second binding
surface to maintain.

Acceptance criteria:

- iOS production Matrix traffic is SDK-owned.
- Swift binding provenance and desktop crate provenance are documented.
- Shared Synara events/account data have identical semantics.
- No iOS behavior regresses due to version alignment.
- iOS build, unit, UI, push, and TestFlight gates remain green.

Validation:

- Swift unit and UI tests;
- generic iOS Simulator build;
- device/TestFlight smoke where required;
- shared fixture comparison;
- two-client desktop/iOS live Synapse scenarios.

### Phase 13 — Reliability, performance, security, and packaging qualification

Goal: prove the replacement is safer and at least as capable as the baseline.

Tasks:

- **P13.1 — Long-running soak and reconnect testing**
- **P13.2 — Crash/restart/store-recovery matrix**
- **P13.3 — Large-room and encrypted-history performance**
- **P13.4 — Memory/CPU/disk/network regression analysis**
- **P13.5 — Security/privacy review and dependency audit**
- **P13.6 — macOS/Linux packaging, signing, notarization, updater, and migration**
- **P13.7 — Accessibility and UI regression pass**
- **P13.8 — Production diagnostics and support runbook**

Required performance budgets relative to the committed Phase 0 baseline:

- startup-to-ready p95: no worse than 10%;
- room-switch-to-first-stable-render p95: no worse than 10%;
- initial encrypted timeline p95: no worse than 10%;
- pagination p95: no worse than 10%;
- idle CPU after settled sync: no regression beyond measurement noise agreed in
  Phase 0;
- memory after the standard large-account scenario: no worse than 15%;
- no unbounded memory, channel, media-cache, or store growth;
- timeline scroll/anchor acceptance remains at least as good as baseline.

Any exceeded budget requires a written explanation and explicit approval; a
mean improvement does not excuse a severe p95/p99 regression.

Acceptance criteria:

- 24-hour signed-in soak without duplicate sync owners, task leaks, missed
  updates, or unbounded growth.
- Repeated offline/online, suspend/resume, crash/relaunch, and account-switch
  cycles preserve state.
- Security review has no unresolved critical/high finding.
- Packages install, launch, migrate, update, and uninstall correctly.
- Diagnostics are sufficient to support failures without collecting content or
  secrets.

Validation:

- automated soak and fault injection;
- Phase 0 benchmark comparison;
- `cargo audit`/approved Rust dependency policy;
- `npm audit --omit=dev --audit-level=high`;
- secret/log scanning;
- macOS universal app, DMG/updater, signing, and notarization checks;
- Linux deb/AppImage/Arch packaging checks as applicable;
- production smoke checklist.

### Phase 14 — Final deletion, documentation, and release gate

Goal: leave no migration scaffolding, stale guidance, or hidden fallback.

Tasks:

- **P14.1 — Delete temporary probes/scaffolding not serving permanent tests**
- **P14.2 — Delete legacy session/store compatibility code after the approved
  retention boundary, or clearly isolate time-bounded cleanup code**
- **P14.3 — Update architecture, knowledge base, contributor, security, build,
  release, diagnostics, and support documentation**
- **P14.4 — Add permanent zero-JS-SDK and no-raw-runtime-Matrix CI gates**
- **P14.5 — Produce final traceability and parity evidence**
- **P14.6 — Final integration-branch review and PR to `main`**

Acceptance criteria:

- Definition of done in Section 15 is fully evidenced.
- No obsolete plan claim remains in authoritative documentation.
- No temporary backend abstraction or selector exists.
- Every parity row links to passing evidence.
- All CI/release checks pass on the final commit.
- Final PR contains migration, risk, test, rollout, and rollback summaries.

Validation:

- run the complete cutover/final command hierarchy in Section 13.4;
- rerun the AST/package-lock/raw-network zero-usage audits from a clean checkout;
- build and inspect final macOS, Linux, and iOS artifacts;
- trace every definition-of-done item to committed automated or manual evidence;
- review the final diff from `main` for obsolete scaffolding, fallback paths,
  weakened controls, and undocumented dependency changes;
- require green final PR checks and explicit user approval before merge.

## 13. Validation command hierarchy

Commands may evolve as the plan adds dedicated scripts, but changes must update
this section and CI together.

### 13.1 Minimum per Rust task

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
git diff --check
```

### 13.2 Minimum per TypeScript task

```sh
npm --prefix synara run typecheck
npm --prefix synara run typecheck:modernization
npm --prefix synara run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
git diff --check
```

### 13.3 Phase gate

```sh
npm run check:repo-layout
npm run check:versions
npm run check:matrix-boundaries
npm run check:quality-gates
npm run check:synapse-harness
npm run check:production-smoke
npm run check:release-updater
npm run typecheck:modernization
npm run test:modernization
npm --prefix synara run typecheck
npm --prefix synara run test:browser:timeline
npm --prefix synara run test:timeline-performance
```

Plus all Rust commands in Section 13.1 and the phase-specific integration tests.

### 13.4 Cutover/final gate

- full phase gate;
- `npm run test:synapse-integration` against the documented disposable Synapse;
- packaged macOS desktop two-client integration;
- packaged Linux desktop two-client integration;
- iOS build/tests and cross-client scenarios;
- macOS/Linux package smoke workflows;
- signing/notarization/updater validation;
- dependency/security/privacy scans;
- manual acceptance matrix.

No failed test may be waived merely as “unrelated” without reproducing it on the
base integration commit and recording evidence.

## 14. Merge and review gates

### Task PR gate

- Task acceptance criteria satisfied.
- Implementation-agent result and evidence retained long enough for review.
- Complete diff independently reviewed.
- No blocking or high-severity finding.
- Required local commands reproduced by orchestrator.
- CI green.
- No unplanned dependency or scope change.
- Old path deleted or convergence metric improved exactly as promised.

### Phase merge gate

- Every task PR accepted into the integration branch.
- Phase acceptance criteria evidenced in a phase report.
- Full phase command suite green.
- Capability/parity matrix updated.
- Risk register updated.
- Main branch changes reconciled and retested where needed.

### Final PR gate

- All phases accepted.
- No critical/high security, privacy, data-loss, crypto, lifecycle, or call
  parity finding remains.
- Full replacement definition of done passes.
- Release artifacts and transition UX validated on real supported platforms.
- Rollout and rollback are operationally executable.
- User explicitly approves merge to `main`.

## 15. Definition of done

The program is complete only when all of the following are true:

1. Desktop production and test code has zero `matrix-js-sdk` imports.
2. Desktop package manifests and lockfiles contain no `matrix-js-sdk` or its
   npm crypto-WASM dependency.
3. No JavaScript Matrix client, sync loop, crypto backend, IndexedDB Matrix
   store, or authenticated media service-worker path remains.
4. Tauri Rust is the sole desktop Matrix client owner.
5. Product Matrix traffic on desktop and iOS goes through Matrix Rust SDK,
   including typed SDK requests where no convenience API exists.
6. Runtime raw `/_matrix/` networking audits have zero unapproved findings.
7. Every Section 7 feature has automated and manual parity evidence.
8. Element Call/widget behavior meets current requirements without a hidden JS
   SDK exception.
9. E2EE, verification, backup, recovery, and encrypted history work across
   restart, migration, and multiple devices.
10. Legacy session transition is safe, comprehensible, retryable, and does not
    destroy recoverable user data.
11. macOS and Linux package, signing/notarization, updater, and smoke gates pass.
12. iOS remains green and its remaining direct Matrix HTTP exceptions are gone.
13. Performance/reliability budgets pass or have explicit user-approved
    exceptions.
14. Security/privacy review has no unresolved critical/high finding.
15. Temporary scaffolding and migration-only compatibility layers are removed or
    have a named, time-bounded deletion gate.
16. Documentation describes the actual final architecture and versions.
17. The final integration PR is independently reviewed and explicitly approved.

## 16. Initial risk register

| Risk                                                      | Severity                 | Required mitigation/gate                                                   |
| --------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------- |
| Rust 0.18.0 requires Rust 1.93 versus current 1.77.2      | High                     | Phase 0/1 cross-platform build and release-toolchain proof                 |
| `matrix-sdk-ui` APIs and widget feature stability         | High                     | Exact pin, compile probes, call gate, no upgrade drift                     |
| Element Call/MatrixRTC parity                             | Critical to cutover      | Phase 10 must pass before JS SDK removal                                   |
| JS IndexedDB crypto store cannot be reused safely         | Critical data/identity   | New-device/recovery transition; no store/token guessing                    |
| 220 production files import JS SDK models                 | High schedule/complexity | AST inventory, task-size limits, zero-usage burn-down                      |
| IPC ordering/backpressure can recreate timeline defects   | High                     | Versioned snapshot/delta protocol, property/fault tests                    |
| Two client owners can corrupt semantics or duplicate work | Critical                 | Single supervisor; no production dual backend/selector                     |
| Custom agent events lose raw content                      | High                     | Exact raw-content probe and shared golden fixtures                         |
| UIA/registration API gaps                                 | High                     | Phase 0 capability proof; upstream change if required                      |
| Search behavior differs                                   | Medium                   | Explicit server/local-search decision and parity tests                     |
| Media byte transport harms memory/security                | High                     | Native cache/local protocol; no JSON byte payloads                         |
| Store-key or diagnostics leak                             | Critical                 | Keyring, redaction, secret scanning, security review                       |
| Long-lived feature branch diverges from main              | High                     | Controlled periodic reconciliation and phase retest                        |
| Implementation agent strays or masks failures             | High                     | Bounded task packets, independent review, correction loop, rejection rules |
| CI cancellation permits unvalidated merges                | High                     | Green non-cancelled run on reviewed SHA before every merge                 |
| Store deletion races live client/tasks                    | Critical data loss       | R0.5 close barrier, injected-failure tests, lifecycle acceptance report    |
| Store paths escape through symlinks                       | High security            | R0.4 canonical confinement and adversarial filesystem tests                |
| IPC integer/stream ambiguity corrupts state               | High correctness         | R0.3 frozen cross-language wire contract and boundary/property tests       |
| Migration scaffolding becomes permanent                   | High                     | Definition-of-done zero gates and Phase 14 deletion review                 |

## 17. Required planning artifacts before code execution

The following files must exist and be reviewed before Phase 2 acceptance or any
new P3 implementation:

- SDK usage inventory JSON and Markdown report;
- feature parity traceability matrix;
- Matrix Rust SDK 0.18.0 capability dossier;
- upstream/version provenance record;
- Rust 1.93/Tauri compatibility report;
- IPC protocol specification and schemas;
- session/store transition ADR;
- security/threat model for tokens, stores, IPC, media, diagnostics, and cleanup;
- baseline performance/reliability report;
- test matrix and disposable Synapse topology;
- native implementation-agent task packet and review report template;
- risk register with owners/status.

As of the 2026-07-25 audit, the dedicated threat model, test/Synapse topology,
native-agent review template, and owned/statused risk register are missing.
R0.2 must close those gaps; an existing implementation does not waive them.

No implementation task may ask its agent to “figure out the architecture.” The
agent implements a bounded task from reviewed artifacts and stops when the plan
does not answer a material design question.

## 18. Immediate next actions

1. Land R0.1 first so every later correction is reviewed against truthful,
   non-cancelled quality gates.
2. Complete R0.2 and resolve the architecture questions it exposes before
   delegating lifecycle, storage, or IPC changes.
3. Land R0.3–R0.6 as bounded task PRs into the integration branch, with the
   complete finding-specific negative tests from the review report.
4. Execute R0.7 on disposable Synapse and both supported desktop platforms; do
   not wire a production Rust login/sync path or create a backend selector.
5. Complete R0.8 and close Phase 0, Phase 1, Phase 2, and P3.1 only where every
   acceptance row has evidence.
6. Re-plan P3.2 against the corrected contracts, then resume P3.2–P3.8 and
   Phases 4–14 in their existing order.
7. Keep PR `#39` to `main` open and unmerged until the final gate passes and the
   user gives explicit approval.
