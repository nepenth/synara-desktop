# ROE-XX Implementation Plan: Title

Status: gated implementation proposal; not approved for implementation.

Do not use this template for initial research. It is available only after an
accepted [research memo](RESEARCH-MEMO-TEMPLATE.md) recommends proceeding and
any required ADR amendment or replacement has been accepted.

| Field               | Value                                                 |
| ------------------- | ----------------------------------------------------- |
| Accepted memo       | `<path and commit>`                                   |
| Owner               | Unassigned                                            |
| Reviewers           | Unassigned                                            |
| Last source census  | `<date and commit>`                                   |
| Related workstreams | `<IDs or none>`                                       |
| ADR baseline        | `<IDs, last-reviewed dates, and source commit>`       |
| ADR impact          | `<none, conforms, or accepted amendment/replacement>` |

## 1. Accepted recommendation and evidence

Restate the accepted bounded change, supporting evidence, expected benefit,
principal cost, and explicit stop conditions. Do not broaden the memo.

## 2. User and product requirements

Number feature requirements `FR-01`, security/privacy requirements `SR-01`,
performance/reliability requirements `PR-01`, and compatibility requirements
`CR-01`. Cover normal, empty, offline, concurrent, malformed, upgraded,
cancelled, resource-limited, accessibility, and platform-lifecycle cases as
applicable.

## 3. Confirmed current-state census

| Concern  | Rust/Core authority | Desktop observation/rendering | iOS observation/rendering | Tests/evidence |
| -------- | ------------------- | ----------------------------- | ------------------------- | -------------- |
| Behavior |                     |                               |                           |                |

Identify DTOs/events, storage, background tasks, platform APIs, current failure
behavior, and behavior that is already shared.

## 4. Scope and closed boundaries

### In scope

- ...

### Out of scope

- UI layout, widgets, gestures, selection, colors, typography, animation,
  viewport math, locale presentation, and OS delivery unless an accepted ADR
  explicitly says otherwise.
- ...

List prohibited secrets, paths, and bytes. State how ADRs 0001–0005 apply.
Identify each constraint as a hard invariant, accepted platform boundary, or
current technology preference.

## 5. Ownership and invariants

Document the single authority, concurrency owner, persistence owner, platform
observations, adapters, failure behavior, resource bounds, and invariants.

## 6. Typed design

### Domain model

### Commands, queries, DTOs, and events

### Errors and cancellation

### Persistence, versioning, and migration

### Observability and redaction

Explain why IPC/UniFFI cost and schema churn are justified by the accepted
problem.

## 7. Client cutover

### macOS/Linux React/Tauri

### iOS SwiftUI/UniFFI

Name the old owners removed in each client. There must be no ambiguous
dual-owner interval.

## 8. Delivery slices

Each slice must be independently testable and reversible.

| Slice | Behavior | Boundaries changed | Proof | Old owner removed | Rollback |
| ----- | -------- | ------------------ | ----- | ----------------- | -------- |
| 1     |          |                    |       |                   |          |

## 9. Test and live-proof plan

Define acceptance criteria, not only commands. Cover applicable Rust unit and
property tests, serialization contracts, ordering/cancellation/failure
injection, local Synapse single- and multi-client proof, crypto fixtures,
desktop integration/performance, iOS unit/simulator/physical-device/NSE proof,
malformed and adversarial events, resource limits, accessibility,
localization, offline/upgrade/rollback behavior, and release diagnostics.

## 10. Rollout and rollback

Define data migration order, mixed-version behavior, abort thresholds,
diagnostics, exact rollback mechanics, and removal criteria. Feature flags are
allowed only when they do not create permanent dual ownership.

## 11. Risks and decisions

| Risk/question | Impact | Evidence needed | Owner | Blocks implementation? |
| ------------- | ------ | --------------- | ----- | ---------------------- |
|               |        |                 |       |                        |

## 12. Adversarial review

Record objections, severity, disposition, and plan changes. Include the
strongest argument for retaining the existing owner.

## 13. Final acceptance gate

- [ ] Accepted memo and source census remain current.
- [ ] Requirements and exclusions are approved.
- [ ] Boundary complies with accepted ADRs.
- [ ] ADR baseline is still current; later amendments have been evaluated.
- [ ] Both client cutovers remove superseded owners.
- [ ] Security and performance budgets are measurable.
- [ ] Automated and live proof demonstrate one authority end to end.
- [ ] Rollback is feasible and tested.
- [ ] Implementation owner and reviewers are assigned.
