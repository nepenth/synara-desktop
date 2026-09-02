# Tracking board

Status vocabulary: `queued` · `researching` · `in-review` · `accepted` ·
`closed` · `human-gate` · `blocked`.

Integration tip: update the SHA after each feature-branch merge.

| Cluster | IDs | Prior | Overnight status | Memo | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Residual engine census | ROE-01 | Already owned | closed | [memo](../memos/ROE-01-orchestration-memo.md) | ACCEPT_WITH_NITS `#1081` | Nits recorded in memo |
| Residual engine census | ROE-02 | Already owned | closed | [memo](../memos/ROE-02-verification-memo.md) | ACCEPT `#1083` | No missing Core input |
| Residual engine census | ROE-03 | Extend rows only | closed | [memo](../memos/ROE-03-timeline-rows-memo.md) | ACCEPT `#1086` | No missing field; shared `thread_root` omission is not an extract |
| Notifications and agent policy | ROE-08 | Highest residual | human-gate | [memo](../memos/ROE-08-agent-approval-memo.md) | ACCEPT `#1082` | Extract eligibility; no implementation (D10) |
| Notifications and agent policy | ROE-07 | Policy yes, delivery no | closed | [memo](../memos/ROE-07-notification-policy-memo.md) | ACCEPT_WITH_NITS `#1085` | Settings already Core; delivery stay platform; nits recorded |
| Message format and safety | ROE-04 | Stay platform-side | closed | [memo](../memos/ROE-04-message-format-memo.md) | ACCEPT_WITH_NITS `#1089` | Stay platform; misleading `formatted_body` comment; nits recorded |
| Message format and safety | ROE-12 | Shared rules/fixtures | closed | [memo](../memos/ROE-04-message-format-memo.md) | ACCEPT_WITH_NITS `#1089` | Cluster with ROE-04; fixtures described, not landed |
| Read and list semantics | ROE-05 | Visibility contract | researching | — | — | After fixtures; viewport stays platform |
| Read and list semantics | ROE-06 | Split; census Core helpers | closed | [memo](../memos/ROE-06-room-sort-memo.md) | ACCEPT_WITH_NITS `#1087` | Stay platform; unused helpers; nits recorded |
| Account data and drafts | ROE-09 | Already owned | closed | [memo](../memos/ROE-09-notes-memo.md) | ACCEPT `#1084` | No second notes engine |
| Account data and drafts | ROE-10 | Split | closed | [memo](../memos/ROE-10-drafts-memo.md) | ACCEPT `#1088` | Already owned split; leftover UniFFI / Jotai are seams |
| Media metadata | ROE-11 | Metadata only | researching | — | — | Subordinate to ADR 0005; no bytes on envelope |

## Merged into the feature branch

| When | SHA | What |
| --- | --- | --- |
| 2026-09-01 | `5f9c4e71` | Program operating docs |
| 2026-09-01 | `57df6dec` | First lanes marked active; no memos yet |
| 2026-09-01 | `0b6c4297` | Tick while researchers still in flight |
| 2026-09-01 | `9b97ae4e` | Memo PRs tracked; ROE-09 started |
| 2026-09-01 | `9e71af13` | ROE-08 memo merged (`#1082`) |
| 2026-09-01 | `eb994ec4` | ROE-08 human-gate (D10); ROE-07 assigned |
| 2026-09-01 | `339b0f1b` | ROE-02 memo merged (`#1083`) |
| 2026-09-01 | `161d684b` | ROE-02 marked closed |
| 2026-09-01 | `53d7b2c4` | ROE-01 memo merged (`#1081`) |
| 2026-09-01 | `71e6067f` | ROE-01 nits recorded; ROE-03 started |
| 2026-09-01 | `a046e871` | ROE-09 memo merged (`#1084`) |
| 2026-09-01 | `ed69be18` | ROE-09 closed; ROE-06 started |
| 2026-09-01 | `2dc701f6` | ROE-07 memo put into review |
| 2026-09-01 | `b6797c3a` | ROE-03 memo put into review |
| 2026-09-01 | `5b8f966c` | ROE-07 memo merged (`#1085`) |
| 2026-09-01 | `e1157d49` | ROE-03 memo merged (`#1086`) |
| 2026-09-01 | `b361f395` | ROE-06 memo merged (`#1087`) |
| 2026-09-01 | `0bb6f421` | ROE-10 memo merged (`#1088`) |
| 2026-09-01 | `a1ede8f5` | ROE-04/12 memo merged (`#1089`) |

## Human implementation gate

| Item | Memo | Recommendation | Status |
| --- | --- | --- | --- |
| ROE-08 eligibility owner | [ROE-08-agent-approval-memo.md](../memos/ROE-08-agent-approval-memo.md) | Make Core `is_agent_approval_prompt` the sole prompt-eligibility owner; keep cards and OS delivery platform-side | Waiting on an explicit human implementation decision (D10). Not authorized tonight. |
