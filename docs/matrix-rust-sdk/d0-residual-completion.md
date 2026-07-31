# D0 residual completion — finish partial verticals before new work

| Field                     | Value                                                                                                                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status                    | **Active — tip `fe62885` after #253 V-SEND.4 + #256 docs handoff; serial next #254 hierarchy (CI)** (2026-07-31)                                                                              |
| Policy                    | [full-vertical-policy.md](full-vertical-policy.md)                                                                                                                                            |
| Integration tip at policy | `0400306` (D0.1–D0.5 merged; D0.5 was **crypto minimum**)                                                                                                                                     |
| Current integration tip   | `fe628859836f0b3e8f6dab696f7739074f28311c` (docs [#256](https://github.com/nepenth/synara-desktop/pull/256) after product [#253](https://github.com/nepenth/synara-desktop/pull/253) `b558344`) |
| Active PRs                | Draft [#254](https://github.com/nepenth/synara-desktop/pull/254) V-ROOMS.2b hierarchy (CI); [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE **HOLD-merge**                |
| Execution                 | **Grok-only**; `rig-operating-path` skill under `.grok/skills/` (local agent skill)                                                                                                           |
| Tip inventory (measured)  | `synara/src` import files **187** production / **10** test / **197** total; repository-wide import files **200** (committed inventory); string refs under `synara/src` **205**                 |

## Policy trigger

User directive: **no incomplete cuts**. Anything previously accepted as “minimum / residual / plateau” must be **surfaced and fully re-implemented** before new verticals (media, widgets, registry, etc.) proceed.

## Do not merge as complete

| PR / artifact                                                                          | Why blocked under full-vertical policy                                            |
| -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| **#221** D0.6 “approved residual plateau” (232 files / 292 imports; 0 imports removed) | Explicit incomplete shell + plateau; not full burn-down or full capability rewire |
| D0.5 docs claiming “crypto minimum” done                                               | Vertical incomplete until product crypto UX is native                             |

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

| ID             | Status        | Capability                                   | Current evidence                                                     | Closure requirement                                                                                                                                                                                                                                                                                                                                                              |
| -------------- | ------------- | -------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-CRYPTO.1** | **DONE**      | Device verification UX                       | Live `matrix/verification/live.rs`, IPC, native verification UI      | Native owner plus legacy verification deletion; 232/292 → 223/280 direct desktop-runtime imports; see [v-crypto-1-verification.md](v-crypto-1-verification.md)                                                                                                                                                                                                                   |
| **V-CRYPTO.2** | **DONE**      | Cross-signing readiness/setup                | Live `matrix/cross_signing/live.rs`, setup/auth UI                   | Native owner plus legacy setup/status/reset deletion; 223/280 → 222/279 direct desktop-runtime imports; see [v-crypto-2-cross-signing.md](v-crypto-2-cross-signing.md)                                                                                                                                                                                                           |
| **V-CRYPTO.3** | **DONE**      | Key backup restore/recovery                  | Live `matrix/backup/live.rs`, native backup UI/hooks                 | Native owner plus legacy UI/listener/progress deletion; 222/279 → 219/276 direct desktop-runtime imports; see [v-crypto-3-key-backup.md](v-crypto-3-key-backup.md)                                                                                                                                                                                                               |
| **V-CRYPTO.4** | **DONE**      | SSSS bootstrap/unlock                        | Live `matrix/secret_storage/live.rs`, native secret-storage UI/hooks | Native owner plus legacy recovery derivation/checking, account-data path, JS key-cache, dead UI, and JS-only test deletion; 219/276 → 218/275 direct desktop-runtime imports; see [v-crypto-4-secret-storage.md](v-crypto-4-secret-storage.md)                                                                                                                                   |
| **V-CRYPTO.5** | **DONE #227** | Room-key export/import retained product path | Single Rust IPC owner; legacy WebView owner/helper deleted           | Reviewed-SHA validation proved parity, retry safety, privacy, deletion, and ledger evidence                                                                                                                                                                                                                                                                                      |
| **V-CRYPTO.6** | **DONE #235** | Automatic UTD/history recovery               | Live managed timeline + P5.10/P8.7 state; safe event readback        | SDK-owned late-key readback, guided native recovery settings, and JS retry/decryption-listener deletion; see [v-crypto-6-utd-recovery.md](v-crypto-6-utd-recovery.md)                                                                                                                                                                                                            |
| **V-CRYPTO.7** | **DONE #236** | Device list/trust presentation and actions   | Live `matrix/devices/live.rs`, SDK-neutral UI, native rename/delete  | Merged at `528a510`; reviewed, green product/test head `192be46`; Rust-owned snapshots/trust/readback and device-delete UIAA at delivery; V-AUTH.1 #238 removes its SSO continuation. JS `CryptoApi`, device model/listener/polling and dead UIA owners deleted; 218/273 → 212/265; live proof unclaimed; see [v-crypto-7-device-list-trust.md](v-crypto-7-device-list-trust.md) |

**Also required for V-CRYPTO complete:** encrypted timeline decrypt + encrypted send remain (already on tip from D0.5 machine path); extend with recovery so history is restorable when keys exist server-side.

Migration decision already decided: **`D-KEY-RECOVERY`** in [migration-ux-decision.md](migration-ux-decision.md).

### V-AUTH — complete auth vertical (D0.1 gaps)

| ID           | Capability                     | Residual today               | Done when                                                                                                                                                                                                                                                                                                                                 |
| ------------ | ------------------------------ | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V-AUTH.1** | Complete desktop SSO removal   | **Merged #238** at `08a185e` | Every desktop SSO entry point, browser/callback/token-completion route, JS SSO owner/import, and native device-delete SSO UIAA continuation is deleted. No Rust replacement, Matrix-ID prompt, inferred identity, pending-store/adoption route, or fallback. Required CI green; production importers 201→197 and repository-wide 215→211. |
| **V-AUTH.2** | Token login                    | Out of D0.1                  | Native token login if product retains it                                                                                                                                                                                                                                                                                                  |
| **V-AUTH.3** | UIA flows used by product auth | Largely js                   | UIA stages for retained flows native or product-owned without live js client                                                                                                                                                                                                                                                              |
| **V-AUTH.4** | Register / reset-password      | js                           | Re-home if product keeps them on desktop                                                                                                                                                                                                                                                                                                  |

### V-ROOMS — room list / membership vertical (D0.2 gaps)

| ID            | Capability                           | Residual today                                                                                                                                                                                            | Done when                                                                           |
| ------------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **V-ROOMS.1** | Invites list                         | **DONE #241** at integration `2c48fd45a08200a6e3491f100912f086e8458b3b`; candidate `7ac2c48` passed required scope, Synapse, desktop/runtime, and quality CI; production 197→194, repository-wide 211→208 | Native invites projection/actions/avatar route plus active JS-owner deletion landed |
| **V-ROOMS.2** | Spaces / hierarchy / lobby           | **2a DONE**; **2b active draft** [#254](https://github.com/nepenth/synara-desktop/pull/254) — hierarchy not complete until merge + deletion evidence                                                                                           | Native space hierarchy ownership (2a parent map + 2b lobby/summaries)               |
| **V-ROOMS.2a** | Space parent map (nav/unread)       | **DONE #247** at integration `a9196894edff24bddb76279aaf010fa8f56ffe47`; production **190→189**, repository-wide **203→202**; live proof Not confirmed (not a reopen blocker)                               | Native parent map; JS roomToParents binder deleted                                  |
| **V-ROOMS.2b** | Space hierarchy summaries / lobby   | **Active draft** [#254](https://github.com/nepenth/synara-desktop/pull/254) head `e3a0b3d` rebased on tip after #253; CI in progress — **not merged; not done** until native+deletion close                                                    | Native hierarchy summary ownership; residual graph mutations called out explicitly  |
| **V-ROOMS.3** | Unread / notification badges on list | **DONE #245** at integration `efc90d59e6009f45589ce42a29a6f7ebafcf7624`; candidate `a81e026`; production **194→192**, repository-wide **208→205**; live badge proof Not confirmed (not a reopen blocker) | Native unread map drives list badges; JS unread owner deleted                       |
| **V-ROOMS.4** | Typing indicators (if list/shell)    | **DONE #246** at integration `151948c8c2329ee6f0b37b8757607b3ac8bb44e7`; candidate `c4df9ed`; production **192→190**, repository-wide **205→203**; live typing proof Not confirmed (not a reopen blocker) | Native typing projection + send; JS typing owners deleted                           |
| **V-ROOMS.5** | `m.direct` DM room-id map            | **DONE #249** at `d17ab2c` (candidate `708aef7`); production **189→187** / **202→200**; live proof Not confirmed                                                                                                                                   | Native DM map read; JS mDirect binder deleted                                       |
| **V-ROOMS.5w** | `m.direct` DM writers               | **DONE #251** at `0fb0fe4` (candidate `e4e2639`); native add/remove; JS writer helpers deleted; importers **187→187**                                                                                                                                 | Native write IPC; JS writer helpers deleted                                         |
| **V-ROOMS.5r** | `m.direct` DM user list             | **Merged** [#252](https://github.com/nepenth/synara-desktop/pull/252) at `9579ea4`; native `userIds` owns `mDirectUsersAtom` / `useDirectUsers`; JS AccountData reader deleted; importers **187→187**                                              | Native user-list projection; JS useDirectUsers AccountData owner deleted            |

### V-TIMELINE — full timeline read vertical (D0.3 gaps)

| ID               | Capability                               | Residual today                                                                                                                                                                                                                                                                                                                                                                                   | Done when                                                                                       |
| ---------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| **V-TIMELINE.1** | Virtualized timeline on native DTOs      | **HOLD-merge** [#240](https://github.com/nepenth/synara-desktop/pull/240) (head `bf9ac48` at truth-up; incomplete full-replacement contract). Presenter unselected; **do not** delete `RoomTimeline.tsx`. Continue implementation after #254; merge only when native+deletion+runtime proof close. | Complete all render/action/media paths and runtime proof before presenter selection or deletion |
| **V-TIMELINE.2** | Live ordered updates (not only poll)     | Native ordered delta stream/strict presenter bridge exists; not active                                                                                                                                                                                                                                                                                                                           | Bind the active presenter to exact stream deltas                                                |
| **V-TIMELINE.3** | Reactions, receipts, read markers        | Native read/unread command/readback; reaction candidate separate                                                                                                                                                                                                                                                                                                                                 | Native projections + UI and unread-position frontier                                            |
| **V-TIMELINE.4** | Rich/media/state event render            | Native opaque media route and image/file/audio/video/sticker projection exist in draft `5e0c2a5`; remaining rich/state parity is open                                                                                                                                                                                                                                                            | Parity renderers on native DTOs                                                                 |
| **V-TIMELINE.5** | Focused-event open / jump / pins / notes | Native focused and normal-open placement exist; active jump/pins/notes remain residual                                                                                                                                                                                                                                                                                                           | Native ownership                                                                                |

### V-SEND — full send vertical (D0.4 gaps)

| ID           | Capability                              | Residual today                                                                                                                                                                                                                                                                         | Done when                                               |
| ------------ | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **V-SEND.1** | Attachments / media upload send         | **DONE #248** at integration `90be0f4`; Synapse native attachment proof Confirmed; GIF/avatar/call/forward residual remain elsewhere                                                                                      | Native send queue + IPC + JS owner deletion             |
| **V-SEND.2** | Reactions                               | **Merged** [#239](https://github.com/nepenth/synara-desktop/pull/239) at `988cdc2`; Synapse native reaction proof Confirmed on reviewed head                                                                                                                                           | Native commands/readback plus active JS writer deletion |
| **V-SEND.3** | Polls                                   | **Merged** [#250](https://github.com/nepenth/synara-desktop/pull/250) at `88ed143`; Synapse native poll proof Confirmed on reviewed head `761d2ef`; relation read residual remains in V-TIMELINE                                                                                  | Native                                                  |
| **V-SEND.4** | Emotes / notices / rich HTML + mentions | **Merged** [#253](https://github.com/nepenth/synara-desktop/pull/253) at `b558344`; native `matrix_send_text` owns type/HTML/mentions/reply; composer `mx.sendMessage` emote/notice fallback removed; Synapse rich-message proof on CI; whole importers **187→187** (no whole-file importer burn) | Product parity for retained composer features           |
| **V-SEND.5** | Threads                                 | residual                                                                                                                                                                                                                                                                               | Native thread send/relations                            |

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

1. **Finish #254** V-ROOMS.2b hierarchy (required CI green → undraft → merge). Do not claim hierarchy done without native ownership **and** physical JS-owner deletion on the merged head.
2. **Continue #240** V-TIMELINE as **HOLD-merge**: implement toward the full replacement contract; **do not** select `NativeTimelinePresenter` or delete `RoomTimeline.tsx` until render/action/media route + runtime proof close.
3. Remaining residual rows in this ledger (V-SEND.5 threads, remaining V-AUTH/V-ROOMS gaps, etc.), each as full verticals with measured deletion deltas.
4. Keep **#221** D0.6 plateau unmerged (zero-deletion incomplete shell).
5. Keep **L1** foundation PRs parked (`#109`, `#193`, `#196`, `#198`, `#199`, `#201`, `#203`, `#204`, `#207`, `#208`, `#209`) while the residual queue is active — L1 modules under `src-tauri/src/matrix/{verification,backup,cross_signing,devices,room_keys,utd_recovery}/` are **inputs**, not done.
6. **V-BURN** final convergence audit + npm dependency removal only after owning verticals delete their importers.
7. **Only then** new verticals: media display polish beyond send, widgets, registry, calls, etc. — each as full verticals under [full-vertical-policy.md](full-vertical-policy.md).

Historical retention: V-ROOMS.1 production **197→194** / repository-wide **211→208**; tip inventory remains production **187** / repository-wide **200** until a later vertical records a new measured delta.

## Scoreboard (replacement metrics)

| Metric                                                            | Target                                                                               |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Open rows in this residual table                                  | **0** for claimed-complete verticals                                                 |
| js importers for a claimed-complete capability                    | **0** production files                                                               |
| Capability-owner/file deletion delta per completed vertical       | **Negative and recorded**; zero-deletion completion is rejected                      |
| Repository-wide direct `matrix-js-sdk` import delta               | Recorded and non-increasing; zero is allowed only for an indirectly owned capability |
| New PRs with “minimum / incomplete / plateau residual” acceptance | **0**                                                                                |
| Phase-gate crypto / cutover claims                                | Only after V-CRYPTO + owning verticals complete                                      |

## Orchestrator

Loop must:

1. **Not** merge #221 as D0.6 complete.
2. Treat V-CRYPTO.1–.7 as done only where native + deletion evidence already landed; do not re-open as “wired only.”
3. Preserve V-CRYPTO.5 [#227](https://github.com/nepenth/synara-desktop/pull/227) as done; its legacy deletion, retry safety, privacy, and reviewed-SHA evidence passed.
4. Preserve V-AUTH.1 [#238](https://github.com/nepenth/synara-desktop/pull/238) SSO removal (no replacement route).
5. Serial after tip: **#254 → #240 → remaining residual rows**. Grok-only execution.
6. Update [PROGRESS.md](PROGRESS.md) / [CONTINUATION.md](CONTINUATION.md) / [SESSION-HANDOFF.md](SESSION-HANDOFF.md) with product wiring, deletion deltas, and residual closure.
7. Refuse new L1-only or new non-residual verticals until this queue is cleared or user reorders explicitly.
8. Never relabel “merged native path” as vertical **done** without physical JS-owner deletion evidence for that capability.
