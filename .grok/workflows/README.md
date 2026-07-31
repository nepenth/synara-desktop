# Matrix Rust hybrid orchestration

Hybrid setup for the full-replacement program on
`feature/matrix-rust-sdk-full-replacement`:

| Piece | Role |
| --- | --- |
| **Thin babysit loop** (scheduler, ~4–8m) | CI wait → tip-merge → undraft/merge when full-vertical criteria met |
| **`matrix-vertical-slice`** | Bounded implement one residual ID → verify → draft PR |
| **`matrix-pr-babysit`** | One-shot care for a single PR number |
| **`matrix-residual-audit`** | Docs-only import/residual truth-up |

## Policy (always)

- Full vertical only: UI → Tauri IPC → live `matrix-sdk` **plus** physical JS owner deletion for that capability
- No dogfood / plateau residual acceptance
- No `dual_backend`
- Never merge umbrella PR **#39** → `main` without explicit user approval
- Never revive closed plateau **#221**

## Run workflows

Copies also live under `~/.grok/workflows/` so they register without project-folder trust.

From Grok Build:

```text
/workflow matrix-vertical-slice {"slice_id":"V-SEND.5"}
/workflow matrix-pr-babysit {"pr":"254"}
/workflow matrix-pr-babysit {"pr":"240","hold_cutover":"true","allow_merge":"false"}
/workflow matrix-residual-audit
```

Or the `workflow` tool with `name` / `script_path` + `args`. Watch runs in `/workflows`.

**Project path note:** `.grok/workflows/` in-repo requires folder trust for `script_path` launches; user-global copies under `~/.grok/workflows/` are always trusted for discovery by name.

Smoke-checked with `validate_only` (metadata + compile + one canned-host path). That is not a live CI/git proof.

## Loop vs workflow

- **Loop:** durable babysit across long CI; no heavy implementation.
- **Workflow:** one bounded multi-agent burst with a report artifact.

Do not put “finish entire V-BURN program” in a single workflow run.
