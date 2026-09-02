# P0.7 — Migration UX decision record

| Field                              | Value                                                                                                                   |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Task                               | **P0.7** Migration UX decision record                                                                                   |
| Date                               | 2026-07-24                                                                                                              |
| Work branch at generation          | `matrix-rust/p0.7-migration-ux`                                                                                         |
| Base integration tip at generation | `feature/matrix-rust-sdk-full-replacement` @ `9e0cfcae8c377161a73ce8e01f889f485e233e5e`                                 |
| Tip message                        | `docs(matrix): merge P0.6 performance baseline` (P0.1–**P0.6 MERGED**; PR #46 for P0.6)                                 |
| Machine twin                       | [`migration-ux-decision.json`](migration-ux-decision.json)                                                              |
| Artifact / integration state       | `landed` / `merged`                                                                                                     |
| Strict acceptance / Phase 0 gate   | `open` / `open` — policy evidence exists, but the phase gate remains open; see [`program-status.md`](program-status.md) |
| Verdict                            | `migration_ux_decided` (product policy for Phase 3 implementers)                                                        |

Authoritative program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md) §3 (full replacement), §8 (session/store transition), Phase 3 (auth/legacy transition), Phase 11–14 cutover/retention notes.

Related:

- Handoff: [`README.md`](README.md)
- Secure credentials: [`../desktop-secure-secret-storage-plan.md`](../desktop-secure-secret-storage-plan.md)
- Archived short proposal: [`../adr/archive/2026-07-24-matrix-rust-sdk-migration-ux-proposal.md`](../adr/archive/2026-07-24-matrix-rust-sdk-migration-ux-proposal.md)

**This document is the single source of truth for migration UX policy.** Implementers must not invent dual-backend paths, token/device reuse into a fresh crypto store, or concurrent multi-account promises beyond FR-7.9-011.

**Execution / cutover shape** (how we build and flip owners) is documented in
[`cutover-operating-model.md`](cutover-operating-model.md). Product-owner
confirmation (2026-07-27): clean-break re-login and local Matrix wipe are
acceptable for this desktop client; prefer that over elaborate JS→Rust session
migration. Sole owner after cutover is Rust only — never a runtime SDK selector.

---

## 1. Executive decision summary

| ID                      | Status                        | One-line summary                                                                                                                    | Owning tasks                                |
| ----------------------- | ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `D-LEGACY-DETECT`       | `decided`                     | Detect legacy session via inert signals (IDB names, credential envelope, fallback keys, cutover marker)—never start `matrix-js-sdk` | P3.7                                        |
| `D-REAUTH`              | `decided`                     | Reauthentication is **required** on cutover for users with a legacy Matrix session                                                  | P3.2, P3.3, P3.7                            |
| `D-NEW-DEVICE`          | `decided`                     | Create a new Matrix device via normal login/SSO with platform name `Synara macOS` / `Synara Linux`                                  | P3.2, P3.3                                  |
| `D-TOKEN-CONTINUITY`    | `decided`                     | **Do not** copy access token / device ID into a fresh Rust crypto store by default (§8.1)                                           | P3.5, P3.7, threat review if ever revisited |
| `D-KEY-RECOVERY`        | `decided`                     | After reauth: guided recovery key / secret storage / key backup restore before promising encrypted history                          | P3.8, P8.5, P8.7                            |
| `D-TRANSITION-COMPLETE` | `decided`                     | Complete only when Rust store opens, session secrets are native-persisted, and sync readiness is confirmed                          | P3.7, P4.1, P2.x store                      |
| `D-LEGACY-RETAIN`       | `decided-with-open-parameter` | Keep legacy IndexedDB **inert** for a bounded window; never reopen JS client                                                        | P3.7, P14.2                                 |
| `D-CLEANUP`             | `decided`                     | Explicit, idempotent, scoped cleanup after success (+ optional early confirm); never non-Matrix local data                          | P3.7, P3.8, P14.2                           |
| `D-ROLLBACK`            | `decided`                     | Rollback = prior product build + preserved inert legacy data; **not** dual-runtime                                                  | P11.x packaging, P13.6, P14.2               |
| `D-PRESERVE-LOCAL`      | `decided`                     | Drafts, platform settings, downloads, unrelated storage survive cutover and cleanup                                                 | P3.7, P3.8, P6 drafts                       |
| `D-ACCOUNT-SWITCH`      | `decided`                     | Sequential single-active account isolation; separate store dirs/keys/generations; no concurrent multi-account promise               | P3.6, P3.7                                  |
| `D-LOGOUT-WIPE`         | `decided`                     | Logout and local wipe are distinct actions with distinct confirmations                                                              | P3.8, P2.x                                  |
| `D-USER-COPY`           | `decided`                     | Honest one-time sign-in + recovery messaging; never secrets in copy                                                                 | P3.7, P3.8, product copy                    |
| `D-FAILURE`             | `decided`                     | Failed transition leaves legacy data intact and offers retry                                                                        | P3.7                                        |
| `D-NO-DUAL-BACKEND`     | `decided`                     | Hard constraint: no dual production backend, selector, or dual-client same session                                                  | All phases                                  |

