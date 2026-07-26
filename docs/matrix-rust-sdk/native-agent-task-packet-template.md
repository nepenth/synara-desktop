# Native implementation-agent task packet template

| Field                             | Value                                                                                          |
| --------------------------------- | ---------------------------------------------------------------------------------------------- |
| Artifact state                    | `ready_for_independent_review`                                                                 |
| Schema version                    | `1.0`                                                                                          |
| Authoritative validation contract | [`schemas/native-agent-task-packet.schema.json`](schemas/native-agent-task-packet.schema.json) |
| Acceptance authority              | Independent reviewer only; the implementation agent cannot accept its own work                 |

> This is a human-readable template for a bounded implementation packet. The
> completed packet's JSON twin must validate against the linked schema. Do not
> delegate the task while a material architecture question remains. Do not
> change a gate or task status merely because implementation or merge occurred.

Canonical instances live only under
`docs/matrix-rust-sdk/governance/task-packets/` and use the suffix
`.task-packet.json`. Symlinks, non-regular files, alternate suffixes, duplicate
JSON keys, and governance `schema_id` values outside that root are invalid.
Missing roots or zero instances validate contracts only and claim no governed
instance acceptance.

## 1. Packet and task identity

| JSON field       | Required value                                                                        |
| ---------------- | ------------------------------------------------------------------------------------- |
| `schema_version` | `1.0`                                                                                 |
| `schema_id`      | `https://synara.invalid/schemas/matrix-rust-sdk/native-agent-task-packet.schema.json` |
| `packet_id`      | Unique packet identifier, for example `R0.3-A`                                        |
| `task.id`        | Plan/remediation task ID                                                              |
| `task.title`     | Bounded task title                                                                    |
| `task.objective` | One outcome-oriented paragraph defining completion                                    |

Populate `writer.identity` with the stable implementation-writer identity and
set `writer.role` to exactly `implementation_writer`. Every field in
`writer.git_pr_authority` is explicitly
`false`: `commit`, `push`, `rebase`, `switch_branch`, `open_pr`, `merge_pr`,
`delete_unrelated_files`, and `modify_program_plan`. The orchestrator alone
owns Git/PR actions and program-plan changes. A packet delegates bounded file
work, never repository control.

## 2. Git and PR context

Record immutable branch context before work begins. The full 40-character base
SHA is mandatory.

| JSON field                       | Value                                      |
| -------------------------------- | ------------------------------------------ |
| `git_context.integration_branch` | `feature/matrix-rust-sdk-full-replacement` |
| `git_context.base_sha`           | `<40-character integration SHA>`           |
| `git_context.work_branch`        | `<task work branch>`                       |
| `git_context.pr_target`          | `feature/matrix-rust-sdk-full-replacement` |
| `git_context.expected_pr_state`  | `draft` or `ready-for-review`              |
| `git_context.required_ci_checks` | Nonempty unique exact required check names |

`integration_branch` and `pr_target` are semantically equal and never `main`.
The writer must stop if the actual base, work branch, or target differs from
this packet; the writer must not switch or repair branch state.

## 3. File and dependency scope

### Allowed paths

Populate `file_scope.allowed_paths` with typed objects `{path, kind}`, where
`kind` is `file` or `directory`. Files match only exactly; directories match
themselves and descendants. Reject absolute, drive/UNC, `.`, `..`, empty
segments, trailing slash, control/NUL, or backslash-ambiguous paths.

- `<allowed path>`

### Prohibited paths

Populate `file_scope.prohibited_paths`:

- `<prohibited path>`

### Generated and out-of-scope paths

- `file_scope.generated_paths`: `<exact normalized generated file or empty array>`
- `file_scope.out_of_scope_paths`: `<typed excluded entry or empty array>`

### Dependency policy

| JSON field                                            | Required decision                                                      |
| ----------------------------------------------------- | ---------------------------------------------------------------------- |
| `dependency_policy.changes_allowed`                   | `true` or `false`                                                      |
| `dependency_policy.allowed_changes`                   | Exact approved dependencies/features; empty when changes are forbidden |
| `dependency_policy.prohibited_changes`                | Explicitly prohibited manifests, versions, or features                 |
| `dependency_policy.unlisted_change_requires_approval` | Always `true`                                                          |

