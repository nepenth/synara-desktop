import assert from "node:assert/strict";
import { mkdtemp, mkdir, symlink, writeFile, readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  INTEGRATION_BRANCH,
  REVIEW_REPORT_SCHEMA_ID,
  TASK_PACKET_SCHEMA_ID,
  auditSupportedSchema,
  discoverCanonicalRoot,
  formatDiagnostics,
  inventoryTrackedJsonFiles,
  normalizeRepositoryPath,
  parseJsonStrict,
  validateProductionReview,
  validateCanonicalInstanceSchemaId,
  validateReviewReport,
  validateSchemaInstance,
  validateTaskPacket,
} from "../lib/matrix-rust-governance.mjs";

const BASE = "1".repeat(40);
const HEAD = "2".repeat(40);
const authority = Object.fromEntries(
  [
    "commit",
    "push",
    "rebase",
    "switch_branch",
    "open_pr",
    "merge_pr",
    "delete_unrelated_files",
    "modify_program_plan",
  ].map((field) => [field, false])
);
const categories = [
  "scope-dependency",
  "git-base-target",
  "upstream-architecture",
  "invariant",
  "prerequisite-validation-evidence",
  "authority-approval",
];

function packet() {
  return {
    schema_version: "1.0",
    schema_id: TASK_PACKET_SCHEMA_ID,
    packet_id: "R0.2-D",
    writer: {
      identity: "Writer A",
      role: "implementation_writer",
      git_pr_authority: { ...authority },
    },
    task: {
      id: "R0.2-D",
      title: "Governance",
      objective: "Produce bounded governance artifacts.",
    },
    git_context: {
      integration_branch: INTEGRATION_BRANCH,
      base_sha: BASE,
      work_branch: "matrix-rust/r0.2-d",
      pr_target: INTEGRATION_BRANCH,
      expected_pr_state: "draft",
      required_ci_checks: ["Quality gate"],
    },
    file_scope: {
      allowed_paths: [{ path: "docs/a.json", kind: "file" }],
      prohibited_paths: [{ path: "secrets", kind: "directory" }],
      generated_paths: [],
      out_of_scope_paths: [{ path: "other", kind: "directory" }],
    },
    dependency_policy: {
      changes_allowed: false,
      allowed_changes: [],
      prohibited_changes: ["package-lock.json"],
      unlisted_change_requires_approval: true,
    },
    traceability: {
      plan_sections: ["R0.2-D"],
      feature_requirement_ids: [],
      clause_ids: [],
      capability_ids: [],
      gap_ids: [],
      risk_ids: [],
      decision_ids: [],
    },
    upstream_evidence: {
      repository_url: "https://github.com/matrix-org/matrix-rust-sdk",
      release_tag: "v1",
      commit_sha: "3".repeat(40),
      permalinks: [
        {
          label: "API",
          url: `https://github.com/matrix-org/matrix-rust-sdk/blob/${"3".repeat(
            40
          )}/a`,
          claim: "API exists.",
          required: true,
        },
      ],
    },
    prerequisites: {
      gates: [
        { id: "R0.1", required_state: "accepted", evidence: "docs/gate.md" },
      ],
      required_artifacts: [{ path: "docs/input.json", purpose: "Input" }],
      blocking_assumptions: ["Baseline is reproducible."],
    },
    architecture: {
      decided_architecture: ["One owner."],
      constraints: ["No raw HTTP."],
      material_questions_remaining: [],
    },
    behavior: {
      requirements: [
        {
          id: "REQ-1",
          description: "Validate governance.",
          clauses: ["Reject invalid binding."],
          acceptance_criteria_ids: ["AC-1"],
        },
      ],
      non_goals: ["No product feature."],
    },
    invariants: {
      security: ["No secrets."],
      privacy: ["No content logs."],
      lifecycle: ["No background owner."],
      failure: ["Fail closed."],
    },
    ordered_work: [
      { step: 1, instruction: "Implement contract.", outputs: ["docs/a.json"] },
    ],
    validations: {
      automated: {
        required: true,
        cases: [
          {
            id: "AUTO-1",
            description: "Run tests.",
            execution_kind: "command",
            execution: "node --test",
            environment: ["Node 22"],
            expected_evidence: "Exit zero.",
          },
        ],
        waiver: null,
      },
      live_synapse: {
        required: false,
        cases: [],
        waiver: "No runtime behavior.",
      },
      fixtures: {
        required: false,
        cases: [],
        waiver: "No cross-language fixture.",
      },
      platform: {
        required: false,
        cases: [],
        waiver: "Platform-neutral artifact.",
      },
      manual: {
        required: false,
        cases: [],
        waiver: "Automated semantics cover behavior.",
      },
    },
    criterion_evidence_map: [
      {
        criterion_id: "AC-1",
        evidence_requirements: ["Test output."],
        validation_ids: ["AUTO-1"],
      },
    ],
    stop_escalate_conditions: categories.map((category, index) => ({
      id: `STOP-${index + 1}`,
      category,
      condition: `${category} mismatch is observed.`,
      required_action: "Stop and report evidence.",
      decision_authority: "Orchestrator",
    })),
    prohibitions: {
      new_matrix_js_sdk_use: false,
      production_raw_matrix_http: false,
      backend_selector: false,
      dual_matrix_client: false,
      fixture_helper_or_compile_only_as_product_support: false,
      suppressed_failures: false,
      writer_self_acceptance: false,
    },
    handback_requirements: {
      changed_files: true,
      commit_ids: true,
      validation_results: true,
      residuals: true,
      risks: true,
      pr_url: true,
      final_status: true,
    },
  };
}