---

## 2. Safety position

### 2.1 IndexedDB ≠ SQLite

JavaScript IndexedDB Matrix state/crypto stores (`web-sync-store`, `crypto-store`, `matrix-js-sdk::matrix-sdk-crypto`, `matrix-js-sdk::matrix-sdk-crypto-meta` — see current `MATRIX_LOCAL_STORE_NAMES`) and native Matrix Rust SDK SQLite stores are **not assumed compatible**. There is no silent store conversion path.

### 2.2 No unsafe token / device reuse

Reusing an access token and device ID against a **fresh** crypto store can break identity continuity and decryption. Per plan §8.1:

> No implementation may copy token/device identifiers into the Rust store until a written threat and continuity review proves it safe.

**Product default for this ADR:** never copy. Prefer **new device + recovery**. Any future exception requires a separate written threat/continuity review and an explicit superseding decision—not an implementation shortcut.

### 2.3 One Matrix owner after cutover

After the Phase 11 atomic cutover commit, only Matrix Rust SDK may own Matrix session, sync, crypto, and room state. Legacy IndexedDB may remain on disk only as **inert** data for rollback/support—never opened by a JS Matrix client in the new product.

### 2.4 Native credentials

Session secrets after Rust login live in the OS credential store (Keychain / Secret Service) per [`desktop-secure-secret-storage-plan.md`](../desktop-secure-secret-storage-plan.md). Tokens must not re-enter WebView storage or IPC as long-lived product state (Phase 3 acceptance).

---

## 3. Happy-path cutover UX flow

Numbered steps for a user who upgrades into the first Rust-owned desktop release while a legacy Matrix session still exists on the machine:

1. **Launch (Rust-only bootstrap).** App starts with Matrix Rust lifecycle only. No `matrix-js-sdk` client construction, no `initRustCrypto` wasm path, no JS sync loop.
2. **Legacy detection (inert).** Transition coordinator detects legacy presence using `D-LEGACY-DETECT` signals without starting the JS SDK.
3. **Explain transition.** Show a one-time migration screen: one sign-in required; encrypted history depends on recovery key / backup; drafts and settings are kept; old session data stays unused until cleanup.
4. **Reauthenticate.** User completes password login or SSO/OAuth (`D-REAUTH`). Homeserver discovery uses existing product flows.
5. **New device.** Login creates a new Matrix device with platform display name (`Synara macOS` or `Synara Linux`) (`D-NEW-DEVICE`). No token/device import from legacy store (`D-TOKEN-CONTINUITY`).
6. **Open Rust stores.** Create/open native SQLite state/event-cache/crypto stores for the account identity; bind store encryption keys in the native secret store (`D-TRANSITION-COMPLETE` prerequisites).
7. **Persist session secrets.** Write the new session envelope to the native credential store only after successful login/store open path succeeds.
8. **Key recovery.** Prompt for recovery key / secret storage unlock and key backup restore when available (`D-KEY-RECOVERY`). User may defer with explicit acknowledgement that encrypted history may remain unavailable until recovery succeeds.
9. **Sync readiness.** Start sync; wait until product “ready” criteria are met (room list / first sync ready per Phase 4 readiness model) (`D-TRANSITION-COMPLETE`).
10. **Mark transition complete.** Persist a non-secret cutover completion marker (account + app generation scoped). Legacy IndexedDB remains inert (`D-LEGACY-RETAIN`).
11. **Enter product.** Normal app shell; no second Matrix client.
12. **Later cleanup.** After the retention window (or early user confirm), offer scoped cleanup of legacy Matrix IndexedDB and legacy token keys only (`D-CLEANUP`). Non-Matrix local data always preserved (`D-PRESERVE-LOCAL`).

