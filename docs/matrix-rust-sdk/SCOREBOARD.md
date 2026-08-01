# Matrix Rust full-replacement — scoreboard

| Field | Value |
|-------|-------|
| Updated | 2026-08-01 |
| Tip | `8995add1` on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **163** (plan baseline was **220**) |
| Dual backend | **false** (forbidden) |
| Umbrella #39 | **Do not merge** without explicit approval |

## Done (high level)

| Area | Evidence |
|------|----------|
| Crypto vertical | V-CRYPTO done earlier |
| Core send | text/attachment/reaction/poll/thread/sticker/GIF native |
| Rooms | hierarchy + space writers |
| Auth | SSO removed; token non-retention; register; password reset; login-flow discovery; UIA multi-stage non-retention; **loginUtil fail-closed #279** |
| Poll-in-thread | #282 |
| Timeline contract | #240 native DTO/stream/actions + presenter code |
| Cutover policy | Approved; residual map #286; pack-read inventory #287 |
| Send residual inventories | forward #290; avatar #291; pack-write #292 (docs only) |
| V-SEND.R-EDIT | **#283** native `m.replace` merged |
| V-TIMELINE.C3 checklist | **#294** docs-only stream verify checklist |
| V-TIMELINE.C1 | **#285** NativeTimelinePresenter owns RoomView |
| V-TIMELINE.C2 | **#289** delete `RoomTimeline` (imports **165→164**, allowlist **169→168**) |
| CI | Parallel Validate #284 |

## In flight

| PR | What |
|----|------|
| **This PR** | V-SEND.R-FORWARD residual close (MessageForwardItem + forward.ts deleted) |

## Left (ordered)

### Timeline cutover
1. ~~**C1** land #285~~ ✅
2. ~~**C2** delete RoomTimeline~~ ✅ #289
3. **C3** live re-verify stream deltas (checklist #294; map claims no gap)
4. **C4** media/render parity on selected presenter
5. **C5** pins/notes/jump live proof

### Send / media residuals (inventories done; implement remains)
- **V-SEND.R-PACK-READ** implement (inventory #287) — preferred next on `product.rs`
- ~~**V-SEND.R-FORWARD**~~ **this PR** (dialog deleted; native presenter sole path)
- **V-SEND.R-PACK-WRITE** / **PACK-UPLOAD** implement (inventory #292)
- **V-SEND.R-AVATAR-UPLOAD** / **R-ROOM-PROFILE** implement (inventory #291)
- **V-SEND.R-CALL-UPLOAD** / **R-GIF-PACK**
- **V-SEND.R-DEVTOOL** (low priority)

### Convergence
- **V-BURN.1–3** zero live JS client + drop npm `matrix-js-sdk` after residual owners clear

### Not residual free-for-all
- Widgets/calls polish beyond residual IDs — full verticals after burn queue allows