function report(source = packet()) {
  const domain = {
    verdict: "pass",
    evidence: ["Reviewed final diff."],
    rationale: "Applicable contract reviewed.",
  };
  return {
    schema_version: "1.0",
    schema_id: REVIEW_REPORT_SCHEMA_ID,
    report_id: "REV-R0.2-D",
    task_id: source.task.id,
    task_packet_path:
      "docs/matrix-rust-sdk/governance/task-packets/r0.2-d.task-packet.json",
    subject: { base_sha: BASE, head_sha: HEAD },
    review_context: {
      integration_branch: INTEGRATION_BRANCH,
      work_branch: source.git_context.work_branch,
      base_sha: BASE,
      head_sha: HEAD,
      pr_url: "https://github.com/nepenth/synara-desktop/pull/80",
    },
    reviewer: {
      identity: "Reviewer B",
      role: "independent_reviewer",
      independent_of_implementation: true,
      attestation:
        "I reviewed the complete final diff and did not implement the reviewed changes.",
      reviewed_at: "2026-07-26T00:02:00Z",
    },
    scope_audit: {
      allowed_paths: [...source.file_scope.allowed_paths],
      actual_changed_paths: ["docs/a.json"],
      generated_paths: [],
      prohibited_changed_paths: [],
      verdict: "pass",
      evidence: ["git diff --name-only"],
    },
    packet_conformance: {
      dependency_policy: {
        changes_detected: false,
        actual_changes: [],
        verdict: "pass",
        evidence: ["No manifest changed."],
      },
      prerequisites: {
        gates: [
          {
            id: "R0.1",
            required_state: "accepted",
            packet_evidence: "docs/gate.md",
            verified: true,
          },
        ],
        required_artifacts: [
          {
            path: "docs/input.json",
            purpose: "Input",
            verified: true,
          },
        ],
        blocking_assumptions: [
          {
            assumption: "Baseline is reproducible.",
            verified: true,
            evidence: "Baseline command passed.",
          },
        ],
        verdict: "pass",
        evidence: ["Gate evidence reviewed."],
      },
    },
    requirement_matrix: [
      {
        criterion_id: "AC-1",
        verdict: "pass",
        evidence: ["AUTO-1"],
        notes: "Observed.",
      },
    ],
    upstream_api_verification: {
      repository_url: "https://github.com/matrix-org/matrix-rust-sdk",
      release_tag: "v1",
      commit_sha: "3".repeat(40),
      permalinks: [
        {
          label: "API",
          url: source.upstream_evidence.permalinks[0].url,
          claim: "API exists.",
          required: true,
          verified: true,
        },
      ],
      verdict: "pass",
      notes: "Pinned source verified.",
    },
    final_diff_review: {
      base_sha: BASE,
      head_sha: HEAD,
      reviewed_range: `${BASE}..${HEAD}`,
      reviewed_complete_final_diff: true,
      correction_rounds: 1,
      last_correction_reviewed: true,
      evidence: ["git diff"],
    },
    audit_domains: {
      security_privacy: { ...domain },
      lifecycle_concurrency: { ...domain },
      ipc_contracts: { ...domain },
      shared_ios: { ...domain },
      raw_matrix_http: { ...domain },
      dual_matrix_owner: { ...domain },
      legacy_deletion: { ...domain },
      error_handling: { ...domain },
    },
    validation_runs: [
      {
        id: "AUTO-1",
        required: true,
        execution_kind: "command",
        execution: "node --test",
        cwd: ".",
        environment: ["Node 22"],
        base_sha: BASE,
        head_sha: HEAD,
        started_at: "2026-07-26T00:00:00Z",
        finished_at: "2026-07-26T00:01:00Z",
        exit_code: 0,
        result: "pass",
        evidence: "Test output.",
      },
    ],
    ci_checks: [
      {
        name: "Quality gate",
        url: "https://github.com/nepenth/synara-desktop/actions/runs/1",
        base_sha: BASE,
        head_sha: HEAD,
        required: true,
        status: "success",
        cancelled: false,
      },
    ],
    findings: [],
    residuals: [],
    verdict: "accept",
    signature: {
      identity: "Reviewer B",
      role: "independent_reviewer",
      signed_at: "2026-07-26T00:03:00Z",
      reviewed_base_sha: BASE,
      reviewed_head_sha: HEAD,
      decision: "accept",
      method: "github-review",
      reference:
        "https://github.com/nepenth/synara-desktop/pull/80#pullrequestreview-1",
    },
  };
}