**Users with no legacy session** (fresh install or already wiped): skip migration UI; use normal first-run login.

**Users who already completed transition** (cutover marker + Rust session present): restore Rust session only; do not re-show full migration unless recovery/repair is needed.

---

## 4. Decision catalog

### D-LEGACY-DETECT — Detect legacy session without starting JS SDK

| Field        | Value                                                   |
| ------------ | ------------------------------------------------------- |
| Status       | `decided`                                               |
| Owning tasks | **P3.7** (primary); P3.6 restore paths must not regress |

**Decision.** Legacy presence is determined only from **inert** signals:

1. **Cutover marker absent or incomplete** for this install generation, **and**
2. One or more of:
   - Known Matrix IndexedDB database names exist (`MATRIX_LOCAL_STORE_NAMES` or successor list maintained in product)—detect via IndexedDB factory listing / existence checks **without** opening a Matrix client or crypto API;
   - Legacy fallback session keys exist in localStorage (`synara_access_token`, `synara_device_id`, `synara_user_id`, `synara_base_url`, optional generation)—presence is enough to trigger migration UX; **do not** use these values to bootstrap a JS client after cutover;
   - Pre-cutover native session envelope exists that was written for the JS product path and is **not** a post-cutover Rust session (implementation distinguishes generations/markers).

**Must not:** construct `MatrixClient`, call `initRustCrypto`, start sync, or open legacy crypto stores “just to check.”

**Rationale.** Plan §8.2 requires detection without starting `matrix-js-sdk`. Phase 3 acceptance: legacy transition never starts the JS client.

---

### D-REAUTH — Reauthentication required

| Field        | Value                        |
| ------------ | ---------------------------- |
| Status       | `decided`                    |
| Owning tasks | **P3.2**, **P3.3**, **P3.7** |

**Decision.** If legacy Matrix session signals are present and Rust transition is not complete, the user **must** re-sign-in (password and/or SSO/OAuth as supported). Silent continuation on the old access token is **not** product policy for cutover.

**Rationale.** Aligns with §8.1 (no token/device reuse into fresh crypto), §8.2 (one-time sign-in may be required), and full-replacement atomic cutover (no dual-client restore of the old device).

---

### D-NEW-DEVICE — New device policy and naming

| Field        | Value                                                     |
| ------------ | --------------------------------------------------------- |
| Status       | `decided`                                                 |
| Owning tasks | **P3.2**, **P3.3**; device list UX P8.x / FR-7.9-003 path |

**Decision.**

- Prefer a **new** Matrix device created through normal login/SSO.
- Initial device display name: **`Synara macOS`** or **`Synara Linux`** (plan §7.1; iOS remains `Synara iOS` for that platform—not part of desktop cutover UX).
- The previous JS-era device remains a separate other-device until the user signs it out from Settings → Devices or it is otherwise invalidated.
- Do not attempt to “claim” the old device ID for the Rust store.

**Rationale.** Plan §8.2 prefers clearly named new devices; §3.1 forbids concurrent reuse of one device ID from two SDK stores.

---

### D-TOKEN-CONTINUITY — No default token/device copy into fresh crypto store

| Field        | Value                                                                                                   |
| ------------ | ------------------------------------------------------------------------------------------------------- |
| Status       | `decided`                                                                                               |
| Owning tasks | **P3.5**, **P3.7**; any exception requires dedicated threat/continuity review before Phase 3 acceptance |

**Decision.** Default and current product policy: **do not** copy access token, refresh token, or device ID from legacy JS session material into a newly created Rust crypto/state store to “skip” reauth.

Post-cutover, the only legitimate session secrets are those established by the Rust login/restore path and stored per secure-secret-storage plan.

**Rationale.** Plan §8.1 and risk register (“JS IndexedDB crypto store cannot be reused safely”).

