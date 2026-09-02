# Historical program decisions

Status: historical decisions from the 2026-09-01 research run, with promotion-
review corrections added on 2026-09-02. These do not amend ADRs. An ADR still
wins if a memo disagrees.

| ID | Date | Decision |
| --- | --- | --- |
| D1 | 2026-09-01 | A human chartered overnight **docs-only** execution of this portfolio on `feature/rust-ownership-residual-census`. This is the research charter required by the [agent guide](../AGENT-GUIDE.md). |
| D2 | 2026-09-01 | All worker PRs target the feature branch. The feature branch must not merge to `main` overnight. Promotion to `main` is a separate human action. |
| D3 | 2026-09-01 | Product implementation, Core/UniFFI surface changes, and shared-Core playbook or goal-graph edits are forbidden until a later decision explicitly opens an implementation gate after an accepted memo (and any required ADR amendment). |
| D4 | 2026-09-01 | Deep-cluster order tonight: ROE-08, then ROE-07. Message-format fixtures (ROE-04/12) are next only if the agent-policy cluster is idle on a human gate or closed. |
| D5 | 2026-09-01 | Parallel census-and-close is allowed for ROE-01, ROE-02, and ROE-09 because their priors are already owned. They must not expand into implementation plans. |
| D6 | 2026-09-01 | Multi-agent split: orchestrator assigns and merges; a researcher authors exactly one memo PR; a different agent reviews at an exact HEAD. The author cannot ACCEPT their own memo. |
| D7 | 2026-09-01 | This program is not S38, not P4-engine-ready work, and not P5. It must not claim those gates or invent a new shared-Core phase. |
| D8 | 2026-09-01 | [CENSUS.md](CENSUS.md) is a starting snapshot from `main` `011cf39a`. Every memo must re-verify cited paths on the commit it records. |
| D9 | 2026-09-01 | A memo recommendation of extract or proceed is a **stop**, not a start. Record it and wait for a human implementation decision. |
| D10 | 2026-09-01 | Accepted memo [ROE-08-agent-approval-memo.md](../memos/ROE-08-agent-approval-memo.md) (`#1082`, ACCEPT at `cd1c655b`) recommends extracting Core `is_agent_approval_prompt` as the sole eligibility owner. **Human implementation gate is now open as a question and closed as a coding path.** Do not delete TypeScript/Swift detectors, add Core routes, or write a plan until a later decision explicitly authorizes implementation. |
| D11 | 2026-09-02 | Promotion review found that D10 is too narrow: in-app generic reaction paths can bypass Core approval validation, Hermes implements typed session approval and configurable expiry that the original memo did not fully census, and recognition alone is insufficient. D10 remains historical; [A2](ACTIONS.md#a2--hermes-approval-contract-and-authority) supersedes it as the current research gate. |
| D12 | 2026-09-02 | “Ownership closed” is not equivalent to “feature complete.” Confirmed product defects, transport/parity gaps, product decisions, and missing proof remain open in [ACTIONS.md](ACTIONS.md), even where a memo correctly finds one owner. |
| D13 | 2026-09-02 | After reviewing the promotion findings, the human explicitly authorized the bounded A1–A8 and A11 remediation subset on this feature branch, with delegated implementation and independent review, and authorized the primary orchestrator to commit and merge accepted work. This supersedes D2 and D3 only for that subset. A9 and A10 remain evidence/future investigations, no unrelated release work is authorized, and missing live evidence cannot be marked complete. |

Future corrections add rows rather than rewriting historical decisions.
