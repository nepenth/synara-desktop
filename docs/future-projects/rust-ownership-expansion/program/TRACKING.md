# Tracking board

Status vocabulary: `queued` · `researching` · `in-review` · `accepted` ·
`closed` · `human-gate` · `blocked`.

Integration tip: update the SHA after each feature-branch merge.

| Cluster | IDs | Prior | Overnight status | Memo | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Residual engine census | ROE-01 | Already owned | closed | [memo](../memos/ROE-01-orchestration-memo.md) | ACCEPT_WITH_NITS `#1081` | Nits recorded in memo |
| Residual engine census | ROE-02 | Already owned | closed | [memo](../memos/ROE-02-verification-memo.md) | ACCEPT `#1083` | No missing Core input |
| Residual engine census | ROE-03 | Extend rows only | researching | — | — | No second row model |
| Notifications and agent policy | ROE-08 | Highest residual | human-gate | [memo](../memos/ROE-08-agent-approval-memo.md) | ACCEPT `#1082` | Extract eligibility; no implementation (D10) |
| Notifications and agent policy | ROE-07 | Policy yes, delivery no | researching | — | — | Policy only; do not reopen ROE-08 |
| Message format and safety | ROE-04 | Stay platform-side | queued | — | — | Fixtures before types; no AST |
| Message format and safety | ROE-12 | Shared rules/fixtures | queued | — | — | Pair with ROE-04; fix “already-sanitized” claim if still present |
| Read and list semantics | ROE-05 | Visibility contract | queued | — | — | After fixtures unless ROE-08 is gated |
| Read and list semantics | ROE-06 | Split; census Core helpers | researching | — | — | Census whether Core helpers are consumed |
| Account data and drafts | ROE-09 | Already owned | closed | [memo](../memos/ROE-09-notes-memo.md) | ACCEPT `#1084` | No second notes engine |
| Account data and drafts | ROE-10 | Split | queued | — | — | Reply metadata vs composer state |
| Media metadata | ROE-11 | Metadata only | queued | — | — | Subordinate to ADR 0005 |

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

## Human implementation gate

| Item | Memo | Recommendation | Status |
| --- | --- | --- | --- |
| ROE-08 eligibility owner | [ROE-08-agent-approval-memo.md](../memos/ROE-08-agent-approval-memo.md) | Make Core `is_agent_approval_prompt` the sole prompt-eligibility owner; keep cards and OS delivery platform-side | Waiting on an explicit human implementation decision (D10). Not authorized tonight. |