---

### D-KEY-RECOVERY — Recovery sequence

| Field        | Value                                                                                                         |
| ------------ | ------------------------------------------------------------------------------------------------------------- |
| Status       | `decided`                                                                                                     |
| Owning tasks | **P3.8** (copy/entry UX), **P8.5** (backup/recovery setup/restore/repair), **P8.7** (UTD / encrypted history) |

**Decision.** After successful reauth and Rust store open:

1. Detect whether secret storage / key backup / cross-signing recovery is available for the account (via Rust SDK APIs—not JS).
2. If recovery material is expected, present a guided flow:
   - enter recovery key / passphrase (never log, never put secrets in diagnostics or copy templates);
   - unlock secret storage;
   - restore key backup when present;
   - surface verification/trust follow-ups consistent with FR-7.9-002/005/006 product meaning under Rust (P8.x).
3. If the user has no recovery key, show honest outcomes: they can use the app, but historical encrypted messages may remain undecryptable until they complete recovery from another device or accept loss.
4. Deferral is allowed only with explicit user acknowledgement; do not silently mark “encrypted history ready.”
5. Room-key file import/export (FR-7.9-007) remains a separate advanced path if retained—not a substitute for server backup recovery in the default cutover script.

**Rationale.** Plan §8.2; Phase 8 is release-grade E2EE before final cutover, but Phase 3 must not leave users without a defined recovery UX entry point after reauth.

---

### D-TRANSITION-COMPLETE — Readiness criteria

| Field        | Value                                                                                           |
| ------------ | ----------------------------------------------------------------------------------------------- |
| Status       | `decided`                                                                                       |
| Owning tasks | **P3.7** coordinator; **P2.x** store open; **P4.1** sync readiness; **P3.5** credential persist |

**Decision.** Mark transition complete only when **all** of the following hold:

1. Rust Matrix client and stores opened successfully for the authenticated user.
2. Session secrets persisted to the native credential store (or documented unsupported-store state blocks “complete” with clear UX—not a silent WebView fallback as permanent design).
3. Sync has reached product readiness (same readiness semantics Phase 4 defines for first usable room list / connected state—not merely “login HTTP 200”).
4. Cutover completion marker written (non-secret; includes userId + install/session generation).
5. No JS Matrix client was started during the attempt.

**Optional but recommended before “complete” messaging that promises E2EE history:** recovery unlock success **or** explicit user deferral acknowledgement (`D-KEY-RECOVERY`).

**Rationale.** Plan §8.2: confirm Rust store usable and sync readiness before marking complete. Phase 3: failed transition preserves legacy data.

---

### D-LEGACY-RETAIN — Inert retention window

| Field        | Value                                                                                 |
| ------------ | ------------------------------------------------------------------------------------- |
| Status       | `decided-with-open-parameter`                                                         |
| Owning tasks | **P3.7** (retain policy enforcement), **P14.2** (delete compatibility after boundary) |

**Decision.**

- After successful transition, legacy Matrix IndexedDB (and any leftover JS fallback token keys not already cleared) remain on disk as **inert** data.
- The new product **never reopens** those stores with `matrix-js-sdk`.
- Retention is **bounded** by product policy parameter `legacy_retention` (see Open parameters).
- Purpose: support rollback to a prior build, support diagnostics, and reduce irreversible data loss if the first Rust release needs emergency revert via previous installer/build.

**Recommended default (`legacy_retention`):** the **later of**:

- **30 calendar days** after transition-complete timestamp, **or**
- **2 application release versions** after the first release that completed transition for this install.

Either bound elapsing enables default cleanup eligibility; user may clean earlier with confirmation (`D-CLEANUP`).

**Rationale.** Plan §8.2 bounded rollback window; P14.2 retention boundary.

---

### D-CLEANUP — Scoped, idempotent cleanup

| Field        | Value                         |
| ------------ | ----------------------------- |
| Status       | `decided`                     |
| Owning tasks | **P3.7**, **P3.8**, **P14.2** |

**Decision.**