An unlisted file or dependency change is a scope escalation, not an implicit
extension of the packet.

## 4. Traceability

Populate every array, using an empty array only when that identifier class is
truly inapplicable:

| JSON field                             | Values                                                |
| -------------------------------------- | ----------------------------------------------------- |
| `traceability.plan_sections`           | Exact plan/review headings or line-stable section IDs |
| `traceability.feature_requirement_ids` | `FR-...` rows                                         |
| `traceability.clause_ids`              | Clause-level IDs for composite requirements           |
| `traceability.capability_ids`          | Accepted `SC-...` capability IDs                      |
| `traceability.gap_ids`                 | `GAP-...` IDs                                         |
| `traceability.risk_ids`                | Owned risk-register IDs                               |
| `traceability.decision_ids`            | Accepted ADR/decision IDs                             |

Every requirement below must map to a criterion and evidence row in Section 11.

## 5. Pinned upstream evidence

| JSON field                         | Value                                           |
| ---------------------------------- | ----------------------------------------------- |
| `upstream_evidence.repository_url` | `https://github.com/matrix-org/matrix-rust-sdk` |
| `upstream_evidence.release_tag`    | Exact approved release tag                      |
| `upstream_evidence.commit_sha`     | Exact 40-character tag commit                   |

For each `upstream_evidence.permalinks[]` entry, provide:

- `label`: API/source identifier;
- `url`: exact
  `https://github.com/matrix-org/matrix-rust-sdk/blob/<commit_sha>/...`
  permalink, never a moving branch or tag;
- `claim`: the precise claim established by that source.
- `required`: whether acceptance requires this exact source to be verified.

A docs.rs page or compile probe establishes only what it actually proves. It
must not be described as live product parity.

## 6. Prerequisites and gates

### Gates

For each `prerequisites.gates[]` entry, record:

- `id`;
- `required_state`: `accepted`, `closed`, `green`, or `reviewed`;
- `evidence`: committed artifact, report, or same-SHA CI reference.

### Required artifacts

For each `prerequisites.required_artifacts[]` entry, record `path`, `purpose`,
and, where frozen, `sha256`.

### Blocking assumptions

`prerequisites.blocking_assumptions` lists assumptions the writer must verify
before implementation. A failed assumption triggers Section 12.

## 7. Decided architecture

### Architecture decisions

Populate `architecture.decided_architecture` with concrete decisions the writer
must implement, not options to choose between.

- `<decided architecture>`

### Constraints

Populate `architecture.constraints` with non-negotiable boundaries.

- `<constraint>`

`architecture.material_questions_remaining` must be `[]`. If it is not empty,
the packet is not delegable and the orchestrator must resolve the question.

## 8. Required behavior and non-goals

For every `behavior.requirements[]` entry, provide:

| Field                     | Meaning                                                    |
| ------------------------- | ---------------------------------------------------------- |
| `id`                      | Stable requirement ID                                      |
| `description`             | Observable behavior to implement                           |
| `clauses`                 | Exhaustive independent clauses; no permissive alternatives |
| `acceptance_criteria_ids` | IDs mapped in Section 11                                   |

### Non-goals

Populate `behavior.non_goals` explicitly. If no product behavior is being added,
say so as a concrete non-goal rather than leaving ambiguity.

- `<non-goal>`

## 9. Invariants

Each invariant class is mandatory. For a genuinely inapplicable class, provide
a specific scoped statement explaining what must remain unchanged.

### Security (`invariants.security`)

- `<security invariant>`

### Privacy (`invariants.privacy`)

- `<privacy invariant>`

### Lifecycle (`invariants.lifecycle`)

- `<startup/shutdown/cancellation/ownership invariant>`

### Failure (`invariants.failure`)

- `<failure, retry, rollback, or preservation invariant>`

## 10. Ordered implementation work

Create one `ordered_work[]` entry per step:

1. `step`: positive integer.
2. `instruction`: exact bounded action.
3. `outputs`: files, symbols, tests, or evidence produced by the step.

