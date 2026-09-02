# Archived operating protocol: 2026-09-01 residual census

Status: historical. This protocol governed the completed docs-only run on
`feature/rust-ownership-residual-census`; it does not govern future work.

## Historical charter and boundaries

The run used one orchestrator, one researcher per memo, and a different
reviewer for each worker PR. Worker PRs `#1081`–`#1091` targeted the feature
branch. Researchers were limited to `docs/future-projects/**`; product code,
Core and UniFFI changes, ADR amendments, goal-graph changes, and promotion to
`main` were outside the run.

The integration branch started from `main` `011cf39a`. Source cited by a memo
was required to be re-read at the memo’s recorded commit rather than inferred
from [CENSUS.md](CENSUS.md).

## Historical review rule

Each worker PR required a top-level verdict from an agent other than the
author, at the exact reviewed HEAD:

```markdown
## Verdict: ACCEPT | ACCEPT_WITH_NITS | REJECT
Exact HEAD: <full sha>
```

The review bar required a desktop/Core/iOS census, ownership taxonomy,
constraint classification, and a good-faith stay-put alternative. The original
bar was intentionally focused on duplicate ownership. The 2026-09-02 promotion
review found that this focus allowed real product and proof gaps to be called
“closed” when no ownership extraction was needed. [ACTIONS.md](ACTIONS.md)
corrects that category error.

## Historical sequencing

ROE-08 was the first deep cluster. Already-owned priors were researched in
parallel, followed by message format/safety, visibility, and remaining census
lanes. An extract/proceed recommendation was a stop requiring a later human
decision; it was not authorization to implement.

## Lessons retained

- Close **ownership** when there is one correct owner; do not thereby close a
  defect, parity gap, or missing evidence.
- Integrate accepted reviewer corrections into authoritative prose rather
  than leaving known inaccuracies in an appendix.
- Record review provenance in the memo header.
- Convert time-bound branch instructions into a dated historical record after
  the run so future agents cannot mistake them for standing policy.
- Any implementation starts from current source and current ADR/goal-graph
  state, not directly from this archived protocol.