- **When offered:** after transition complete; encourage after retention window; allow early cleanup only with strong confirmation that rollback using legacy data will no longer be possible.
- **What is deleted (scoped):** exact legacy Matrix IndexedDB names for this product (`MATRIX_LOCAL_STORE_NAMES` list or successor), legacy fallback session token keys only, and any migration-only temporary flags that are safe to drop.
- **What is never deleted by Matrix cleanup:** drafts, platform settings, downloads, non-Matrix IndexedDB/localStorage, media cache policy owned by Phase 7, unrelated browser storage.
- **Idempotent:** cleanup may run more than once; missing stores are success.
- **Confirmations:** destructive copy is explicit; no silent wipe of legacy stores on first success.
- **Logout path:** may clear **Rust** session/stores per `D-LOGOUT-WIPE` without necessarily forcing legacy cleanup if still in retention window (implementation must keep scopes separate).

**Rationale.** Plan §8.2 explicit cleanup after success; idempotent and scoped.

---

### D-ROLLBACK — Bounded window; meaning of rollback

| Field        | Value                                                                        |
| ------------ | ---------------------------------------------------------------------------- |
| Status       | `decided`                                                                    |
| Owning tasks | Packaging/release **P13.6**, final docs **P14.x**; product behavior **P3.7** |

**Decision.** Rollback during the retention window means:

1. User installs or runs a **prior product build** that still uses `matrix-js-sdk` (operational rollback—not an in-app SDK toggle).
2. Inert legacy IndexedDB / leftover session material may still be present so that prior build can resume **if** those stores were not cleaned up.
3. The Rust-era product does **not** implement dual-runtime or “switch back to JS” in the same binary.

**Rollback does not mean:** dual-backend selector, concurrent JS+Rust clients, or restoring the old device ID into the Rust store.

**After cleanup or after retention + auto-offer cleanup accepted:** rollback to prior build may require full re-login on the old build as well—copy must not over-promise.

**Rationale.** Plan §3.1 atomic cutover; §9/§14 operational rollback summaries; no dual production backend.

---

### D-PRESERVE-LOCAL — Non-Matrix local data

| Field        | Value                                                    |
| ------------ | -------------------------------------------------------- |
| Status       | `decided`                                                |
| Owning tasks | **P3.7**, **P3.8**, drafts ownership in messaging phases |

**Decision.** Drafts, platform settings, downloads, and unrelated local configuration **survive** migration, failed attempts, successful transition, and Matrix-scoped cleanup. Plan §7.4: drafts remain local and survive the migration.

**Rationale.** Plan §8.2 preserve non-Matrix user data.

---

### D-ACCOUNT-SWITCH — Store isolation (honest multi-account)

| Field        | Value              |
| ------------ | ------------------ |
| Status       | `decided`          |
| Owning tasks | **P3.6**, **P3.7** |

**Decision.**

- Account switching produces **separate** Rust store directories, store keys, and session generations (plan §8.2 / §8.3).
- Product cutover inherits current honesty from **FR-7.9-011 (`partial`)**: **sequential single-active** account isolation only. Concurrent dual clients / parallel per-userId product sessions are **not** promised by this ADR.
- On identity change (different userId login), clear/replace the previous **active** Matrix store path using explicit generation guards—semantic successor to `clearMatrixStoresForIdentityChange`, not concurrent multi-store UI.
- Do not re-open or re-promote FR-7.9-011 beyond sequential single-active without a new product decision outside this ADR.

**Rationale.** Plan wants separate stores; handoff/FR evidence shows concurrent multi-account is not current product. Cutover must not silently expand scope.

---

### D-LOGOUT-WIPE — Logout vs local wipe

| Field        | Value                              |
| ------------ | ---------------------------------- |
| Status       | `decided`                          |
| Owning tasks | **P3.8**, store lifecycle **P2.x** |

**Decision.** Two distinct actions:

| Action         | Server session                                                               | Native credentials      | Local Matrix stores (Rust)                                                                         | Legacy inert IndexedDB                                               |
| -------------- | ---------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Logout**     | End/invalidate current session (and remote logout flows as product supports) | Remove session envelope | Stop client; default: clear or seal per Phase 2/3 implementation tests—must be explicit and tested | Unchanged by default during retention unless user also chose cleanup |
| **Local wipe** | May skip remote logout if offline; prefer remote when online                 | Remove                  | **Destroy** local Matrix stores for the target account only                                        | Optional separate confirm if wiping “all Matrix data on this device” |

