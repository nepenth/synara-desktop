# D0 residual completion — finish partial verticals before new work

| Field | Value |
| --- | --- |
| Status | **Active — blocking forward work** (2026-07-28) |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) |
| Integration tip at policy | `0400306` (D0.1–D0.5 merged; D0.5 was **crypto minimum**) |

## Policy trigger

User directive: **no dogfood cuts**. Anything previously accepted as “minimum / residual / plateau” must be **surfaced and fully re-implemented** before new verticals (media, widgets, registry, etc.) proceed.

## Do not merge as complete

| PR / artifact | Why blocked under full-vertical policy |
| --- | --- |
| **#221** D0.6 “approved residual plateau” (232 files / 292 imports; 0 imports removed) | Explicit dogfood shell + plateau; not full burn-down or full capability rewire |
| D0.5 docs claiming “crypto minimum” done | Vertical incomplete until product crypto UX is native |

Leave #221 **draft / unmerged** unless reworked into a full vertical with empty residual for its claimed scope (or split into full slices). Do not treat “native shell without js client” alone as D0 complete.

---

## Priority completion queue (serial)

Order is **fix incomplete crypto first** (largest intentional product gap on tip), then widen earlier verticals that still leave product capabilities on js-sdk.

### V-CRYPTO — full crypto product vertical (was D0.5 residual)

**Done when:** native session owns the product crypto surfaces that Synara already ships (or SDK-supported equivalents), without matrix-js-sdk crypto client.

| ID | Capability | Product surfaces today (js) | Host foundations (parts only) | Done when |
| --- | --- | --- | --- | --- |
| **V-CRYPTO.1** | Device verification UX | `DeviceVerification*`, settings `Verification`, `useVerificationRequest`, `verification.ts` | `matrix/verification/*` | Verify/inbound request flows via Rust IPC + SDK; no js `CryptoApi` for native session |
| **V-CRYPTO.2** | Cross-signing readiness / setup product path | Device verification setup, status | `matrix/cross_signing/*` | Bootstrap/status/setup product path native; secrets never in webview |
| **V-CRYPTO.3** | Key backup restore / recovery UI | `BackupRestore`, `useKeyBackup`, `backupRestore.ts` | `matrix/backup/*` | Restore/setup/repair via Rust; privacy-safe errors only |
| **V-CRYPTO.4** | SSSS / secret storage bootstrap & unlock | `SecretStorage.tsx` | SDK secret storage + P8 parts | Unlock/bootstrap import UI native; no secrets over IPC |
| **V-CRYPTO.5** | Interactive key-share / room-key flows | (js crypto paths) | `matrix/room_keys/*` | Key-share prompts / export-import advanced path as product retained |
| **V-CRYPTO.6** | UTD recovery UX | timeline UTD placeholder only today | `matrix/utd_recovery/*`, timeline UTD | User-visible recovery/retry controls; not permanent opaque UTD only |
| **V-CRYPTO.7** | Device list / trust presentation | `DeviceTile`, `OtherDevices`, device hooks | `matrix/devices/*` | Device list + trust badges from Rust projections |

**Also required for V-CRYPTO complete:** encrypted timeline decrypt + encrypted send remain (already on tip from D0.5 machine path); extend with recovery so history is restorable when keys exist server-side.

Migration decision already decided: **`D-KEY-RECOVERY`** in [migration-ux-decision.md](migration-ux-decision.md).

### V-AUTH — complete auth vertical (D0.1 gaps)

| ID | Capability | Residual today | Done when |
| --- | --- | --- | --- |
| **V-AUTH.1** | SSO login | Out of D0.1 scope | Desktop SSO happy path via Rust, no js `createClient` for session |
| **V-AUTH.2** | Token login | Out of D0.1 | Native token login if product retains it |
| **V-AUTH.3** | UIA flows used by product auth | Largely js | UIA stages for retained flows native or product-owned without live js client |
| **V-AUTH.4** | Register / reset-password | js | Re-home if product keeps them on desktop |

