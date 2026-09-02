# Tracking board

Status vocabulary: `queued` · `researching` · `in-review` · `accepted` ·
`closed` · `human-gate` · `blocked`.

Integration tip: update the SHA after each feature-branch merge.

| Cluster | IDs | Prior | Overnight status | Memo | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Residual engine census | ROE-01 | Already owned | researching | — | — | Census-and-close; first wave |
| Residual engine census | ROE-02 | Already owned | researching | — | — | Bounded iOS continuity question only |
| Residual engine census | ROE-03 | Extend rows only | queued | — | — | After first wave; no second row model |
| Notifications and agent policy | ROE-08 | Highest residual | researching | — | — | Deep cluster #1 |
| Notifications and agent policy | ROE-07 | Policy yes, delivery no | queued | — | — | After ROE-08 memo; do not fork approval policy |
| Message format and safety | ROE-04 | Stay platform-side | queued | — | — | Fixtures before types; no AST |
| Message format and safety | ROE-12 | Shared rules/fixtures | queued | — | — | Pair with ROE-04; fix “already-sanitized” claim if still present |
| Read and list semantics | ROE-05 | Visibility contract | queued | — | — | After fixtures unless ROE-08 is gated |
| Read and list semantics | ROE-06 | Split; census Core helpers | queued | — | — | Close-first if a researcher is free later |
| Account data and drafts | ROE-09 | Already owned | queued | — | — | Next census-and-close after 01/02 |
| Account data and drafts | ROE-10 | Split | queued | — | — | Reply metadata vs composer state |
| Media metadata | ROE-11 | Metadata only | queued | — | — | Subordinate to ADR 0005 |

## Merged into the feature branch

| When | SHA | What |
| --- | --- | --- |
| 2026-09-01 | *(this commit)* | Program operating docs; no memos yet |

## Human implementation gate

Empty. A proceed/extract memo does not fill this table.
