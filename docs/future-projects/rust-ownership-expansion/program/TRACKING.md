# Tracking board

Status vocabulary: `queued` · `researching` · `in-review` · `accepted` ·
`closed` · `human-gate` · `blocked`.

Integration tip: update the SHA after each feature-branch merge.

| Cluster | IDs | Prior | Overnight status | Memo | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Residual engine census | ROE-01 | Already owned | in-review | [#1081](https://github.com/nepenth/synara-desktop/pull/1081) | pending | Researcher: close |
| Residual engine census | ROE-02 | Already owned | in-review | [#1083](https://github.com/nepenth/synara-desktop/pull/1083) | pending | Researcher: close; no missing Core input |
| Residual engine census | ROE-03 | Extend rows only | queued | — | — | After first wave; no second row model |
| Notifications and agent policy | ROE-08 | Highest residual | in-review | [#1082](https://github.com/nepenth/synara-desktop/pull/1082) | pending | Researcher: extract eligibility; STOP |
| Notifications and agent policy | ROE-07 | Policy yes, delivery no | queued | — | — | After ROE-08 review; do not fork approval policy |
| Message format and safety | ROE-04 | Stay platform-side | queued | — | — | Fixtures before types; no AST |
| Message format and safety | ROE-12 | Shared rules/fixtures | queued | — | — | Pair with ROE-04; fix “already-sanitized” claim if still present |
| Read and list semantics | ROE-05 | Visibility contract | queued | — | — | After fixtures unless ROE-08 is gated |
| Read and list semantics | ROE-06 | Split; census Core helpers | queued | — | — | Close-first if a researcher is free later |
| Account data and drafts | ROE-09 | Already owned | researching | — | — | Census-and-close started after 01/02 PRs |
| Account data and drafts | ROE-10 | Split | queued | — | — | Reply metadata vs composer state |
| Media metadata | ROE-11 | Metadata only | queued | — | — | Subordinate to ADR 0005 |

## Merged into the feature branch

| When | SHA | What |
| --- | --- | --- |
| 2026-09-01 | `5f9c4e71` | Program operating docs |
| 2026-09-01 | `57df6dec` | First lanes marked active; no memos yet |
| 2026-09-01 | `0b6c4297` | Tick while researchers still in flight |

## Human implementation gate

Empty until a reviewer ACCEPTs an extract/proceed memo. #1082 is a
candidate only.
