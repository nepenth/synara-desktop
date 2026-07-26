# Independent implementation review report template

| Field                             | Value                                                                                |
| --------------------------------- | ------------------------------------------------------------------------------------ |
| Artifact state                    | `ready_for_independent_review`                                                       |
| Schema version                    | `1.0`                                                                                |
| Authoritative validation contract | [`schemas/review-report.schema.json`](schemas/review-report.schema.json)             |
| Review rule                       | Review the complete final diff independently; merge state is not acceptance evidence |

> This template records an independent reviewer decision for one bounded task.
> The report's JSON twin must validate against the linked schema. The
> implementation writer cannot be the accepting reviewer or sign the report as
> reviewer. Re-review the entire final diff after every correction round.

## 1. Report and task identity

| JSON field         | Value                       |
| ------------------ | --------------------------- |
| `schema_version`   | `1.0`                       |
| `report_id`        | `<unique review report ID>` |
| `task_id`          | `<task ID from packet>`     |
| `task_packet_path` | `<reviewed packet path>`    |

## 2. Exact review context

| JSON field                          | Value                                   |
| ----------------------------------- | --------------------------------------- |
| `review_context.integration_branch` | `<integration branch>`                  |
| `review_context.work_branch`        | `<reviewed work branch>`                |
| `review_context.base_sha`           | `<full 40-character base SHA>`          |
| `review_context.head_sha`           | `<full 40-character reviewed head SHA>` |
| `review_context.pr_url`             | `<HTTPS PR URL>`                        |

The diff range, validation, CI, and signature must all refer to this exact head.
If the head changes, repeat the affected review and issue an updated report.

## 3. Independent reviewer attestation

Populate `reviewer`:

| Field                           | Required value                                                                   |
| ------------------------------- | -------------------------------------------------------------------------------- |
| `identity`                      | Named reviewer identity                                                          |
| `role`                          | Reviewer role                                                                    |
| `independent_of_implementation` | `true`                                                                           |
| `attestation`                   | `I reviewed the complete final diff and did not implement the reviewed changes.` |
| `reviewed_at`                   | ISO 8601 timestamp                                                               |

If the reviewer implemented any reviewed change, assign a different accepting
reviewer. A writer handback or self-check is useful evidence but is not this
attestation.

## 4. Changed-file scope audit

Populate `scope_audit`:

- `allowed_paths`: exact paths from the packet;
- `actual_changed_paths`: every path in the complete base-to-head diff;
- `generated_paths`: generated artifacts in that diff;
- `prohibited_changed_paths`: every violation, or `[]`;
- `verdict`: `pass`, `fail`, or `blocked`;
- `evidence`: commands/diff artifacts used to establish scope.

An `accept` verdict requires `scope_audit.verdict == pass` and no prohibited
changed path.

## 5. Requirement and acceptance matrix

Create one `requirement_matrix[]` entry for every packet acceptance criterion:

| Field          | Meaning                                                    |
| -------------- | ---------------------------------------------------------- |
| `criterion_id` | Exact packet criterion ID                                  |
| `verdict`      | `pass`, `fail`, or `blocked`                               |
| `evidence`     | Source, automated, live, platform, fixture, or UI evidence |
| `notes`        | Concrete reviewer reasoning and limits                     |

Do not collapse independent clauses into alternatives. A passing helper, mock,
fixture, compile probe, or renamed UI control cannot satisfy a required product
behavior unless the packet explicitly defines that as the criterion.

## 6. Pinned upstream API verification

Populate `upstream_api_verification` with:

- exact `release_tag` and 40-character `commit_sha` from the task packet;
- every checked source `permalink`, its exact `claim`, and `verified` boolean;
- overall `verdict` (`pass`, `fail`, or `blocked`);
- `notes` distinguishing source/API shape, compile proof, semantic proof, live
  proof, and UI acceptance.

Never validate against moving upstream `main`. An `SC-...` ID by itself is not
API verification.

## 7. Complete final-diff review

Populate `final_diff_review`:

| Field                          | Required evidence                                   |
| ------------------------------ | --------------------------------------------------- |
| `reviewed_range`               | `<40-char base>..<40-char head>`                    |
| `reviewed_complete_final_diff` | `true`                                              |
| `correction_rounds`            | Number of writer correction rounds                  |
| `last_correction_reviewed`     | `true` only after re-reviewing the final correction |
| `evidence`                     | Diff/stat commands and retained review notes        |

An incremental patch review does not replace review of the final complete diff.

## 8. Required audit domains

Each `audit_domains` entry has `verdict`, non-empty `evidence`, and `rationale`.
Allowed domain verdicts are `pass`, `fail`, `blocked`, and `not-applicable`.
`not-applicable` requires a task-specific rationale; it is not a blank check.

### Security and privacy (`security_privacy`)

Review tokens, credentials, store keys, recovery material, content, media,
diagnostics, logs, URLs/paths, persistence, redaction, and secret boundaries.

### Lifecycle and concurrency (`lifecycle_concurrency`)

Review ownership, startup order, cancellation, close/join barriers, stale
generation handling, retries, crash/reopen, logout, wipe, and partial failure.

### IPC contracts (`ipc_contracts`)

