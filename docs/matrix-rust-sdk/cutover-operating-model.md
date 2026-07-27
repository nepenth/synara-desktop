# Cutover operating model — Matrix Rust SDK full replacement

| Field | Value |
| --- | --- |
| Status | **Canonical** (product policy + execution model) |
| Date | 2026-07-27 |
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Authoritative plan | [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md) §3, Phase 11 |
| Related | [`migration-ux-decision.md`](migration-ux-decision.md), [`implementation-handoff.md`](implementation-handoff.md), [`program-status.md`](program-status.md) |

This document is the **single short canonical statement** of *how* we execute the
replacement. Task inventories and phase lists still live in the full plan; live
delivery state lives in the status ledger. When those conflict with this model on
*execution shape*, this model wins until the plan is updated to match.

---

## 1. End state (non-negotiable)

| Requirement | Meaning |
| --- | --- |
| **One Matrix owner** | After cutover, only Matrix Rust SDK owns session, sync, crypto, and room state |
| **No dual production backend** | No user-visible or hidden runtime flag to choose JS vs Rust |
| **No long-term dual maintenance** | Do not keep both SDKs as coequal product paths |
| **Branch → prove → merge** | Build and dogfood on the integration branch; merge to `main` only with explicit approval |
| **Clean-break client migration** | Re-login, clear cache, wipe local Matrix dirs are acceptable for the sole desktop user |

Architecture after cutover:

```text
UI (React)  →  versioned IPC + Synara DTOs  →  Rust supervisor + matrix-sdk  →  homeserver
```

Not:

```text
UI  →  matrix-js-sdk (browser)  →  homeserver   // removed
UI  →  if (flag) js else rust                   // forbidden forever
```

---

## 2. Why not file-by-file “swap the package”

~220+ production files under `synara/src` touch `matrix-js-sdk` today. That map is a
**coverage burn-down checklist**, not a rewrite order.

`matrix-js-sdk` runs in the **WebView/frontend**. Matrix Rust SDK runs in the
**Tauri host**. Replacement is a **boundary change** (IPC to a Rust owner), not an
in-process package swap. Concurrent JS + Rust clients for the same Matrix session
are forbidden (crypto/device/sync integrity).

---

## 3. Execution model (what we do)

### A. Capability-first vertical slices on the integration branch

Build and prove the **Rust Matrix owner** by user-visible capability, not by random
TS import sites:

| Priority band | Example slices |
| --- | --- |
| Session entry | Discovery, login-flow list, password/token login, device naming |
| Session lifecycle | Refresh/persist secrets, restore, logout, wipe |
| Live state | Sync readiness, room list, membership, unread |
| Timeline | Read/send/paginate, receipts, relations |
| Crypto | Verification, backup/recovery as required |
| Rest of product | Spaces, search, media, calls, settings, … |

Each slice:

1. Implement under `src-tauri/src/matrix/` (+ IPC/DTO contracts as needed).
2. Unit / harness / CI evidence (no secrets in logs or DTOs).
3. Land on the integration branch as harness foundation until product cutover.
4. Do **not** introduce a backend selector.

UI rewires talk only to **Synara IPC/DTOs** once the host owns that capability —
never to a second Matrix client.

### B. Interim monorepo state is transitional, not dual-product

Until the cutover event:

- Shipping product on the branch may still use `matrix-js-sdk`.
- Rust code grows as the **future sole owner** (harness / foundation / future product path).
- Status ledger: `matrix-js-sdk-only` + `harness-foundation-only` + `dual_backend: false`.

This is temporary scaffolding on the feature branch. It is **not** a supported
dual-SDK product mode and must not grow a config flag.

### C. Atomic sole-owner cutover (then burn down JS)

When the **core dogfood path** works on the integration branch (not when every
formal phase gate is closed):

1. Bootstrap starts **only** the Rust Matrix lifecycle.
2. Stop constructing any `matrix-js-sdk` client / JS sync / JS crypto init.
3. User re-authenticates (new device; no token/device copy into a fresh crypto store).
4. Delete obsolete JS Matrix init, stores, and dependencies (Phase 11 burn-down).
5. Allowlisted js-sdk importers only decrease after cutover; guardrails ban new ones.

**Dogfood readiness (minimum core path)** before sole-owner flip on the branch:

1. Log in via Rust (password and/or supported token path).
2. Restart restores the session from native secrets + Rust store.
3. Room list + basic timeline.
4. Send messages.
5. Logout / local wipe are safe and distinct.

Incomplete non-core features may be stubbed or absent during early dogfood; that
is preferable to two live Matrix clients.

### D. Merge to `main`

- Integration branch is the system under test.
- Umbrella / `main` merge only with **explicit user approval**.
- Prefer confidence from CI + real client use over residual process theater.

---

## 4. Explicit non-goals

| Non-goal | Reason |
| --- | --- |
| Runtime JS/Rust selector | Permanent dual product; plan §3.1 |
| Elaborate JS→Rust session/token migration | Clean-break + D-TOKEN-CONTINUITY; crypto risk |
| Concurrent dual clients same session | Device/crypto corruption |
| File-by-file import rewrite without host owner | Wrong boundary; thrash |
| Blocking product slices on residual R0 formal thrash | Prefer real capability + real safety |

P3.7 legacy transition, if ever implemented, is **detection + reauth messaging +
optional inert cleanup** — not dual runtime and not token continuity into a fresh store.

---

## 5. Priority order for agents / orchestrators

1. Keep required CI green on product PRs; merge to integration when confident.
2. Advance the next **vertical capability slice** (auth → session → sync → timeline…).
3. Real safety only for residual R0 (privacy, store confinement, no dual-backend).
4. Docs-only / formal residual work never blocks product slices.
5. Never merge to `main` / PR #39 without explicit user approval.
6. Never add dual-backend, selector, or silent token/device reuse.

---

## 6. Relation to the 112-task plan

The phased plan (P0–P14) remains the **feature and risk checklist**. Execution
order is **capability vertical slices** toward dogfood cutover, then Phase 11
burn-down, then main merge. Inventory counts (`N/112`) measure artifact landing,
not “dual-SDK progress” and not automatic phase-gate closure.

Machine status: [`program-status.json`](program-status.json) /
[`program-status.md`](program-status.md).
