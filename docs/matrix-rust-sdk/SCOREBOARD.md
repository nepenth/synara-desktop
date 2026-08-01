# Matrix Rust full-replacement — scoreboard

| Field | Value |
|-------|-------|
| Updated | 2026-08-01 |
| Tip | `738abb30` on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **163** (plan baseline was **220**) |
| Dual backend | **false** (forbidden) |
| Umbrella #39 | **Do not merge** without explicit user approval |

## Done (high level)

| Area | Evidence |
|------|----------|
| Crypto vertical | V-CRYPTO done earlier |
| Core send | text/attachment/reaction/poll/thread/sticker/GIF native |
| Rooms | hierarchy + space writers |
| Auth | SSO removed; token non-retention; register; password reset; login-flows; UIA non-retention; loginUtil **#279** |
| Poll-in-thread | #282 |
| Timeline contract | #240 |
| V-SEND.R-EDIT | **#283** native `m.replace` |
| V-TIMELINE.C1 | **#285** NativeTimelinePresenter owns RoomView |
| V-TIMELINE.C2 | **#289** delete RoomTimeline |
| V-SEND.R-FORWARD | **#296** legacy dialog + forward.ts deleted |
| V-SEND.R-PACK-READ snapshot | **#297** native get + hooks (subscribe residual) |
| V-SEND.R-AVATAR-UPLOAD user | **#303** upload_media + set_own_avatar/display_name; Profile.tsx fail-closed |
| CI | Parallel Validate #284 |
| Docs/checklists | #294 C3; #302 C4/C5; residual truth #298/#301; scoreboards #293/#295/#300 |

## In flight

| PR | What |
|----|------|
| (none product) | Next: pack-write implement; pack-read subscribe; room-profile residual |

## Left (ordered)

### Timeline
1. ~~C1~~ ✅ #285 · ~~C2~~ ✅ #289
2. **C3** live stream re-verify (checklist #294)
3. **C4** media/render parity (checklist #302)
4. **C5** pins/notes/jump live proof (checklist #302)

### Send / media residuals
- Pack-read **subscribe** + physical JS utils delete (snapshot #297 landed)
- **V-SEND.R-PACK-WRITE** / PACK-UPLOAD implement (inventory #292)
- **R-ROOM-PROFILE** room name/topic/avatar (user avatar #303 done)
- CALL-UPLOAD / GIF-PACK / DEVTOOL (low)

### Convergence
- **V-BURN.1–3** zero live JS client + drop npm `matrix-js-sdk`
- **#39** umbrella — explicit approval only

### Not residual free-for-all
- Widgets/calls polish beyond residual IDs after burn queue allows