function codes(errors) {
  return new Set(errors.map((entry) => entry.code));
}

test("positive packet and report satisfy all semantic contracts", () => {
  const source = packet();
  assert.deepEqual(validateTaskPacket(source), []);
  assert.deepEqual(validateReviewReport(report(source), source), []);
});

test("writer identity, all-false Git/PR authority, branch, and target are enforced", () => {
  const source = packet();
  source.writer.git_pr_authority.push = true;
  source.git_context.integration_branch = "main";
  source.git_context.pr_target = "main";
  const result = codes(validateTaskPacket(source));
  assert(result.has("PACKET_WRITER_AUTHORITY"));
  assert(result.has("PACKET_INTEGRATION_BRANCH"));
  assert(result.has("PACKET_PR_TARGET"));
});

test("all six concrete categorized stop classes are mandatory", () => {
  const source = packet();
  source.stop_escalate_conditions.pop();
  source.stop_escalate_conditions[0].condition = "";
  const result = codes(validateTaskPacket(source));
  assert(result.has("PACKET_STOP_CATEGORY"));
  assert(result.has("PACKET_STOP_CONCRETE"));
});

test("typed validation execution is mandatory and automated cases must be commands", () => {
  const source = packet();
  source.validations.automated.cases[0].execution_kind = "procedure";
  assert(codes(validateTaskPacket(source)).has("PACKET_AUTOMATED_COMMAND"));
  const valid = packet();
  valid.validations.manual = {
    required: true,
    cases: [
      {
        id: "MAN-1",
        description: "Inspect.",
        execution_kind: "procedure",
        execution: "Inspect the final rendered artifact.",
        environment: ["Desktop"],
        expected_evidence: "Screenshot.",
      },
    ],
    waiver: null,
  };
  valid.criterion_evidence_map[0].validation_ids.push("MAN-1");
  assert.deepEqual(validateTaskPacket(valid), []);
});

test("base/head, diff, validation, CI, report, reviewer, and signature bindings are exact", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.subject.head_sha = "4".repeat(40);
  reviewed.final_diff_review.reviewed_range = `${BASE}..${"4".repeat(40)}`;
  reviewed.validation_runs[0].head_sha = "4".repeat(40);
  reviewed.ci_checks[0].base_sha = "4".repeat(40);
  reviewed.signature.reviewed_base_sha = "4".repeat(40);
  reviewed.signature.identity = "Someone else";
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_SUBJECT_BINDING"));
  assert(result.has("REPORT_RANGE_BINDING"));
  assert(result.has("REPORT_VALIDATION_SUBJECT"));
  assert(result.has("REPORT_CI_SUBJECT"));
  assert(result.has("REPORT_SIGNATURE_IDENTITY"));
  assert(result.has("REPORT_ACCEPT_INVALID"));
});