Crash recovery **must not** auto-wipe (plan §8.3). UI labels and confirmations must not conflate “Sign out” with “Delete local data.”

**Rationale.** Plan §8.3 distinct semantics; Phase 3 P3.8 task.

---

### D-USER-COPY — Messaging principles and draft strings

| Field        | Value                                                             |
| ------------ | ----------------------------------------------------------------- |
| Status       | `decided`                                                         |
| Owning tasks | **P3.7**, **P3.8**, final product copy polish deferred to release |

**Principles (binding):**

1. **No secrets in copy, logs, or templates** — never recovery keys, tokens, device IDs, passwords, or event plaintext in UI strings that could be screenshotted into docs; never auto-fill recovery keys into share sheets.
2. **Set expectations** — one-time sign-in is required; encrypted history needs recovery key/backup; this is not “silent seamless migration.”
3. **Preserve trust** — say drafts/settings stay; say old app data is unused until cleanup; say rollback needs previous version if still retained.
4. **Honest failure** — failed attempt did not delete old data; user can retry.
5. **No dual-backend language** — do not offer “use old engine” in product UI.
6. **Accessibility** — critical steps work with keyboard and screen readers (implementation detail in Phase 3 UI).

**Draft user strings** (non-final; labeled draft):

| Key                      | Draft copy                                                                                                                                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `migration.title`        | Update your signed-in session                                                                                                                                                                                   |
| `migration.body`         | Synara now uses a more secure session engine. You’ll sign in once more. Your drafts and settings stay on this computer. Encrypted message history is restored with your recovery key.                           |
| `migration.cta_continue` | Continue to sign in                                                                                                                                                                                             |
| `migration.cta_learn`    | What happens to my data?                                                                                                                                                                                        |
| `migration.learn_body`   | We don’t reuse your previous secure store automatically. After you sign in, we’ll help restore encryption keys if you have a recovery key. Older Matrix data on this device stays unused until you clean it up. |
| `recovery.title`         | Restore encryption                                                                                                                                                                                              |
| `recovery.body`          | Enter your recovery key to unlock encrypted message history on this device. We never send your recovery key to Synara servers as app telemetry.                                                                 |
| `recovery.defer`         | Continue without restoring now                                                                                                                                                                                  |
| `recovery.defer_warn`    | Some older encrypted messages may not be readable until you restore.                                                                                                                                            |
| `transition.success`     | You’re set. Synara is ready on this device.                                                                                                                                                                     |
| `transition.failure`     | We couldn’t finish the update. Your previous data on this device was not removed. You can try again.                                                                                                            |
| `cleanup.title`          | Remove older Matrix data?                                                                                                                                                                                       |
| `cleanup.body`           | This deletes unused data from the previous session engine on this device. Drafts and settings are kept. You can’t use that older data to roll back afterward.                                                   |
| `cleanup.confirm`        | Delete older Matrix data                                                                                                                                                                                        |
| `logout.title`           | Sign out?                                                                                                                                                                                                       |
| `wipe.title`             | Delete local Matrix data?                                                                                                                                                                                       |
| `wipe.body`              | This removes Synara’s Matrix data for this account on this device. It is different from signing out on the server.                                                                                              |

---

### D-FAILURE — Failed transition preserves legacy data

| Field        | Value                           |
| ------------ | ------------------------------- |
| Status       | `decided`                       |
| Owning tasks | **P3.7**; crash cases **P13.2** |

**Decision.**

- Any failure before transition-complete leaves legacy IndexedDB and legacy session material **intact**.
- Partial Rust stores from a failed attempt must not brick retry: either discard incomplete Rust store for that generation or repair with explicit user guidance—never delete legacy as “compensation.”
- UI always offers **Retry**.
- Crash mid-transition does not auto-wipe (plan §8.3).

**Rationale.** Phase 3 acceptance: failed transition preserves legacy data and offers retry.

---

### D-NO-DUAL-BACKEND — Hard constraint restated

| Field        | Value                                               |
| ------------ | --------------------------------------------------- |
| Status       | `decided`                                           |
| Owning tasks | All phases; enforced especially P11.4–P11.10, P14.4 |

