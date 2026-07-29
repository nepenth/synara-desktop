# Cutover operating model — Matrix Rust SDK full replacement

| Field              | Value                                                                                                                                                      |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status             | **Canonical** (product policy + execution model)                                                                                                           |
| Date               | 2026-07-27                                                                                                                                                 |
| Integration branch | `feature/matrix-rust-sdk-full-replacement`                                                                                                                 |
| Authoritative plan | [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md) §3, Phase 11                                                  |
| Related            | [`migration-ux-decision.md`](migration-ux-decision.md), [`implementation-handoff.md`](implementation-handoff.md), [`program-status.md`](program-status.md) |

This document is the **single short canonical statement** of _how_ we execute the
replacement. Task inventories and phase lists still live in the full plan; live
delivery state lives in the status ledger. When those conflict with this model on
_execution shape_, this model wins until the plan is updated to match.

---

## 1. End state (non-negotiable)

| Requirement                       | Meaning                                                                                  |
| --------------------------------- | ---------------------------------------------------------------------------------------- |
| **One Matrix owner**              | After cutover, only Matrix Rust SDK owns session, sync, crypto, and room state           |
| **No dual production backend**    | No user-visible or hidden runtime flag to choose JS vs Rust                              |
| **No long-term dual maintenance** | Do not keep both SDKs as coequal product paths                                           |
| **Branch → prove → merge**        | Build and dogfood on the integration branch; merge to `main` only with explicit approval |
| **Clean-break client migration**  | Re-login, clear cache, wipe local Matrix dirs are acceptable for the sole desktop user   |

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

## 2b. D0 full-vertical pivot (active 2026-07-28)

**Current execution mode on this branch:** [d0-dogfood-epic.md](d0-dogfood-epic.md).

- Stop prioritizing new L1-only harness PRs.
- **Break the branch product** if needed: Rust owns Matrix capability-by-capability.
- The former D0.1–D0.5 “dogfood minimum” sequence is historical and incomplete.
- Current binding order is the residual queue in [d0-residual-completion.md](d0-residual-completion.md): crypto completion/deletion → auth → rooms → timeline → send → final convergence.
- No dual-backend. Clean-break re-login OK. Temporary brokenness between serial full verticals is acceptable.
- **Physical deletion happens per vertical**; final convergence is not a bulk warehouse for retained JS owners.

## 3. Execution model (what we do)

### A. Capability-first vertical slices on the integration branch

Build and prove the **Rust Matrix owner** by user-visible capability, not by random
TS import sites:

| Priority band     | Example slices                                                  |
| ----------------- | --------------------------------------------------------------- |
| Session entry     | Discovery, login-flow list, password/token login, device naming |
| Session lifecycle | Refresh/persist secrets, restore, logout, wipe                  |
| Live state        | Sync readiness, room list, membership, unread                   |
| Timeline          | Read/send/paginate, receipts, relations                         |
| Crypto            | Verification, backup/recovery as required                       |
| Rest of product   | Spaces, search, media, calls, settings, …                       |

Each slice:

1. Implement under `src-tauri/src/matrix/` (+ IPC/DTO contracts as needed).
2. Unit / harness / CI evidence (no secrets in logs or DTOs).
3. Wire the product UI to the native owner.
4. Delete the superseded JS implementation/imports, compatibility branch, and obsolete tests/types for that capability.
5. Record the importer/file delta and update the residual/progress ledgers.
6. Do **not** introduce a backend selector.

UI rewires talk only to **Synara IPC/DTOs** once the host owns that capability —
never to a second Matrix client.

### B. Interim monorepo state is transitional, not dual-product

While the serial vertical queue is incomplete:

- A native session may use Rust-owned completed/wired capabilities while untouched product capabilities still have legacy JS code in the repository.
- The same signed-in session must never start both live Matrix clients.
- Temporary legacy code exists only until its owning vertical deletes it; Synara does not preserve it as a supported browser product.
- Status must distinguish L1 foundation, L2 live wiring, L3 per-vertical product cutover, and L4 repository convergence.

This is temporary scaffolding on the feature branch. It is **not** a supported
dual-SDK product mode and must not grow a config flag.

### C. Per-vertical cutover, then repository convergence

For each claimed product capability:

1. The managed Rust client and IPC path become the capability owner.
2. The UI consumes Synara-owned DTOs and intentions only.
3. The replaced JS implementation/imports are deleted in the same vertical.
4. Shared UI is made SDK-neutral rather than duplicated behind a native/legacy branch.
5. Importer counts decrease and guardrails prohibit new importers.

The initial D0 core wiring landed before the full-deletion clarification. Those
rows remain incomplete until their named deletion residuals close; “the native
path works” is not retroactive acceptance.

After all owning verticals have performed their deletion, V-BURN:

1. proves no live JS client or product importer remains;
2. removes obsolete bootstrap, stores, allowlists, service-worker ownership, and tests;
3. drops the `matrix-js-sdk` dependency and lockfile entries;
4. runs final product/release qualification.

### D. Merge to `main`

- Integration branch is the system under test.
- Umbrella / `main` merge only with **explicit user approval**.
- Prefer confidence from CI + real client use over residual process theater.

---

## 4. Explicit non-goals

| Non-goal                                                | Reason                                                                 |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| Runtime JS/Rust selector                                | Permanent dual product; plan §3.1                                      |
| Elaborate JS→Rust session/token migration               | Clean-break + D-TOKEN-CONTINUITY; crypto risk                          |
| Concurrent dual clients same session                    | Device/crypto corruption                                               |
| File-by-file import rewrite without host owner          | Wrong boundary; thrash                                                 |
| Native branch plus retained legacy branch declared done | Defers physical convergence and creates two maintained implementations |
| Blocking product slices on residual R0 formal thrash    | Prefer real capability + real safety                                   |

P3.7 legacy transition, if ever implemented, is **detection + reauth messaging +
optional inert cleanup** — not dual runtime and not token continuity into a fresh store.

---

## 5. Priority order for agents / orchestrators

1. Keep required CI green on product PRs; merge to integration when confident.
2. Advance the next residual/full vertical in [d0-residual-completion.md](d0-residual-completion.md), including physical deletion.
3. Real safety only for residual R0 (privacy, store confinement, no dual-backend).
4. Docs-only / formal residual work never blocks product slices.
5. Never merge to `main` / PR #39 without explicit user approval.
6. Never add dual-backend, selector, or silent token/device reuse.

---

## 6. Relation to the 112-task plan

The phased plan (P0–P14) remains the **feature and risk checklist**. Execution
order is **capability vertical slices** with per-vertical deletion, then Phase 11
convergence/dependency removal, then main merge. Inventory counts (`N/112`) measure artifact landing,
not “dual-SDK progress” and not automatic phase-gate closure.

Machine status: [`program-status.json`](program-status.json) /
[`program-status.md`](program-status.md).

Delivery layers (L1 harness vs L2 live vs L3 cutover): [`delivery-layers.md`](delivery-layers.md).
