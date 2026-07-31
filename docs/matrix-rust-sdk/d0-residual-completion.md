# D0 residual completion — finish partial verticals before new work

<!-- matrix-rust-program-status-link -->

| Field                     | Value                                                                                                                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status                    | **Active — tip `e6db76c7`; this PR V-AUTH.3b login multi-stage UIA non-retention; parallel password loginUtil #279; #240 HOLD** (2026-07-31) |
| Policy                    | [full-vertical-policy.md](full-vertical-policy.md)                                                                                                                                            |
| Integration tip at policy | `0400306` (D0.1–D0.5 merged; D0.5 was **crypto minimum**)                                                                                                                                     |
| Current integration tip   | `e6db76c7` (after [#278](https://github.com/nepenth/synara-desktop/pull/278) / [#277](https://github.com/nepenth/synara-desktop/pull/277) / [#276](https://github.com/nepenth/synara-desktop/pull/276)) |
| Active PRs                | **This PR** V-AUTH.3b UIA login-stage non-retention; password `loginUtil` [#279](https://github.com/nepenth/synara-desktop/pull/279); [#240](https://github.com/nepenth/synara-desktop/pull/240) HOLD |

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
| **V-AUTH.2** | Token login                    | **DONE #262** at `56d1544` — product **non-retention** close; unused Rust token-login foundation removed | Explicit product decision: desktop does not retain `m.login.token` |
| **V-AUTH.3** | UIA / login-flow discovery for product auth | **DONE #276** at `4d33227f` — native `matrix_login_flows`; AuthFlowsLoader live js client deleted; allowlist **175→171**; production **172→169** | Discovery re-homed fail-closed |
| **V-AUTH.3b** | Login multi-stage UIA stage execution | **DONE — product non-retention (this PR)** — login is password-only single-shot; no multi-stage login UI/js client; no invented `matrix_uia_*` IPC; register/reset/device-delete stages already native | See [v-auth-3b-uia-login-stage.md](v-auth-3b-uia-login-stage.md) |
| **V-AUTH.3p** | Password login desktop fail-closed (`loginUtil`) | **Open** [#279](https://github.com/nepenth/synara-desktop/pull/279) — delete js `createClient`/`loginRequest` fallback; desktop only `matrix_login_password` | Independent of 3b; do not dual-edit #279 owners from this PR |
| **V-AUTH.4** | Register / reset-password      | Split: **4a DONE #263**; **4b DONE #266** (allowlist **191→175**; production import files **172**); tip after docs #274 is `48991e77` | 4a+4b native re-home complete; live Synapse registration e2e unclaimed |

### V-ROOMS — room list / membership vertical (D0.2 gaps)

| ID            | Capability                           | Residual today                                                                                                                                                                                            | Done when                                                                           |
| ------------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **V-ROOMS.1** | Invites list                         | **DONE #241** at integration `2c48fd45a08200a6e3491f100912f086e8458b3b`; candidate `7ac2c48` passed required scope, Synapse, desktop/runtime, and quality CI; production 197→194, repository-wide 211→208 | Native invites projection/actions/avatar route plus active JS-owner deletion landed |
| **V-ROOMS.2** | Spaces / hierarchy / lobby           | **2a DONE**; **2b DONE #254** hierarchy summaries; **2c DONE #268** local graph/mutations                                                                 | Native space hierarchy ownership (2a parent map + 2b lobby + 2c writers)            |
| **V-ROOMS.2a** | Space parent map (nav/unread)       | **DONE #247** at integration `a9196894edff24bddb76279aaf010fa8f56ffe47`; production **190→189**, repository-wide **203→202**; live proof Not confirmed (not a reopen blocker)                               | Native parent map; JS roomToParents binder deleted                                  |
| **V-ROOMS.2b** | Lobby hierarchy-summary reads      | **DONE #254** at `9c0b51e`; native typed hierarchy pagination; JS `getRoomHierarchy` / `IHierarchyRoom` deleted; files **187 / 200**, direct lines **249→244**; live proof unclaimed                                     | Native hierarchy summaries; JS hierarchy request/types deleted                      |
| **V-ROOMS.2c** | Local graph + mutations/reordering | **DONE #268** at `ac6ae435` — native `matrix_space_children_snapshot` / `matrix_space_child_set` / `matrix_space_child_remove` / `matrix_restricted_join_reparent`; JS SpaceChild writers deleted; inventory flat **184/197**; live authenticated proof unclaimed. #267 partial **CLOSED** (prefer full #268). | Native graph/readback and mutation commands; delete superseded JS owners             |
| **V-ROOMS.3** | Unread / notification badges on list | **DONE #245** at integration `efc90d59e6009f45589ce42a29a6f7ebafcf7624`; candidate `a81e026`; production **194→192**, repository-wide **208→205**; live badge proof Not confirmed (not a reopen blocker) | Native unread map drives list badges; JS unread owner deleted                       |
| **V-ROOMS.4** | Typing indicators (if list/shell)    | **DONE #246** at integration `151948c8c2329ee6f0b37b8757607b3ac8bb44e7`; candidate `c4df9ed`; production **192→190**, repository-wide **205→203**; live typing proof Not confirmed (not a reopen blocker) | Native typing projection + send; JS typing owners deleted                           |
| **V-ROOMS.5** | `m.direct` DM room-id map            | **DONE #249** at `d17ab2c` (candidate `708aef7`); production **189→187** / **202→200**; live proof Not confirmed                                                                                                                                   | Native DM map read; JS mDirect binder deleted                                       |
| **V-ROOMS.5w** | `m.direct` DM writers               | **DONE #251** at `0fb0fe4` (candidate `e4e2639`); native add/remove; JS writer helpers deleted; importers **187→187**                                                                                                                                 | Native write IPC; JS writer helpers deleted                                         |
| **V-ROOMS.5r** | `m.direct` DM user list             | **DONE #252** at `9579ea4`; `matrix_mdirect_snapshot.userIds` owns `mDirectUsersAtom` / `useDirectUsers`; JS AccountData reader deleted; importers **187→187**; live proof Not confirmed                                                                 | Native user-list projection; JS useDirectUsers AccountData owner deleted            |

### V-TIMELINE — full timeline read vertical (D0.3 gaps)

| ID               | Capability                               | Residual today                                                                                                                                                                                                                                                                                                                                                                                   | Done when                                                                                       |
| ---------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| **V-TIMELINE.1** | Virtualized timeline on native DTOs      | **HOLD-cutover** [#240](https://github.com/nepenth/synara-desktop/pull/240): implement/tip-merge OK; presenter **unselected**; no `RoomTimeline` deletion; no cutover claim. Native DTO shell + action gates exist; full product parity and live authenticated viewport proof remain open. Feature-parity audit CI may fail on the draft head — not a green cutover signal. | Complete all render/action/media paths and runtime proof before presenter selection or deletion |
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
| **V-SEND.4** | Emotes / notices / rich HTML + mentions | **DONE #253** at `b558344`; native `matrix_send_text` owns emote/notice/HTML/mentions/reply; composer JS fallback removed for retained types; Synapse rich-message proof gated in CI                                                                                                  | Product parity for retained composer features           |
| **V-SEND.5** | Threads                                 | **DONE #258** at `5c6e6e8`; native composer text/attachment `m.thread` via `threadRoot`; Synapse thread-send proof Confirmed; sticker/GIF/poll-in-thread + thread list/timeline remain residual | Native thread send/relations                            |

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

1. Treat V-ROOMS.1 as integrated at `2c48fd45a08200a6e3491f100912f086e8458b3b`; retain its measured production 197→194 and repository-wide 211→208 deletion deltas.
2. Serial after tip `e6db76c7`: V-AUTH.3 **DONE #276**; **this PR** closes V-AUTH.3b login multi-stage UIA as **non-retention**. Remaining auth residual: password `loginUtil` fail-closed [#279](https://github.com/nepenth/synara-desktop/pull/279). V-TIMELINE [#240] HOLD-cutover only.
3. Keep V-TIMELINE [#240] **HOLD-cutover**: implement and tip-merge are OK; **do not** select `NativeTimelinePresenter`, delete `RoomTimeline.tsx`, or claim cutover until full contract + runtime proof.
4. After V-AUTH.3b: do **not** invent unused `matrix_uia_*` login-stage IPC; land password `loginUtil` residual via #279 (or tip-merge equivalent) without dual_backend.
5. **V-BURN** final convergence audit + npm dependency removal
6. **Only then** new verticals: media display polish beyond send, widgets, registry, calls, etc. — each as full verticals under [full-vertical-policy.md](full-vertical-policy.md)

L1 modules under `src-tauri/src/matrix/{verification,backup,cross_signing,devices,room_keys,utd_recovery}/` are **inputs**, not done.

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
2. Treat V-CRYPTO.1–.6 as done.
3. Preserve V-CRYPTO.5 [#227](https://github.com/nepenth/synara-desktop/pull/227) as done; its legacy deletion, retry safety, privacy, and reviewed-SHA evidence passed.
4. Execute V-AUTH.1 as complete desktop SSO removal: JS and native SSO ownership must both be deleted, with no replacement route.
5. Update [PROGRESS.md](PROGRESS.md) with product wiring, deletion deltas, and residual closure.
6. Refuse new L1-only or new non-residual verticals until this queue is cleared or user reorders explicitly.

See [program-status.md](program-status.md).
