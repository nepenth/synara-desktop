# Session handoff — Matrix Rust full replacement

| Field | Value |
| --- | --- |
| Written | 2026-07-31 (America/New_York) |
| Audience | Next orchestrator / implementation agent |
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Tip at handoff | `88ed14308227b2eec2bed4fc33d97cfa0a2270f3` — Merge PR [#250](https://github.com/nepenth/synara-desktop/pull/250) (V-SEND.3 polls) |
| Tip may move | Yes — another agent is finishing [#253](https://github.com/nepenth/synara-desktop/pull/253) serially |
| Usage | GPT-5.6 sol/terra API credits **exhausted**; Cursor plan ~**97%** — be efficient; prefer Cursor/local agents |

Live trackers (must stay accurate when tip moves):

- [CONTINUATION.md](CONTINUATION.md)
- [PROGRESS.md](PROGRESS.md)
- [implementation-handoff.md](implementation-handoff.md)
- [d0-residual-completion.md](d0-residual-completion.md)
- [full-vertical-policy.md](full-vertical-policy.md)

## Goal

Complete Synara desktop Matrix client replacement as serial, product-visible
verticals:

```text
React UI → versioned Tauri IPC + Synara DTOs → live Rust matrix-sdk
```

**Complete replacement only.** No minima/plateau, no dual backend/runtime
selector, no concurrent JS/Rust Matrix clients for one session. Physical
deletion of the superseded JS owner happens **in each vertical**.

## Never-main / never-#39 rule

- Work only on `feature/matrix-rust-sdk-full-replacement` (and PRs into it).
- Umbrella PR [#39](https://github.com/nepenth/synara-desktop/pull/39) targets
  `main` — **never merge without explicit user approval**.
- Do **not** modify `/Users/nepenthe/git_repos/synara_project/synara-desktop`.
  Use `/private/tmp/synara-codex-*` worktrees only.

## Current tip

```text
88ed14308227b2eec2bed4fc33d97cfa0a2270f3
Merge pull request #250 from nepenth/matrix-rust/v-send-3-polls
```

Verify before acting:

```bash
git fetch origin feature/matrix-rust-sdk-full-replacement
git rev-parse origin/feature/matrix-rust-sdk-full-replacement
git log -1 --oneline origin/feature/matrix-rust-sdk-full-replacement
```

Inventory on tip at handoff: production import files **187**, repository-wide
**200** (baseline 232 / 292).

### Merged this session (docs were stale until this handoff)

| PR | Merge SHA | Meaning |
| --- | --- | --- |
| [#251](https://github.com/nepenth/synara-desktop/pull/251) V-ROOMS.5w | `0fb0fe4` | native m.direct writers |
| [#243](https://github.com/nepenth/synara-desktop/pull/243) docs | `31b4a30` | tracking at older tip |
| [#252](https://github.com/nepenth/synara-desktop/pull/252) V-ROOMS.5r | `9579ea4` | native m.direct user list |
| [#250](https://github.com/nepenth/synara-desktop/pull/250) V-SEND.3 | `88ed143` | native poll start/response |

## Worktrees under `/private/tmp` that matter

| Path | Branch / role | Notes |
| --- | --- | --- |
| `/private/tmp/synara-codex-docs-current` | docs tip-sync / this handoff | Safe for docs PRs |
| `/private/tmp/synara-codex-v-send-4-rich-messages` | `matrix-rust/v-send-4-rich-messages` → [#253](https://github.com/nepenth/synara-desktop/pull/253) | **Do not fight** — another agent finishing serially |
| `/private/tmp/synara-codex-v-rooms-2b-hierarchy` | `matrix-rust/v-rooms-2b-hierarchy` → [#254](https://github.com/nepenth/synara-desktop/pull/254) | Dirty vs tip; may show conflict markers — rebase after #253 |
| `/private/tmp/synara-codex-v-timeline-contract` | `matrix-rust/v-timeline-contract` → [#240](https://github.com/nepenth/synara-desktop/pull/240) | HOLD; contract incomplete |
| `/private/tmp/synara-codex-v-rooms-5r-mdirect-users` | merged via #252 | historical |
| `/private/tmp/synara-codex-v-send-3-polls` | merged via #250 | historical |

Stale/other `synara-codex-*` worktrees may exist; prefer fresh checkouts from
`origin/feature/matrix-rust-sdk-full-replacement` when in doubt.

## Open PRs and exact next actions

Serial order: **#253 → #254 → continue #240 → merge #240 only when closable**.

| PR | URL | Disposition | Exact next action |
| --- | --- | --- | --- |
| [#253](https://github.com/nepenth/synara-desktop/pull/253) V-SEND.4 | https://github.com/nepenth/synara-desktop/pull/253 | **Active / serial now** (draft; head `486b3d7` contains tip; rich-message proof green; Validate + attachment proof in progress at handoff) | If still open: wait for required CI green on exact head → undraft → `gh pr merge --merge`. Do not steal the worktree. |
| [#254](https://github.com/nepenth/synara-desktop/pull/254) V-ROOMS.2b | https://github.com/nepenth/synara-desktop/pull/254 | **Next** (draft; **dirty**; head `71c2877`; behind tip) | After #253 merges: rebase onto new tip, resolve conflicts, required CI, undraft, `gh pr merge --merge`. |
| [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE | https://github.com/nepenth/synara-desktop/pull/240 | **HOLD** (draft; head `bf9ac48`; required CI green / mergeable clean at handoff) | Close remaining full-replacement contract gaps; **do not** select NativeTimelinePresenter or delete `RoomTimeline.tsx` until contract + runtime proof complete; then merge. |
| [#221](https://github.com/nepenth/synara-desktop/pull/221) D0.6 | https://github.com/nepenth/synara-desktop/pull/221 | **HOLD** | Plateau / zero importer deletion — do not merge as complete. |
| L1 parked | #109, #193, #196, #198, #199, #201, #203, #204, #207, #208, #209 | **HOLD (parked)** | Do not merge until residual queue allows. |
| [#39](https://github.com/nepenth/synara-desktop/pull/39) → main | https://github.com/nepenth/synara-desktop/pull/39 | **HOLD** | Never merge without explicit user approval. |

## Hard constraints

1. **Complete replacement** — wired ≠ done; each slice deletes its JS owner.
2. **Presenter / RoomTimeline** — never select NativeTimelinePresenter or delete
   `RoomTimeline.tsx` until the V-TIMELINE contract is complete (full
   render/action/media route + runtime proof).
3. **V-SEND / V-ROOMS sequencing** — finish #253 before rebasing/merging #254;
   then continue #240. Do not start new media/widgets/notifications/calls
   verticals before the residual queue allows.
4. **No dual backend** / no runtime selector / no live JS Matrix client fallback
   after native ownership is selected.
5. **Secrets** — passphrases/recovery only as one-way command inputs; never in
   IPC responses, events, diagnostics, logs, or generated docs.
6. **Worktrees** — `/private/tmp/synara-codex-*` only; never the home git_repos
   checkout.

## CI / merge style

- Base every product/docs PR on `feature/matrix-rust-sdk-full-replacement`.
- Require exact-head green for required jobs (Validate, Quality, scoped Synapse
  proofs when touched, etc.).
- Prefer draft until green; then undraft.
- Merge with **`gh pr merge --merge`** (merge commit) into integration only.
- Docs-only PRs historically land when CI is docs-light/green; still wait for
  required checks.
- Do not force-push shared branches; do not merge #39/main without approval.

## What NOT to do

- Do not merge main / #39 without explicit approval.
- Do not fight the #253 worktree while another agent owns CI→merge.
- Do not select NativeTimelinePresenter or delete `RoomTimeline` early.
- Do not merge #221 as D0.6 complete.
- Do not merge parked L1 foundation PRs while residual queue is active.
- Do not accept “minimum / plateau / wired” as done.
- Do not burn GPT-5.6 API credits; they are exhausted — keep Cursor usage lean.
- Do not edit `/Users/nepenthe/git_repos/synara_project/synara-desktop`.

## Resume prompt (copy-paste)

```text
Resume Matrix Rust full-replacement orchestration.

Goal: complete Synara desktop Matrix replacement as serial full verticals
(React → versioned Tauri IPC/DTOs → live Rust matrix-sdk), with physical JS
owner deletion per vertical. No dual backend. No minima/plateau.

Integration branch: feature/matrix-rust-sdk-full-replacement
NEVER merge main / PR #39 without explicit user approval.
Work ONLY under /private/tmp/synara-codex-* worktrees.
Do NOT modify /Users/nepenthe/git_repos/synara_project/synara-desktop.

Handoff file (read first):
docs/matrix-rust-sdk/SESSION-HANDOFF.md
Also sync: CONTINUATION.md, PROGRESS.md, implementation-handoff.md,
d0-residual-completion.md.

Tip at handoff (VERIFY — may have moved if #253 merged):
88ed14308227b2eec2bed4fc33d97cfa0a2270f3
  = Merge pull request #250 (V-SEND.3 polls)

Merged recently: #251 (5w), #243 (docs), #252 (5r), #250 (SEND.3).
Inventory on that tip: production 187 / repo-wide 200.

Usage: GPT-5.6 sol/terra API credits exhausted; Cursor plan ~97%.
Be efficient; prefer Cursor/local agents; docs/CI-light when possible.

Serial order (binding):
1) Finish #253 V-SEND.4 if still open
   https://github.com/nepenth/synara-desktop/pull/253
   Another agent may own CI→merge — do NOT fight
   /private/tmp/synara-codex-v-send-4-rich-messages.
   When required CI green on exact head: undraft, gh pr merge --merge.
2) Then #254 V-ROOMS.2b
   https://github.com/nepenth/synara-desktop/pull/254
   Dirty vs tip at handoff — rebase onto post-#253 tip, fix conflicts
   (worktree /private/tmp/synara-codex-v-rooms-2b-hierarchy may be messy),
   required CI, undraft, gh pr merge --merge.
3) Then continue #240 V-TIMELINE contract gaps until closable
   https://github.com/nepenth/synara-desktop/pull/240
   Head was bf9ac48 with required CI green — NOT permission to cut over.
   NEVER select NativeTimelinePresenter or delete RoomTimeline.tsx until
   full action/media route + runtime proof complete; then merge #240.

Hard HOLDs: #221 plateau; L1 parked (#109/#193/#196/#198/#199/#201/#203/#204/#207/#208/#209); #39.

CI/merge: base=integration; exact-head green; prefer draft until green;
merge with gh pr merge --merge into integration only.

After any merge: fetch tip, update CONTINUATION/PROGRESS/residual/handoff,
and continue the serial queue.
```