test("writer and reviewer must differ by both identity and role", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.reviewer.role = source.writer.role;
  reviewed.signature.role = source.writer.role;
  assert(
    codes(validateReviewReport(reviewed, source)).has(
      "REPORT_REVIEWER_INDEPENDENCE"
    )
  );
});

test("scope verdict cannot hide undeclared, generated, prohibited, or out-of-scope changes", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.scope_audit.actual_changed_paths.push("other/leak.txt");
  reviewed.scope_audit.generated_paths.push("other/leak.txt");
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_GENERATED_SCOPE_PARITY"));
  assert(result.has("REPORT_SCOPE_TRUTH"));
  assert(result.has("REPORT_SCOPE_VERDICT"));
});

test("dependency, prerequisite, requirement, validation, and upstream evidence must match packet", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.packet_conformance.dependency_policy = {
    changes_detected: false,
    actual_changes: ["ajv"],
    verdict: "pass",
    evidence: ["Claim"],
  };
  reviewed.packet_conformance.prerequisites.gates = [];
  reviewed.requirement_matrix[0].criterion_id = "AC-WRONG";
  reviewed.validation_runs[0].execution = "true";
  reviewed.upstream_api_verification.permalinks[0].claim = "Different claim.";
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_DEPENDENCY_TRUTH"));
  assert(result.has("REPORT_PREREQUISITE_PARITY"));
  assert(result.has("REPORT_REQUIREMENT_PARITY"));
  assert(result.has("REPORT_VALIDATION_EVIDENCE"));
  assert(result.has("REPORT_UPSTREAM_PARITY"));
});

test("passing validation requires exit zero and passing upstream requires every required link", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.validation_runs[0].exit_code = 1;
  reviewed.upstream_api_verification.permalinks[0].verified = false;
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_VALIDATION_RESULT"));
  assert(result.has("REPORT_UPSTREAM_VERDICT"));
});

test("accepted risk requires durable bounded independent authority", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.findings = [
    {
      id: "F-1",
      severity: "medium",
      status: "accepted-risk",
      title: "Risk",
      rationale: "Known.",
      required_correction: "None authorized.",
      disposition: "Accepted.",
      evidence: ["Review"],
      risk_acceptance: {
        authority_identity: source.writer.identity,
        authority_role: source.writer.role,
        approval_reference: "x",
        bounded_rationale: "",
        review_by: "later",
      },
    },
  ];
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_RISK_REFERENCE"));
  assert(result.has("REPORT_RISK_BOUNDS"));
  assert(result.has("REPORT_RISK_WRITER"));
});

test("critical/high accepted risk requires external authority and forbids reviewer self-acceptance", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.findings = [
    {
      id: "F-2",
      severity: "high",
      status: "accepted-risk",
      title: "Risk",
      rationale: "Known.",
      required_correction: "None authorized.",
      disposition: "Accepted.",
      evidence: ["Review"],
      risk_acceptance: {
        authority_identity: reviewed.reviewer.identity,
        authority_role: reviewed.reviewer.role,
        approval_reference: "docs/risk.md#F-2",
        bounded_rationale: "Accepted only through the review date.",
        review_by: "2026-08-01",
      },
    },
  ];
  assert(
    codes(validateReviewReport(reviewed, source)).has(
      "REPORT_HIGH_RISK_AUTHORITY"
    )
  );
});

test("open/resolved findings forbid accepted-risk payload and signature must match verdict", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.verdict = "request_changes";
  reviewed.findings = [
    {
      id: "F-3",
      severity: "low",
      status: "resolved",
      title: "Fixed",
      rationale: "Fixed.",
      required_correction: "Fix.",
      disposition: "Done.",
      evidence: ["Diff"],
      risk_acceptance: {
        authority_identity: "Owner",
        authority_role: "program_owner",
        approval_reference: "docs/risk.md#F-3",
        bounded_rationale: "N/A",
        review_by: "2026-08-01",
      },
    },
  ];
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_RISK_EXCLUSIVITY"));
  assert(result.has("REPORT_SIGNATURE_DECISION"));
});

