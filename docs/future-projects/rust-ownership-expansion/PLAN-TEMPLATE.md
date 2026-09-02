# ROE-XX Plan: Title

Status: draft research plan; not approved for implementation.

| Field               | Value                                                       |
| ------------------- | ----------------------------------------------------------- |
| Owner               | Unassigned                                                  |
| Reviewers           | Unassigned                                                  |
| Last source census  | `<date and commit>`                                         |
| Related workstreams | `<IDs or none>`                                             |
| ADR impact          | `<none, conforms to ADR 0004, or replacement ADR required>` |

## 1. Executive recommendation

Choose one: already correctly owned; stay platform-side; extract a bounded
subset; proceed with Core ownership; blocked on an ADR/product decision.

State confidence, expected benefit, principal cost, and the strongest reason
the recommendation may be wrong.

## 2. User and product problem

Describe observable behavior, affected clients, evidence of divergence or
risk, and why changing ownership is expected to help.

## 3. Current-state census

| Concern  | Rust/Core owner | Desktop owner | iOS owner | Tests/evidence |
| -------- | --------------- | ------------- | --------- | -------------- |
| Behavior |                 |               |           |                |

Include source paths, DTOs/events, storage, background tasks, platform APIs,
and current failure behavior. Identify behavior that is already shared.

## 4. Scope

### In scope

- ...

### Out of scope

- UI layout, widgets, gestures, colors, typography, and animation unless an
  accepted ADR explicitly says otherwise.
- ...

## 5. Requirements

### Feature and functional requirements

Number requirements `FR-01`, `FR-02`, and so on. Include normal, empty,
offline, concurrent, malformed, upgraded, and cancellation cases.

### Security and privacy requirements

Number requirements `SR-01`, `SR-02`, and so on. Cover trust boundaries,
secret/byte handling, redaction, resource bounds, and fail-closed behavior.

### Performance and reliability requirements

Number requirements `PR-01`, `PR-02`, and so on. Define measurable latency,
memory, throughput, startup, backpressure, ordering, and recovery budgets.

### Accessibility and platform requirements

Describe what Core must expose so each presenter can produce a native,
accessible experience without Core prescribing presentation.

### Compatibility requirements

Cover Matrix protocol compatibility, stored data, DTO/event versions, mixed
client versions, downgrade behavior, and unknown future variants.

## 6. Ownership and boundary analysis

Document the proposed source of truth, concurrency owner, persistence owner,
platform adapters, and prohibited data. Explain how the design complies with
ADR 0004 or why a replacement decision is required.

## 7. Alternatives

Compare at least:

1. no ownership change;
2. a narrow extraction;
3. the proposed Core model.

Include complexity, parity, performance, operability, migration risk, and
reversibility. Record rejected alternatives and why they lose.

## 8. Proposed design

### Domain model and invariants

### Typed commands, queries, DTOs, and events

### Error and cancellation model

### Persistence and migration

### Observability and diagnostics

### Resource and abuse limits

## 9. Client adoption

### macOS/Linux React/Tauri

### iOS SwiftUI/UniFFI

State how old owners are removed and how dual ownership is prevented during
cutover.

## 10. Delivery slices

Each slice must be independently testable and reviewable.

| Slice | Behavior delivered | Code boundaries | Proof | Removal/rollback |
| ----- | ------------------ | --------------- | ----- | ---------------- |
| 1     |                    |                 |       |                  |

## 11. Test and validation plan

Address, where applicable:

- Rust unit and property tests;
- serialization and cross-language contract tests;
- concurrency, ordering, cancellation, and failure-injection tests;
- local Synapse single-client and multi-client proofs;
- crypto/device fixtures without production credentials;
- desktop React/Tauri integration and performance tests;
- iOS unit, simulator, notification-extension, and physical-device tests;
- malformed/adversarial Matrix events and resource-limit tests;
- accessibility, localization, offline, upgrade, and rollback checks;
- release smoke and diagnostics needed to prove the operating path.

Define acceptance criteria, not merely commands to run.

## 12. Rollout, rollback, and compatibility

Define feature flags if justified, data migration order, mixed-version behavior,
telemetry/diagnostics, abort thresholds, and exact rollback mechanics.

## 13. Risks and open decisions

| Risk/question | Impact | Evidence needed | Owner | Blocks implementation? |
| ------------- | ------ | --------------- | ----- | ---------------------- |
|               |        |                 |       |                        |

## 14. Adversarial review

Record objections, severity, disposition, and any plan changes. Include at
least one argument for keeping the behavior platform-side.

## 15. Final acceptance gate

- [ ] Current-state census reviewed.
- [ ] Requirements accepted.
- [ ] Boundary complies with accepted ADRs or replacement ADR accepted.
- [ ] Both client adoption paths are complete.
- [ ] Security and performance budgets are measurable.
- [ ] Validation proves one owner end to end.
- [ ] Rollback is feasible and tested.
- [ ] Superseded code has explicit removal criteria.
- [ ] Implementation owner and reviewers assigned.
