# ROE-XX Research Memo: Title

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                           |
| ------------------ | ----------------------------------------------- |
| Workstream/cluster | `<IDs>`                                         |
| Research owner     | Unassigned                                      |
| Reviewers          | Unassigned                                      |
| Source census      | `<date and commit>`                             |
| ADR baseline       | `<IDs, last-reviewed dates, and source commit>` |

## Observable problem

Describe user-visible divergence, correctness risk, or duplicated authority.
Do not begin with a language or proposed API.

## Current ownership census

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
|         |           |         |     |                |

Classify each concern as Core authority, platform observation, or platform
rendering. Classify each relevant constraint as a hard invariant, accepted
platform boundary, or current technology preference. Identify the earliest
actual divergence.

## Boundary constraints

Record applicable ADR and goal-graph constraints, secrets/paths/bytes,
lifecycle and latency limits, and behaviors that must stay platform-side.

## Alternatives

Compare:

1. no ownership change;
2. a bounded extraction or shared fixture/contract;
3. a broader Core model.

State what evidence would falsify each option.

## Recommendation

Choose one: already correctly owned; stay platform-side; extract a bounded
subset; proceed with Core ownership; or requires a product/ADR decision.

State confidence, supporting evidence, the strongest objection, unresolved
questions, and the regression proof needed to keep the ownership decision
stable. Separately classify every unresolved finding as a product defect,
security/evidence gap, transport/parity gap, future architecture option, or
product decision. “Already owned” must not erase those findings.

## Next gate

- For **already owned** or **stay platform-side**: close only the ownership
  question. Preserve any defect, proof, parity, or product-decision action in
  the relevant durable action register.
- For **bounded extraction** or **proceed**: stop until a human accepts the
  recommendation and any required ADR amendment or replacement is accepted.
  Only then may an implementation plan be written.