test("schema contracts are Draft 2020-12, closed, and contain schema-level conditions", async () => {
  const taskSchema = JSON.parse(
    await readFile(
      "docs/matrix-rust-sdk/schemas/native-agent-task-packet.schema.json",
      "utf8"
    )
  );
  const reportSchema = JSON.parse(
    await readFile(
      "docs/matrix-rust-sdk/schemas/review-report.schema.json",
      "utf8"
    )
  );
  assert.equal(
    taskSchema.$schema,
    "https://json-schema.org/draft/2020-12/schema"
  );
  assert.equal(
    reportSchema.$schema,
    "https://json-schema.org/draft/2020-12/schema"
  );
  assert.equal(taskSchema.additionalProperties, false);
  assert.equal(reportSchema.additionalProperties, false);
  assert.equal(
    taskSchema.properties.git_context.properties.pr_target.const,
    INTEGRATION_BRANCH
  );
  assert.equal(
    taskSchema.$defs.automatedValidationGroup.allOf[1].properties.cases.items
      .properties.execution_kind.const,
    "command"
  );
  assert(
    reportSchema.$defs.finding.allOf.some((condition) =>
      condition.then?.required?.includes("risk_acceptance")
    )
  );
  assert.deepEqual(validateSchemaInstance(taskSchema, packet()), []);
  assert.deepEqual(validateSchemaInstance(reportSchema, report()), []);
  const open = packet();
  open.unreviewed_escape_hatch = true;
  assert(
    codes(validateSchemaInstance(taskSchema, open)).has(
      "SCHEMA_ADDITIONAL_PROPERTY"
    )
  );
});

test("diagnostics are stable, sorted, and do not expose object values", () => {
  const output = formatDiagnostics([
    { code: "B", path: "z", message: "safe" },
    { code: "A", path: "a", message: "safe" },
  ]);
  assert.deepEqual(output, ["A a: safe", "B z: safe"]);
  assert(!output.join("\n").includes("secret-value"));
});

test("production checker validates contracts with no bypass arguments", () => {
  const result = spawnSync(
    process.execPath,
    ["scripts/check-matrix-rust-governance-artifacts.mjs"],
    { encoding: "utf8" }
  );
  assert.equal(result.status, 0, result.stderr);
  const bypass = spawnSync(
    process.execPath,
    ["scripts/check-matrix-rust-governance-artifacts.mjs", "--skip"],
    { encoding: "utf8" }
  );
  assert.equal(bypass.status, 1);
  assert.match(bypass.stderr, /ARGUMENTS_FORBIDDEN/);
});

test("strict JSON rejects malformed text and duplicate object keys", () => {
  assert.throws(
    () => parseJsonStrict('{"schema_id":1,"schema_id":2}'),
    /Duplicate/
  );
  assert.throws(() => parseJsonStrict('{"schema_id":'), /JSON/);
});

test("canonical discovery is zero-state safe and rejects wrong suffixes and symlinks", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-discovery-"));
  assert.deepEqual(
    discoverCanonicalRoot(
      root,
      "governance/task-packets",
      ".task-packet.json",
      "task packet"
    ),
    { files: [], errors: [] }
  );
  const canonicalRoot = path.join(root, "governance", "task-packets");
  await mkdir(canonicalRoot, { recursive: true });
  await writeFile(path.join(canonicalRoot, "wrong.json"), "{}");
  await symlink(
    path.join(canonicalRoot, "wrong.json"),
    path.join(canonicalRoot, "escape.task-packet.json")
  );
  const result = discoverCanonicalRoot(
    root,
    "governance/task-packets",
    ".task-packet.json",
    "task packet"
  );
  assert(codes(result.errors).has("DISCOVERY_SUFFIX"));
  assert(codes(result.errors).has("DISCOVERY_SYMLINK"));
});

