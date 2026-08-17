# 14 — Long-running Cloud Agent (continue the language-boundary loop)

A normal Cloud Agent **stops when the current prompt ends**. A
[Long-running agent](https://cursor.com/blog/long-running-agents) uses a
different harness: it **proposes a plan, waits for approval**, then can
work for hours or days on that scoped task.

This file is the standing launch recipe. It does not enable the team
setting (a human admin must do that). It does not start P5.

Root `AGENTS.md` is gitignored in this repo (local dumps only). Put
Cloud standing instructions here and in the playbook, not in a
forced-add `AGENTS.md`.

## What you (human) must do

Long-running is a **team feature**. This repo cannot flip it.

1. Open [Cloud Agents settings](https://cursor.com/dashboard/cloud-agents).
2. Under **Team feature settings**, enable **Long running agents**.
   Ultra / Teams / Enterprise only. Save is instant.
3. Use a **single-repo** environment (`nepenth/synara-desktop` only).
   Long-running is **not available for multi-repo** environments; picking
   a multi-repo environment disables the toggle.
4. Open [cursor.com/agents](https://cursor.com/agents) → **New agent**.
5. Repository: `nepenth/synara-desktop`.
6. In the **model picker**, choose **Long-running**.
7. Paste the prompt in the next section.
8. When the agent returns a plan, **read it and approve** (or edit it).
   It will not execute until you approve.

You still merge PRs and unblock owner decisions. The agent does not
merge. P5 stays operator-gated.

## Standing prompt (paste this)

```text
Continue the synara-desktop language-boundary / shared-native-core program.

Read, in order:
1. AGENTS.md
2. docs/shared-native-core/11-implementer-playbook.md (§5 then §9)
3. docs/shared-native-core/13-language-boundary-goal-graph.md if present
4. docs/adr/0004-rust-language-boundaries.md if present (else PR #1000)
5. Open PRs: #1000 (ADR 0004) and #1001 (P4-S12–S15 on cursor/p4-s12-start-sync-7875)

Start from the latest implementation tip, not a stale main graph:
- Prefer branch cursor/p4-s12-start-sync-7875 / PR #1001 if it is still open.
- If that PR has merged, start from origin/main and refresh the graph.

Loop until every required graph node is landed or blocked:
- One P4 family at a time. Local cargo tests. Commit. Push. Draft PR.
- GitHub Actions minutes are exhausted. Delay Actions. Do not treat skipped CI as failure.
- Do not start P5. Do not claim iOS-on-engine or P4 acceptance.
- Do not register the 21 leftovers on Core::command.
- Do not put product events on Platform::emit.
- Do not rewrite UI in Slint/Dioxus/egui. Do not do Tauri iOS.

Current honesty (update if the graph has moved):
- S12 start_sync, S13 restore bootstrap, S14 timeline view-delta poll, S15 leftover owner status are on #1001.
- Desktop native media cutover is blocked: iOS leftover media stays fail-closed (playbook decision 15); bytes must not cross Core::command; do not register matrix_send_attachment.
- While media is blocked, do not invent a byte channel. Instead work P4 engine-ready gaps that do not need bytes: product iOS timeline rows from existing SharedCore snapshot/paginate (SharedCoreTimelineService must not keep returning .empty); remaining no-op emit families using the same poll-queue pattern as S14 (presence, devices, join_rules, image_packs) if an owner already emits; keep leftovers fail-closed.
- Stop if the only remaining work is P5, Apple UniFFI generate, a live homeserver, a merge, or a media byte-channel owner decision.

Propose a plan first. Wait for approval. Then execute.
```

## What this agent will not do

- Enable the team setting for you.
- Merge #1000 / #1001.
- Start P5 or TestFlight.
- Retire `browser-encrypt-attachment` / shrink `synara/src/sw.ts` until
  both shells have a native byte channel that is not `Core::command`.