**Decision.**

- No user-visible or persistent SDK selector.
- No simultaneous `matrix-js-sdk` + Matrix Rust SDK for one app session.
- No permanent matrix-js-sdk runtime path after cutover.
- No dual-client same device ID / same session.
- Harnesses/tests may exercise Rust before cutover but must not ship as a second production sync loop.

**Rationale.** Plan §3.1 non-negotiable; definition of done §15.

---

## 5. Failure modes and user-visible outcomes

| Failure mode                                | User-visible outcome                                                                                                               | Data outcome                                  |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Legacy detect false positive                | Migration UI; user signs in; harmless                                                                                              | No legacy delete                              |
| Login/SSO cancelled                         | Remain on migration/login; not “complete”                                                                                          | Legacy intact                                 |
| Login success, store open fails             | Error + Retry; do not claim ready                                                                                                  | Legacy intact; incomplete Rust store isolated |
| Credential store unavailable                | Clear unsupported/migration state (secure-storage plan); do not complete with silent long-lived WebView tokens as permanent design | Legacy intact until resolved                  |
| Recovery key wrong                          | Error; remain recoverable; optional defer with warning                                                                             | Legacy intact                                 |
| Sync never reaches ready                    | Do not mark complete; Retry / reconnect guidance                                                                                   | Legacy intact                                 |
| Crash mid-transition                        | On relaunch: detect incomplete; resume migration, not auto-wipe                                                                    | Legacy intact                                 |
| Cleanup blocked (open IDB)                  | Error; retry later; do not half-delete without reporting                                                                           | Best-effort; idempotent retry                 |
| Account switch during incomplete transition | Block or serialize; one active generation                                                                                          | No dual client                                |

---

## 6. Rollback policy

See `D-ROLLBACK`. Summary:

- **In-app:** no “switch to JavaScript Matrix.”
- **Operational:** prior signed build + retained inert legacy data within window.
- **After cleanup:** no promise that prior build restores without re-login.
- Final release notes (P14) must include operational rollback steps consistent with this ADR.

---

## 7. Cleanup policy

See `D-CLEANUP` and `D-LEGACY-RETAIN`. Summary:

1. Success + marker → inert retain.
2. Eligibility after retention parameter **or** early user confirm.
3. Scoped delete of Matrix legacy stores + token keys only.
4. Idempotent; confirmations required for early cleanup.
5. P14.2 removes migration scaffolding after approved boundary.

---

## 8. User copy guidelines + draft strings

Binding principles and draft strings are under `D-USER-COPY`. Additional rules:

- Prefer “session engine” / “sign in again” language over internal names (`matrix-js-sdk`, SQLite, wasm) in primary UI.
- Secondary help may name “recovery key” (user-facing Matrix concept) but not dump key material.
- Support docs may reference this ADR; they must not ship secrets from test accounts.

---

## 9. Explicit non-goals

- Dual-backend, SDK selector, A/B Matrix engines in production.
- Silent token/device import into fresh Rust crypto store.
- Automatic invisible migration that skips reauth.
- Concurrent multi-account clients (FR-7.9-011 remains partial sequential single-active).
- Reopening JS client “for recovery only” after cutover.
- Implementing production session/migration code in P0.7 (docs only).
- Changing FR-7.8–7.11 statuses or re-promoting FR-7.9-011.
- Windows support (out of release matrix unless separately authorized).
- Converting IndexedDB → SQLite binary store formats.

---

## 10. Open parameters

| Parameter                               | Status                                       | Recommended default                                                                                  | Owner to finalize      |
| --------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ---------------------- |
| `legacy_retention`                      | `decided-with-open-parameter`                | Later of **30 days** after transition-complete **or** **2 app releases**                             | Product + P3.7 / P14.2 |
| `early_cleanup_allowed`                 | `decided` (boolean true with strong confirm) | **true**                                                                                             | P3.7                   |
| `recovery_required_to_complete`         | `decided`                                    | **false** — complete allowed with explicit deferral acknowledgement; do not claim E2EE history ready | P3.7 / P8.5            |
| Exact cutover marker schema             | `deferred-to-phase`                          | Non-secret JSON: `{ userId, completedAt, appVersion, generation }`                                   | P3.7                   |
| Incomplete Rust store discard vs repair | `deferred-to-phase`                          | Prefer discard incomplete generation + retry                                                         | P2.x / P3.7            |
| Auto-prompt cleanup vs settings-only    | `deferred-to-phase`                          | Soft prompt once after retention; always available in Settings                                       | P3.7 / P14.2           |