test("canonical discovery rejects symlinked root components", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-discovery-root-"));
  const outside = await mkdtemp(
    path.join(os.tmpdir(), "synara-discovery-outside-")
  );
  await symlink(outside, path.join(root, "governance"));
  const result = discoverCanonicalRoot(
    root,
    "governance/task-packets",
    ".task-packet.json",
    "task packet"
  );
  assert(codes(result.errors).has("DISCOVERY_ROOT"));
});

test("canonical instances require an exact schema ID", () => {
  assert(
    codes(validateCanonicalInstanceSchemaId("task packet", {})).has(
      "INSTANCE_SCHEMA_ID"
    )
  );
  assert(
    codes(
      validateCanonicalInstanceSchemaId("review report", {
        schema_id: `${REVIEW_REPORT_SCHEMA_ID}-typo`,
      })
    ).has("INSTANCE_SCHEMA_ID")
  );
  assert.deepEqual(
    validateCanonicalInstanceSchemaId("task packet", {
      schema_id: TASK_PACKET_SCHEMA_ID,
    }),
    []
  );
});

test("strict schema subset rejects unsupported keywords and invalid calendar values", async () => {
  assert(
    codes(auditSupportedSchema({ type: "string", minProperties: 1 })).has(
      "SCHEMA_KEYWORD_UNSUPPORTED"
    )
  );
  const schema = JSON.parse(
    await readFile(
      "docs/matrix-rust-sdk/schemas/review-report.schema.json",
      "utf8"
    )
  );
  const invalid = report();
  invalid.reviewer.reviewed_at = "2026-02-30T00:00:00Z";
  assert(codes(validateSchemaInstance(schema, invalid)).has("SCHEMA_FORMAT"));
});

test("path normalization rejects absolute, traversal, empty, trailing, control, drive, UNC, and backslash ambiguity", () => {
  for (const candidate of [
    "/a",
    "C:/a",
    "a/../b",
    "a/./b",
    "a//b",
    "a/",
    "a\\b",
    "a\0b",
    "//server/share",
  ])
    assert.equal(normalizeRepositoryPath(candidate), null, candidate);
  assert.equal(normalizeRepositoryPath("docs/a.json"), "docs/a.json");
});

test("exact file scope does not admit a same-prefix path", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.scope_audit.actual_changed_paths = ["docs/a.json/escape"];
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_SCOPE_TRUTH"));
  assert(result.has("REPORT_SCOPE_VERDICT"));
});

test("moving-main upstream links and repository substitutions fail", () => {
  const source = packet();
  source.upstream_evidence.repository_url = "https://github.com/example/fork";
  source.upstream_evidence.permalinks[0].url =
    "https://github.com/matrix-org/matrix-rust-sdk/blob/main/a";
  const result = codes(validateTaskPacket(source));
  assert(result.has("PACKET_UPSTREAM_REPOSITORY"));
  assert(result.has("PACKET_UPSTREAM_PERMALINK"));
});

test("upstream permalinks reject traversal, encoded ambiguity, query, and credentials", () => {
  const sha = "3".repeat(40);
  for (const url of [
    `https://github.com/matrix-org/matrix-rust-sdk/blob/${sha}/../main/a`,
    `https://github.com/matrix-org/matrix-rust-sdk/blob/${sha}/%2e%2e/main/a`,
    `https://github.com/matrix-org/matrix-rust-sdk/blob/${sha}/a%2fb`,
    `https://github.com/matrix-org/matrix-rust-sdk/blob/${sha}/a?moving=1`,
    `https://user@github.com/matrix-org/matrix-rust-sdk/blob/${sha}/a`,
  ]) {
    const source = packet();
    source.upstream_evidence.permalinks[0].url = url;
    assert(
      codes(validateTaskPacket(source)).has("PACKET_UPSTREAM_PERMALINK"),
      url
    );
  }
});