### V-ROOMS — room list / membership vertical (D0.2 gaps)

| ID | Capability | Residual today | Done when |
| --- | --- | --- | --- |
| **V-ROOMS.1** | Invites list | js residual | Native invites projection + UI |
| **V-ROOMS.2** | Spaces / hierarchy / lobby | js | Native space hierarchy ownership |
| **V-ROOMS.3** | Unread / notification badges on list | partial | Native unread map drives list badges |
| **V-ROOMS.4** | Typing indicators (if list/shell) | js | Native if product shows them |

### V-TIMELINE — full timeline read vertical (D0.3 gaps)

| ID | Capability | Residual today | Done when |
| --- | --- | --- | --- |
| **V-TIMELINE.1** | Virtualized timeline on native DTOs | not virtualized | Restore viewport/virtualization on native rows |
| **V-TIMELINE.2** | Live ordered updates (not only poll) | 1s poll | Streamed/diff updates from host |
| **V-TIMELINE.3** | Reactions, receipts, read markers | not projected | Native projections + UI |
| **V-TIMELINE.4** | Rich/media/state event render | slim text path | Parity renderers on native DTOs |
| **V-TIMELINE.5** | Focused-event open / jump / pins / notes | residual | Native ownership |

### V-SEND — full send vertical (D0.4 gaps)

| ID | Capability | Residual today | Done when |
| --- | --- | --- | --- |
| **V-SEND.1** | Attachments / media upload send | js | Native send queue + IPC |
| **V-SEND.2** | Reactions | js | Native |
| **V-SEND.3** | Polls | js | Native |
| **V-SEND.4** | Emotes / notices / rich HTML + mentions | plain text only | Product parity for retained composer features |
| **V-SEND.5** | Threads | residual | Native thread send/relations |

### V-BURN — import burn-down (real D0.6)

| ID | Capability | Residual today | Done when |
| --- | --- | --- | --- |
| **V-BURN.1** | Remove live js client from all desktop product paths | #221 plateau only | No desktop product path constructs/starts matrix-js-sdk client |
| **V-BURN.2** | Drive `matrix-js-sdk` product importers toward **0** (or types moved to product DTOs) | ~232 files / ~292 imports | Count decreases per PR; zero or signed types-only residual with product approval |
| **V-BURN.3** | Drop npm dependency when importers are gone | blocked on V-BURN.2 | package.json clean |

---

## Execution order (binding)

1. **V-CRYPTO.1 → .7** (full crypto product; closes D0.5 dogfood debt)  
2. **V-AUTH** remaining desktop auth surfaces  
3. **V-ROOMS** / **V-TIMELINE** / **V-SEND** gaps for capabilities the product still exposes  
4. **V-BURN** real burn-down (not plateau)  
5. **Only then** new verticals: media display polish beyond send, widgets, registry, calls, etc. — each as full verticals under [full-vertical-policy.md](full-vertical-policy.md)

L1 modules under `src-tauri/src/matrix/{verification,backup,cross_signing,devices,room_keys,utd_recovery}/` are **inputs**, not done.

## Scoreboard (replace dogfood metrics)

| Metric | Target |
| --- | --- |
| Open rows in this residual table | **0** for claimed-complete verticals |
| js importers for a claimed-complete capability | **0** production files |
| New PRs with “minimum / dogfood / plateau residual” acceptance | **0** |
| Phase-gate crypto / cutover claims | Only after V-CRYPTO + owning verticals complete |

## Orchestrator

Loop must:

1. **Not** merge #221 as D0.6 complete.  
2. Dispatch **V-CRYPTO** full product slices next (start V-CRYPTO.1).  
3. Update [PROGRESS.md](PROGRESS.md) when residual rows close.  
4. Refuse new L1-only or new non-residual verticals until this queue is cleared or user reorders explicitly.