Review versioning, DTO ownership, bounds, sequencing, errors, payload size,
secret/content leakage, Rust/TypeScript agreement, and invalid-corpus behavior.

### Shared iOS (`shared_ios`)

Review shared contracts, canonical fixtures, SDK/protocol ownership, and iOS
build/test impact.

### Raw Matrix HTTP (`raw_matrix_http`)

Audit runtime `/_matrix/` networking and prove each retained exception is
approved, typed, and task-scoped. A new convenience fetch is a failure.

### Dual Matrix owner (`dual_matrix_owner`)

Audit for backend selectors, dual clients, competing sync/crypto/store owners,
and temporary paths that could become permanent.

### Legacy deletion (`legacy_deletion`)

Audit deletion scope, open handles/tasks, canonical confinement, recovery,
idempotence, and preservation of unrelated local data.

### Error handling (`error_handling`)

Audit swallowed errors, permissive fallbacks, privacy-safe diagnostics,
retry/repair behavior, and truthful UI state.

An `accept` verdict permits only `pass` or justified `not-applicable` across all
domains.

## 9. Independently rerun validation

Each `validation_runs[]` entry records:

- stable `id`;
- exact `command` and `cwd`;
- relevant non-secret `environment` values;
- ISO 8601 `started_at` and `finished_at`;
- integer `exit_code`;
- `result`: `pass`, `fail`, or `blocked`;
- retained `evidence`, such as an artifact path or concise output record.

Use the final reviewed head. Inspect that tests exercise the required behavior,
not only that the command exits successfully. An `accept` report requires every
listed required rerun to pass.

## 10. Exact-SHA CI

For every `ci_checks[]` entry record:

- workflow/check `name`;
- HTTPS run `url`;
- full `head_sha`;
- whether it is `required`;
- `status`: `success`, `failure`, `cancelled`, or `pending`;
- `cancelled`: explicit boolean.

Acceptance requires all required checks to be green, non-cancelled, and attached
to `review_context.head_sha`. A previous, superseded, or cancelled run is not
merge evidence.

## 11. Findings and correction disposition

For every `findings[]` entry record:

- `id`, `severity` (`critical`, `high`, `medium`, `low`, or `nit`), and
  `status` (`open`, `resolved`, or `accepted-risk`);
- `title`, `rationale`, and optional exact `file`/`line`;
- `required_correction`;
- `disposition` explaining the resulting change or authorized risk decision;
- evidence that the final diff resolves or preserves the finding.

When `status` is `accepted-risk`, `risk_acceptance` is mandatory and contains:

- `authority_identity`: the named approving authority;
- `authority_role`: the authority's role for this risk class;
- `approval_reference`: a durable approval URL, decision, or signed record;
- `bounded_rationale`: the exact accepted scope and why acceptance is justified,
  capped by the schema to prevent an unbounded narrative substitute;
- `review_by`: an ISO 8601 date on which the acceptance expires or must be
  reviewed.

`risk_acceptance` is forbidden on `open` and `resolved` findings. Thus an
accepted critical/high risk cannot validate using only a vague disposition.

Push evidence-backed corrections to the writer. After correction, rerun relevant
tests and review the complete final diff. `accept` permits no open finding.
Critical/high accepted risk additionally requires the program's explicit risk
authority; schema validity alone does not grant that authority.

## 12. Owned residuals

Each `residuals[]` entry must include:

| Field                  | Required value                                  |
| ---------------------- | ----------------------------------------------- |
| `id`                   | Stable residual/gate ID                         |
| `description`          | Exact missing behavior or evidence              |
| `status`               | `open`, `closed`, or `approved-scope-exception` |
| `owner`                | Named accountable owner                         |
| `owning_task`          | Concrete task ID                                |
| `gate`                 | Gate that prevents premature acceptance/cutover |
| `disposition_evidence` | Closure evidence or explicit scope approval     |

“Later,” “follow-up,” or merge state without an owner and gate is invalid.

## 13. Verdict

Set `verdict` to exactly one of:

- `accept`: all criteria pass; scope and upstream verification pass; final
  correction was reviewed; audit domains pass/are justified not-applicable;
  reviewer reruns pass; required CI is green and non-cancelled on the exact SHA;
  no finding remains open.
- `request_changes`: actionable implementation or evidence defects remain.
- `blocked`: progress requires an architecture, upstream, authority, platform,
  environment, or prerequisite decision outside the writer's task scope.

Merge state, elapsed effort, or a mostly passing test suite does not change this
decision rule.

## 14. Reviewer signature

Populate `signature` only after deciding the report verdict:

| Field               | Required value                                                  |
| ------------------- | --------------------------------------------------------------- |
| `identity`          | Independent reviewer identity                                   |
| `role`              | Reviewer/approver role                                          |
| `signed_at`         | ISO 8601 timestamp                                              |
| `reviewed_head_sha` | Exact full reviewed head SHA                                    |
| `decision`          | Same value as `verdict`                                         |
| `method`            | `github-review`, `git-signed-commit`, or `document-attestation` |
| `reference`         | PR review URL, signed commit, or durable attestation reference  |

The implementation writer must not place a reviewer signature in this report.
This template and schema remain `ready_for_independent_review` until a separate
reviewer approves them; their existence is not their acceptance.
