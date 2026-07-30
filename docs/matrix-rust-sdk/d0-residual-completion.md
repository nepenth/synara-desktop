# D0 residual completion — finish partial verticals before new work

| Field                     | Value                                                                  |
| ------------------------- | ---------------------------------------------------------------------- |
| Status                    | **Active — V-AUTH.1 #238 merged; V-ROOMS.1 #241, V-SEND.2 #239, and V-TIMELINE #240 remain draft candidates** (2026-07-30) |
| Policy                    | [full-vertical-policy.md](full-vertical-policy.md)                     |
| Integration tip at policy | `0400306` (D0.1–D0.5 merged; D0.5 was **crypto minimum**)              |
| Current integration tip   | `08a185e` (V-AUTH.1 #238 merged with required CI green)                  |
| Active PRs                | Draft [#239](https://github.com/nepenth/synara-desktop/pull/239) V-SEND.2; [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE; [#241](https://github.com/nepenth/synara-desktop/pull/241) V-ROOMS.1 |

## Policy trigger

User directive: **no incomplete cuts**. Anything previously accepted as “minimum / residual / plateau” must be **surfaced and fully re-implemented** before new verticals (media, widgets, registry, etc.) proceed.

## Do not merge as complete

| PR / artifact                                                                          | Why blocked under full-vertical policy                                         |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| **#221** D0.6 “approved residual plateau” (232 files / 292 imports; 0 imports removed) | Explicit incomplete shell + plateau; not full burn-down or full capability rewire |
| D0.5 docs claiming “crypto minimum” done                                               | Vertical incomplete until product crypto UX is native                          |

Leave #221 **draft / unmerged** unless reworked into a full vertical with empty residual for its claimed scope (or split into full slices). Do not treat “native shell without js client” alone as D0 complete.

---

## Priority completion queue (serial)

Order is **fix incomplete crypto first** (largest intentional product gap on tip), then widen earlier verticals that still leave product capabilities on js-sdk.

Status language in this ledger is strict:

- **wired** — the native product path landed, but superseded JS code/imports remain;
- **done** — native path, parity, tests, privacy boundary, and physical JS deletion all landed;
- **active** — the current serial slice/PR.

### V-CRYPTO — full crypto product vertical (was D0.5 residual)

**Done when:** native session owns the product crypto surfaces that Synara already ships (or SDK-supported equivalents), without matrix-js-sdk crypto client.

| ID             | Status                     | Capability                                   | Current evidence                                                     | Closure requirement                                                                                                                                                                                                                            |
| -------------- | -------------------------- | -------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-CRYPTO.1** | **DONE**                   | Device verification UX                       | Live `matrix/verification/live.rs`, IPC, native verification UI      | Native owner plus legacy verification deletion; 232/292 → 223/280 direct desktop-runtime imports; see [v-crypto-1-verification.md](v-crypto-1-verification.md)                                                                                 |
| **V-CRYPTO.2** | **DONE**                   | Cross-signing readiness/setup                | Live `matrix/cross_signing/live.rs`, setup/auth UI                   | Native owner plus legacy setup/status/reset deletion; 223/280 → 222/279 direct desktop-runtime imports; see [v-crypto-2-cross-signing.md](v-crypto-2-cross-signing.md)                                                                         |
| **V-CRYPTO.3** | **DONE**                   | Key backup restore/recovery                  | Live `matrix/backup/live.rs`, native backup UI/hooks                 | Native owner plus legacy UI/listener/progress deletion; 222/279 → 219/276 direct desktop-runtime imports; see [v-crypto-3-key-backup.md](v-crypto-3-key-backup.md)                                                                             |
| **V-CRYPTO.4** | **DONE**                   | SSSS bootstrap/unlock                        | Live `matrix/secret_storage/live.rs`, native secret-storage UI/hooks | Native owner plus legacy recovery derivation/checking, account-data path, JS key-cache, dead UI, and JS-only test deletion; 219/276 → 218/275 direct desktop-runtime imports; see [v-crypto-4-secret-storage.md](v-crypto-4-secret-storage.md) |
| **V-CRYPTO.5** | **DONE #227**              | Room-key export/import retained product path | Single Rust IPC owner; legacy WebView owner/helper deleted           | Reviewed-SHA validation proved parity, retry safety, privacy, deletion, and ledger evidence                                                                                                                                                    |
| **V-CRYPTO.6** | **DONE #235**              | Automatic UTD/history recovery               | Live managed timeline + P5.10/P8.7 state; safe event readback        | SDK-owned late-key readback, guided native recovery settings, and JS retry/decryption-listener deletion; see [v-crypto-6-utd-recovery.md](v-crypto-6-utd-recovery.md)                                                                          |
| **V-CRYPTO.7** | **DONE #236**             | Device list/trust presentation and actions   | Live `matrix/devices/live.rs`, SDK-neutral UI, native rename/delete  | Merged at `528a510`; reviewed, green product/test head `192be46`; Rust-owned snapshots/trust/readback and device-delete UIAA at delivery; V-AUTH.1 #238 removes its SSO continuation. JS `CryptoApi`, device model/listener/polling and dead UIA owners deleted; 218/273 → 212/265; live proof unclaimed; see [v-crypto-7-device-list-trust.md](v-crypto-7-device-list-trust.md) |

**Also required for V-CRYPTO complete:** encrypted timeline decrypt + encrypted send remain (already on tip from D0.5 machine path); extend with recovery so history is restorable when keys exist server-side.

Migration decision already decided: **`D-KEY-RECOVERY`** in [migration-ux-decision.md](migration-ux-decision.md).

### V-AUTH — complete auth vertical (D0.1 gaps)

| ID           | Capability                     | Residual today    | Done when                                                                    |
| ------------ | ------------------------------ | ----------------- | ---------------------------------------------------------------------------- |
| **V-AUTH.1** | Complete desktop SSO removal    | **Merged #238** at `08a185e` | Every desktop SSO entry point, browser/callback/token-completion route, JS SSO owner/import, and native device-delete SSO UIAA continuation is deleted. No Rust replacement, Matrix-ID prompt, inferred identity, pending-store/adoption route, or fallback. Required CI green; production importers 201→197 and repository-wide 215→211. |
| **V-AUTH.2** | Token login                    | Out of D0.1       | Native token login if product retains it                                     |
| **V-AUTH.3** | UIA flows used by product auth | Largely js        | UIA stages for retained flows native or product-owned without live js client |
| **V-AUTH.4** | Register / reset-password      | js                | Re-home if product keeps them on desktop                                     |

### V-ROOMS — room list / membership vertical (D0.2 gaps)

| ID            | Capability                           | Residual today | Done when                            |
| ------------- | ------------------------------------ | -------------- | ------------------------------------ |
| **V-ROOMS.1** | Invites list                         | Native candidate #241 rebased on V-AUTH; required CI running; production 197→194 | Native invites projection/actions/avatar route plus active JS-owner deletion |
| **V-ROOMS.2** | Spaces / hierarchy / lobby           | js             | Native space hierarchy ownership     |
| **V-ROOMS.3** | Unread / notification badges on list | partial        | Native unread map drives list badges |
| **V-ROOMS.4** | Typing indicators (if list/shell)    | js             | Native if product shows them         |

### V-TIMELINE — full timeline read vertical (D0.3 gaps)

| ID               | Capability                               | Residual today  | Done when                                      |
| ---------------- | ---------------------------------------- | --------------- | ---------------------------------------------- |
| **V-TIMELINE.1** | Virtualized timeline on native DTOs      | Unselected native DTO presenter exists; no active cutover | Restore viewport and retained render/action paths |
| **V-TIMELINE.2** | Live ordered updates (not only poll)     | Native ordered delta stream/strict presenter bridge exists; not active | Bind the active presenter to exact stream deltas |
| **V-TIMELINE.3** | Reactions, receipts, read markers        | not projected   | Native projections + UI                        |
| **V-TIMELINE.4** | Rich/media/state event render            | slim text path  | Parity renderers on native DTOs                |
| **V-TIMELINE.5** | Focused-event open / jump / pins / notes | residual        | Native ownership                               |

### V-SEND — full send vertical (D0.4 gaps)

| ID           | Capability                              | Residual today  | Done when                                     |
| ------------ | --------------------------------------- | --------------- | --------------------------------------------- |
| **V-SEND.1** | Attachments / media upload send         | js              | Native send queue + IPC                       |
| **V-SEND.2** | Reactions                               | Native candidate #239 rebased on V-AUTH; required CI running; direct JS owner calls reduced, importer files 197→197 | Native commands/readback plus active JS writer deletion |
| **V-SEND.3** | Polls                                   | js              | Native                                        |
| **V-SEND.4** | Emotes / notices / rich HTML + mentions | plain text only | Product parity for retained composer features |
| **V-SEND.5** | Threads                                 | residual        | Native thread send/relations                  |

### V-BURN — final convergence audit (real D0.6)

| ID           | Capability                                                  | Residual today                                             | Done when                                                                                                                 |
| ------------ | ----------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **V-BURN.1** | Prove no live JS client remains in any desktop product path | Per-vertical deletion should already have removed owners   | No product path constructs/starts `matrix-js-sdk`; guardrail and product tests enforce it                                 |
| **V-BURN.2** | Audit repository-wide importer zero                         | Import counts must already decrease per completed vertical | Zero production importers; any proposed types-only exception requires explicit product sign-off and a named removal owner |
| **V-BURN.3** | Drop npm dependency and obsolete JS stores/bootstrap        | blocked on V-BURN.2                                        | `package.json`, lockfile, startup/store/service-worker code, and allowlist are clean                                      |

V-BURN is not permission to defer known capability deletion. It catches
cross-cutting leftovers and removes the dependency after the owning verticals
have already deleted their implementations.

---

## Execution order (binding)

1. Validate V-ROOMS.1 and V-SEND.2 after their V-AUTH rebases; record only their measured deletion deltas.
2. Continue **V-AUTH** remaining desktop auth surfaces, deleting each superseded JS owner in its slice.
3. Continue **V-ROOMS** / **V-TIMELINE** / **V-SEND** gaps, with physical deletion per completed capability.
4. **V-BURN** final convergence audit + npm dependency removal
5. **Only then** new verticals: media display polish beyond send, widgets, registry, calls, etc. — each as full verticals under [full-vertical-policy.md](full-vertical-policy.md)

L1 modules under `src-tauri/src/matrix/{verification,backup,cross_signing,devices,room_keys,utd_recovery}/` are **inputs**, not done.

## Scoreboard (replacement metrics)

| Metric                                                         | Target                                                                               |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Open rows in this residual table                               | **0** for claimed-complete verticals                                                 |
| js importers for a claimed-complete capability                 | **0** production files                                                               |
| Capability-owner/file deletion delta per completed vertical    | **Negative and recorded**; zero-deletion completion is rejected                      |
| Repository-wide direct `matrix-js-sdk` import delta            | Recorded and non-increasing; zero is allowed only for an indirectly owned capability |
| New PRs with “minimum / incomplete / plateau residual” acceptance | **0**                                                                             |
| Phase-gate crypto / cutover claims                             | Only after V-CRYPTO + owning verticals complete                                      |

## Orchestrator

Loop must:

1. **Not** merge #221 as D0.6 complete.
2. Treat V-CRYPTO.1–.6 as done.
3. Preserve V-CRYPTO.5 [#227](https://github.com/nepenth/synara-desktop/pull/227) as done; its legacy deletion, retry safety, privacy, and reviewed-SHA evidence passed.
4. Execute V-AUTH.1 as complete desktop SSO removal: JS and native SSO ownership must both be deleted, with no replacement route.
5. Update [PROGRESS.md](PROGRESS.md) with product wiring, deletion deltas, and residual closure.
6. Refuse new L1-only or new non-residual verticals until this queue is cleared or user reorders explicitly.