test("PR, review signature, and required CI URLs bind to exact Synara resources", () => {
  const source = packet();
  const external = report(source);
  external.review_context.pr_url = "https://github.com/example/repo/pull/80";
  external.ci_checks[0].url = "https://github.com/example/repo/actions/runs/1";
  const externalCodes = codes(validateReviewReport(external, source));
  assert(externalCodes.has("REPORT_PR_URL"));
  assert(externalCodes.has("REPORT_CI_URL"));

  const different = report(source);
  different.signature.reference =
    "https://github.com/nepenth/synara-desktop/pull/81#pullrequestreview-1";
  assert(
    codes(validateReviewReport(different, source)).has(
      "REPORT_SIGNATURE_REFERENCE"
    )
  );
});

test("truthful request-changes and blocked reports may enumerate violations and failing required CI", () => {
  const source = packet();
  for (const verdict of ["request_changes", "blocked"]) {
    const reviewed = report(source);
    reviewed.verdict = verdict;
    reviewed.signature.decision = verdict;
    reviewed.scope_audit.actual_changed_paths = ["other/leak.txt"];
    reviewed.scope_audit.prohibited_changed_paths = ["other/leak.txt"];
    reviewed.scope_audit.verdict = "fail";
    reviewed.ci_checks[0].status =
      verdict === "blocked" ? "pending" : "failure";
    const result = validateReviewReport(reviewed, source);
    assert(!codes(result).has("REPORT_SCOPE_TRUTH"));
    assert(!codes(result).has("REPORT_CI_REQUIRED_STATE"));
    assert.deepEqual(result, []);
  }
});

test("production diff inventory disables rename detection and catches both rename paths", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.scope_audit.actual_changed_paths = ["docs/a.json"];
  const runner = (_command, arguments_) => {
    if (arguments_[0] === "diff") {
      assert(arguments_.includes("--no-renames"));
      return {
        status: 0,
        stdout: Buffer.from("secrets/old.json\0docs/a.json\0"),
      };
    }
    return { status: 0, stdout: Buffer.alloc(0) };
  };
  assert(
    codes(
      validateProductionReview(reviewed, source, process.cwd(), runner)
    ).has("PRODUCTION_GIT_DIFF_PARITY")
  );
});

test("tracked JSON inventory failure is fatal, deterministic, and secret-free", () => {
  const result = inventoryTrackedJsonFiles(process.cwd(), () => ({
    status: 1,
    stdout: "",
    stderr: "TOP-SECRET PATH",
  }));
  assert.deepEqual(result.files, []);
  assert.deepEqual(formatDiagnostics(result.errors), [
    "INVENTORY_GIT repository inventory: Tracked JSON inventory could not be established.",
  ]);
  assert(!JSON.stringify(result).includes("TOP-SECRET"));
});

test("schema audit rejects malformed supported-keyword values", () => {
  const malformed = {
    type: ["string", "string"],
    pattern: "[",
    required: ["x", "x"],
    properties: [],
    items: "not-a-schema",
    allOf: [],
    minimum: Number.NaN,
    minLength: 2,
    maxLength: 1,
    $ref: "#/$defs/missing",
  };
  const result = codes(auditSupportedSchema(malformed));
  assert(result.has("SCHEMA_KEYWORD_VALUE"));
  assert(result.has("SCHEMA_REF_UNSUPPORTED"));
});

test("empty schema references fail audit and validation", () => {
  const emptyReference = { $ref: "" };
  assert(
    codes(auditSupportedSchema(emptyReference)).has("SCHEMA_REF_UNSUPPORTED")
  );
  assert(
    codes(validateSchemaInstance(emptyReference, "anything")).has("SCHEMA_REF")
  );
});

test("arbitrary signature and traversal risk approval references fail", () => {
  const source = packet();
  const reviewed = report(source);
  reviewed.signature.reference = "https://example.com/review";
  reviewed.findings = [
    {
      id: "F-X",
      severity: "medium",
      status: "accepted-risk",
      title: "Risk",
      rationale: "Known.",
      required_correction: "Bounded.",
      disposition: "Accepted.",
      evidence: ["Review"],
      risk_acceptance: {
        authority_identity: "Owner",
        authority_role: "program_owner",
        approval_reference: "docs/../secret.md#risk",
        bounded_rationale: "Until review.",
        review_by: "2026-07-26",
      },
    },
  ];
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_SIGNATURE_REFERENCE"));
  assert(result.has("REPORT_RISK_REFERENCE"));
});