The writer must not reorder work when doing so would bypass a prerequisite,
contract freeze, or safety gate.

## 11. Validation and acceptance evidence

Each validation category is an object with `required`, `cases`, and `waiver`.
When required, `cases` must be non-empty and `waiver` must be `null`. When not
required, record a concrete waiver rationale; omission is not a waiver.

### Automated (`validations.automated`)

For every case record `id`, `description`, `execution_kind` (`command` or
`procedure`), exact nonempty `execution`, `environment`, and
`expected_evidence`. Automated cases must use `command`; other categories may
use either kind. A report must reproduce kind, text, and environment exactly.

### Live disposable Synapse (`validations.live_synapse`)

Record server topology/environment, clients/accounts, actions, exact product
observables, failure cases, cleanup, and retained logs/artifacts. A mock transport
or direct raw HTTP probe does not substitute for required product/SDK behavior.

### Fixtures (`validations.fixtures`)

Record canonical fixture paths, consumers, mutation/negative cases, and expected
cross-language evidence.

### Platform (`validations.platform`)

Record each required OS, architecture, package/signing mode, command or manual
procedure, and retained evidence.

### Manual/UI (`validations.manual`)

Record preconditions, exact user actions, independent observable assertions,
and failure behavior. A helper-only, renamed-control, compile-only, or
fixture-only result is not product acceptance.

### Criterion-to-evidence mapping

For every `criterion_evidence_map[]` entry:

- `criterion_id`: an acceptance criterion referenced by Section 8;
- `evidence_requirements`: exact artifacts and observables required to pass;
- `validation_ids`: validation cases that establish the criterion.

Every criterion must map to evidence, and every required validation must support
at least one criterion.

## 12. Stop and escalate conditions

Each `stop_escalate_conditions[]` entry contains `id`, `category`, `condition`,
`required_action`, and `decision_authority`. Include exactly one concrete
condition for every mandatory category:

- `scope-dependency`: file, dependency, generated-output, or task scope would expand;
- `git-base-target`: actual base SHA, work branch, integration branch, or PR target differs;
- `upstream-architecture`: stable/typed SDK support is absent, upstream is experimental
  beyond an approved gate, or an architecture question remains unresolved;
- `invariant`: a security, privacy, lifecycle, or failure invariant cannot be preserved;
- `prerequisite-validation-evidence`: a prerequisite, baseline, required validation,
  or exact evidence requirement cannot be reproduced;
- `authority-approval`: the requested action lacks explicit authority or approval.

The writer stops and reports evidence. It does not invent raw HTTP, a backend
selector, a second Matrix owner, or a weakened acceptance substitute.

## 13. Mandatory prohibitions

The following `prohibitions` fields are all required and must all be `false`:

| Field                                               | Prohibited behavior                                            |
| --------------------------------------------------- | -------------------------------------------------------------- |
| `new_matrix_js_sdk_use`                             | Adding or expanding `matrix-js-sdk` use                        |
| `production_raw_matrix_http`                        | Runtime raw `/_matrix/` networking                             |
| `backend_selector`                                  | A JS/Rust runtime selector or alternate backend                |
| `dual_matrix_client`                                | Simultaneous JS and Rust client ownership                      |
| `fixture_helper_or_compile_only_as_product_support` | Claiming non-product evidence proves product behavior          |
| `suppressed_failures`                               | Hiding errors, ignored test failures, or permissive fallback   |
| `writer_self_acceptance`                            | Writer reviewing, signing, or accepting its own implementation |

An approved typed SDK request is not “raw Matrix HTTP.” If typed support is
absent, use Section 12 rather than weakening this prohibition.

## 14. Required handback

All `handback_requirements` flags are `true`. The implementation agent must
return:

- exact changed-file list;
- commit IDs, or an explicit statement that no commit was authorized;
- commands, environments, results, and retained validation evidence;
- every remaining residual and owning gate;
- introduced or changed risks;
- PR URL, or an explicit statement that no PR was authorized;
- final writer status such as `ready_for_independent_review` or `blocked`.

The only successful writer status is `ready_for_independent_review`. It is not
`reviewed`, `accepted`, or permission to merge.