No `blocked-on-user` items: plan §8 already answers the product direction; open parameters are numeric/schema details with recommended defaults so Phase 0 can close.

---

## 11. Acceptance checklist for Phase 3 implementers

- [ ] Legacy detection never constructs `matrix-js-sdk` / starts JS crypto (`D-LEGACY-DETECT`).
- [ ] Users with legacy session are forced through reauth (`D-REAUTH`).
- [ ] New devices named `Synara macOS` / `Synara Linux` (`D-NEW-DEVICE`).
- [ ] No token/device copy into fresh crypto store (`D-TOKEN-CONTINUITY`).
- [ ] Recovery UX entry after login; secrets never logged (`D-KEY-RECOVERY`, `D-USER-COPY`).
- [ ] Transition-complete only after store open + native secret persist + sync ready (`D-TRANSITION-COMPLETE`).
- [ ] Failed paths leave legacy data intact and offer Retry (`D-FAILURE`).
- [ ] Drafts/settings/downloads preserved (`D-PRESERVE-LOCAL`).
- [ ] Cleanup scoped, confirmed, idempotent (`D-CLEANUP`).
- [ ] Logout vs wipe distinct (`D-LOGOUT-WIPE`).
- [ ] Account switch sequential single-active; separate store dirs/keys/generations (`D-ACCOUNT-SWITCH`); no concurrent multi-account claim.
- [ ] No dual-backend toggle (`D-NO-DUAL-BACKEND`).
- [ ] Tests: cancel login, crash mid-transition, wrong recovery key, two-account sequential switch, cleanup idempotency, invalid/expired token (Phase 3 validation list).
- [ ] No access/refresh token left in WebView storage after Rust login success (Phase 3 acceptance).

---

## 12. Relationship to FR-7.9-\* (statuses unchanged)

This ADR **does not** modify traceability FR statuses. Implementers must preserve honesty:

| FR         | Status (handoff)                                | Migration UX implication                                                                                                 |
| ---------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| FR-7.9-001 | `implemented` (JS boot order)                   | Cutover replaces IndexedDB+wasm boot with native SQLite boot under Rust; order semantics (store before sync) still apply |
| FR-7.9-006 | `implemented` (recovery UX product meaning)     | Rehome under P8.5/P3.8; cutover recovery sequence must match product meaning, not compile-only gates                     |
| FR-7.9-011 | **`partial`** concurrent multi-account          | Cutover **must not** claim concurrent multi-account; sequential single-active isolation only                             |
| FR-7.9-012 | `implemented` (continuity / no wipe on restore) | Rust restore must not wipe healthy stores; fresh-login identity change remains explicit                                  |
| FR-7.9-013 | **`partial`** corruption                        | Continuity anomaly guidance remains non-destructive; true integrity repair still partial—do not invent silent auto-wipe  |

---

## 13. Reviewer checklist

- [ ] All required decision IDs present with status and owning tasks.
- [ ] Aligns with plan §3 full replacement and §8 safety (no dual-backend; no unsafe token reuse).
- [ ] FR-7.9-011 honesty preserved (sequential single-active only).
- [ ] User copy principles present; sample strings contain no secrets.
- [ ] MD and JSON twins synchronized.
- [ ] Docs only—no production session/migration code, no dual-backend scaffolding.
- [x] P0.7 artifact is merged; current strict acceptance remains separately
      tracked in `program-status.json`.
- [ ] No FR-7.8–7.11 rewrites or FR-7.9-011 re-promotion.

---

## 14. Phase 0 acceptance note

Plan Phase 0: “The migration UX is approved before session code is written.”

This record provides that approval surface: decisions are `decided` or `decided-with-open-parameter` with recommended defaults. Implementation begins at Phase 3 (`P3.7` coordinator and related auth tasks), not in P0.7.