test("expired risk and Unicode/case/whitespace independence bypasses fail", () => {
  const source = packet();
  source.writer.identity = " Writer A ";
  const reviewed = report(source);
  reviewed.reviewer.identity = "writer a";
  reviewed.signature.identity = "WRITER A";
  reviewed.findings = [
    {
      id: "F-Y",
      severity: "high",
      status: "accepted-risk",
      title: "Risk",
      rationale: "Known.",
      required_correction: "Bounded.",
      disposition: "Accepted.",
      evidence: ["Review"],
      risk_acceptance: {
        authority_identity: "Owner",
        authority_role: "security_owner",
        approval_reference: "docs/risk.md#risk",
        bounded_rationale: "Until review.",
        review_by: "2026-07-25",
      },
    },
  ];
  const result = codes(validateReviewReport(reviewed, source));
  assert(result.has("REPORT_REVIEWER_INDEPENDENCE"));
  assert(result.has("REPORT_RISK_EXPIRED"));
});

test("all-optional CI, duplicate IDs, invented procedure text, and prohibited dependencies fail", () => {
  const source = packet();
  source.behavior.requirements.push(
    structuredClone(source.behavior.requirements[0])
  );
  const reviewed = report(source);
  reviewed.ci_checks[0].required = false;
  reviewed.validation_runs[0].execution = "Invented manual command";
  reviewed.packet_conformance.dependency_policy = {
    changes_detected: true,
    actual_changes: ["package-lock.json"],
    verdict: "pass",
    evidence: ["Claim"],
  };
  const packetCodes = codes(validateTaskPacket(source));
  const reportCodes = codes(validateReviewReport(reviewed, source));
  assert(packetCodes.has("PACKET_REQUIREMENT_DUPLICATE"));
  assert(reportCodes.has("REPORT_CI_PARITY"));
  assert(reportCodes.has("REPORT_VALIDATION_EVIDENCE"));
  assert(reportCodes.has("REPORT_DEPENDENCY_TRUTH"));
});

test("production verification rejects a Git changed-path omission", () => {
  const current = spawnSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
  }).stdout.trim();
  const prior = spawnSync("git", ["rev-parse", "HEAD^"], {
    encoding: "utf8",
  }).stdout.trim();
  const source = packet();
  source.git_context.base_sha = prior;
  const reviewed = report(source);
  for (const location of [
    reviewed.subject,
    reviewed.final_diff_review,
    reviewed.signature,
  ]) {
    if (Object.hasOwn(location, "base_sha")) location.base_sha = prior;
    if (Object.hasOwn(location, "head_sha")) location.head_sha = current;
    if (Object.hasOwn(location, "reviewed_base_sha"))
      location.reviewed_base_sha = prior;
    if (Object.hasOwn(location, "reviewed_head_sha"))
      location.reviewed_head_sha = current;
  }
  reviewed.review_context.base_sha = prior;
  reviewed.review_context.head_sha = current;
  reviewed.final_diff_review.reviewed_range = `${prior}..${current}`;
  reviewed.scope_audit.actual_changed_paths = [];
  assert(
    codes(validateProductionReview(reviewed, source, process.cwd())).has(
      "PRODUCTION_GIT_DIFF_PARITY"
    )
  );
});

test("production repository references reject symlink escapes", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-governance-"));
  await mkdir(path.join(root, "docs"));
  await writeFile(path.join(root, "outside.md"), "safe");
  await symlink(
    path.join(root, "outside.md"),
    path.join(root, "docs", "risk.md")
  );
  const source = packet();
  const reviewed = report(source);
  reviewed.findings = [
    {
      id: "F-Z",
      severity: "medium",
      status: "accepted-risk",
      title: "Risk",
      rationale: "Known.",
      required_correction: "Bounded.",
      disposition: "Accepted.",
      evidence: ["Review"],
      risk_acceptance: {
        authority_identity: "Owner",
        authority_role: "program_owner",
        approval_reference: "docs/risk.md#risk",
        bounded_rationale: "Until review.",
        review_by: "2026-07-26",
      },
    },
  ];
  assert(
    codes(validateProductionReview(reviewed, source, root)).has(
      "PRODUCTION_RISK_REFERENCE"
    )
  );
});
