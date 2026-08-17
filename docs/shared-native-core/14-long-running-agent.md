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
1. docs/shared-native-core/14-long-running-agent.md (this file)
2. docs/shared-native-core/11-implementer-playbook.md (§5 then §9)
3. docs/shared-native-core/13-language-boundary-goal-graph.md if present
4. docs/adr/0004-rust-language-boundaries.md if present (else PR #1000)
5. Open PRs only for work that is still open. #1000, #1001, #1002,
   #1003, #1004, #1005, and #1006 have merged.

Start from the latest implementation tip, not a stale main graph:
- Start from `origin/main` (`76f67441` after #1006). Refresh the graph.

Loop until every required graph node is landed or blocked:
- One P4 family at a time. Local cargo tests. Commit. Push. Draft PR.
- GitHub Actions minutes are exhausted. Delay Actions. Do not treat skipped CI as failure.
- Do not start P5. Do not claim iOS-on-engine or P4 acceptance.
- Do not register the 21 leftovers on Core::command.
- Do not put product events on Platform::emit.
- Do not rewrite UI in Slint/Dioxus/egui. Do not do Tauri iOS.

Current honesty (update if the graph has moved):
- S12–S37 landed on `main` via #1001. ADR 0004 landed via #1000.
  Desktop JS media retire landed via #1006 (`76f67441`).
- Hosted iOS CI stays paused (#1003). Live homeserver proof stays paused
  until an environment can launch the app.
- Desktop live timeline media uses `synara-media://` + handle resolve.
  Composer send is native-only. JS `browser-encrypt-attachment` is
  retired. `sw.ts` is a stub. Leftover encrypted `mxc://` fail-closes.
  Leftover avatar `<img src=mxc://>` display remains. iOS leftover
  media I/O stays fail-closed (decision 15). Do not register
  `matrix_send_attachment`.
- Do not invent a byte channel. Do not invent S38. Do not start P5.
  Do not claim iOS-on-engine or P4 engine ready.
- Remaining non-P5 work that needs a Mac or a live app: Apple generate
  for new UniFFI fields, dual-platform Core bugfix proof, live
  homeserver proof after the app can launch. Optional later: leftover
  avatar display via native blob URLs. Re-enable hosted iOS CI only
  when an operator asks.

Propose a plan first. Wait for approval. Then execute.
```

## What this agent will not do

- Enable the team setting for you.
- Merge already-landed #1000 / #1001 / #1002 / #1003 / #1004 / #1005 / #1006 again.
- Start P5 or TestFlight.
- Hand-edit generated Swift or invent a no-bindgen Apple generate path.
- Invent a dual-platform Core bugfix proof that did not run on iOS.
