# Matrix replacement operating-path contract

This contract applies to one capability-bounded vertical at a time. It keeps the
replacement moving through its intended owner route without expanding every
slice into a general hardening, test, or cleanup project.

## Intended route

| Field                    | Contract                                                                                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Goal                     | Re-home one named retained Matrix capability to the Rust SDK and physically delete its superseded JavaScript owner.                                                                                                |
| Actor                    | Primary Codex orchestrator, one bounded implementer, and one independent reviewer.                                                                                                                                 |
| Starting state           | Clean integration tip; one queue item selected; no competing product PR.                                                                                                                                           |
| Correct first action     | Read the binding queue and define retained behavior, owning source boundary, deletion set, non-goals, and expected evidence.                                                                                       |
| Owner route              | Integration tip → bounded implementation → focused validation → independent exact-head review → required PR CI → integration merge.                                                                                |
| State transitions        | queued → scoped → implemented → reviewed → CI green → merged → ledger advanced.                                                                                                                                    |
| Side effects             | One branch/PR, capability-owned source deletion, generated inventory/ledger updates, and the integration merge.                                                                                                    |
| Authority boundaries     | Orchestrator owns scope and merge; implementer owns only its packet; reviewer is read-only; CI owns required repository gates; the user alone authorizes PR #39/main.                                              |
| Completion point         | The reviewed PR is merged into the integration branch.                                                                                                                                                             |
| Authoritative readback   | GitHub merged-PR state and integration SHA, generated SDK inventory, machine program ledger, and current continuation/progress documents agree.                                                                    |
| Acceptance criteria      | Native owner retained; legacy owner deleted; parity/privacy boundaries preserved; actual inventory delta recorded; scoped validation and required CI green; independent review has no blocker.                     |
| Disqualifying deviations | Extra feature work, incidental cleanup, speculative hardening, unearned tests/guardrails, fallback or dual ownership, parallel product slices, retry presented as a clean run, or main/#39 merge without approval. |

## Evidence budget

Use the smallest evidence set that can falsify the slice's actual risks:

1. Run formatting and the owning module/product tests for changed behavior.
2. Run TypeScript typecheck/lint when the changed boundary is TypeScript.
3. Regenerate and check inventory/ledger only when ownership or imports change.
4. Use the repository's already-required PR CI as the broad integration proof;
   do not routinely duplicate its complete suite locally.
5. Add or retain a test only when it protects changed retained behavior, a
   privacy/authority/side-effect boundary, or a reproduced defect that remains
   after simplifying the owning path.

Deleting tests that only exercise the deleted JavaScript owner is expected.
An incidental failure outside the route is recorded separately. An environment
failure such as disk exhaustion is repaired at the environment boundary and the
exact affected check is rerun; it does not justify product code or new tests.

Before adding any test, linter, gate, guardrail, retry, fallback, compatibility
path, detector, or telemetry assertion, answer:

1. Which confirmed operating path does it preserve?
2. What concrete failure remains after core-path simplification?
3. Which authority, integrity, privacy, destructive-action, or side-effect
   boundary does it protect?
4. Why is the existing focused validation plus required CI insufficient?
5. Which owner will maintain the mechanism and its failure behavior?

If the answers are not concrete, the mechanism is out of scope.

## Divergence and proof rule

When the actual route diverges, repair the earliest owner-controlled boundary,
then rerun the complete affected path from a clean start. A retry without a
repair remains part of the failed attempt. Report each proof as `Confirmed`,
`Not confirmed`, or `Failed`; a green label alone is not proof.

This delivery-path verdict does not automatically prove a live end-user Matrix
workflow. Require a separate real-operation proof only when the vertical's
acceptance criteria explicitly call for it.
