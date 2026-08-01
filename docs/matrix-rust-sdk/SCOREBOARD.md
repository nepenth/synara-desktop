# Matrix Rust full-replacement — scoreboard

| Field | Value |
|-------|-------|
| Updated | 2026-08-01 |
| Tip | `b21578e9` on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **163** (plan baseline was **220**) |
| Dual backend | **false** (forbidden) |
| Umbrella #39 | **Do not merge** without explicit user approval |

## Done (high level)

| Area | Evidence |
|------|----------|
| Crypto / Auth / Rooms / core send | Prior verticals closed |
| V-SEND.R-EDIT | **#283** |
| V-TIMELINE.C1/C2 | **#285** / **#289** |
| V-SEND.R-FORWARD | **#296** |
| V-SEND.R-PACK-READ snapshot | **#297** (subscribe residual) |
| V-SEND.R-AVATAR user | **#303** |
| V-SEND.R-PACK-WRITE personal | **#306** |
| CI / docs | #284; C3–C5 checklists #294/#302 |

## In flight

| PR | What |
|----|------|
| (none) | Next: room-profile; pack global/room write; pack-read subscribe |

## Left (ordered)

1. **R-ROOM-PROFILE** room name/topic/avatar
2. Pack-write **global/room** + PACK-UPLOAD
3. Pack-read **subscribe** + JS utils delete
4. **C3–C5** live proofs
5. CALL-UPLOAD / GIF-PACK / DEVTOOL (low)
6. **V-BURN.1–3** → drop npm matrix-js-sdk
7. **#39** only with explicit approval
