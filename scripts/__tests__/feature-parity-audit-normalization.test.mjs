import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { EventEmitter } from "node:events";
import { readFileSync, renameSync } from "node:fs";
import {
  cp,
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  realpath,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { PassThrough } from "node:stream";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const TEST_FILE = fileURLToPath(import.meta.url);
const REPOSITORY_ROOT = path.resolve(HERE, "../..");
const BENCHMARK_TEST_NAME =
  "full 119-row audit and v2 checkers stay deterministic and bounded in temporary Git";
const BENCHMARK_CHILD_ENV = "SYNARA_TRACEABILITY_ISOLATED_BENCHMARK";
const BENCHMARK_CHILD_VALUE = "feature-parity-rss-v1";
const BENCHMARK_CHILD_MARKER = "SYNARA_ISOLATED_BENCHMARK_EXECUTED_V1";
const BENCHMARK_CHILD_OUTPUT_LIMIT = 32 * 1024;
const LIBRARY_PATH = path.join(
  REPOSITORY_ROOT,
  "scripts/lib/feature-parity-traceability-v2.mjs"
);
const AUDIT_CHECKER = path.join(
  REPOSITORY_ROOT,
  "scripts/check-feature-parity-audit-normalization.mjs"
);
const AUDIT_GENERATOR = path.join(
  REPOSITORY_ROOT,
  "scripts/generate-feature-parity-audit-normalization.mjs"
);
const TRACEABILITY_CHECKER = path.join(
  REPOSITORY_ROOT,
  "scripts/check-feature-parity-traceability-v2.mjs"
);
const TRACEABILITY_GENERATOR = path.join(
  REPOSITORY_ROOT,
  "scripts/generate-feature-parity-traceability-v2.mjs"
);
const MIGRATOR = path.join(
  REPOSITORY_ROOT,
  "scripts/migrate-feature-parity-traceability-v2.mjs"
);
const AUDIT_PATH =
  "docs/matrix-rust-sdk/reviews/r0.2-e-audit-normalization-119.json";
const SECTION_COUNTS = {
  7.1: 13,
  7.2: 13,
  7.3: 17,
  7.4: 11,
  7.5: 11,
  7.6: 8,
  7.7: 9,
  7.8: 9,
  7.9: 13,
  "7.10": 7,
  7.11: 8,
};
const STATUS_CORRECTIONS = new Map(
  [
    "FR-7.1-004",
    "FR-7.1-010",
    "FR-7.2-003",
    "FR-7.2-008",
    "FR-7.2-011",
    "FR-7.3-007",
    "FR-7.3-009",
    "FR-7.3-011",
    "FR-7.4-006",
    "FR-7.4-007",
    "FR-7.4-009",
    "FR-7.4-011",
    "FR-7.6-002",
    "FR-7.6-006",
    "FR-7.8-005",
    "FR-7.8-006",
    "FR-7.8-007",
    "FR-7.9-004",
    "FR-7.10-003",
  ].map((id) => [id, ["implemented", "partial"]])
);
STATUS_CORRECTIONS.set("FR-7.1-009", ["implemented", "not_exposed"]);
STATUS_CORRECTIONS.set("FR-7.5-010", ["implemented", "missing"]);
STATUS_CORRECTIONS.set("FR-7.6-008", ["partial", "implemented"]);
STATUS_CORRECTIONS.set("FR-7.11-004", ["partial", "implemented"]);

const api = await import(pathToFileURL(LIBRARY_PATH));

function git(cwd, args, options = {}) {
  const result = spawnSync("git", args, {
    cwd,
    encoding: Object.hasOwn(options, "encoding") ? options.encoding : "utf8",
    env: {
      ...process.env,
      GIT_CONFIG_NOSYSTEM: "1",
      HOME: cwd,
      XDG_CONFIG_HOME: cwd,
    },
    input: options.input,
    maxBuffer: 64 * 1024 * 1024,
  });
  assert.equal(
    result.status,
    0,
    `git ${args.join(" ")} failed: ${String(result.stderr)}`
  );
  return typeof result.stdout === "string"
    ? result.stdout.trim()
    : result.stdout;
}

async function temporaryGitRepository(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-r02-e1-audit-"));
  t.after(async () => rm(root, { recursive: true, force: true }));
  git(root, ["init", "--quiet"]);
  git(root, ["config", "user.name", "Synara Test"]);
  git(root, ["config", "user.email", "synara-test@example.invalid"]);
  return root;
}

async function temporaryLocalGitClone(t) {
  const createdParent = await mkdtemp(
    path.join(os.tmpdir(), "synara-r02-e1-benchmark-")
  );
  const parent = await realpath(createdParent);
  const root = path.join(parent, "repository");
  t.after(async () => rm(parent, { recursive: true, force: true }));
  const result = spawnSync(
    "git",
    ["clone", "--quiet", "--local", "--no-checkout", REPOSITORY_ROOT, root],
    {
      cwd: parent,
      encoding: "utf8",
      env: {
        ...process.env,
        GIT_CONFIG_NOSYSTEM: "1",
        HOME: parent,
        XDG_CONFIG_HOME: parent,
      },
      shell: false,
      windowsHide: true,
    }
  );
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stderr, /https?:|ssh:|git@/iu);
  git(root, ["checkout", "--quiet", "--detach", "HEAD"]);
  return { parent, root };
}

async function commitFixture(root) {
  await mkdir(path.join(root, "src"), { recursive: true });
  await writeFile(path.join(root, "src/lf.txt"), "alpha\nbeta\n", "utf8");
  await writeFile(path.join(root, "src/crlf.txt"), "one\r\ntwo\r\n", "utf8");
  await writeFile(path.join(root, "src/no-final-lf.txt"), "last line", "utf8");
  await writeFile(
    path.join(root, "src/non-utf8.bin"),
    Buffer.from([0x66, 0x80, 0x6f])
  );
  await symlink("lf.txt", path.join(root, "src/link.txt"));
  git(root, ["add", "src"]);
  git(root, ["commit", "--quiet", "-m", "fixture"]);
  return git(root, ["rev-parse", "HEAD"]);
}

async function commitAuditArtifact(root, value, message = "audit artifact") {
  const absolute = path.join(root, AUDIT_PATH);
  await mkdir(path.dirname(absolute), { recursive: true });
  await writeFile(absolute, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  git(root, ["add", AUDIT_PATH]);
  git(root, ["commit", "--quiet", "-m", message]);
  return git(root, ["rev-parse", "HEAD"]);
}

function runNode(
  script,
  args = [],
  cwd = REPOSITORY_ROOT,
  extraEnvironment = {}
) {
  return spawnSync(process.execPath, [script, ...args], {
    cwd,
    encoding: "utf8",
    env: {
      ...process.env,
      NO_COLOR: "1",
      ...extraEnvironment,
    },
    timeout: 30_000,
  });
}

function assertCliSuccess(result, label) {
  assert.equal(result.status, 0, `${label}: ${result.stderr}`);
  assert.equal(result.signal, null, label);
  assert.equal(result.stderr, "", label);
  assert.match(result.stdout, /PASS/u, label);
  assert.doesNotMatch(
    result.stdout,
    /\/private\/tmp|\/tmp\/|[A-Za-z]:\\|https?:|ssh:|git@|secret/iu,
    label
  );
}

function scriptedGitProcesses(handlers = {}) {
  const children = [];
  const spawnProcess = (command, args) => {
    assert.equal(command, "git");
    const kind = args.includes("--batch-check") ? "check" : "content";
    const child = new EventEmitter();
    child.stdin = new PassThrough();
    child.stdout = new PassThrough();
    child.stderr = new PassThrough();
    child.exitCode = null;
    child.signalCode = null;
    child.closeEvents = 0;
    let input = "";
    child.stdin.on("data", (chunk) => {
      input += chunk.toString("utf8");
      for (;;) {
        const newline = input.indexOf("\n");
        if (newline < 0) break;
        const request = input.slice(0, newline);
        input = input.slice(newline + 1);
        handlers[kind]?.onRequest?.(request, child);
      }
    });
    child.stdin.on("finish", () => {
      handlers[kind]?.onFinish?.(child);
      if (!child.stdout.destroyed && !child.stdout.writableEnded)
        child.stdout.end();
      if (!child.stderr.destroyed && !child.stderr.writableEnded)
        child.stderr.end();
      child.exitCode = handlers[kind]?.exitCode ?? 0;
      child.signalCode = handlers[kind]?.signal ?? null;
      if (child.signalCode) child.exitCode = null;
      queueMicrotask(() => {
        child.closeEvents += 1;
        child.emit("close", child.exitCode, child.signalCode);
      });
    });
    children.push({ child, kind });
    return child;
  };
  return { children, spawnProcess };
}

async function settlesWithin(promise, milliseconds = 2_000) {
  let timeout;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timeout = setTimeout(
          () => reject(new Error("Operation did not settle before timeout.")),
          milliseconds
        );
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
}

function assertFakeGitChildrenClosed(children) {
  assert.equal(children.length, 2);
  for (const { child } of children) {
    assert.equal(child.stdin.writableEnded, true);
    assert.equal(child.closeEvents, 1);
    assert.ok(child.exitCode !== null || child.signalCode !== null);
  }
}

async function measureOperation(operation) {
  let peakRssBytes = process.memoryUsage().rss;
  const sample = () => {
    peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);
  };
  const interval = setInterval(sample, 1);
  const started = process.hrtime.bigint();
  try {
    const value = await operation();
    sample();
    return {
      value,
      elapsedMs: Number(process.hrtime.bigint() - started) / 1_000_000,
      peakRssBytes,
    };
  } finally {
    clearInterval(interval);
  }
}

function boundedChildOutput(value) {
  const output = String(value ?? "");
  if (output.length <= BENCHMARK_CHILD_OUTPUT_LIMIT) return output;
  return `[first ${
    output.length - BENCHMARK_CHILD_OUTPUT_LIMIT
  } characters omitted]\n${output.slice(-BENCHMARK_CHILD_OUTPUT_LIMIT)}`;
}

function runIsolatedBenchmarkChild() {
  const childEnvironment = {
    ...process.env,
    [BENCHMARK_CHILD_ENV]: BENCHMARK_CHILD_VALUE,
  };
  delete childEnvironment.NODE_TEST_CONTEXT;
  const result = spawnSync(
    process.execPath,
    ["--test", `--test-name-pattern=^${BENCHMARK_TEST_NAME}$`, TEST_FILE],
    {
      cwd: REPOSITORY_ROOT,
      env: childEnvironment,
      encoding: "utf8",
      timeout: 120_000,
      maxBuffer: 1024 * 1024,
      shell: false,
      windowsHide: true,
    }
  );
  assert.equal(
    result.status,
    0,
    [
      `isolated benchmark child failed (status=${String(
        result.status
      )}, signal=${String(result.signal)}, error=${String(
        result.error?.code ?? "none"
      )})`,
      `stdout:\n${boundedChildOutput(result.stdout)}`,
      `stderr:\n${boundedChildOutput(result.stderr)}`,
    ].join("\n")
  );
  assert.match(
    result.stdout,
    new RegExp(BENCHMARK_CHILD_MARKER, "u"),
    `isolated benchmark child did not execute the guarded body\nstdout:\n${boundedChildOutput(
      result.stdout
    )}\nstderr:\n${boundedChildOutput(result.stderr)}`
  );
}

function normalizedDiagnostics(result) {
  if (Array.isArray(result)) return result;
  if (Array.isArray(result?.diagnostics)) return result.diagnostics;
  if (Array.isArray(result?.errors)) return result.errors;
  return [];
}

function resultValid(result) {
  if (typeof result === "boolean") return result;
  if (typeof result?.valid === "boolean") return result.valid;
  if (typeof result?.ok === "boolean") return result.ok;
  return normalizedDiagnostics(result).length === 0;
}

function bytesOf(value) {
  return Buffer.isBuffer(value)
    ? value
    : Buffer.from(value instanceof Uint8Array ? value : String(value), "utf8");
}

function coveredPointers(result) {
  const pointers = result?.coveredPointers;
  return pointers instanceof Set ? [...pointers] : pointers;
}

function productionShapedAuditFixture() {
  const rows = [];
  for (const [section, count] of Object.entries(SECTION_COUNTS)) {
    for (let index = 1; index <= count; index += 1) {
      const id = `FR-${section}-${String(index).padStart(3, "0")}`;
      const clauseId = `${id}.C001`;
      const correction = STATUS_CORRECTIONS.get(id);
      const corrected = Boolean(correction);
      const recordedStatus = correction?.[0] ?? "partial";
      const auditedStatus = correction?.[1] ?? "partial";
      const row = {
        id,
        origin: "plan-section-7",
        section_id: section,
        plan: {
          heading: `Requirement ${id}`,
          normalized_text: `Normalized requirement ${id}.`,
          line_range: { start: rows.length + 1, end: rows.length + 1 },
        },
        audit: {
          state: "corrected_pending_rereview",
          subject_sha: api.AUDITED_SOURCE_COMMIT,
          reviewer: null,
          reviewed_at: null,
          report_ids: [],
          audited_payload_sha256: "0".repeat(64),
        },
        current_product: {
          status: auditedStatus,
          qualifiers: ["Source-inspected current-product result."],
          summary: `Current product is partial for ${id}.`,
          status_correction: {
            recorded_status: recordedStatus,
            audited_status: auditedStatus,
            changed: corrected,
            explanation: corrected
              ? "Source inspection corrected the prior rollup."
              : "Source inspection retained the prior rollup.",
          },
        },
        clauses: [
          {
            id: clauseId,
            text: `Stable clause for ${id}.`,
            current_product_status: auditedStatus,
            qualifiers: ["Runtime validation remains separate."],
            source_evidence: [
              {
                id: `EV-${id}-C001-SRC-001`,
                path: api.AUDITED_PLAN_PATH,
                symbol: "Matrix Rust SDK",
                lines: { start: 1, end: 1 },
                source_sha: api.AUDITED_SOURCE_COMMIT,
                snippet_sha256:
                  "0e778c34a2f5b6da49f87d6b0f14780fa4cd329a57777aeb038daecaf72f466b",
                role: "reachability_entry",
                explanation:
                  "The pinned plan heading is the synthetic reachability entry fixture.",
              },
            ],
            absence_evidence: [],
            causality: {
              kind: "product_owned",
              api_or_event_names: [],
              evidence_ids: [],
              product_ownership_reason:
                "This synthetic clause is product-owned.",
              matrix_boundary:
                "No Matrix network boundary is crossed by the fixture.",
            },
            existing_tests: [],
            required_tests: [
              {
                id: `TST-${id}-REQ-001`,
                evidence_class: "unit",
                task_ids: ["R0.2-E"],
                acceptance:
                  "Validate this clause without claiming runtime-green status.",
                status: "planned",
              },
            ],
            outcomes: {
              success: ["The behavior succeeds."],
              failure: ["The behavior fails closed."],
              fallback: ["No unsupported fallback is claimed."],
              cleanup: ["No resource survives cleanup."],
            },
            reachability: {
              id: `REACH-${id}-001`,
              kind: "reachable",
              entry_evidence_ids: [`EV-${id}-C001-SRC-001`],
              ordered_chain: ["Entry reaches the audited clause."],
              absence_evidence_ids: [],
              explanation: "The synthetic chain is explicit.",
            },
            rust_mapping: {
              capability_ids: [],
              gap_ids: [`GAP-${id}`],
              task_ids: ["P3.2"],
              gate_ids: ["R0.8"],
              validation_report_ids: [],
              qualification:
                "Current-product evidence does not prove Rust readiness.",
            },
            blocker_ids: ["R0.8"],
            risk_ids: [],
            architecture_decision_ids: [],
          },
        ],
        rust_cutover: {
          readiness: "blocked",
          implementation_subject_sha: null,
          matrix_owner: "matrix_js_sdk",
          surviving_raw_matrix_http: false,
          surviving_matrix_js_owner: true,
          capability_ids: [],
          gap_ids: [`GAP-${id}`],
          task_ids: ["P3.2"],
          gate_ids: ["R0.8"],
          blocker_ids: ["R0.8"],
          validation_report_ids: [],
          qualification: "Rust cutover is independently blocked.",
        },
        blocker_ids: ["R0.8"],
        task_ids: ["P3.2"],
        manual_acceptance_ids: [],
        migration_disposition_ids: [],
        security_privacy_lifecycle_ids: [],
        architecture_decision_ids: [],
        legacy_inventory_context: {
          file_paths: [],
          method_candidates: [],
          listener_candidates: [],
          model_coupling: [],
          behavior_evidence_prohibited: true,
        },
      };
      if (id === "FR-7.4-011") {
        row.current_product.qualifiers.push(
          "Medium correctness/privacy risk; no unsupported plaintext or key leak claim.",
          "Reject event.isEncrypted() && !event.getClearContent().",
          "Reject event.isDecryptionFailure()."
        );
      }
      row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
      rows.push(row);
    }
  }
  const blockers = Object.entries(api.EXPECTED_CENTRAL_RISK_CONTRACT).map(
    ([id, expected]) => ({
      id,
      kind: "risk",
      severity: expected.severity,
      status: expected.status,
      owner_task_ids: [...expected.owner_task_ids],
      authority: "Orchestrator review and task-specific accepted evidence.",
      affected_requirement_ids: [
        id === "MRSDK-R036" ? "FR-7.4-011" : rows[0].id,
      ],
      affected_clause_ids: [
        id === "MRSDK-R036"
          ? rows.find(({ id: rowId }) => rowId === "FR-7.4-011").clauses[0].id
          : rows[0].clauses[0].id,
      ],
      threat_ids: [...expected.threat_ids],
      boundary_ids: [...expected.boundary_ids],
      closure_criteria: [...expected.closure_criteria],
      closure_evidence: [],
      qualification: "This is an open risk, not an observed compromise.",
    })
  );
  const artifact = {
    schema_version: "1.0",
    audit_id: "R0.2-E-AUDIT-NORMALIZATION-119",
    subject: {
      repository: "synara-desktop",
      source_commit: api.AUDITED_SOURCE_COMMIT,
      plan: {
        path: api.AUDITED_PLAN_PATH,
        blob_oid: api.AUDITED_PLAN_BLOB,
        file_sha256: api.AUDITED_PLAN_SHA256,
      },
      risk_register: {
        commit_sha: api.RISK_REGISTER_COMMIT,
        path: api.RISK_REGISTER_PATH,
        blob_oid: api.RISK_REGISTER_BLOB,
        file_sha256: api.RISK_REGISTER_SHA256,
        canonical_sha256: api.RISK_REGISTER_CANONICAL_SHA256,
      },
      audit_method: "Source-inspected normalization at an immutable commit.",
    },
    source_inputs: api.SOURCE_INPUTS.map((input) => ({
      ...input,
      adapter_version: "r0.2-e1-v1",
      ingestion_notes: ["Logical durable provenance; no scratch path."],
    })),
    coverage: {
      expected_row_count: 119,
      actual_row_count: 119,
      unique_requirement_count: 119,
      section_counts: { ...SECTION_COUNTS },
      requirement_id_sha256: api.sha256(
        api.canonicalize(rows.map(({ id }) => id))
      ),
      status_correction_count: 23,
      sections_7_3_through_7_7_status_correction_subtotal: 11,
      sections_7_3_through_7_7_non_status_correction_manifest: structuredClone(
        api.EXPECTED_73_77_NON_STATUS_CORRECTIONS
      ),
      sections_7_3_through_7_7_identified_correction_count: 16,
    },
    vocabularies: {
      current_product_status: [
        "implemented",
        "partial",
        "missing",
        "not_exposed",
      ],
      rust_cutover_readiness: ["blocked"],
      causality: ["product_owned"],
      source_evidence_role: ["reachability_entry"],
      blocker_kind: ["risk"],
      severity: ["critical", "high", "medium"],
      architecture_decision_status: ["unresolved"],
      audit_state: ["corrected_pending_rereview"],
      validation_evidence_class: ["unit"],
    },
    architecture_decisions: Array.from({ length: 26 }, (_, index) => ({
      id: `AD-${String(index + 1).padStart(2, "0")}`,
      decision: `Architecture decision ${index + 1} remains explicit.`,
      status: "unresolved",
      owner: "Architecture owner",
      scope_authority: "The owner may decide only the documented scope.",
      affected_requirement_ids: [rows[index].id],
      affected_clause_ids: [rows[index].clauses[0].id],
      closure_evidence: [],
    })),
    blockers_and_risks: blockers,
    rows,
    normalization_rules: {
      fully_expanded_rows: true,
      no_ephemeral_or_indirect_evidence: true,
      test_source_is_not_runtime_evidence: true,
      current_product_separate_from_rust_cutover: true,
      canonicalization:
        "UTF-8 canonical JSON with UTF-16 code-unit key ordering.",
      row_mutable_json_pointers: [...api.ROW_MUTABLE_POINTERS],
      audit_payload_removed_json_pointers: [
        "/review",
        "/digests/canonical_payload_sha256",
      ],
    },
    review: {
      state: "pending_independent_review",
      reviewer: null,
      reviewed_at: null,
      report_ids: [],
      notes: [],
    },
    digests: {
      canonical_payload_sha256: "1".repeat(64),
      requirement_id_sha256: api.sha256(
        api.canonicalize(rows.map(({ id }) => id).sort())
      ),
      clause_id_sha256: api.sha256(
        api.canonicalize(
          rows.flatMap(({ clauses }) => clauses.map(({ id }) => id)).sort()
        )
      ),
      generated_markdown_file_sha256: null,
    },
  };
  artifact.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(artifact, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  return artifact;
}

function syntheticAuditIdentity(audit) {
  return {
    path: api.AUDIT_PATH,
    introducing_commit_sha: "3".repeat(40),
    blob_oid: "4".repeat(40),
    file_sha256: api.sha256(Buffer.from(api.prettyJson(audit))),
    canonical_semantic_sha256: api.sha256(api.canonicalize(audit)),
  };
}

function refreshAuditDigests(audit) {
  for (const row of audit.rows)
    row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
  audit.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(audit, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  return audit;
}

function refreshV2Summary(value) {
  value.summary.requirement_count = value.requirements.length;
  value.summary.clause_count = value.requirements.reduce(
    (count, row) => count + row.clauses.length,
    0
  );
  for (const status of Object.keys(value.summary.current_product_status_counts))
    value.summary.current_product_status_counts[status] =
      value.requirements.filter(
        (row) => row.current_product.status === status
      ).length;
  for (const readiness of Object.keys(
    value.summary.rust_cutover_readiness_counts
  ))
    value.summary.rust_cutover_readiness_counts[readiness] =
      value.requirements.filter(
        (row) => row.rust_cutover.readiness === readiness
      ).length;
  value.summary.accepted_audit_report_count = value.audit_reports.filter(
    (report) => report.verdict === "accept"
  ).length;
  value.summary.accepted_validation_report_count =
    value.validation_reports.filter(
      (report) => report.status === "pass"
    ).length;
  value.summary.open_blocker_count = value.blockers.filter((blocker) =>
    api.isUnresolvedRiskStatus(blocker.status)
  ).length;
  value.summary.unresolved_critical_high_blocker_count = value.blockers.filter(
    (blocker) =>
      api.isUnresolvedRiskStatus(blocker.status) &&
      new Set(["critical", "high"]).has(blocker.severity)
  ).length;
  value.summary.open_architecture_decision_count =
    value.architecture_decisions.filter((decision) =>
      new Set(["unresolved", "proposed"]).has(decision.status)
    ).length;
  return value;
}

function lifecycleAuthorization({
  id,
  entityKind = "requirement",
  entityId,
  sequence,
  operation = "advance",
  sourceEntity = null,
  sourceEntitySha256 = sourceEntity
    ? api.sha256(api.canonicalize(sourceEntity))
    : null,
  fromPayloadSha256,
  toState,
  previousAuthorization = null,
  reportId,
  subjectSha,
  reviewer,
  reviewedAt,
  rationale = "Reviewed lifecycle transition is authorized by durable evidence.",
  rollbackReason = null,
  rollbackEvidence = [],
}) {
  const projection = {
    requirement: "rust-cutover-row-state-v1",
    blocker: "blocker-lifecycle-state-v1",
    architecture_decision: "architecture-decision-lifecycle-state-v1",
  }[entityKind];
  assert.ok(projection, `unsupported lifecycle kind ${entityKind}`);
  const authorization = {
    id,
    entity_kind: entityKind,
    entity_id: entityId,
    projection,
    sequence,
    operation,
    source_entity: sourceEntity ? structuredClone(sourceEntity) : null,
    source_entity_sha256: sourceEntitySha256,
    from_payload_sha256: fromPayloadSha256,
    to_state: structuredClone(toState),
    to_payload_sha256: api.sha256(api.canonicalize(toState)),
    previous_authorization_sha256: previousAuthorization
      ? api.lifecycleAuthorizationDigest(previousAuthorization)
      : null,
    report_id: reportId,
    subject_sha: subjectSha,
    reviewer,
    reviewed_at: reviewedAt,
    rationale,
    rollback_reason: rollbackReason,
    rollback_evidence: [...rollbackEvidence],
    authorization_sha256: "0".repeat(64),
  };
  authorization.authorization_sha256 =
    api.lifecycleAuthorizationDigest(authorization);
  return authorization;
}

function refreshLifecycle(value, audit) {
  value.lifecycle = api.deriveLifecycleManifest(
    value,
    audit,
    value.lifecycle?.previous_artifact ?? null
  );
  return value;
}

function refreshLatestRequirementAuthorization(value, audit, rowIndex = 0) {
  const row = value.requirements[rowIndex];
  const authorizations = value.validation_reports.flatMap(
    (report) => report.lifecycle_authorizations
  );
  const authorization = authorizations
    .filter(
      (entry) =>
        entry.entity_kind === "requirement" && entry.entity_id === row.id
    )
    .at(-1);
  assert.ok(authorization, `missing lifecycle authorization for ${row.id}`);
  authorization.to_state = api.cutoverStateProjection(row);
  authorization.to_payload_sha256 = api.sha256(
    api.canonicalize(authorization.to_state)
  );
  authorization.authorization_sha256 =
    api.lifecycleAuthorizationDigest(authorization);
  refreshLifecycle(value, audit);
  return authorization;
}

function rehashLifecycleAuthorizations(report) {
  let previous = null;
  for (const authorization of report.lifecycle_authorizations) {
    authorization.previous_authorization_sha256 = previous
      ? api.lifecycleAuthorizationDigest(previous)
      : null;
    authorization.authorization_sha256 =
      api.lifecycleAuthorizationDigest(authorization);
    previous = authorization;
  }
}

function artifactIdentity(root, commitSha) {
  const bytes = git(root, ["show", `${commitSha}:${api.V1_PATH}`], {
    encoding: null,
  });
  const value = api.parseCanonicalJson(bytes);
  return {
    commit_sha: commitSha,
    blob_oid: git(root, ["rev-parse", `${commitSha}:${api.V1_PATH}`]),
    file_sha256: api.sha256(bytes),
    canonical_sha256: api.sha256(api.canonicalize(value)),
  };
}

async function persistLifecycleReport(root, reportPath, payload, label) {
  const bytes = Buffer.from(api.prettyJson(payload));
  await mkdir(path.dirname(path.join(root, reportPath)), { recursive: true });
  await writeFile(path.join(root, reportPath), bytes);
  git(root, ["add", reportPath]);
  git(root, ["commit", "--quiet", "-m", label]);
  const commitSha = git(root, ["rev-parse", "HEAD"]);
  return {
    ...payload,
    storage_commit_sha: commitSha,
    storage_blob_oid: git(root, ["rev-parse", `${commitSha}:${reportPath}`]),
    storage_projection: "report-record-minus-storage-identity-v1",
    path: reportPath,
    file_sha256: api.sha256(bytes),
  };
}

async function commitV2Artifact(root, value, audit, predecessor, label) {
  value.lifecycle.previous_artifact = predecessor
    ? artifactIdentity(root, predecessor)
    : null;
  refreshV2Summary(value);
  refreshLifecycle(value, audit);
  await writeFile(
    path.join(root, api.V1_PATH),
    Buffer.from(api.prettyJson(value))
  );
  git(root, ["add", api.V1_PATH]);
  git(root, ["commit", "--quiet", "-m", label]);
  return git(root, ["rev-parse", "HEAD"]);
}

async function productionCliRepository(
  t,
  audit = productionShapedAuditFixture()
) {
  const cloned = await temporaryLocalGitClone(t);
  const { root } = cloned;
  git(root, ["checkout", "--quiet", "--detach", "HEAD"]);
  git(root, ["config", "user.name", "Synara Test"]);
  git(root, ["config", "user.email", "synara-test@example.invalid"]);
  const tooling = [
    api.AUDIT_SCHEMA_PATH,
    api.V2_SCHEMA_PATH,
    "scripts/lib/feature-parity-traceability-v2.mjs",
    "scripts/check-feature-parity-audit-normalization.mjs",
    "scripts/generate-feature-parity-audit-normalization.mjs",
    "scripts/check-feature-parity-traceability-v2.mjs",
    "scripts/generate-feature-parity-traceability-v2.mjs",
    "scripts/migrate-feature-parity-traceability-v2.mjs",
  ];
  for (const relative of tooling) {
    await mkdir(path.dirname(path.join(root, relative)), { recursive: true });
    await cp(path.join(REPOSITORY_ROOT, relative), path.join(root, relative));
  }
  const auditJson = Buffer.from(api.prettyJson(audit));
  const auditMarkdown = api.renderAuditMarkdown(audit).bytes;
  await mkdir(path.dirname(path.join(root, api.AUDIT_PATH)), {
    recursive: true,
  });
  await writeFile(path.join(root, api.AUDIT_PATH), auditJson);
  await writeFile(path.join(root, api.AUDIT_MARKDOWN_PATH), auditMarkdown);
  git(root, ["add", ...tooling, api.AUDIT_PATH, api.AUDIT_MARKDOWN_PATH]);
  git(root, ["commit", "--quiet", "-m", "complete traceability CLI fixture"]);
  return { ...cloned, audit, auditJson, auditMarkdown };
}

test("shared library exposes the complete packet-specified audit API", () => {
  for (const name of [
    "canonicalize",
    "sha256",
    "validateSchema",
    "validateAudit",
    "renderAuditMarkdown",
    "runCli",
  ]) {
    assert.equal(typeof api[name], "function", `missing export ${name}`);
  }
});

test("canonical JSON uses UTF-16 key ordering and preserves array order", () => {
  const value = {
    "\ud83d\ude00": 3,
    "\u00e9": 4,
    z: 1,
    a: 2,
    nested: [{ z: false, a: null }, "unchanged"],
  };
  assert.equal(
    bytesOf(api.canonicalize(value)).toString("utf8"),
    '{"a":2,"nested":[{"a":null,"z":false},"unchanged"],"z":1,"\u00e9":4,"\ud83d\ude00":3}'
  );
  assert.deepEqual(
    api.canonicalize(value),
    api.canonicalize(structuredClone(value))
  );
});

test("canonical JSON rejects every out-of-domain numeric/string value", () => {
  for (const value of [
    1.5,
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
    Number.MAX_SAFE_INTEGER + 1,
    -0,
    "\ud800",
    undefined,
    1n,
  ]) {
    assert.throws(() => api.canonicalize({ value }), undefined, String(value));
  }
  assert.throws(() => api.canonicalize({ value: Symbol("x") }));
  assert.throws(() => api.canonicalize({ value() {} }));
});

test("canonical JSON fails closed on holes, accessors, hidden values, cycles, and invalid UTF-8", () => {
  const sparse = [];
  sparse.length = 1;
  assert.throws(() => api.canonicalize(sparse), /sparse|hole/iu);
  let getterCalled = false;
  const accessor = {};
  Object.defineProperty(accessor, "secret", {
    enumerable: true,
    get() {
      getterCalled = true;
      return "value";
    },
  });
  assert.throws(() => api.canonicalize(accessor), /accessor/iu);
  assert.equal(getterCalled, false);
  const hidden = {};
  Object.defineProperty(hidden, "hidden", { enumerable: false, value: true });
  assert.throws(() => api.canonicalize(hidden), /hidden/iu);
  const cycle = {};
  cycle.self = cycle;
  assert.throws(() => api.canonicalize(cycle), /cyclic/iu);
  assert.throws(
    () =>
      api.parseCanonicalJson(
        Buffer.from([
          0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xc3, 0x28, 0x22, 0x7d,
        ])
      ),
    /encoded data|UTF-8/iu
  );
});

test("SHA-256 hashes exact bytes rather than normalized text", () => {
  assert.equal(
    api.sha256(Buffer.from("abc", "utf8")),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  );
  assert.notEqual(
    api.sha256(Buffer.from("one\ntwo\n", "utf8")),
    api.sha256(Buffer.from("one\r\ntwo\r\n", "utf8"))
  );
  assert.notEqual(
    api.sha256(Buffer.from("last", "utf8")),
    api.sha256(Buffer.from("last\n", "utf8"))
  );
});

test("risk taxonomy normalization and unresolved-summary semantics are explicit", () => {
  for (const source of [
    "critical",
    "critical_data_loss",
    "critical_data_identity",
    "critical_to_cutover",
  ])
    assert.equal(api.normalizeRiskSeverity(source), "critical", source);
  assert.equal(api.normalizeRiskSeverity("high"), "high");
  assert.equal(api.normalizeRiskSeverity("unknown"), null);
  assert.equal(api.normalizeRiskStatus("blocked_on_decision"), "blocked");
  assert.equal(api.normalizeRiskStatus("accepted"), "accepted_risk");
  for (const status of ["open", "mitigating", "blocked", "accepted_risk"])
    assert.equal(api.isUnresolvedRiskStatus(status), true, status);
  for (const status of ["closed"])
    assert.equal(api.isUnresolvedRiskStatus(status), false, status);
});

test("audit Markdown is deterministic and covers escaped RFC 6901 leaves", () => {
  const artifact = {
    schema_version: "1.0",
    audit_id: "R0.2-E-AUDIT-NORMALIZATION-119",
    specimen: {
      "tilde~key": "first",
      "slash/key": ["second", { value: 3 }],
    },
  };
  const first = api.renderAuditMarkdown(artifact);
  const second = api.renderAuditMarkdown(structuredClone(artifact));
  assert.deepEqual(bytesOf(first.bytes), bytesOf(second.bytes));
  assert.deepEqual(coveredPointers(first), coveredPointers(second));
  assert.deepEqual(coveredPointers(first), [
    "/audit_id",
    "/schema_version",
    "/specimen/slash~1key/0",
    "/specimen/slash~1key/1/value",
    "/specimen/tilde~0key",
  ]);
  const markdown = bytesOf(first.bytes).toString("utf8");
  assert.match(markdown, /generated/i);
  assert.match(markdown, /JSON Pointer/u);
  for (const pointer of coveredPointers(first)) {
    assert.equal(
      markdown.split(`| ${JSON.stringify(pointer)} |`).length - 1,
      1,
      `pointer ${pointer} must occur exactly once in the appendix`
    );
  }
});

test("schema validator and semantic audit validator classify structural errors", async () => {
  const malformed = { schema_version: "1.0" };
  const structural = await api.validateSchema(
    "https://synara.invalid/schemas/matrix-rust-sdk/feature-parity-audit-normalization.schema.json",
    malformed
  );
  assert.equal(resultValid(structural), false);
  assert.ok(normalizedDiagnostics(structural).length > 0);
  const semantic = await api.validateAudit(malformed, {
    gitObjects: null,
    repoRoot: REPOSITORY_ROOT,
    skipGitEvidence: true,
  });
  assert.equal(resultValid(semantic), false);
  assert.ok(normalizedDiagnostics(semantic).length > 0);
});

test("production-shaped 119-row audit passes schema and non-Git semantic checks", async (t) => {
  const artifact = productionShapedAuditFixture();
  assert.deepEqual(api.validateSchema("audit", artifact), []);
  assert.deepEqual(
    await api.validateAudit(artifact, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
    }),
    []
  );

  const v1 = api.parseCanonicalJson(
    git(
      REPOSITORY_ROOT,
      ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`],
      { encoding: null }
    )
  );
  const sourceIdentity = {
    commit_sha: api.AUDITED_SOURCE_COMMIT,
    blob_oid: api.V1_BLOB,
    file_sha256: api.V1_SHA256,
  };
  await t.test(
    "source audit keeps required tests baseline-planned",
    async () => {
      const audit = productionShapedAuditFixture();
      const auditSchema = api.parseCanonicalJson(
        await readFile(path.join(REPOSITORY_ROOT, api.AUDIT_SCHEMA_PATH))
      );
      const v2Schema = api.parseCanonicalJson(
        await readFile(path.join(REPOSITORY_ROOT, api.V2_SCHEMA_PATH))
      );
      assert.deepEqual(auditSchema.$defs.requiredTest.properties.status, {
        const: "planned",
      });
      assert.equal(
        Object.hasOwn(
          auditSchema.$defs.requiredTest.properties,
          "execution_contract"
        ),
        false
      );
      assert.equal(
        Object.hasOwn(auditSchema.$defs, "executionContract"),
        false
      );
      assert.equal(Object.hasOwn(v2Schema.$defs, "executionContract"), true);
      const auditIdentity = syntheticAuditIdentity(audit);
      const migrated = api.migrateV1({
        v1,
        audit,
        sourceIdentity,
        auditIdentity,
      });
      assert.deepEqual(api.validateSchema("audit", audit), []);
      assert.deepEqual(api.validateSchema("v2", migrated), []);
      assert.equal(
        migrated.requirements.every((row) =>
          row.clauses.every((clause) =>
            clause.required_tests.every(
              (requiredTest) =>
                requiredTest.status === "planned" &&
                !Object.hasOwn(requiredTest, "execution_contract")
            )
          )
        ),
        true
      );
    }
  );
  for (const scenario of [
    {
      name: "source audit rejects non-planned required test",
      mutate(requiredTest) {
        requiredTest.status = "validation_pending";
        requiredTest.execution_contract = {
          kind: "command",
          runner_id: "NODE_TEST",
          invocation_id: `INV-${requiredTest.id.slice(4)}-001`,
          invocation_fingerprint_sha256: api.sha256(
            Buffer.from("source-audit-must-not-carry-contract")
          ),
          assertion_ids: [`ASSERT-${requiredTest.id.slice(4)}-001`],
        };
      },
    },
    {
      name: "source audit rejects planned test execution contract",
      mutate(requiredTest) {
        requiredTest.execution_contract = {
          kind: "command",
          runner_id: "NODE_TEST",
          invocation_id: `INV-${requiredTest.id.slice(4)}-001`,
          invocation_fingerprint_sha256: api.sha256(
            Buffer.from("source-audit-must-not-carry-contract")
          ),
          assertion_ids: [`ASSERT-${requiredTest.id.slice(4)}-001`],
        };
      },
    },
  ])
    await t.test(scenario.name, async () => {
      const audit = productionShapedAuditFixture();
      scenario.mutate(audit.rows[0].clauses[0].required_tests[0]);
      refreshAuditDigests(audit);
      assert.ok(api.validateSchema("audit", audit).length > 0);
      const auditErrors = await api.validateAudit(audit, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
      });
      assert.ok(
        auditErrors.some(({ code }) => code === "AUDIT_REQUIRED_TEST_CONTRACT"),
        JSON.stringify(auditErrors)
      );
      assert.throws(
        () =>
          api.migrateV1({
            v1,
            audit,
            sourceIdentity,
            auditIdentity: syntheticAuditIdentity(audit),
          }),
        /planned, authority-free source-audit baseline/u
      );
    });

  await t.test(
    "duplicate required-test IDs are rejected before and after migration",
    async () => {
      const audit = productionShapedAuditFixture();
      audit.rows[1].clauses[0].required_tests[0].id =
        audit.rows[0].clauses[0].required_tests[0].id;
      refreshAuditDigests(audit);
      assert.deepEqual(api.validateSchema("audit", audit), []);
      const auditErrors = await api.validateAudit(audit, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
      });
      assert.ok(
        auditErrors.some(
          ({ code, pointer }) =>
            code === "AUDIT_REQUIRED_TEST_IDS_GLOBAL" && pointer === "/rows"
        ),
        JSON.stringify(auditErrors)
      );
      assert.throws(
        () =>
          api.migrateV1({
            v1,
            audit,
            sourceIdentity,
            auditIdentity: syntheticAuditIdentity(audit),
          }),
        /planned, authority-free source-audit baseline/u
      );
    }
  );

  for (const scenario of [
    {
      name: "accepted audit state with report reference",
      auditCode: "AUDIT_ACCEPTED_STATE_AUTHORITY",
      v2Code: "V2_AUDIT_REPORT_REFERENCE",
      mutate(audit) {
        Object.assign(audit.rows[0].audit, {
          state: "accepted",
          reviewer: "Independent reviewer",
          reviewed_at: "2026-07-26T12:00:00.000Z",
          report_ids: ["AUDIT-MISSING"],
        });
      },
    },
    {
      name: "non-accepted row audit-report reference",
      auditCode: "AUDIT_REPORT_REFERENCE_AUTHORITY",
      v2Code: "V2_AUDIT_REPORT_REFERENCE",
      mutate(audit) {
        audit.rows[0].audit.report_ids = ["AUDIT-MISSING"];
      },
    },
    {
      name: "row validation-report reference",
      auditCode: "AUDIT_VALIDATION_REFERENCE_AUTHORITY",
      v2Code: "V2_VALIDATION_REPORT_REFERENCE",
      mutate(audit) {
        audit.rows[0].rust_cutover.validation_report_ids = [
          "VALIDATION-MISSING",
        ];
      },
    },
    {
      name: "clause validation-report reference",
      auditCode: "AUDIT_VALIDATION_REFERENCE_AUTHORITY",
      v2Code: "V2_VALIDATION_REPORT_REFERENCE",
      mutate(audit) {
        audit.rows[0].clauses[0].rust_mapping.validation_report_ids = [
          "VALIDATION-MISSING",
        ];
      },
    },
    {
      name: "unverified implementation subject",
      auditCode: "AUDIT_IMPLEMENTATION_SUBJECT_AUTHORITY",
      v2Code: "V2_GIT_EVIDENCE_SKIPPED",
      mutate(audit) {
        audit.rows[0].rust_cutover.implementation_subject_sha = "f".repeat(40);
      },
    },
  ])
    await t.test(scenario.name, async () => {
      const audit = productionShapedAuditFixture();
      scenario.mutate(audit);
      refreshAuditDigests(audit);
      assert.deepEqual(api.validateSchema("audit", audit), []);
      const auditErrors = await api.validateAudit(audit, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
      });
      assert.ok(
        auditErrors.some(({ code }) => code === scenario.auditCode),
        JSON.stringify(auditErrors)
      );
      assert.throws(
        () =>
          api.migrateV1({
            v1,
            audit,
            sourceIdentity,
            auditIdentity: syntheticAuditIdentity(audit),
          }),
        /planned, authority-free source-audit baseline/u
      );
    });
});

test("audit pins the complete plan and ordered source-input provenance", async () => {
  const cases = [
    {
      code: "AUDIT_PLAN_IDENTITY",
      mutate(audit) {
        audit.subject.plan.blob_oid = "9".repeat(40);
      },
    },
    {
      code: "AUDIT_SOURCE_INPUT",
      mutate(audit) {
        [
          audit.source_inputs[0].section_range,
          audit.source_inputs[1].section_range,
        ] = [
          audit.source_inputs[1].section_range,
          audit.source_inputs[0].section_range,
        ];
      },
    },
    {
      code: "AUDIT_SOURCE_INPUT",
      mutate(audit) {
        audit.source_inputs[0].adapter_version = "attacker-adapter";
      },
    },
    {
      code: "AUDIT_SOURCE_INPUT",
      mutate(audit) {
        audit.source_inputs[0].ingestion_notes = ["Unpinned scratch input."];
      },
    },
    {
      code: "AUDIT_SOURCE_INPUT",
      mutate(audit) {
        [audit.source_inputs[0], audit.source_inputs[1]] = [
          audit.source_inputs[1],
          audit.source_inputs[0],
        ];
      },
    },
  ];
  for (const scenario of cases) {
    const audit = productionShapedAuditFixture();
    scenario.mutate(audit);
    refreshAuditDigests(audit);
    const errors = await api.validateAudit(audit, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
    });
    assert.ok(
      errors.some(({ code }) => code === scenario.code),
      JSON.stringify({ scenario: scenario.code, errors })
    );
  }
});

test("pinned v1 migration produces a schema-valid and semantically valid v2 artifact", async () => {
  const v1 = api.parseCanonicalJson(
    git(
      REPOSITORY_ROOT,
      ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`],
      { encoding: null }
    )
  );
  const audit = productionShapedAuditFixture();
  const auditIdentity = syntheticAuditIdentity(audit);
  const migrated = api.migrateV1({
    v1,
    audit,
    sourceIdentity: {
      commit_sha: api.AUDITED_SOURCE_COMMIT,
      blob_oid: api.V1_BLOB,
      file_sha256: api.V1_SHA256,
    },
    auditIdentity,
  });
  assert.equal(
    migrated.coverage_contract
      .sections_7_3_through_7_7_status_correction_subtotal,
    11
  );
  assert.equal(
    migrated.coverage_contract
      .sections_7_3_through_7_7_identified_correction_count,
    16
  );
  assert.deepEqual(api.validateSchema("v2", migrated), []);
  assert.deepEqual(
    await api.validateTraceability(migrated, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
      durableAuditIdentity: auditIdentity,
      durableAudit: audit,
    }),
    []
  );

  const acceptedRisk = structuredClone(migrated);
  acceptedRisk.blockers.push({
    ...structuredClone(acceptedRisk.blockers[0]),
    id: "MRSDK-R999",
    severity: "low",
    status: "accepted_risk",
    owner_task_ids: ["R0.2-E"],
    threat_ids: [],
    boundary_ids: [],
    closure_criteria: ["Risk acceptance is explicitly recorded."],
  });
  refreshV2Summary(acceptedRisk);
  assert.equal(
    acceptedRisk.summary.open_blocker_count,
    migrated.summary.open_blocker_count + 1,
    "accepted risk remains unresolved and increments the blocking count"
  );
  assert.equal(
    (
      await api.validateTraceability(acceptedRisk, {
        repoRoot: REPOSITORY_ROOT,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_SUMMARY"),
    false
  );

  const forgedV2Subtotal = structuredClone(migrated);
  forgedV2Subtotal.coverage_contract.sections_7_3_through_7_7_status_correction_subtotal = 16;
  assert.ok(
    (
      await api.validateTraceability(forgedV2Subtotal, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(
      ({ code, pointer }) =>
        code === "V2_COVERAGE" && pointer === "/coverage_contract"
    )
  );

  const missingIdentity = await api.validateTraceability(migrated, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
    durableAudit: audit,
  });
  assert.ok(missingIdentity.some(({ code }) => code === "V2_AUDIT_IDENTITY"));
  const missingAudit = await api.validateTraceability(migrated, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
    durableAuditIdentity: auditIdentity,
  });
  assert.ok(missingAudit.some(({ code }) => code === "V2_AUDIT_BINDING"));

  const mismatchedIdentity = {
    ...auditIdentity,
    canonical_semantic_sha256: "9".repeat(64),
  };
  const mismatchedIdentityResult = await api.validateTraceability(migrated, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
    durableAuditIdentity: mismatchedIdentity,
    durableAudit: audit,
  });
  assert.ok(
    mismatchedIdentityResult.some(({ code }) => code === "V2_AUDIT_BINDING")
  );

  const staleRequirement = structuredClone(migrated);
  staleRequirement.requirements[0].plan.normalized_text += " stale";
  const staleRequirementResult = await api.validateTraceability(
    staleRequirement,
    {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
      durableAuditIdentity: auditIdentity,
      durableAudit: audit,
    }
  );
  assert.equal(
    staleRequirementResult.some(({ code }) => code === "V2_ROW_DIGEST"),
    false
  );
  assert.ok(
    staleRequirementResult.some(({ code }) => code === "V2_AUDIT_ROW_BINDING")
  );

  const recomputedAttack = structuredClone(staleRequirement);
  recomputedAttack.requirements[0].audit.audited_payload_sha256 =
    api.auditedRowDigest(recomputedAttack.requirements[0]);
  const recomputedAttackResult = await api.validateTraceability(
    recomputedAttack,
    {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
      durableAuditIdentity: auditIdentity,
      durableAudit: audit,
    }
  );
  assert.equal(
    recomputedAttackResult.some(({ code }) => code === "V2_ROW_DIGEST"),
    true
  );
  assert.ok(
    recomputedAttackResult.some(({ code }) => code === "V2_AUDIT_ROW_BINDING")
  );

  const staleSummary = structuredClone(migrated);
  staleSummary.summary.open_blocker_count += 1;
  assert.ok(
    (
      await api.validateTraceability(staleSummary, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_SUMMARY")
  );

  const forgedProvenance = structuredClone(migrated);
  forgedProvenance.provenance.plan.blob_oid = "9".repeat(40);
  assert.ok(
    (
      await api.validateTraceability(forgedProvenance, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_PROVENANCE")
  );

  const forgedRiskProvenance = structuredClone(migrated);
  forgedRiskProvenance.provenance.risk_register.canonical_sha256 = "0".repeat(
    64
  );
  assert.ok(
    (
      await api.validateTraceability(forgedRiskProvenance, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_PROVENANCE")
  );

  const forgedLedger = structuredClone(migrated);
  forgedLedger.cx_extras_note += " altered";
  const forgedDigest = api.sha256(
    api.canonicalize(forgedLedger.cx_extras_note)
  );
  const digestRecord =
    forgedLedger.migration_provenance.preserved_ledger_digests.find(
      ({ key }) => key === "cx_extras_note"
    );
  digestRecord.pre_migration_canonical_sha256 = forgedDigest;
  digestRecord.post_migration_canonical_sha256 = forgedDigest;
  assert.ok(
    (
      await api.validateTraceability(forgedLedger, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_LEDGER_DIGEST")
  );
});

test("Rust ready and accepted audit states require exact durable Git reports", async (t) => {
  const audit = productionShapedAuditFixture();
  const sourceRow = audit.rows[0];
  const sourceClause = sourceRow.clauses[0];
  sourceClause.required_tests[0].evidence_class = "integration";
  sourceClause.required_tests.push({
    id: `TST-${sourceRow.id}-REQ-002`,
    evidence_class: "integration",
    task_ids: ["R0.2-E"],
    acceptance:
      "Validate the ready subject through the closed scenario harness.",
    status: "planned",
  });
  const unrelatedScopeRow = audit.rows[1];
  for (const blocker of audit.blockers_and_risks) {
    if (blocker.affected_requirement_ids.includes(sourceRow.id)) {
      blocker.affected_requirement_ids = [unrelatedScopeRow.id];
      blocker.affected_clause_ids = [unrelatedScopeRow.clauses[0].id];
    }
  }
  for (const decision of audit.architecture_decisions) {
    if (decision.affected_requirement_ids.includes(sourceRow.id)) {
      decision.affected_requirement_ids = [unrelatedScopeRow.id];
      decision.affected_clause_ids = [unrelatedScopeRow.clauses[0].id];
    }
  }
  refreshAuditDigests(audit);
  const { root } = await productionCliRepository(t, audit);
  const v1 = api.parseCanonicalJson(
    git(root, ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`], {
      encoding: null,
    })
  );
  const implementationSubject = git(root, ["rev-parse", "HEAD"]);
  const auditIdentity = api.deriveDurableAuditIdentity(root);
  const value = api.migrateV1({
    v1,
    audit,
    sourceIdentity: {
      commit_sha: api.AUDITED_SOURCE_COMMIT,
      blob_oid: api.V1_BLOB,
      file_sha256: api.V1_SHA256,
    },
    auditIdentity,
  });
  const auditedRow = value.requirements[0];
  const clause = auditedRow.clauses[0];
  const baselineState = api.cutoverStateProjection(auditedRow);
  const sourceEntitySha256 = api.sha256(
    api.canonicalize(api.sourceAuditRowProjection(auditedRow))
  );
  const auditReportId = "AUDIT-READY-001";
  const validationReportId = "VALIDATION-READY-001";
  auditedRow.audit.state = "accepted";
  auditedRow.audit.reviewer = "Independent reviewer";
  auditedRow.audit.reviewed_at = "2026-07-26T12:00:00.000Z";
  auditedRow.audit.report_ids = [auditReportId];
  clause.rust_mapping.validation_report_ids = [validationReportId];
  const commandInvocationFingerprint = api.sha256(
    Buffer.from("cargo test --locked matrix::ready_subject")
  );
  clause.required_tests[0].status = "accepted";
  clause.required_tests[0].execution_contract = {
    kind: "command",
    runner_id: "CARGO_TEST",
    invocation_id: `INV-${clause.required_tests[0].id.slice(4)}-001`,
    invocation_fingerprint_sha256: commandInvocationFingerprint,
    assertion_ids: [`ASSERT-${clause.required_tests[0].id.slice(4)}-001`],
  };
  const scenarioTest = clause.required_tests[1];
  scenarioTest.status = "accepted";
  scenarioTest.execution_contract = {
    kind: "scenario",
    runner_id: "PLATFORM_SCENARIO",
    invocation_id: `INV-${auditedRow.id}-REQ-002-001`,
    invocation_fingerprint_sha256: api.sha256(
      Buffer.from("scenario:ready-subject")
    ),
    assertion_ids: [`ASSERT-${auditedRow.id}-REQ-002-001`],
  };
  auditedRow.rust_cutover = {
    ...auditedRow.rust_cutover,
    readiness: "ready",
    implementation_subject_sha: implementationSubject,
    matrix_owner: "matrix_rust_sdk",
    surviving_matrix_js_owner: false,
    gap_ids: [],
    gate_ids: [],
    blocker_ids: [],
    validation_report_ids: [validationReportId],
    qualification: "Ready only with exact accepted evidence.",
  };
  const readyState = api.cutoverStateProjection(auditedRow);
  const plannedState = structuredClone(readyState);
  plannedState.rust_cutover.readiness = "implementation_planned";
  for (const requiredTestGroup of plannedState.required_tests)
    for (const requiredTest of requiredTestGroup.tests) {
      requiredTest.status = "planned";
      requiredTest.execution_contract = null;
    }
  const pendingState = structuredClone(readyState);
  pendingState.rust_cutover.readiness = "validation_pending";
  for (const requiredTestGroup of pendingState.required_tests)
    for (const requiredTest of requiredTestGroup.tests)
      requiredTest.status = "validation_pending";
  const auditPath = "docs/matrix-rust-sdk/reviews/ready-audit.json";
  const validationPath = "docs/matrix-rust-sdk/validation/ready.json";
  const executionPath =
    "docs/matrix-rust-sdk/validation/executions/EXEC-000001.json";
  const scenarioExecutionPath =
    "docs/matrix-rust-sdk/validation/executions/EXEC-000002.json";
  const auditPayload = {
    id: auditReportId,
    subject_sha: api.AUDITED_SOURCE_COMMIT,
    artifact_payload_sha256: audit.digests.canonical_payload_sha256,
    covered_requirement_ids: [auditedRow.id],
    covered_clause_ids: [clause.id],
    verdict: "accept",
    reviewer: "Independent reviewer",
    reviewed_at: "2026-07-26T12:00:00.000Z",
  };
  const executionPayload = {
    schema_version: "1.0",
    id: "EXEC-000001",
    kind: "command",
    test_case_id: clause.required_tests[0].id,
    runner_id: "CARGO_TEST",
    invocation_id: clause.required_tests[0].execution_contract.invocation_id,
    invocation_fingerprint_sha256: commandInvocationFingerprint,
    subject_sha: implementationSubject,
    started_at: "2026-07-26T12:30:00.000Z",
    finished_at: "2026-07-26T12:31:00.000Z",
    result: "pass",
    exit_code: 0,
    assertions: [
      {
        assertion_id:
          clause.required_tests[0].execution_contract.assertion_ids[0],
        result: "pass",
      },
    ],
  };
  executionPayload.result_fingerprint_sha256 =
    api.executionResultFingerprint(executionPayload);
  const executionBytes = Buffer.from(api.prettyJson(executionPayload));
  await mkdir(path.dirname(path.join(root, executionPath)), {
    recursive: true,
  });
  await writeFile(path.join(root, executionPath), executionBytes);
  git(root, ["add", executionPath]);
  git(root, ["commit", "--quiet", "-m", "durable execution transcript"]);
  const executionStorageCommit = git(root, ["rev-parse", "HEAD"]);
  const executionRecord = {
    ...executionPayload,
    storage_commit_sha: executionStorageCommit,
    storage_blob_oid: git(root, [
      "rev-parse",
      `${executionStorageCommit}:${executionPath}`,
    ]),
    storage_projection: "execution-record-minus-storage-identity-v1",
    path: executionPath,
    file_sha256: api.sha256(executionBytes),
    canonical_sha256: api.sha256(api.canonicalize(executionPayload)),
  };
  const scenarioExecutionPayload = {
    schema_version: "1.0",
    id: "EXEC-000002",
    kind: "scenario",
    test_case_id: scenarioTest.id,
    runner_id: "PLATFORM_SCENARIO",
    invocation_id: scenarioTest.execution_contract.invocation_id,
    invocation_fingerprint_sha256:
      scenarioTest.execution_contract.invocation_fingerprint_sha256,
    subject_sha: implementationSubject,
    started_at: "2026-07-26T12:32:00.000Z",
    finished_at: "2026-07-26T12:33:00.000Z",
    result: "pass",
    exit_code: null,
    assertions: [
      {
        assertion_id: scenarioTest.execution_contract.assertion_ids[0],
        result: "pass",
      },
    ],
  };
  scenarioExecutionPayload.result_fingerprint_sha256 =
    api.executionResultFingerprint(scenarioExecutionPayload);
  const scenarioExecutionBytes = Buffer.from(
    api.prettyJson(scenarioExecutionPayload)
  );
  await writeFile(
    path.join(root, scenarioExecutionPath),
    scenarioExecutionBytes
  );
  git(root, ["add", scenarioExecutionPath]);
  git(root, ["commit", "--quiet", "-m", "durable scenario transcript"]);
  const scenarioExecutionStorageCommit = git(root, ["rev-parse", "HEAD"]);
  const scenarioExecutionRecord = {
    ...scenarioExecutionPayload,
    storage_commit_sha: scenarioExecutionStorageCommit,
    storage_blob_oid: git(root, [
      "rev-parse",
      `${scenarioExecutionStorageCommit}:${scenarioExecutionPath}`,
    ]),
    storage_projection: "execution-record-minus-storage-identity-v1",
    path: scenarioExecutionPath,
    file_sha256: api.sha256(scenarioExecutionBytes),
    canonical_sha256: api.sha256(api.canonicalize(scenarioExecutionPayload)),
  };
  const unrelatedExecutionPath =
    "docs/matrix-rust-sdk/validation/executions/EXEC-999998.json";
  const unrelatedExecutionPayload = { status: "pass" };
  const unrelatedExecutionBytes = Buffer.from(
    api.prettyJson(unrelatedExecutionPayload)
  );
  await writeFile(
    path.join(root, unrelatedExecutionPath),
    unrelatedExecutionBytes
  );
  git(root, ["add", unrelatedExecutionPath]);
  git(root, ["commit", "--quiet", "-m", "unrelated passing JSON fixture"]);
  const unrelatedExecutionStorageCommit = git(root, ["rev-parse", "HEAD"]);
  const validationPayload = {
    id: validationReportId,
    subject_sha: implementationSubject,
    evidence_class: "integration",
    status: "pass",
    reviewer: "Independent validator",
    reviewed_at: "2026-07-26T12:45:00.000Z",
    accepted_at: "2026-07-26T13:00:00.000Z",
    covered_requirement_ids: [auditedRow.id],
    covered_clause_ids: [clause.id],
    executions: [executionRecord, scenarioExecutionRecord],
    test_contract_snapshots: clause.required_tests.map((requiredTest) => ({
      test_id: requiredTest.id,
      requirement_id: auditedRow.id,
      clause_id: clause.id,
      evidence_class: requiredTest.evidence_class,
      execution_contract: structuredClone(requiredTest.execution_contract),
    })),
    lifecycle_authorizations: [],
    limitations: [],
  };
  const firstAuthorization = lifecycleAuthorization({
    id: "AUTH-READY-001",
    entityId: auditedRow.id,
    sequence: 1,
    sourceEntitySha256,
    fromPayloadSha256: api.sha256(api.canonicalize(baselineState)),
    toState: plannedState,
    reportId: validationReportId,
    subjectSha: implementationSubject,
    reviewer: validationPayload.reviewer,
    reviewedAt: validationPayload.reviewed_at,
    rationale:
      "Implementation planning and exact test contracts were reviewed.",
  });
  const secondAuthorization = lifecycleAuthorization({
    id: "AUTH-READY-002",
    entityId: auditedRow.id,
    sequence: 2,
    sourceEntitySha256,
    fromPayloadSha256: firstAuthorization.to_payload_sha256,
    toState: pendingState,
    previousAuthorization: firstAuthorization,
    reportId: validationReportId,
    subjectSha: implementationSubject,
    reviewer: validationPayload.reviewer,
    reviewedAt: validationPayload.reviewed_at,
    rationale:
      "Validation-pending state was reviewed against the exact contracts.",
  });
  const thirdAuthorization = lifecycleAuthorization({
    id: "AUTH-READY-003",
    entityId: auditedRow.id,
    sequence: 3,
    sourceEntitySha256,
    fromPayloadSha256: secondAuthorization.to_payload_sha256,
    toState: readyState,
    previousAuthorization: secondAuthorization,
    reportId: validationReportId,
    subjectSha: implementationSubject,
    reviewer: validationPayload.reviewer,
    reviewedAt: validationPayload.reviewed_at,
    rationale: "Ready state was reviewed after both exact executions passed.",
  });
  validationPayload.lifecycle_authorizations.push(
    firstAuthorization,
    secondAuthorization,
    thirdAuthorization
  );
  await mkdir(path.dirname(path.join(root, auditPath)), { recursive: true });
  await mkdir(path.dirname(path.join(root, validationPath)), {
    recursive: true,
  });
  const auditBytes = Buffer.from(api.prettyJson(auditPayload));
  const validationBytes = Buffer.from(api.prettyJson(validationPayload));
  await writeFile(path.join(root, auditPath), auditBytes);
  await writeFile(path.join(root, validationPath), validationBytes);
  git(root, ["add", auditPath, validationPath]);
  git(root, ["commit", "--quiet", "-m", "durable validation reports"]);
  const storageCommit = git(root, ["rev-parse", "HEAD"]);
  const storageTree = git(root, ["rev-parse", `${storageCommit}^{tree}`]);
  const unrelatedStorageCommit = git(root, [
    "commit-tree",
    storageTree,
    "-m",
    "unrelated durable report history",
  ]);
  const storageIdentity = (reportPath, bytes) => ({
    storage_commit_sha: storageCommit,
    storage_blob_oid: git(root, [
      "rev-parse",
      `${storageCommit}:${reportPath}`,
    ]),
    storage_projection: "report-record-minus-storage-identity-v1",
    path: reportPath,
    file_sha256: api.sha256(bytes),
  });
  value.audit_reports = [
    { ...auditPayload, ...storageIdentity(auditPath, auditBytes) },
  ];
  value.validation_reports = [
    {
      ...validationPayload,
      ...storageIdentity(validationPath, validationBytes),
    },
  ];
  refreshV2Summary(value);
  refreshLifecycle(value, audit);
  assert.deepEqual(api.validateSchema("v2", value), []);
  const v2Bytes = Buffer.from(api.prettyJson(value));
  const v2Markdown = api.renderTraceabilityMarkdown(value).bytes;
  await writeFile(path.join(root, api.V1_PATH), v2Bytes);
  await writeFile(path.join(root, api.V1_MARKDOWN_PATH), v2Markdown);
  git(root, ["add", api.V1_PATH, api.V1_MARKDOWN_PATH]);
  git(root, ["commit", "--quiet", "-m", "evolved durable v2 ready state"]);
  let checkV2Stdout = "";
  let checkV2Stderr = "";
  assert.equal(
    await api.runCli([], {
      kind: "check-v2",
      repositoryRoot: root,
      stdout: {
        write(chunk) {
          checkV2Stdout += chunk;
        },
      },
      stderr: {
        write(chunk) {
          checkV2Stderr += chunk;
        },
      },
    }),
    0,
    `production-equivalent check-v2 must accept the evolved durable state\n${checkV2Stdout}${checkV2Stderr}`
  );
  let migrationCheckStdout = "";
  let migrationCheckStderr = "";
  assert.equal(
    await api.runCli(["--check"], {
      kind: "migrate",
      repositoryRoot: root,
      stdout: {
        write(value) {
          migrationCheckStdout += value;
        },
      },
      stderr: {
        write(value) {
          migrationCheckStderr += value;
        },
      },
    }),
    1,
    "migrate --check must remain a baseline-migration comparison and reject an evolved v2 artifact"
  );
  assert.equal(migrationCheckStdout, "");
  assert.match(migrationCheckStderr, /differs from deterministic migration/u);
  await assert.rejects(
    api.validateTraceability(value, {
      repoRoot: REPOSITORY_ROOT,
      durableAuditIdentity: auditIdentity,
      durableAudit: audit,
    }),
    /Git object adapter is required/u
  );
  assert.ok(
    (
      await api.validateTraceability(value, {
        repoRoot: REPOSITORY_ROOT,
        skipGitEvidence: true,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      })
    ).some(({ code }) => code === "V2_GIT_EVIDENCE_SKIPPED")
  );

  const reader = new api.GitObjectReader(root);
  const validate = (candidate) =>
    api.validateTraceability(candidate, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: reader,
      durableAuditIdentity: auditIdentity,
      durableAudit: audit,
    });
  const persistValidationReport = async (candidate, label) => {
    const payload = api.durableReportProjection(
      candidate.validation_reports[0]
    );
    const bytes = Buffer.from(api.prettyJson(payload));
    await writeFile(path.join(root, validationPath), bytes);
    git(root, ["add", validationPath]);
    git(root, ["commit", "--quiet", "-m", `validation timing: ${label}`]);
    const commitSha = git(root, ["rev-parse", "HEAD"]);
    candidate.validation_reports[0] = {
      ...payload,
      storage_commit_sha: commitSha,
      storage_blob_oid: git(root, [
        "rev-parse",
        `${commitSha}:${validationPath}`,
      ]),
      storage_projection: "report-record-minus-storage-identity-v1",
      path: validationPath,
      file_sha256: api.sha256(bytes),
    };
    return candidate;
  };
  try {
    assert.deepEqual(
      api.executionTranscriptPrivacyViolations(executionPayload),
      [],
      "allowlisted identifiers and a private invocation fingerprint are safe"
    );
    const matrixApiPathArtifact = structuredClone(value);
    matrixApiPathArtifact.requirements[0].clauses[0].text =
      "The Matrix API route /_matrix/client/v3/sync remains valid audited prose.";
    assert.deepEqual(
      api.validateSchema("v2", matrixApiPathArtifact),
      [],
      "the transcript-only privacy contract must not reject Matrix API paths in broader audit prose"
    );
    assert.deepEqual(await validate(value), []);
    assert.ok(
      clause.rust_mapping.gap_ids.length > 0 &&
        clause.rust_mapping.gate_ids.length > 0,
      "Rust-ready state retains immutable audited clause gap/gate history"
    );

    const durableAttestationMutations = [
      {
        name: "wrong authorization review time",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations[0].reviewed_at =
            "2026-07-26T12:44:00.000Z";
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_BINDING"],
      },
      {
        name: "missing report review time with authorization",
        mutate(candidate) {
          delete candidate.validation_reports[0].reviewed_at;
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_ATTESTATION"],
      },
      {
        name: "null report review time with authorization",
        mutate(candidate) {
          candidate.validation_reports[0].reviewed_at = null;
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_ATTESTATION"],
      },
      {
        name: "execution finishes after report review",
        mutate(candidate) {
          candidate.validation_reports[0].reviewed_at =
            "2026-07-26T12:32:30.000Z";
          for (const authorization of candidate.validation_reports[0]
            .lifecycle_authorizations)
            authorization.reviewed_at = "2026-07-26T12:32:30.000Z";
          rehashLifecycleAuthorizations(candidate.validation_reports[0]);
        },
        codes: ["V2_EXECUTION_REVIEW_ORDER"],
      },
      {
        name: "passing report accepted before review",
        mutate(candidate) {
          candidate.validation_reports[0].accepted_at =
            "2026-07-26T12:44:00.000Z";
        },
        codes: ["V2_VALIDATION_ACCEPTANCE_ORDER"],
      },
    ];
    for (const scenario of durableAttestationMutations) {
      const candidate = structuredClone(value);
      scenario.mutate(candidate);
      await persistValidationReport(candidate, scenario.name);
      const errors = await validate(candidate);
      for (const code of scenario.codes)
        assert.ok(
          errors.some((entry) => entry.code === code),
          `${
            scenario.name
          }: missing ${code} in durable Git chain: ${JSON.stringify(errors)}`
        );
    }

    const immutableProjectionMutations = [
      ["current product", (row) => (row.current_product.summary += " changed")],
      [
        "status correction",
        (row) =>
          (row.current_product.status_correction.explanation += " changed"),
      ],
      ["plan", (row) => (row.plan.normalized_text += " changed")],
      [
        "source evidence",
        (row) => (row.clauses[0].source_evidence[0].explanation += " changed"),
      ],
      [
        "absence evidence",
        (row) =>
          row.clauses[0].absence_evidence.push({
            id: `EV-${row.id}-C001-ABS-999`,
            source_sha: api.AUDITED_SOURCE_COMMIT,
            expression: "never-present-r0-2-e-fixture",
            mode: "literal",
            roots: ["src"],
            exclusions: [],
            expected_match_count: 0,
            explanation: "Immutable absence-evidence insertion attack.",
          }),
      ],
      [
        "causality",
        (row) => (row.clauses[0].causality.matrix_boundary += " changed"),
      ],
      [
        "existing tests",
        (row) =>
          row.clauses[0].existing_tests.push({
            id: `TST-${row.id}-EX-999`,
            path: api.AUDITED_PLAN_PATH,
            symbol: "# Matrix Rust SDK Full-Replacement Plan",
            lines: { start: 1, end: 1 },
            source_sha: api.AUDITED_SOURCE_COMMIT,
            snippet_sha256: row.clauses[0].source_evidence[0].snippet_sha256,
            assertions: ["Immutable existing-test insertion attack."],
            limitations: ["Not runtime evidence."],
          }),
      ],
      [
        "outcomes",
        (row) => row.clauses[0].outcomes.success.push("Changed outcome."),
      ],
      [
        "reachability",
        (row) => (row.clauses[0].reachability.explanation += " changed"),
      ],
      [
        "required test insertion",
        (row) =>
          row.clauses[0].required_tests.push({
            id: `TST-${row.id}-REQ-999`,
            evidence_class: "unit",
            task_ids: ["R0.2-E"],
            acceptance: "Immutable required-test insertion attack.",
            status: "planned",
          }),
      ],
      ["required test deletion", (row) => row.clauses[0].required_tests.pop()],
      [
        "required test reorder",
        (row) => row.clauses[0].required_tests.reverse(),
      ],
      [
        "required test identity",
        (row) =>
          (row.clauses[0].required_tests[0].id = `TST-${row.id}-REQ-999`),
      ],
      [
        "required test evidence class",
        (row) => (row.clauses[0].required_tests[0].evidence_class = "unit"),
      ],
      [
        "required test tasks",
        (row) => row.clauses[0].required_tests[0].task_ids.push("P3.2"),
      ],
      [
        "required test acceptance",
        (row) => (row.clauses[0].required_tests[0].acceptance += " changed"),
      ],
      [
        "clause Rust capability",
        (row) => row.clauses[0].rust_mapping.capability_ids.push("CAP-ATTACK"),
      ],
      [
        "clause Rust gap",
        (row) => row.clauses[0].rust_mapping.gap_ids.push("GAP-ATTACK"),
      ],
      [
        "clause Rust task",
        (row) => row.clauses[0].rust_mapping.task_ids.push("P99.9"),
      ],
      [
        "clause Rust gate",
        (row) => row.clauses[0].rust_mapping.gate_ids.push("GATE-ATTACK"),
      ],
      [
        "clause Rust qualification",
        (row) => (row.clauses[0].rust_mapping.qualification += " changed"),
      ],
      [
        "row Rust capability",
        (row) => row.rust_cutover.capability_ids.push("CAP-ATTACK"),
      ],
      ["row Rust task", (row) => row.rust_cutover.task_ids.push("P99.9")],
    ];
    for (const [name, mutate] of immutableProjectionMutations) {
      const candidate = structuredClone(value);
      mutate(candidate.requirements[0]);
      const errors = await validate(candidate);
      assert.ok(
        errors.some(({ code }) => code === "V2_AUDIT_ROW_BINDING"),
        `${name}: immutable projection mutation escaped: ${JSON.stringify(
          errors
        )}`
      );
    }

    const lifecycleMutations = [
      {
        name: "mutable state without authorization",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations = [];
          refreshLifecycle(candidate, audit);
        },
        codes: ["V2_LIFECYCLE_UNAUTHORIZED_STATE"],
      },
      {
        name: "wrong state digest",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations[0].to_payload_sha256 =
            "0".repeat(64);
        },
        codes: ["V2_LIFECYCLE_TO_STATE"],
      },
      {
        name: "cross-row state replay",
        mutate(candidate) {
          const authorization =
            candidate.validation_reports[0].lifecycle_authorizations[0];
          authorization.to_state = api.cutoverStateProjection(
            candidate.requirements[1]
          );
          authorization.to_payload_sha256 = api.sha256(
            api.canonicalize(authorization.to_state)
          );
          authorization.authorization_sha256 =
            api.lifecycleAuthorizationDigest(authorization);
        },
        codes: ["V2_LIFECYCLE_TO_STATE"],
      },
      {
        name: "wrong authorization row",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations[0].entity_id =
            candidate.requirements[1].id;
        },
        codes: ["V2_LIFECYCLE_TO_STATE"],
      },
      {
        name: "wrong authorization reviewer",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations[0].reviewer =
            "Different reviewer";
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_BINDING"],
      },
      {
        name: "wrong authorization report subject",
        mutate(candidate) {
          candidate.validation_reports[0].subject_sha =
            api.AUDITED_SOURCE_COMMIT;
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_BINDING"],
      },
      {
        name: "insufficient changed-clause authorization",
        mutate(candidate) {
          candidate.validation_reports[0].covered_clause_ids = [];
        },
        codes: ["V2_LIFECYCLE_ROW_SCOPE"],
      },
      {
        name: "changed clause omits authorization report reference",
        mutate(candidate) {
          candidate.requirements[0].clauses[0].rust_mapping.validation_report_ids =
            [];
          refreshLatestRequirementAuthorization(candidate, audit);
        },
        codes: ["V2_RUST_READY"],
      },
      {
        name: "duplicate row authorization replay",
        mutate(candidate) {
          candidate.validation_reports[0].lifecycle_authorizations.push(
            structuredClone(
              candidate.validation_reports[0].lifecycle_authorizations[0]
            )
          );
        },
        codes: ["V2_LIFECYCLE_AUTHORIZATION_IDS"],
      },
      {
        name: "accepted test without exact passing execution",
        mutate(candidate) {
          candidate.validation_reports[0].executions.pop();
        },
        codes: ["V2_ACCEPTED_TEST_EXECUTION", "V2_RUST_READY"],
      },
      {
        name: "ready with pending required test",
        mutate(candidate) {
          candidate.requirements[0].clauses[0].required_tests[0].status =
            "validation_pending";
          refreshLatestRequirementAuthorization(candidate, audit);
        },
        codes: ["V2_RUST_READY"],
      },
      ...["gap_ids", "gate_ids", "blocker_ids"].map((field) => ({
        name: `ready with residual row ${field}`,
        mutate(candidate) {
          candidate.requirements[0].rust_cutover[field] = [
            `RESIDUAL-${field.toUpperCase()}`,
          ];
          refreshLatestRequirementAuthorization(candidate, audit);
        },
        codes: ["V2_RUST_READY"],
      })),
      ...[
        ["matrix_owner", "matrix_js_sdk"],
        ["surviving_raw_matrix_http", true],
        ["surviving_matrix_js_owner", true],
      ].map(([field, unsafeValue]) => ({
        name: `ready with invalid ownership field ${field}`,
        mutate(candidate) {
          candidate.requirements[0].rust_cutover[field] = unsafeValue;
          refreshLatestRequirementAuthorization(candidate, audit);
        },
        codes: ["V2_RUST_READY"],
      })),
    ];
    for (const scenario of lifecycleMutations) {
      const candidate = structuredClone(value);
      scenario.mutate(candidate);
      const errors = await validate(candidate);
      for (const code of scenario.codes)
        assert.ok(
          errors.some((entry) => entry.code === code),
          `${scenario.name}: missing ${code}: ${JSON.stringify(errors)}`
        );
    }

    const acceptedRiskBlocksReady = structuredClone(value);
    const acceptedRiskBlocker = {
      id: "MRSDK-R999",
      kind: "risk",
      severity: "high",
      status: "accepted_risk",
      owner_task_ids: ["R0.2-E"],
      authority: "Explicit reviewed risk disposition authority.",
      affected_requirement_ids: [auditedRow.id],
      affected_clause_ids: [clause.id],
      threat_ids: [],
      boundary_ids: [],
      closure_criteria: ["The risk is eliminated before Rust readiness."],
      closure_evidence: [],
      qualification:
        "Accepted risk remains unresolved and blocks the affected ready row.",
    };
    acceptedRiskBlocksReady.blockers.push(acceptedRiskBlocker);
    acceptedRiskBlocksReady.validation_reports[0].lifecycle_authorizations.unshift(
      lifecycleAuthorization({
        id: "AUTH-CREATE-ACCEPTED-RISK-001",
        entityKind: "blocker",
        entityId: acceptedRiskBlocker.id,
        sequence: 1,
        operation: "create",
        sourceEntity: api.blockerSourceProjection(acceptedRiskBlocker),
        fromPayloadSha256: null,
        toState: api.blockerLifecycleProjection(acceptedRiskBlocker),
        reportId: validationReportId,
        subjectSha: implementationSubject,
        reviewer: "Independent validator",
        reviewedAt: "2026-07-26T12:45:00.000Z",
        rationale: "The accepted risk disposition is explicit and reviewed.",
      })
    );
    refreshV2Summary(acceptedRiskBlocksReady);
    refreshLifecycle(acceptedRiskBlocksReady, audit);
    assert.equal(
      acceptedRiskBlocksReady.summary.open_blocker_count,
      value.summary.open_blocker_count + 1
    );
    const acceptedRiskErrors = await validate(acceptedRiskBlocksReady);
    assert.ok(
      acceptedRiskErrors.some(({ code }) => code === "V2_RUST_READY") &&
        acceptedRiskErrors.some(
          ({ code }) => code === "V2_LIFECYCLE_HISTORICAL_READY"
        ),
      `accepted risk must block current and historical ready: ${JSON.stringify(
        acceptedRiskErrors
      )}`
    );

    const mutations = [
      {
        name: "missing validation report",
        mutate(candidate) {
          candidate.requirements[0].rust_cutover.validation_report_ids = [
            "VALIDATION-MISSING",
          ];
        },
        codes: ["V2_VALIDATION_REPORT_REFERENCE", "V2_RUST_READY"],
      },
      {
        name: "wrong validation scope",
        mutate(candidate) {
          candidate.validation_reports[0].covered_requirement_ids = [
            "FR-7.1-002",
          ];
        },
        codes: ["V2_VALIDATION_REPORT_REFERENCE", "V2_RUST_READY"],
      },
      {
        name: "nonpassing validation",
        mutate(candidate) {
          candidate.validation_reports[0].status = "fail";
          candidate.validation_reports[0].reviewer = null;
          candidate.validation_reports[0].accepted_at = null;
          refreshV2Summary(candidate);
        },
        codes: ["V2_RUST_READY"],
      },
      {
        name: "missing audit report",
        mutate(candidate) {
          candidate.requirements[0].audit.report_ids = ["AUDIT-MISSING"];
        },
        codes: [
          "V2_AUDIT_REPORT_REFERENCE",
          "V2_AUDIT_ACCEPTANCE",
          "V2_RUST_READY",
        ],
      },
      {
        name: "rejected audit report",
        mutate(candidate) {
          candidate.audit_reports[0].verdict = "reject";
          refreshV2Summary(candidate);
        },
        codes: ["V2_AUDIT_ACCEPTANCE", "V2_RUST_READY"],
      },
      {
        name: "nonexistent audit storage commit",
        mutate(candidate) {
          candidate.audit_reports[0].storage_commit_sha = "0".repeat(40);
        },
        codes: ["REPORT_GIT"],
      },
      {
        name: "wrong validation storage commit",
        mutate(candidate) {
          candidate.validation_reports[0].storage_commit_sha =
            implementationSubject;
        },
        codes: ["REPORT_MISSING"],
      },
      {
        name: "wrong audit blob",
        mutate(candidate) {
          candidate.audit_reports[0].storage_blob_oid = "0".repeat(40);
        },
        codes: ["REPORT_BLOB_OID"],
      },
      {
        name: "wrong validation digest",
        mutate(candidate) {
          candidate.validation_reports[0].file_sha256 = "0".repeat(64);
        },
        codes: ["REPORT_FILE_DIGEST"],
      },
      {
        name: "unrelated audit storage commit",
        mutate(candidate) {
          candidate.audit_reports[0].storage_commit_sha =
            unrelatedStorageCommit;
        },
        codes: ["REPORT_HISTORY"],
      },
      {
        name: "wrong audit file payload",
        mutate(candidate) {
          const other = candidate.validation_reports[0];
          Object.assign(candidate.audit_reports[0], {
            path: other.path,
            storage_blob_oid: other.storage_blob_oid,
            file_sha256: other.file_sha256,
          });
        },
        codes: ["REPORT_PAYLOAD"],
      },
      {
        name: "audit report uses validation namespace",
        mutate(candidate) {
          candidate.audit_reports[0].id = "VALIDATION-WRONG-TYPE";
        },
        codes: ["SCHEMA_PATTERN"],
      },
      {
        name: "orphan requirement report scope",
        mutate(candidate) {
          candidate.audit_reports[0].covered_requirement_ids.push(
            "FR-7.11-999"
          );
        },
        codes: ["V2_REPORT_SCOPE"],
      },
      {
        name: "foreign clause ownership",
        mutate(candidate) {
          candidate.audit_reports[0].covered_clause_ids.push(
            candidate.requirements[1].clauses[0].id
          );
        },
        codes: ["V2_REPORT_SCOPE_OWNERSHIP"],
      },
      {
        name: "passing validation without execution evidence",
        mutate(candidate) {
          candidate.validation_reports[0].executions = [];
        },
        codes: ["SCHEMA_MIN_ITEMS", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution evidence bound to wrong subject",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].subject_sha =
            api.AUDITED_SOURCE_COMMIT;
        },
        codes: ["V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution references an unknown required test",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].test_case_id =
            "TST-FR-7.1-999-REQ-999";
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution references a required test in another clause",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].test_case_id =
            candidate.requirements[1].clauses[0].required_tests[0].id;
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "validation report evidence class differs from required test",
        mutate(candidate) {
          candidate.validation_reports[0].evidence_class = "unit";
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution invocation differs from the owning registry contract",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.invocation_id = `INV-${execution.test_case_id.slice(
            4
          )}-002`;
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution invocation fingerprint differs from its contract",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.invocation_fingerprint_sha256 = "0".repeat(64);
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution assertion set differs from its contract",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.assertions[0].assertion_id = `ASSERT-${execution.test_case_id.slice(
            4
          )}-002`;
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_CONTRACT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution assertion IDs are unique",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.assertions.push(structuredClone(execution.assertions[0]));
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: [
          "SCHEMA_UNIQUE_ITEMS",
          "V2_EXECUTION_CONTRACT",
          "V2_VALIDATION_EXECUTIONS",
        ],
      },
      {
        name: "accepted required test has a closed execution contract",
        mutate(candidate) {
          delete candidate.requirements[0].clauses[0].required_tests[0]
            .execution_contract;
        },
        codes: ["V2_REQUIRED_TEST_CONTRACT", "V2_RUST_READY"],
      },
      {
        name: "execution contract IDs belong to their exact required test",
        mutate(candidate) {
          candidate.requirements[0].clauses[0].required_tests[0].execution_contract.invocation_id =
            "INV-FR-7.1-002-REQ-001-001";
        },
        codes: ["V2_REQUIRED_TEST_CONTRACT", "V2_RUST_READY"],
      },
      {
        name: "Rust-ready clause executes every required test",
        mutate(candidate) {
          candidate.validation_reports[0].executions.pop();
        },
        codes: ["V2_CLAUSE_REQUIRED_TEST_VALIDATION", "V2_RUST_READY"],
      },
      {
        name: "execution result fingerprint covers all semantics",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].result_fingerprint_sha256 =
            "0".repeat(64);
        },
        codes: ["V2_EXECUTION_FINGERPRINT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "passing execution cannot contain a failed assertion",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.assertions[0].result = "fail";
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_RESULT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "failed command cannot have a successful exit",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.result = "fail";
          execution.assertions[0].result = "fail";
          execution.exit_code = 0;
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_RESULT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "blocked command must contain blocked assertions",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.result = "blocked";
          execution.exit_code = 1;
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_RESULT", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "execution timestamps are ordered",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.started_at = "2026-07-26T12:32:00.000Z";
          execution.finished_at = "2026-07-26T12:31:00.000Z";
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["V2_EXECUTION_TIMESTAMPS", "V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "nonexistent execution storage commit",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].storage_commit_sha =
            "0".repeat(40);
        },
        codes: ["EXECUTION_GIT"],
      },
      {
        name: "missing execution transcript",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].storage_commit_sha =
            implementationSubject;
        },
        codes: ["EXECUTION_MISSING"],
      },
      {
        name: "wrong execution transcript blob",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].storage_blob_oid =
            "0".repeat(40);
        },
        codes: ["EXECUTION_BLOB_OID"],
      },
      {
        name: "wrong execution transcript exact-byte digest",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].file_sha256 =
            "0".repeat(64);
        },
        codes: ["EXECUTION_FILE_DIGEST"],
      },
      {
        name: "wrong execution transcript canonical digest",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].canonical_sha256 =
            "0".repeat(64);
        },
        codes: ["EXECUTION_CANONICAL_DIGEST"],
      },
      {
        name: "wrong execution transcript storage projection",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].storage_projection =
            "raw-output-v1";
        },
        codes: ["SCHEMA_CONST"],
      },
      {
        name: "execution transcript path is derived from its execution ID",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].path =
            "docs/matrix-rust-sdk/validation/executions/access_token=syt_secret.json";
        },
        codes: ["SCHEMA_PATTERN", "V2_EXECUTION_STORAGE_PATH"],
      },
      {
        name: "wrong execution transcript payload",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.invocation_id = `INV-${execution.test_case_id.slice(
            4
          )}-002`;
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["EXECUTION_PAYLOAD"],
      },
      {
        name: "unrelated passing JSON cannot substitute for transcript",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          Object.assign(execution, {
            id: "EXEC-999998",
            storage_commit_sha: unrelatedExecutionStorageCommit,
            storage_blob_oid: git(root, [
              "rev-parse",
              `${unrelatedExecutionStorageCommit}:${unrelatedExecutionPath}`,
            ]),
            path: unrelatedExecutionPath,
            file_sha256: api.sha256(unrelatedExecutionBytes),
            canonical_sha256: api.sha256(
              api.canonicalize(unrelatedExecutionPayload)
            ),
          });
          execution.result_fingerprint_sha256 =
            api.executionResultFingerprint(execution);
        },
        codes: ["EXECUTION_PAYLOAD"],
      },
      {
        name: "non-passing execution cannot authorize passing report",
        mutate(candidate) {
          const execution = candidate.validation_reports[0].executions[0];
          execution.result = "fail";
          execution.exit_code = 1;
        },
        codes: ["V2_VALIDATION_EXECUTIONS"],
      },
      {
        name: "raw stdout is forbidden from execution transcript records",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].stdout =
            "raw output is not durable evidence";
        },
        codes: [
          "SCHEMA_ADDITIONAL_PROPERTY",
          "V2_VALIDATION_EXECUTION_PRIVACY",
        ],
      },
      {
        name: "ephemeral local path is forbidden from transcript semantics",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].legacy_invocation =
            "node /tmp/private-fixture.mjs";
        },
        codes: [
          "SCHEMA_ADDITIONAL_PROPERTY",
          "V2_VALIDATION_EXECUTIONS",
          "V2_VALIDATION_EXECUTION_PRIVACY",
        ],
      },
      {
        name: "execution identifiers are bounded",
        mutate(candidate) {
          candidate.validation_reports[0].executions[0].invocation_id =
            "X".repeat(129);
        },
        codes: ["SCHEMA_MAX_LENGTH"],
      },
      {
        name: "wrong Rust implementation subject",
        mutate(candidate) {
          candidate.requirements[0].rust_cutover.implementation_subject_sha =
            storageCommit;
        },
        codes: ["V2_VALIDATION_REPORT_REFERENCE", "V2_RUST_READY"],
      },
      {
        name: "nonexistent Rust implementation subject",
        mutate(candidate) {
          candidate.requirements[0].rust_cutover.implementation_subject_sha =
            "f".repeat(40);
        },
        codes: ["V2_IMPLEMENTATION_SUBJECT", "V2_RUST_READY"],
      },
      {
        name: "unrelated-branch Rust implementation subject",
        mutate(candidate) {
          candidate.requirements[0].rust_cutover.implementation_subject_sha =
            unrelatedStorageCommit;
        },
        codes: ["V2_IMPLEMENTATION_SUBJECT_HISTORY", "V2_RUST_READY"],
      },
      {
        name: "combined report ID collision",
        mutate(candidate) {
          candidate.validation_reports[0].id = candidate.audit_reports[0].id;
        },
        codes: ["V2_REPORT_IDS_GLOBAL"],
      },
      {
        name: "duplicate execution evidence IDs",
        mutate(candidate) {
          candidate.validation_reports[0].executions.push(
            structuredClone(candidate.validation_reports[0].executions[0])
          );
        },
        codes: ["V2_EXECUTION_IDS_GLOBAL", "V2_VALIDATION_EXECUTIONS"],
      },
    ];
    for (const scenario of mutations) {
      const candidate = structuredClone(value);
      scenario.mutate(candidate);
      const errors = await validate(candidate);
      for (const code of scenario.codes)
        assert.ok(
          errors.some((entry) => entry.code === code),
          `${scenario.name}: missing ${code}: ${JSON.stringify(errors)}`
        );
    }

    const privateTranscriptCases = [
      [
        "macOS TMPDIR path",
        "legacy_invocation",
        "node /private/var/folders/8l/private-fixture.mjs",
      ],
      [
        "var folders path",
        "legacy_invocation",
        "node /var/folders/8l/private-fixture.mjs",
      ],
      [
        "workspace path",
        "legacy_invocation",
        "node /workspace/private-fixture.mjs",
      ],
      ["root path", "legacy_invocation", "node /root/private-fixture.mjs"],
      [
        "generic opt path",
        "legacy_invocation",
        "node /opt/private-fixture.mjs",
      ],
      [
        "generic usr path",
        "legacy_invocation",
        "node /usr/local/bin/private-fixture",
      ],
      [
        "Windows drive path",
        "legacy_invocation",
        String.raw`node C:\Users\alice\private-fixture.mjs`,
      ],
      [
        "UNC path",
        "legacy_invocation",
        String.raw`node \\server\share\private-fixture.mjs`,
      ],
      ["file URI", "legacy_invocation", "node file:///private/fixture.mjs"],
      [
        "raw stdout and token",
        "assertion_text",
        "stdout: raw process output access_token=syt_secret",
      ],
      ["raw stderr", "assertion_text", "stderr: raw process failure"],
      ["Matrix user ID", "assertion_text", "user=@alice:example.invalid"],
      ["Matrix room ID", "assertion_text", "room=!private:example.invalid"],
      ["Matrix event ID", "assertion_text", "event=$private:example.invalid"],
      [
        "full URL",
        "assertion_text",
        "https://example.invalid/_matrix/client/v3/sync",
      ],
      [
        "message-like content",
        "assertion_text",
        "message: private conversation content",
      ],
    ];
    const unsafeIdentifierValues = [
      ["full URL", "https://example.invalid/private"],
      ["mxc URL", "mxc://example.invalid/private"],
      ["absolute path", "/private/fixture"],
      ["traversal", "../private/fixture"],
      ["backslash path", String.raw`C:\private\fixture`],
      ["Matrix sigil", "@alice:example.invalid"],
      ["newline", "SAFE\nINJECT"],
      ["shell metacharacter", "SAFE;touch"],
      ["secret-shaped identifier", "ACCESS_TOKEN_SECRET"],
    ];
    for (const field of [
      "schema_version",
      "id",
      "kind",
      "test_case_id",
      "runner_id",
      "invocation_id",
      "subject_sha",
      "started_at",
      "finished_at",
      "result",
      "assertion_id",
    ])
      for (const [valueName, unsafeValue] of unsafeIdentifierValues)
        privateTranscriptCases.push([
          `${valueName} in ${field}`,
          field,
          unsafeValue,
        ]);
    privateTranscriptCases.push(
      ["non-numeric exit code", "exit_code", "access_token=syt_secret"],
      [
        "non-digest invocation fingerprint",
        "invocation_fingerprint_sha256",
        "access_token=syt_secret",
      ],
      [
        "non-digest result fingerprint",
        "result_fingerprint_sha256",
        "access_token=syt_secret",
      ],
      ["forbidden root stdout key", "stdout", "raw process output"],
      ["forbidden root stderr key", "stderr", "raw process failure"],
      ["forbidden root description key", "description", "private prose"],
      ["forbidden root invocation key", "invocation", "cargo test"],
      ["forbidden assertion text key", "assertion_extra", "private prose"]
    );
    for (const [name, key, unsafeValue] of privateTranscriptCases) {
      const unsafeExecutionPayload = structuredClone(executionPayload);
      if (key === "assertion_text")
        unsafeExecutionPayload.assertions = [unsafeValue];
      else if (key === "assertion_id")
        unsafeExecutionPayload.assertions[0].assertion_id = unsafeValue;
      else if (key === "assertion_extra")
        unsafeExecutionPayload.assertions[0].text = unsafeValue;
      else unsafeExecutionPayload[key] = unsafeValue;
      const unsafeExecutionBytes = Buffer.from(
        api.prettyJson(unsafeExecutionPayload)
      );
      await writeFile(path.join(root, executionPath), unsafeExecutionBytes);
      git(root, ["add", executionPath]);
      git(root, ["commit", "--quiet", "-m", `unsafe transcript: ${name}`]);
      const unsafeExecutionCommit = git(root, ["rev-parse", "HEAD"]);
      const unsafeExecutionRecord = {
        ...unsafeExecutionPayload,
        storage_commit_sha: unsafeExecutionCommit,
        storage_blob_oid: git(root, [
          "rev-parse",
          `${unsafeExecutionCommit}:${executionPath}`,
        ]),
        storage_projection: "execution-record-minus-storage-identity-v1",
        path: executionPath,
        file_sha256: api.sha256(unsafeExecutionBytes),
        canonical_sha256: api.sha256(api.canonicalize(unsafeExecutionPayload)),
      };
      const unsafeValidationPayload = {
        ...validationPayload,
        executions: [unsafeExecutionRecord],
      };
      const unsafeValidationBytes = Buffer.from(
        api.prettyJson(unsafeValidationPayload)
      );
      await writeFile(path.join(root, validationPath), unsafeValidationBytes);
      git(root, ["add", validationPath]);
      git(root, ["commit", "--quiet", "-m", `unsafe report: ${name}`]);
      const unsafeValidationCommit = git(root, ["rev-parse", "HEAD"]);
      const unsafeCandidate = structuredClone(value);
      unsafeCandidate.validation_reports[0] = {
        ...unsafeValidationPayload,
        storage_commit_sha: unsafeValidationCommit,
        storage_blob_oid: git(root, [
          "rev-parse",
          `${unsafeValidationCommit}:${validationPath}`,
        ]),
        storage_projection: "report-record-minus-storage-identity-v1",
        path: validationPath,
        file_sha256: api.sha256(unsafeValidationBytes),
      };
      const errors = await validate(unsafeCandidate);
      for (const code of [
        "V2_VALIDATION_EXECUTIONS",
        "V2_VALIDATION_EXECUTION_PRIVACY",
      ])
        assert.ok(
          errors.some((entry) => entry.code === code),
          `${name}: a fully durable unsafe chain was not rejected by ${code}: ${JSON.stringify(
            errors
          )}`
        );
    }
    await writeFile(path.join(root, executionPath), executionBytes);
    await writeFile(path.join(root, validationPath), validationBytes);
    git(root, ["add", executionPath, validationPath]);
    git(root, [
      "commit",
      "--quiet",
      "-m",
      "restore private transcript baseline",
    ]);
    assert.deepEqual(await validate(value), []);

    const acceptedButBlocked = structuredClone(value);
    acceptedButBlocked.requirements[0].rust_cutover.readiness = "blocked";
    acceptedButBlocked.requirements[0].audit.report_ids = [auditReportId];
    acceptedButBlocked.audit_reports[0].verdict = "changes_required";
    refreshV2Summary(acceptedButBlocked);
    assert.ok(
      (await validate(acceptedButBlocked)).some(
        ({ code }) => code === "V2_AUDIT_ACCEPTANCE"
      )
    );

    const staleDerivedSummary = structuredClone(value);
    staleDerivedSummary.validation_reports[0].status = "fail";
    staleDerivedSummary.validation_reports[0].reviewer = null;
    staleDerivedSummary.validation_reports[0].accepted_at = null;
    assert.ok(
      (await validate(staleDerivedSummary)).some(
        ({ code }) => code === "V2_SUMMARY"
      )
    );

    await writeFile(path.join(root, executionPath), `${executionBytes}dirty\n`);
    assert.ok(
      (await validate(value)).some(
        ({ code }) => code === "EXECUTION_WORKTREE_STATE"
      )
    );
    await writeFile(path.join(root, executionPath), executionBytes);

    await writeFile(
      path.join(root, executionPath),
      `${executionBytes}staged\n`
    );
    git(root, ["add", executionPath]);
    assert.ok(
      (await validate(value)).some(
        ({ code }) => code === "EXECUTION_WORKTREE_STATE"
      )
    );
    git(root, ["restore", "--staged", "--worktree", "--", executionPath]);

    const untrackedExecutionPath =
      "docs/matrix-rust-sdk/validation/executions/EXEC-999999.json";
    await writeFile(path.join(root, untrackedExecutionPath), executionBytes);
    const untrackedExecution = structuredClone(value);
    untrackedExecution.validation_reports[0].executions[0].path =
      untrackedExecutionPath;
    untrackedExecution.validation_reports[0].executions[0].id = "EXEC-999999";
    untrackedExecution.validation_reports[0].executions[0].result_fingerprint_sha256 =
      api.executionResultFingerprint(
        untrackedExecution.validation_reports[0].executions[0]
      );
    assert.ok(
      (await validate(untrackedExecution)).some(
        ({ code }) =>
          code === "EXECUTION_WORKTREE_STATE" || code === "EXECUTION_MISSING"
      )
    );
    await rm(path.join(root, untrackedExecutionPath));

    const staleExecutionPayload = {
      ...executionPayload,
      assertions: [
        {
          assertion_id: `ASSERT-${executionPayload.test_case_id.slice(4)}-999`,
          result: "pass",
        },
      ],
    };
    staleExecutionPayload.result_fingerprint_sha256 =
      api.executionResultFingerprint(staleExecutionPayload);
    await writeFile(
      path.join(root, executionPath),
      Buffer.from(api.prettyJson(staleExecutionPayload))
    );
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "replace execution transcript"]);
    assert.ok(
      (await validate(value)).some(({ code }) => code === "EXECUTION_HEAD_BLOB")
    );
    await writeFile(path.join(root, executionPath), executionBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "restore execution transcript"]);
    assert.deepEqual(await validate(value), []);

    const noncanonicalBytes = Buffer.from(
      `${JSON.stringify(executionPayload)}\n`
    );
    await writeFile(path.join(root, executionPath), noncanonicalBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "noncanonical transcript fixture"]);
    const noncanonicalCommit = git(root, ["rev-parse", "HEAD"]);
    const noncanonical = structuredClone(value);
    Object.assign(noncanonical.validation_reports[0].executions[0], {
      storage_commit_sha: noncanonicalCommit,
      storage_blob_oid: git(root, [
        "rev-parse",
        `${noncanonicalCommit}:${executionPath}`,
      ]),
      file_sha256: api.sha256(noncanonicalBytes),
      canonical_sha256: api.sha256(api.canonicalize(executionPayload)),
    });
    assert.ok(
      (await validate(noncanonical)).some(
        ({ code }) => code === "EXECUTION_JSON"
      )
    );
    await writeFile(path.join(root, executionPath), executionBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "restore canonical transcript"]);

    await rm(path.join(root, executionPath));
    await symlink("../ready.json", path.join(root, executionPath));
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "symlink transcript fixture"]);
    assert.ok(
      (await validate(value)).some(({ code }) => code === "EXECUTION_GIT")
    );
    await rm(path.join(root, executionPath));
    await writeFile(path.join(root, executionPath), executionBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "restore regular transcript"]);
    assert.deepEqual(await validate(value), []);

    const failedExecutionPayload = {
      ...executionPayload,
      result: "fail",
      exit_code: 1,
      assertions: [{ assertion_id: "SCOPED_INTEGRATION_TEST", result: "fail" }],
    };
    failedExecutionPayload.result_fingerprint_sha256 =
      api.executionResultFingerprint(failedExecutionPayload);
    const failedExecutionBytes = Buffer.from(
      api.prettyJson(failedExecutionPayload)
    );
    await writeFile(path.join(root, executionPath), failedExecutionBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "failed transcript fixture"]);
    const failedExecutionCommit = git(root, ["rev-parse", "HEAD"]);
    const failedExecution = structuredClone(value);
    failedExecution.validation_reports[0].executions[0] = {
      ...failedExecutionPayload,
      storage_commit_sha: failedExecutionCommit,
      storage_blob_oid: git(root, [
        "rev-parse",
        `${failedExecutionCommit}:${executionPath}`,
      ]),
      storage_projection: "execution-record-minus-storage-identity-v1",
      path: executionPath,
      file_sha256: api.sha256(failedExecutionBytes),
      canonical_sha256: api.sha256(api.canonicalize(failedExecutionPayload)),
    };
    assert.ok(
      (await validate(failedExecution)).some(
        ({ code }) => code === "V2_VALIDATION_EXECUTIONS"
      )
    );
    await writeFile(path.join(root, executionPath), executionBytes);
    git(root, ["add", executionPath]);
    git(root, ["commit", "--quiet", "-m", "restore passing transcript"]);

    await writeFile(path.join(root, auditPath), `${auditBytes}dirty\n`);
    assert.ok(
      (await validate(value)).some(
        ({ code, pointer }) =>
          code === "REPORT_WORKTREE_STATE" &&
          pointer.startsWith("/audit_reports/0")
      )
    );
    await writeFile(path.join(root, auditPath), auditBytes);

    const untrackedPath = "docs/matrix-rust-sdk/reviews/untracked.json";
    await writeFile(path.join(root, untrackedPath), auditBytes);
    const untracked = structuredClone(value);
    untracked.audit_reports[0].path = untrackedPath;
    assert.ok(
      (await validate(untracked)).some(
        ({ code }) =>
          code === "REPORT_WORKTREE_STATE" || code === "REPORT_MISSING"
      )
    );
    await rm(path.join(root, untrackedPath));

    await writeFile(
      path.join(root, auditPath),
      Buffer.from(api.prettyJson({ ...auditPayload, verdict: "reject" }))
    );
    git(root, ["add", auditPath]);
    git(root, ["commit", "--quiet", "-m", "replace durable audit report"]);
    assert.ok(
      (await validate(value)).some(({ code }) => code === "REPORT_HEAD_BLOB"),
      "a stale but clean replaced report must fail"
    );
    await writeFile(path.join(root, auditPath), auditBytes);
    git(root, ["add", auditPath]);
    git(root, ["commit", "--quiet", "-m", "restore durable audit report"]);
    assert.deepEqual(await validate(value), []);

    const blockedWithDirtyAudit = structuredClone(value);
    blockedWithDirtyAudit.requirements[0].rust_cutover.readiness = "blocked";
    await writeFile(path.join(root, auditPath), `${auditBytes}dirty\n`);
    assert.ok(
      (await validate(blockedWithDirtyAudit)).some(
        ({ code }) => code === "REPORT_WORKTREE_STATE"
      ),
      "accepted audit remains Git-verified while Rust readiness is blocked"
    );
    await writeFile(path.join(root, auditPath), auditBytes);

    const siblingBaseTree = git(root, [
      "rev-parse",
      `${executionStorageCommit}^{tree}`,
    ]);
    const siblingSubject = git(root, [
      "commit-tree",
      siblingBaseTree,
      "-p",
      executionStorageCommit,
      "-m",
      "sibling implementation subject",
    ]);
    const currentHead = git(root, ["rev-parse", "HEAD"]);
    const currentTree = git(root, ["rev-parse", `${currentHead}^{tree}`]);
    const siblingExecutionStorage = git(root, [
      "commit-tree",
      currentTree,
      "-p",
      implementationSubject,
      "-m",
      "sibling execution transcript storage",
    ]);
    const mergedHead = git(root, [
      "commit-tree",
      currentTree,
      "-p",
      currentHead,
      "-p",
      siblingSubject,
      "-p",
      siblingExecutionStorage,
      "-m",
      "merge sibling implementation history",
    ]);
    git(root, ["checkout", "--quiet", "--detach", mergedHead]);
    const siblingCandidate = structuredClone(value);
    siblingCandidate.requirements[0].rust_cutover.implementation_subject_sha =
      siblingSubject;
    siblingCandidate.validation_reports[0].subject_sha = siblingSubject;
    siblingCandidate.validation_reports[0].executions[0].subject_sha =
      siblingSubject;
    assert.ok(
      (await validate(siblingCandidate)).some(
        ({ code }) => code === "REPORT_SUBJECT_STORAGE_HISTORY"
      ),
      "a sibling subject merged only after report storage cannot validate that report"
    );
    const siblingExecutionCandidate = structuredClone(value);
    siblingExecutionCandidate.validation_reports[0].executions[0].storage_commit_sha =
      siblingExecutionStorage;
    assert.ok(
      (await validate(siblingExecutionCandidate)).some(
        ({ code }) => code === "EXECUTION_REPORT_HISTORY"
      ),
      "a sibling transcript merged only after report storage cannot validate that report"
    );

    const truncated = api.migrateV1({
      v1,
      audit,
      sourceIdentity: {
        commit_sha: api.AUDITED_SOURCE_COMMIT,
        blob_oid: api.V1_BLOB,
        file_sha256: api.V1_SHA256,
      },
      auditIdentity,
    });
    const aIdentity = api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(truncated)),
      api.V1_PATH
    );
    assert.ok(
      aIdentity,
      "the valid A artifact must be the derived predecessor"
    );
    truncated.lifecycle.previous_artifact = aIdentity;
    refreshLifecycle(truncated, audit);
    await writeFile(
      path.join(root, api.V1_PATH),
      Buffer.from(api.prettyJson(truncated))
    );
    git(root, ["add", api.V1_PATH]);
    git(root, ["commit", "--quiet", "-m", "invalid truncated v2 history B"]);
    const retainedTruncation = structuredClone(value);
    const bIdentity = api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(retainedTruncation)),
      api.V1_PATH
    );
    assert.ok(
      bIdentity,
      "the invalid B artifact must be visible as C's predecessor"
    );
    retainedTruncation.lifecycle.previous_artifact = bIdentity;
    refreshLifecycle(retainedTruncation, audit);
    assert.ok(
      (await validate(retainedTruncation)).some(({ code }) =>
        new Set([
          "V2_LIFECYCLE_REPORT_PREFIX",
          "V2_LIFECYCLE_REPORT_ID_PREFIX",
          "V2_LIFECYCLE_CHAIN_PREFIX",
        ]).has(code)
      ),
      "C must reject an internally valid truncating B anywhere in the complete first-parent v2 epoch"
    );
  } finally {
    await reader.close();
  }
});

test("created decisions and blockers retain cross-kind first-create order", async (t) => {
  const audit = productionShapedAuditFixture();
  const { root } = await productionCliRepository(t, audit);
  const v1 = api.parseCanonicalJson(
    git(root, ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`], {
      encoding: null,
    })
  );
  const auditIdentity = api.deriveDurableAuditIdentity(root);
  const sourceIdentity = {
    commit_sha: api.AUDITED_SOURCE_COMMIT,
    blob_oid: api.V1_BLOB,
    file_sha256: api.V1_SHA256,
  };
  const value = api.migrateV1({
    v1,
    audit,
    sourceIdentity,
    auditIdentity,
  });
  const baselineCommit = await commitV2Artifact(
    root,
    value,
    audit,
    null,
    "baseline v2 lifecycle"
  );
  const affectedRow = value.requirements[26];
  const affectedClause = affectedRow.clauses[0];
  const decision = {
    id: "AD-27",
    decision: "Authorize the first appended architecture decision.",
    status: "unresolved",
    owner: "Architecture owner",
    scope_authority: "Only the exact documented scope may be decided.",
    affected_requirement_ids: [affectedRow.id],
    affected_clause_ids: [affectedClause.id],
    closure_evidence: [],
    superseded_by_id: null,
  };
  const decisionReportId = "VALIDATION-CREATE-AD-001";
  const decisionReportPayload = {
    id: decisionReportId,
    subject_sha: baselineCommit,
    evidence_class: "unit",
    status: "pending",
    reviewer: "Lifecycle reviewer",
    reviewed_at: "2026-07-26T14:00:00.000Z",
    accepted_at: null,
    covered_requirement_ids: [affectedRow.id],
    covered_clause_ids: [affectedClause.id],
    executions: [],
    test_contract_snapshots: [],
    lifecycle_authorizations: [
      lifecycleAuthorization({
        id: "AUTH-CREATE-AD-001",
        entityKind: "architecture_decision",
        entityId: decision.id,
        sequence: 1,
        operation: "create",
        sourceEntity: api.architectureDecisionSourceProjection(decision),
        fromPayloadSha256: null,
        toState: api.architectureDecisionLifecycleProjection(decision),
        reportId: decisionReportId,
        subjectSha: baselineCommit,
        reviewer: "Lifecycle reviewer",
        reviewedAt: "2026-07-26T14:00:00.000Z",
      }),
    ],
    limitations: [],
  };
  value.architecture_decisions.push(decision);
  value.validation_reports.push(
    await persistLifecycleReport(
      root,
      "docs/matrix-rust-sdk/validation/create-ad-27.json",
      decisionReportPayload,
      "authorize created architecture decision"
    )
  );
  const decisionArtifactCommit = await commitV2Artifact(
    root,
    value,
    audit,
    baselineCommit,
    "append created architecture decision"
  );

  const blocker = {
    id: "MRSDK-R999",
    kind: "risk",
    severity: "low",
    status: "open",
    owner_task_ids: ["R0.2-E"],
    authority: "Lifecycle reviewer approval within exact affected scope.",
    affected_requirement_ids: [affectedRow.id],
    affected_clause_ids: [affectedClause.id],
    threat_ids: [],
    boundary_ids: [],
    closure_criteria: ["The appended risk is explicitly dispositioned."],
    closure_evidence: [],
    qualification: "An open appended risk is not a closure claim.",
  };
  const blockerReportId = "VALIDATION-CREATE-BLOCKER-001";
  const blockerReportPayload = {
    id: blockerReportId,
    subject_sha: decisionArtifactCommit,
    evidence_class: "unit",
    status: "pending",
    reviewer: "Lifecycle reviewer",
    reviewed_at: "2026-07-26T15:00:00.000Z",
    accepted_at: null,
    covered_requirement_ids: [affectedRow.id],
    covered_clause_ids: [affectedClause.id],
    executions: [],
    test_contract_snapshots: [],
    lifecycle_authorizations: [
      lifecycleAuthorization({
        id: "AUTH-CREATE-BLOCKER-001",
        entityKind: "blocker",
        entityId: blocker.id,
        sequence: 1,
        operation: "create",
        sourceEntity: api.blockerSourceProjection(blocker),
        fromPayloadSha256: null,
        toState: api.blockerLifecycleProjection(blocker),
        reportId: blockerReportId,
        subjectSha: decisionArtifactCommit,
        reviewer: "Lifecycle reviewer",
        reviewedAt: "2026-07-26T15:00:00.000Z",
      }),
    ],
    limitations: [],
  };
  value.blockers.push(blocker);
  value.validation_reports.push(
    await persistLifecycleReport(
      root,
      "docs/matrix-rust-sdk/validation/create-blocker-r999.json",
      blockerReportPayload,
      "authorize created blocker"
    )
  );
  const createdArtifactCommit = await commitV2Artifact(
    root,
    value,
    audit,
    decisionArtifactCommit,
    "append created blocker after decision"
  );
  const baselineChainCount = api.sourceBaselineManifest(audit).length;
  assert.deepEqual(
    value.lifecycle.entity_chains
      .slice(baselineChainCount)
      .map(({ entity_kind, entity_id }) => [entity_kind, entity_id]),
    [
      ["architecture_decision", "AD-27"],
      ["blocker", "MRSDK-R999"],
    ]
  );
  const reader = new api.GitObjectReader(root);
  try {
    assert.deepEqual(
      await api.validateTraceability(value, {
        repoRoot: root,
        gitObjects: reader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      }),
      []
    );
  } finally {
    await reader.close();
  }

  const acceptedRiskState = api.blockerLifecycleProjection({
    ...blocker,
    status: "accepted_risk",
  });
  const acceptedRiskReportId = "VALIDATION-BLOCKER-ACCEPTED-RISK-001";
  const acceptedRiskAuthorization = lifecycleAuthorization({
    id: "AUTH-BLOCKER-ACCEPTED-RISK-001",
    entityKind: "blocker",
    entityId: blocker.id,
    sequence: 2,
    operation: "advance",
    sourceEntitySha256:
      blockerReportPayload.lifecycle_authorizations[0].source_entity_sha256,
    fromPayloadSha256:
      blockerReportPayload.lifecycle_authorizations[0].to_payload_sha256,
    toState: acceptedRiskState,
    previousAuthorization: blockerReportPayload.lifecycle_authorizations[0],
    reportId: acceptedRiskReportId,
    subjectSha: createdArtifactCommit,
    reviewer: "Lifecycle reviewer",
    reviewedAt: "2026-07-26T16:00:00.000Z",
    rationale: "The explicit risk disposition remains unresolved.",
  });
  value.validation_reports.push(
    await persistLifecycleReport(
      root,
      "docs/matrix-rust-sdk/validation/blocker-accepted-risk.json",
      {
        id: acceptedRiskReportId,
        subject_sha: createdArtifactCommit,
        evidence_class: "unit",
        status: "pending",
        reviewer: "Lifecycle reviewer",
        reviewed_at: "2026-07-26T16:00:00.000Z",
        accepted_at: null,
        covered_requirement_ids: [affectedRow.id],
        covered_clause_ids: [affectedClause.id],
        executions: [],
        test_contract_snapshots: [],
        lifecycle_authorizations: [acceptedRiskAuthorization],
        limitations: [],
      },
      "advance blocker to accepted risk"
    )
  );
  blocker.status = "accepted_risk";
  const acceptedRiskArtifactCommit = await commitV2Artifact(
    root,
    value,
    audit,
    createdArtifactCommit,
    "record unresolved accepted risk"
  );
  const acceptedReader = new api.GitObjectReader(root);
  try {
    assert.deepEqual(
      await api.validateTraceability(value, {
        repoRoot: root,
        gitObjects: acceptedReader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      }),
      []
    );
  } finally {
    await acceptedReader.close();
  }

  const reopenedState = api.blockerLifecycleProjection({
    ...blocker,
    status: "mitigating",
  });
  const reopenedReportId = "VALIDATION-BLOCKER-REOPEN-001";
  const reopenedAuthorization = lifecycleAuthorization({
    id: "AUTH-BLOCKER-REOPEN-001",
    entityKind: "blocker",
    entityId: blocker.id,
    sequence: 3,
    operation: "rollback",
    sourceEntitySha256: acceptedRiskAuthorization.source_entity_sha256,
    fromPayloadSha256: acceptedRiskAuthorization.to_payload_sha256,
    toState: reopenedState,
    previousAuthorization: acceptedRiskAuthorization,
    reportId: reopenedReportId,
    subjectSha: acceptedRiskArtifactCommit,
    reviewer: "Lifecycle reviewer",
    reviewedAt: "2026-07-26T17:00:00.000Z",
    rationale: "Risk mitigation resumed after explicit review.",
    rollbackReason: "Accepted-risk disposition was reopened for mitigation.",
    rollbackEvidence: ["Reviewed reopening disposition."],
  });
  value.validation_reports.push(
    await persistLifecycleReport(
      root,
      "docs/matrix-rust-sdk/validation/blocker-reopen.json",
      {
        id: reopenedReportId,
        subject_sha: acceptedRiskArtifactCommit,
        evidence_class: "unit",
        status: "pending",
        reviewer: "Lifecycle reviewer",
        reviewed_at: "2026-07-26T17:00:00.000Z",
        accepted_at: null,
        covered_requirement_ids: [affectedRow.id],
        covered_clause_ids: [affectedClause.id],
        executions: [],
        test_contract_snapshots: [],
        lifecycle_authorizations: [reopenedAuthorization],
        limitations: [],
      },
      "roll accepted risk back to mitigation"
    )
  );
  blocker.status = "mitigating";
  await commitV2Artifact(
    root,
    value,
    audit,
    acceptedRiskArtifactCommit,
    "reopen accepted risk for mitigation"
  );
  const reopenedReader = new api.GitObjectReader(root);
  try {
    assert.deepEqual(
      await api.validateTraceability(value, {
        repoRoot: root,
        gitObjects: reopenedReader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      }),
      []
    );
  } finally {
    await reopenedReader.close();
  }
});

test("requirement subjects use Git-aware advance and rollback classification", async (t) => {
  const audit = productionShapedAuditFixture();
  const { root } = await productionCliRepository(t, audit);
  const v1 = api.parseCanonicalJson(
    git(root, ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`], {
      encoding: null,
    })
  );
  const auditIdentity = api.deriveDurableAuditIdentity(root);
  const sourceIdentity = {
    commit_sha: api.AUDITED_SOURCE_COMMIT,
    blob_oid: api.V1_BLOB,
    file_sha256: api.V1_SHA256,
  };
  const value = api.migrateV1({
    v1,
    audit,
    sourceIdentity,
    auditIdentity,
  });
  const baselineCommit = await commitV2Artifact(
    root,
    value,
    audit,
    null,
    "subject lifecycle baseline"
  );
  const row = value.requirements[50];
  const sourceEntitySha256 = api.sha256(
    api.canonicalize(api.sourceAuditRowProjection(row))
  );
  const baselineState = api.cutoverStateProjection(row);
  const plannedState = structuredClone(baselineState);
  plannedState.rust_cutover.readiness = "implementation_planned";
  plannedState.rust_cutover.implementation_subject_sha = baselineCommit;
  const firstReportId = "VALIDATION-SUBJECT-001";
  const firstAuthorization = lifecycleAuthorization({
    id: "AUTH-SUBJECT-001",
    entityId: row.id,
    sequence: 1,
    sourceEntitySha256,
    fromPayloadSha256: api.sha256(api.canonicalize(baselineState)),
    toState: plannedState,
    reportId: firstReportId,
    subjectSha: baselineCommit,
    reviewer: "Subject lifecycle reviewer",
    reviewedAt: "2026-07-26T16:00:00.000Z",
  });
  value.validation_reports.push(
    await persistLifecycleReport(
      root,
      "docs/matrix-rust-sdk/validation/subject-001.json",
      {
        id: firstReportId,
        subject_sha: baselineCommit,
        evidence_class: "unit",
        status: "pending",
        reviewer: "Subject lifecycle reviewer",
        reviewed_at: "2026-07-26T16:00:00.000Z",
        accepted_at: null,
        covered_requirement_ids: [row.id],
        covered_clause_ids: [],
        executions: [],
        test_contract_snapshots: [],
        lifecycle_authorizations: [firstAuthorization],
        limitations: [],
      },
      "authorize initial implementation subject"
    )
  );
  row.rust_cutover.readiness = "implementation_planned";
  row.rust_cutover.implementation_subject_sha = baselineCommit;
  const firstArtifactCommit = await commitV2Artifact(
    root,
    value,
    audit,
    baselineCommit,
    "record initial implementation subject"
  );
  const firstArtifact = structuredClone(value);

  const validateCandidate = async (candidate) => {
    const reader = new api.GitObjectReader(root);
    try {
      return await api.validateTraceability(candidate, {
        repoRoot: root,
        gitObjects: reader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      });
    } finally {
      await reader.close();
    }
  };
  assert.deepEqual(await validateCandidate(firstArtifact), []);

  const prepareSecond = async ({
    idSuffix,
    toSubject,
    operation,
    rollbackReason = null,
    rollbackEvidence = [],
  }) => {
    const candidate = structuredClone(firstArtifact);
    const reportId = `VALIDATION-SUBJECT-${idSuffix}`;
    const toState = structuredClone(firstAuthorization.to_state);
    toState.rust_cutover.implementation_subject_sha = toSubject;
    const authorization = lifecycleAuthorization({
      id: `AUTH-SUBJECT-${idSuffix}`,
      entityId: row.id,
      sequence: 2,
      operation,
      sourceEntitySha256,
      fromPayloadSha256: firstAuthorization.to_payload_sha256,
      toState,
      previousAuthorization: firstAuthorization,
      reportId,
      subjectSha: toSubject,
      reviewer: "Subject lifecycle reviewer",
      reviewedAt: "2026-07-26T17:00:00.000Z",
      rollbackReason,
      rollbackEvidence,
    });
    candidate.validation_reports.push(
      await persistLifecycleReport(
        root,
        `docs/matrix-rust-sdk/validation/subject-${idSuffix}.json`,
        {
          id: reportId,
          subject_sha: toSubject,
          evidence_class: "unit",
          status: "pending",
          reviewer: "Subject lifecycle reviewer",
          reviewed_at: "2026-07-26T17:00:00.000Z",
          accepted_at: null,
          covered_requirement_ids: [row.id],
          covered_clause_ids: [],
          executions: [],
          test_contract_snapshots: [],
          lifecycle_authorizations: [authorization],
          limitations: [],
        },
        `authorize subject transition ${idSuffix}`
      )
    );
    candidate.requirements[50].rust_cutover.implementation_subject_sha =
      toSubject;
    candidate.lifecycle.previous_artifact = artifactIdentity(
      root,
      firstArtifactCommit
    );
    refreshV2Summary(candidate);
    refreshLifecycle(candidate, audit);
    return candidate;
  };

  git(root, ["checkout", "--quiet", "--detach", firstArtifactCommit]);
  const descendant = await prepareSecond({
    idSuffix: "002",
    toSubject: firstArtifactCommit,
    operation: "advance",
  });
  await commitV2Artifact(
    root,
    descendant,
    audit,
    firstArtifactCommit,
    "advance to descendant implementation subject"
  );
  assert.deepEqual(await validateCandidate(descendant), []);

  const olderSubject = git(root, ["rev-parse", `${baselineCommit}^`]);
  git(root, ["checkout", "--quiet", "--detach", firstArtifactCommit]);
  const olderAdvance = await prepareSecond({
    idSuffix: "003",
    toSubject: olderSubject,
    operation: "advance",
  });
  assert.ok(
    (await validateCandidate(olderAdvance)).some(
      ({ code }) => code === "V2_LIFECYCLE_SUBJECT_HISTORY"
    )
  );

  git(root, ["checkout", "--quiet", "--detach", firstArtifactCommit]);
  const siblingTree = git(root, ["rev-parse", `${baselineCommit}^{tree}`]);
  const siblingSubject = git(root, [
    "commit-tree",
    siblingTree,
    "-p",
    olderSubject,
    "-m",
    "divergent implementation subject",
  ]);
  const firstArtifactTree = git(root, [
    "rev-parse",
    `${firstArtifactCommit}^{tree}`,
  ]);
  const siblingMerge = git(root, [
    "commit-tree",
    firstArtifactTree,
    "-p",
    firstArtifactCommit,
    "-p",
    siblingSubject,
    "-m",
    "make divergent subject reachable",
  ]);
  git(root, ["checkout", "--quiet", "--detach", siblingMerge]);
  const siblingAdvance = await prepareSecond({
    idSuffix: "004",
    toSubject: siblingSubject,
    operation: "advance",
  });
  assert.ok(
    (await validateCandidate(siblingAdvance)).some(
      ({ code }) => code === "V2_LIFECYCLE_SUBJECT_HISTORY"
    )
  );

  git(root, ["checkout", "--quiet", "--detach", siblingMerge]);
  const siblingRollback = await prepareSecond({
    idSuffix: "005",
    toSubject: siblingSubject,
    operation: "rollback",
    rollbackReason: "The implementation subject moved to a divergent branch.",
    rollbackEvidence: ["Reviewed divergent subject ancestry."],
  });
  await commitV2Artifact(
    root,
    siblingRollback,
    audit,
    firstArtifactCommit,
    "explicitly roll back to divergent subject"
  );
  assert.deepEqual(await validateCandidate(siblingRollback), []);

  git(root, ["checkout", "--quiet", "--detach", firstArtifactCommit]);
  const noOp = await prepareSecond({
    idSuffix: "006",
    toSubject: baselineCommit,
    operation: "rollback",
    rollbackReason: "Attempted no-op rollback.",
    rollbackEvidence: ["No payload changed."],
  });
  assert.ok(
    (await validateCandidate(noOp)).some(
      ({ code }) => code === "V2_LIFECYCLE_NOOP"
    )
  );
});

test("invalid historical terminal states cannot be hidden by a restored head", async (t) => {
  const audit = productionShapedAuditFixture();
  const { root } = await productionCliRepository(t, audit);
  const v1 = api.parseCanonicalJson(
    git(root, ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`], {
      encoding: null,
    })
  );
  const auditIdentity = api.deriveDurableAuditIdentity(root);
  const sourceIdentity = {
    commit_sha: api.AUDITED_SOURCE_COMMIT,
    blob_oid: api.V1_BLOB,
    file_sha256: api.V1_SHA256,
  };
  const baseline = api.migrateV1({
    v1,
    audit,
    sourceIdentity,
    auditIdentity,
  });
  const baselineCommit = await commitV2Artifact(
    root,
    baseline,
    audit,
    null,
    "historical terminal baseline"
  );
  const validateCandidate = async (candidate) => {
    const reader = new api.GitObjectReader(root);
    try {
      return await api.validateTraceability(candidate, {
        repoRoot: root,
        gitObjects: reader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      });
    } finally {
      await reader.close();
    }
  };
  for (const [label, mutate] of [
    [
      "closed blocker",
      (candidate) => {
        candidate.blockers[0].status = "closed";
        candidate.blockers[0].closure_evidence = [];
      },
    ],
    [
      "ready requirement",
      (candidate) => {
        candidate.requirements[50].rust_cutover = {
          ...candidate.requirements[50].rust_cutover,
          readiness: "ready",
          implementation_subject_sha: baselineCommit,
          matrix_owner: "matrix_rust_sdk",
          surviving_raw_matrix_http: false,
          surviving_matrix_js_owner: false,
          gap_ids: [],
          gate_ids: [],
          blocker_ids: [],
        };
      },
    ],
  ]) {
    git(root, ["checkout", "--quiet", "--detach", baselineCommit]);
    const invalidIntermediate = structuredClone(baseline);
    mutate(invalidIntermediate);
    const invalidCommit = await commitV2Artifact(
      root,
      invalidIntermediate,
      audit,
      baselineCommit,
      `invalid historical ${label}`
    );
    const restored = structuredClone(baseline);
    restored.lifecycle.previous_artifact = artifactIdentity(
      root,
      invalidCommit
    );
    refreshLifecycle(restored, audit);
    assert.deepEqual(api.validateSchema("v2", restored), []);
    assert.ok(
      (await validateCandidate(restored)).some(
        ({ code }) => code === "V2_LIFECYCLE_HISTORY"
      ),
      `restored head must reject hidden historical ${label}`
    );
  }
});

test("first-parent lifecycle history handles path versions, reverts, and broken epochs", async (t) => {
  const root = await temporaryGitRepository(t);
  const absolute = path.join(root, api.V1_PATH);
  await mkdir(path.dirname(absolute), { recursive: true });
  const commitArtifact = async (value, label) => {
    await writeFile(absolute, Buffer.from(api.prettyJson(value)));
    git(root, ["add", api.V1_PATH]);
    git(root, ["commit", "--quiet", "-m", label]);
    return git(root, ["rev-parse", "HEAD"]);
  };
  await commitArtifact({ schema_version: "1.0" }, "legacy v1");
  const artifactA = { schema_version: "2.0", marker: "A" };
  const artifactB = { schema_version: "2.0", marker: "B" };
  const commitA = await commitArtifact(artifactA, "lifecycle A");
  await writeFile(path.join(root, "unrelated.txt"), "unchanged path\n");
  git(root, ["add", "unrelated.txt"]);
  git(root, ["commit", "--quiet", "-m", "unrelated commit"]);
  assert.equal(
    api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(artifactB))
    ).commit_sha,
    commitA,
    "unchanged commits must not become path versions"
  );
  const commitB = await commitArtifact(artifactB, "lifecycle B");
  const commitAAgain = await commitArtifact(artifactA, "lifecycle A again");
  assert.equal(
    api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(artifactA))
    ).commit_sha,
    commitB,
    "A to B to A must retain B as A's immediate predecessor"
  );
  assert.equal(
    api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(artifactB))
    ).commit_sha,
    commitAAgain,
    "dirty candidate bytes use the clean HEAD path version as predecessor"
  );

  const sideTree = git(root, ["rev-parse", `${commitB}^{tree}`]);
  const sideCommit = git(root, [
    "commit-tree",
    sideTree,
    "-p",
    commitAAgain,
    "-m",
    "second-parent-only path change",
  ]);
  const firstParentTree = git(root, ["rev-parse", `${commitAAgain}^{tree}`]);
  const mergeCommit = git(root, [
    "commit-tree",
    firstParentTree,
    "-p",
    commitAAgain,
    "-p",
    sideCommit,
    "-m",
    "first-parent lifecycle merge",
  ]);
  git(root, ["checkout", "--quiet", "--detach", mergeCommit]);
  assert.equal(
    api.derivePreviousV2ArtifactIdentity(
      root,
      Buffer.from(api.prettyJson(artifactA))
    ).commit_sha,
    commitB,
    "second-parent-only path changes must not enter first-parent history"
  );

  git(root, ["checkout", "--quiet", "--detach", commitAAgain]);
  await rm(absolute);
  git(root, ["add", "-A", "--", api.V1_PATH]);
  git(root, ["commit", "--quiet", "-m", "delete lifecycle path"]);
  await commitArtifact(artifactA, "reintroduce lifecycle path");
  assert.throws(
    () =>
      api.derivePreviousV2ArtifactIdentity(
        root,
        Buffer.from(api.prettyJson(artifactB))
      ),
    /broken after schema_version 2/iu,
    "deletion and reintroduction break the v2 epoch"
  );

  git(root, ["checkout", "--quiet", "--detach", commitAAgain]);
  await writeFile(absolute, "not json\n");
  git(root, ["add", api.V1_PATH]);
  git(root, ["commit", "--quiet", "-m", "invalid lifecycle bytes"]);
  assert.throws(
    () =>
      api.derivePreviousV2ArtifactIdentity(
        root,
        Buffer.from(api.prettyJson(artifactB))
      ),
    /broken after schema_version 2/iu
  );

  git(root, ["checkout", "--quiet", "--detach", commitAAgain]);
  await commitArtifact({ schema_version: "1.0" }, "v1 after v2");
  assert.throws(
    () =>
      api.derivePreviousV2ArtifactIdentity(
        root,
        Buffer.from(api.prettyJson(artifactB))
      ),
    /broken after schema_version 2/iu
  );

  const shallowParent = await mkdtemp(
    path.join(os.tmpdir(), "synara-r02-e1-lifecycle-shallow-")
  );
  t.after(async () => rm(shallowParent, { recursive: true, force: true }));
  const shallowRoot = path.join(shallowParent, "repository");
  const clone = spawnSync(
    "git",
    ["clone", "--quiet", "--depth=1", `file://${root}`, shallowRoot],
    { encoding: "utf8", shell: false, windowsHide: true }
  );
  assert.equal(clone.status, 0, clone.stderr);
  assert.equal(
    git(shallowRoot, ["rev-parse", "--is-shallow-repository"]),
    "true"
  );
  assert.throws(
    () =>
      api.derivePreviousV2ArtifactIdentity(
        shallowRoot,
        Buffer.from(api.prettyJson(artifactB))
      ),
    /complete non-shallow/iu
  );
});

test(BENCHMARK_TEST_NAME, async (t) => {
  if (process.env[BENCHMARK_CHILD_ENV] !== BENCHMARK_CHILD_VALUE) {
    runIsolatedBenchmarkChild();
    return;
  }
  const { parent, root } = await temporaryLocalGitClone(t);
  assert.match(
    git(root, ["cat-file", "-t", api.AUDITED_SOURCE_COMMIT]),
    /^commit$/u
  );
  const sourceBytes = git(
    root,
    ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`],
    { encoding: null }
  );
  const firstLineEnd = sourceBytes.indexOf(0x0a);
  assert.ok(firstLineEnd >= 0);
  const snippet = sourceBytes.subarray(0, firstLineEnd + 1);
  assert.equal(snippet.toString("utf8"), "{\n");

  const audit = productionShapedAuditFixture();
  for (const row of audit.rows) {
    const clause = row.clauses[0];
    const evidenceId = `EV-${clause.id.replace(".C", "-C")}-SRC-001`;
    clause.source_evidence = [
      {
        id: evidenceId,
        path: api.V1_PATH,
        symbol: "{",
        lines: { start: 1, end: 1 },
        source_sha: api.AUDITED_SOURCE_COMMIT,
        snippet_sha256: api.sha256(snippet),
        role: "reachability_entry",
        explanation:
          "Synthetic benchmark evidence reads an exact immutable Git blob.",
      },
    ];
    clause.causality.evidence_ids = [evidenceId];
    clause.reachability.entry_evidence_ids = [evidenceId];
    row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
  }
  audit.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(audit, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  assert.deepEqual(api.validateSchema("audit", audit), []);

  const auditCalls = [];
  const auditReader = new api.GitObjectReader(root, {
    spawnProcess(command, args, options) {
      auditCalls.push({ command, args: [...args], options });
      return spawn(command, args, options);
    },
  });
  let auditMeasurement;
  try {
    auditMeasurement = await measureOperation(async () => {
      const errors = await api.validateAudit(audit, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: auditReader,
      });
      assert.deepEqual(errors, []);
      const first = api.renderAuditMarkdown(audit).bytes;
      const second = api.renderAuditMarkdown(structuredClone(audit)).bytes;
      assert.deepEqual(first, second);
      return [api.sha256(first), api.sha256(second)];
    });
  } finally {
    await settlesWithin(auditReader.close(), 5_000);
  }

  const v1 = api.parseCanonicalJson(
    git(root, ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`], {
      encoding: null,
    })
  );
  const auditIdentity = syntheticAuditIdentity(audit);
  const v2 = api.migrateV1({
    v1,
    audit,
    sourceIdentity: {
      commit_sha: api.AUDITED_SOURCE_COMMIT,
      blob_oid: api.V1_BLOB,
      file_sha256: api.V1_SHA256,
    },
    auditIdentity,
  });
  assert.deepEqual(api.validateSchema("v2", v2), []);

  const v2Calls = [];
  const v2Reader = new api.GitObjectReader(root, {
    spawnProcess(command, args, options) {
      v2Calls.push({ command, args: [...args], options });
      return spawn(command, args, options);
    },
  });
  let v2Measurement;
  try {
    v2Measurement = await measureOperation(async () => {
      const errors = await api.validateTraceability(v2, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: v2Reader,
        durableAuditIdentity: auditIdentity,
        durableAudit: audit,
      });
      assert.deepEqual(errors, []);
      const first = api.renderTraceabilityMarkdown(v2).bytes;
      const second = api.renderTraceabilityMarkdown(structuredClone(v2)).bytes;
      assert.deepEqual(first, second);
      return [api.sha256(first), api.sha256(second)];
    });
  } finally {
    await settlesWithin(v2Reader.close(), 5_000);
  }

  for (const [name, reader, calls, measurement] of [
    ["audit", auditReader, auditCalls, auditMeasurement],
    ["v2", v2Reader, v2Calls, v2Measurement],
  ]) {
    const result = api.benchmarkResult({
      elapsedMs: measurement.elapsedMs,
      peakRssBytes: measurement.peakRssBytes,
      gitMetrics: reader.metrics,
      outputDigests: measurement.value,
    });
    assert.equal(result.within_time_budget, true, `${name} exceeded 30s`);
    assert.equal(result.within_rss_budget, true, `${name} exceeded 512 MiB`);
    assert.equal(result.git.batch_check_processes, 1);
    assert.equal(result.git.batch_content_processes, 1);
    assert.ok(result.git.content_transfers > 0);
    assert.equal(
      Object.values(result.git.content_transfers_by_oid).every(
        (count) => count === 1
      ),
      true,
      `${name} transferred a unique blob more than once`
    );
    assert.equal(
      Object.values(result.git.content_transfers_by_oid).reduce(
        (sum, count) => sum + count,
        0
      ),
      result.git.content_transfers
    );
    assert.equal(result.output_digests[0], result.output_digests[1]);
    assert.equal(reader.closed, true);
    assert.notEqual(reader.checkProcess.exitCode, null);
    assert.notEqual(reader.contentProcess.exitCode, null);
    assert.deepEqual(
      calls.map(({ args }) => args),
      [
        ["cat-file", "--batch-check"],
        ["cat-file", "--batch"],
      ]
    );
    for (const { command, args, options } of calls) {
      assert.equal(command, "git");
      assert.doesNotMatch(args.join(" "), /https?:|ssh:|git@/iu);
      assert.equal(options.shell, false);
      assert.equal(options.cwd, await realpath(root));
    }
  }

  await rm(parent, { recursive: true, force: true });
  await assert.rejects(() => stat(parent), { code: "ENOENT" });
  process.stdout.write(`# ${BENCHMARK_CHILD_MARKER}\n`);
});

test("119-row audit mutations fail at their stable semantic locations", async () => {
  const mutations = [
    {
      name: "deleted requirement",
      mutate(value) {
        value.rows.pop();
      },
      code: "AUDIT_ROW_COUNT",
    },
    {
      name: "duplicated requirement",
      mutate(value) {
        value.rows[1] = structuredClone(value.rows[0]);
      },
      code: "AUDIT_ROW_IDS",
    },
    {
      name: "moved requirement",
      mutate(value) {
        value.rows[0].section_id = "7.2";
        value.rows[0].audit.audited_payload_sha256 = api.auditedRowDigest(
          value.rows[0]
        );
      },
      code: "AUDIT_SECTION_COUNTS",
    },
    {
      name: "partial clause rolled up as implemented",
      mutate(value) {
        value.rows[0].current_product.status = "implemented";
        value.rows[0].audit.audited_payload_sha256 = api.auditedRowDigest(
          value.rows[0]
        );
      },
      code: "AUDIT_ROLLUP",
    },
    {
      name: "current product promoted to Rust-ready",
      mutate(value) {
        value.rows[0].rust_cutover.readiness = "ready";
        value.rows[0].audit.audited_payload_sha256 = api.auditedRowDigest(
          value.rows[0]
        );
      },
      code: "AUDIT_RUST_READY",
    },
  ];
  for (const mutation of mutations) {
    const artifact = productionShapedAuditFixture();
    mutation.mutate(artifact);
    const result = await api.validateAudit(artifact, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
    });
    assert.ok(
      result.some(({ code }) => code === mutation.code),
      `${mutation.name}: ${JSON.stringify(result)}`
    );
  }
});

test("schema and semantic audit paths agree on structural mutation pointers", async () => {
  const corpus = [
    (value) => {
      value.unexpected = true;
    },
    (value) => {
      delete value.rows[0].plan;
      value.rows[0].audit.audited_payload_sha256 = api.auditedRowDigest(
        value.rows[0]
      );
    },
    (value) => {
      value.rows[0].clauses[0].outcomes.success = [];
      value.rows[0].audit.audited_payload_sha256 = api.auditedRowDigest(
        value.rows[0]
      );
    },
  ];
  for (const mutate of corpus) {
    const artifact = productionShapedAuditFixture();
    mutate(artifact);
    const schemaErrors = api.validateSchema("audit", artifact);
    const combined = await api.validateAudit(artifact, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
    });
    assert.ok(schemaErrors.length > 0);
    const primary = schemaErrors[0];
    assert.ok(
      combined.some(
        ({ code, pointer }) =>
          code === primary.code && pointer === primary.pointer
      ),
      JSON.stringify({ primary, combined })
    );
  }
});

test("audit derives the 23 exact status corrections instead of trusting the summary", async () => {
  assert.equal(api.EXPECTED_73_77_SUBTOTAL, 11);
  assert.equal(api.EXPECTED_73_77_IDENTIFIED_CORRECTION_COUNT, 16);
  assert.deepEqual(
    api.EXPECTED_73_77_NON_STATUS_CORRECTIONS.map(
      ({ requirement_id }) => requirement_id
    ),
    ["FR-7.5-004", "FR-7.5-009", "FR-7.6-003", "FR-7.7-003", "FR-7.7-009"]
  );
  const forgedSubtotal = productionShapedAuditFixture();
  forgedSubtotal.coverage.sections_7_3_through_7_7_status_correction_subtotal = 16;
  const forgedSubtotalResult = await api.validateAudit(forgedSubtotal, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  assert.ok(
    forgedSubtotalResult.some(
      ({ code, pointer }) =>
        code === "AUDIT_SUBTOTAL" &&
        pointer ===
          "/coverage/sections_7_3_through_7_7_status_correction_subtotal"
    )
  );
  const forgedGeneralCorrection = productionShapedAuditFixture();
  forgedGeneralCorrection.coverage.sections_7_3_through_7_7_non_status_correction_manifest[0].requirement_id =
    "FR-7.5-005";
  const forgedGeneralResult = await api.validateAudit(forgedGeneralCorrection, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  assert.ok(
    forgedGeneralResult.some(
      ({ code, pointer }) =>
        code === "AUDIT_NON_STATUS_CORRECTIONS" &&
        pointer ===
          "/coverage/sections_7_3_through_7_7_non_status_correction_manifest"
    )
  );
  const forgedGeneralCount = productionShapedAuditFixture();
  forgedGeneralCount.coverage.sections_7_3_through_7_7_identified_correction_count = 15;
  const forgedGeneralCountResult = await api.validateAudit(forgedGeneralCount, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  assert.ok(
    forgedGeneralCountResult.some(
      ({ code, pointer }) =>
        code === "AUDIT_IDENTIFIED_CORRECTIONS" &&
        pointer ===
          "/coverage/sections_7_3_through_7_7_identified_correction_count"
    )
  );

  const artifact = productionShapedAuditFixture();
  const row = artifact.rows.find(({ id }) => id === "FR-7.1-004");
  row.current_product.status_correction = {
    recorded_status: "partial",
    audited_status: "partial",
    changed: false,
    explanation: "Mutation removes one required correction.",
  };
  row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
  artifact.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(artifact, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  const result = await api.validateAudit(artifact, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  assert.ok(
    result.some(({ code }) => code === "AUDIT_CORRECTIONS"),
    "checker trusted the declared count after an exact correction disappeared"
  );

  const shifted = productionShapedAuditFixture();
  const removed = shifted.rows.find(({ id }) => id === "FR-7.1-004");
  removed.current_product.status_correction = {
    recorded_status: "partial",
    audited_status: "partial",
    changed: false,
    explanation: "Mutation moves a correction away from its pinned row.",
  };
  const invented = shifted.rows.find(({ id }) => id === "FR-7.1-001");
  invented.current_product.status_correction = {
    recorded_status: "implemented",
    audited_status: "partial",
    changed: true,
    explanation: "Mutation invents a replacement correction.",
  };
  for (const row of [removed, invented])
    row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
  shifted.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(shifted, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  const shiftedResult = await api.validateAudit(shifted, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  assert.ok(
    shiftedResult.some(({ code }) => code === "AUDIT_CORRECTIONS"),
    "checker accepted a count-preserving correction substitution"
  );

  const shiftedAcrossSubtotalBoundary = productionShapedAuditFixture();
  const inRange = shiftedAcrossSubtotalBoundary.rows.find(
    ({ id }) => id === "FR-7.3-007"
  );
  inRange.current_product.status_correction = {
    recorded_status: "partial",
    audited_status: "partial",
    changed: false,
    explanation: "Mutation removes an in-range correction.",
  };
  const outOfRange = shiftedAcrossSubtotalBoundary.rows.find(
    ({ id }) => id === "FR-7.8-001"
  );
  outOfRange.current_product.status_correction = {
    recorded_status: "implemented",
    audited_status: "partial",
    changed: true,
    explanation: "Mutation invents an out-of-range correction.",
  };
  for (const row of [inRange, outOfRange])
    row.audit.audited_payload_sha256 = api.auditedRowDigest(row);
  shiftedAcrossSubtotalBoundary.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(shiftedAcrossSubtotalBoundary, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  const boundaryResult = await api.validateAudit(
    shiftedAcrossSubtotalBoundary,
    {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: null,
      skipGitEvidence: true,
    }
  );
  assert.ok(
    boundaryResult.some(
      ({ code, pointer }) =>
        code === "AUDIT_SUBTOTAL" &&
        pointer ===
          "/coverage/sections_7_3_through_7_7_status_correction_subtotal"
    )
  );
});

test("audit retains unresolved decisions and R037/R038 closure targets", async () => {
  const artifact = productionShapedAuditFixture();
  artifact.architecture_decisions[0].status = "approved";
  artifact.blockers_and_risks.find(
    ({ id }) => id === "MRSDK-R037"
  ).closure_criteria = ["Generic OAuth test."];
  artifact.blockers_and_risks.find(
    ({ id }) => id === "MRSDK-R038"
  ).closure_criteria = ["Generic notification test."];
  artifact.digests.canonical_payload_sha256 = api.sha256(
    api.canonicalize(
      api.cloneWithoutPointers(artifact, [
        "/review",
        "/digests/canonical_payload_sha256",
      ])
    )
  );
  const result = await api.validateAudit(artifact, {
    repoRoot: REPOSITORY_ROOT,
    gitObjects: null,
    skipGitEvidence: true,
  });
  const codes = new Set(result.map(({ code }) => code));
  for (const code of ["AUDIT_DECISION_STATE", "AUDIT_R037", "AUDIT_R038"]) {
    assert.ok(codes.has(code), `missing ${code}`);
  }
});

test("pinned security risk authority rejects semantic drift without requiring HEAD retention", async (t) => {
  const { root } = await temporaryLocalGitClone(t);
  const reader = new api.GitObjectReader(root);
  t.after(async () => reader.close());
  const validate = (candidate) =>
    api.validateAudit(refreshAuditDigests(candidate), {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: reader,
    });
  const base = productionShapedAuditFixture();
  assert.deepEqual(await validate(structuredClone(base)), []);

  const scenarios = [
    {
      name: "missing central risk",
      code: "AUDIT_RISK_MAPPING",
      mutate(candidate) {
        candidate.blockers_and_risks = candidate.blockers_and_risks.filter(
          ({ id }) => id !== "MRSDK-R036"
        );
      },
    },
    {
      name: "duplicate central risk",
      code: "AUDIT_RISK_IDS",
      mutate(candidate) {
        candidate.blockers_and_risks.push(
          structuredClone(candidate.blockers_and_risks[0])
        );
      },
    },
    {
      name: "R024 severity downgrade",
      code: "AUDIT_R024",
      mutate(candidate) {
        candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R024"
        ).severity = "medium";
      },
    },
    {
      name: "R027 source taxonomy leaks into normalized severity",
      code: "AUDIT_R027",
      mutate(candidate) {
        candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R027"
        ).severity = "critical_data_loss";
      },
    },
    {
      name: "central status drift",
      code: "AUDIT_R037",
      mutate(candidate) {
        candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R037"
        ).status = "mitigating";
      },
    },
    {
      name: "owner drift",
      code: "AUDIT_R024",
      mutate(candidate) {
        candidate.blockers_and_risks
          .find(({ id }) => id === "MRSDK-R024")
          .owner_task_ids.pop();
      },
    },
    {
      name: "threat drift",
      code: "AUDIT_R030",
      mutate(candidate) {
        candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R030"
        ).threat_ids = [];
      },
    },
    {
      name: "closure shortening",
      code: "AUDIT_R036",
      mutate(candidate) {
        candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R036"
        ).closure_criteria = ["Forwarding tests pass."];
      },
    },
    {
      name: "T17 is confused with boundary B09",
      code: "AUDIT_R038",
      mutate(candidate) {
        const risk = candidate.blockers_and_risks.find(
          ({ id }) => id === "MRSDK-R038"
        );
        risk.threat_ids = ["B09"];
        risk.boundary_ids = ["TM-T17"];
      },
    },
    {
      name: "FR-7.4-011 fabricated as blocker identity",
      code: "AUDIT_FR_7_4_011_RISK",
      mutate(candidate) {
        candidate.blockers_and_risks.push({
          ...structuredClone(
            candidate.blockers_and_risks.find(({ id }) => id === "MRSDK-R036")
          ),
          id: "FR-7.4-011",
        });
      },
    },
    {
      name: "risk-register provenance hash drift",
      code: "AUDIT_RISK_PROVENANCE",
      mutate(candidate) {
        candidate.subject.risk_register.file_sha256 = "0".repeat(64);
      },
    },
    {
      name: "risk-register provenance commit drift",
      code: "AUDIT_RISK_PROVENANCE",
      mutate(candidate) {
        candidate.subject.risk_register.commit_sha = "0".repeat(40);
      },
    },
    {
      name: "risk-register provenance path drift",
      code: "AUDIT_RISK_PROVENANCE",
      mutate(candidate) {
        candidate.subject.risk_register.path =
          "docs/matrix-rust-sdk/other-risk-register.json";
      },
    },
    {
      name: "risk-register provenance blob drift",
      code: "AUDIT_RISK_PROVENANCE",
      mutate(candidate) {
        candidate.subject.risk_register.blob_oid = "0".repeat(40);
      },
    },
    {
      name: "risk-register canonical semantic hash drift",
      code: "AUDIT_RISK_PROVENANCE",
      mutate(candidate) {
        candidate.subject.risk_register.canonical_sha256 = "0".repeat(64);
      },
    },
  ];
  for (const scenario of scenarios) {
    const candidate = structuredClone(base);
    scenario.mutate(candidate);
    const errors = await validate(candidate);
    assert.ok(
      errors.some(({ code }) => code === scenario.code),
      `${scenario.name}: ${JSON.stringify(errors)}`
    );
  }

  await writeFile(
    path.join(root, api.RISK_REGISTER_PATH),
    `${await readFile(path.join(root, api.RISK_REGISTER_PATH), "utf8")}\n`
  );
  assert.deepEqual(await validate(structuredClone(base)), []);
  git(root, ["add", api.RISK_REGISTER_PATH]);
  git(root, ["commit", "--quiet", "-m", "replace risk authority bytes"]);
  assert.deepEqual(
    await validate(structuredClone(base)),
    [],
    "the pinned historical authority remains valid without current-HEAD byte retention"
  );

  const unrelatedRoot = await temporaryGitRepository(t);
  await commitFixture(unrelatedRoot);
  const unrelatedReader = new api.GitObjectReader(unrelatedRoot);
  try {
    assert.ok(
      (
        await api.validateAudit(structuredClone(base), {
          repoRoot: REPOSITORY_ROOT,
          gitObjects: unrelatedReader,
        })
      ).some(({ code }) => code === "RISK_REGISTER_GIT")
    );
  } finally {
    await unrelatedReader.close();
  }
});

test("row digest excludes exactly the five mutable audit pointers", () => {
  const row = {
    id: "FR-7.4-011",
    audit: {
      state: "reviewed_changes_required",
      reviewer: null,
      reviewed_at: null,
      report_ids: [],
      audited_payload_sha256: "0".repeat(64),
      subject_sha: "2".repeat(40),
    },
    current_product: { status: "partial" },
  };
  const original = api.auditedRowDigest(row);
  for (const [field, value] of [
    ["state", "accepted"],
    ["reviewer", "Independent Reviewer"],
    ["reviewed_at", "2026-07-26T12:00:00Z"],
    ["report_ids", ["R0.2-E-REPORT"]],
    ["audited_payload_sha256", "f".repeat(64)],
  ]) {
    const changed = structuredClone(row);
    changed.audit[field] = value;
    assert.equal(api.auditedRowDigest(changed), original, field);
  }
  const stableChange = structuredClone(row);
  stableChange.current_product.status = "implemented";
  assert.notEqual(api.auditedRowDigest(stableChange), original);
  const extraAuditPointer = structuredClone(row);
  extraAuditPointer.audit.unreviewed_note = "material";
  assert.notEqual(api.auditedRowDigest(extraAuditPointer), original);
  const missingPointer = structuredClone(row);
  delete missingPointer.audit.reviewer;
  assert.throws(
    () => api.auditedRowDigest(missingPointer),
    /mutable pointer/iu
  );
});

test("repository evidence paths reject traversal and all non-portable forms", () => {
  for (const candidate of [
    "",
    "/absolute",
    "../outside",
    "src/../outside",
    "src/./file",
    "src//file",
    "src\\file",
    "C:\\repo\\file",
    "src/file/",
    "src\u0000file",
    "e\u0301.txt",
  ]) {
    assert.throws(
      () => api.normalizeRepositoryPath(candidate),
      undefined,
      candidate
    );
  }
  assert.equal(api.normalizeRepositoryPath("src/\u00e9.txt"), "src/\u00e9.txt");
});

test("temporary Git fixtures preserve LF, CRLF, EOF, binary, and symlink identities", async (t) => {
  const root = await temporaryGitRepository(t);
  const commit = await commitFixture(root);
  assert.match(commit, /^[0-9a-f]{40}$/u);
  assert.equal(git(root, ["cat-file", "-t", `${commit}:src/lf.txt`]), "blob");
  assert.equal(git(root, ["cat-file", "-t", `${commit}:src/link.txt`]), "blob");
  const lf = git(root, ["show", `${commit}:src/lf.txt`], { encoding: null });
  const crlf = git(root, ["show", `${commit}:src/crlf.txt`], {
    encoding: null,
  });
  const eof = git(root, ["show", `${commit}:src/no-final-lf.txt`], {
    encoding: null,
  });
  assert.deepEqual(lf, Buffer.from("alpha\nbeta\n"));
  assert.deepEqual(crlf, Buffer.from("one\r\ntwo\r\n"));
  assert.deepEqual(eof, Buffer.from("last line"));
  assert.notEqual(api.sha256(lf), api.sha256(crlf));
});

test("Git object reader batches processes, caches blobs and misses, and closes", async (t) => {
  const root = await temporaryGitRepository(t);
  const commit = await commitFixture(root);
  const reader = new api.GitObjectReader(root);
  t.after(async () => reader.close());

  assert.equal(reader.metrics.batch_check_processes, 1);
  assert.equal(reader.metrics.batch_content_processes, 1);
  const first = await reader.blobAt(commit, "src/lf.txt");
  const transfersAfterFirstRead = reader.metrics.content_transfers;
  const second = await reader.blobAt(commit, "src/lf.txt");
  assert.deepEqual(first, Buffer.from("alpha\nbeta\n"));
  assert.strictEqual(first, second);
  assert.ok(transfersAfterFirstRead >= 1);
  assert.equal(reader.metrics.content_transfers, transfersAfterFirstRead);

  assert.equal(await reader.resolvePath(commit, "src/missing.txt"), null);
  assert.equal(await reader.resolvePath(commit, "src/missing.txt"), null);
  assert.equal(reader.metrics.negative_cache_hits, 1);
  await assert.rejects(() => reader.resolvePath(commit, "../outside"));
  await assert.rejects(() => reader.resolvePath(commit, "src/link.txt"));

  git(root, [
    "update-index",
    "--add",
    "--cacheinfo",
    "160000",
    commit,
    "vendor/submodule",
  ]);
  git(root, ["commit", "--quiet", "-m", "add gitlink fixture"]);
  const gitlinkCommit = git(root, ["rev-parse", "HEAD"]);
  await assert.rejects(() =>
    reader.resolvePath(gitlinkCommit, "vendor/submodule/file.txt")
  );

  await writeFile(path.join(root, "src/lf.txt"), "changed\n", "utf8");
  git(root, ["add", "src/lf.txt"]);
  git(root, ["commit", "--quiet", "-m", "historical source fixture"]);
  const newer = git(root, ["rev-parse", "HEAD"]);
  assert.deepEqual(
    await reader.blobAt(commit, "src/lf.txt"),
    Buffer.from("alpha\nbeta\n")
  );
  assert.deepEqual(
    await reader.blobAt(newer, "src/lf.txt"),
    Buffer.from("changed\n")
  );

  await reader.close();
  assert.equal(reader.closed, true);
  assert.notEqual(reader.checkProcess.exitCode, null);
  assert.notEqual(reader.contentProcess.exitCode, null);
});

test("Git batch processes use explicit no-shell hidden-window options", async (t) => {
  const root = await temporaryGitRepository(t);
  const calls = [];
  const reader = new api.GitObjectReader(root, {
    spawnProcess(command, args, options) {
      calls.push({ command, args, options });
      const child = new EventEmitter();
      child.stdin = new PassThrough();
      child.stdout = new PassThrough();
      child.stderr = new PassThrough();
      child.exitCode = null;
      child.signalCode = null;
      child.stdin.on("finish", () => {
        child.stdout.end();
        child.stderr.end();
        child.exitCode = 0;
        child.emit("close", 0, null);
      });
      return child;
    },
  });
  assert.equal(calls.length, 2);
  for (const { command, args, options } of calls) {
    assert.equal(command, "git");
    assert.ok(Array.isArray(args));
    assert.equal(options.cwd, await realpath(root));
    assert.equal(options.shell, false);
    assert.equal(options.windowsHide, true);
    assert.deepEqual(options.stdio, ["pipe", "pipe", "pipe"]);
  }
  await reader.close();
});

test("every synchronous Git process uses the shared explicit safe options", () => {
  const source = readFileSync(LIBRARY_PATH, "utf8");
  assert.match(
    source,
    /function gitProcessOptions\(cwd\)[\s\S]*?cwd,[\s\S]*?env: safeGitEnvironment\(\),[\s\S]*?shell: false,[\s\S]*?windowsHide: true,/u
  );
  assert.match(
    source,
    /spawnSync\("git", args, \{[\s\S]*?\.\.\.gitProcessOptions\(root\)/u
  );
  assert.match(
    source,
    /\["merge-base", "--is-ancestor", ancestor, descendant\],[\s\S]*?gitProcessOptions\(root\)/u
  );
  assert.match(
    source,
    /function runGitBytes[\s\S]*?spawnSync\("git", args, \{[\s\S]*?\.\.\.gitProcessOptions\(root\)/u
  );
});

test("Git object reader reports SIGTERM batch failure and still closes its peer", async (t) => {
  const root = await temporaryGitRepository(t);
  const reader = new api.GitObjectReader(root);
  reader.checkProcess.kill("SIGTERM");
  await assert.rejects(() => reader.close(), /batch process failed/iu);
  assert.equal(reader.closed, true);
  assert.equal(reader.checkProcess.signalCode, "SIGTERM");
  assert.notEqual(reader.contentProcess.exitCode, null);
});

test("Git batch parser failures settle and close both children without leaks", async (t) => {
  const root = await temporaryGitRepository(t);
  const oid = "a".repeat(40);
  const cases = [
    {
      name: "malformed batch-check header",
      operation: (reader) => reader.object("HEAD"),
      handlers: {
        check: {
          onRequest(_request, child) {
            child.stdout.write("not-a-git-header\n");
          },
        },
      },
      error: /malformed Git batch-check response/iu,
    },
    {
      name: "truncated batch-check header",
      operation: (reader) => reader.object("HEAD"),
      handlers: {
        check: {
          onRequest(_request, child) {
            child.stdout.end(`${oid} blob`);
          },
        },
      },
      error: /unexpected Git batch EOF/iu,
    },
    {
      name: "malformed batch-content header",
      operation: (reader) => reader.bytes(oid),
      handlers: {
        content: {
          onRequest(_request, child) {
            child.stdout.write(`${"b".repeat(40)} blob 3\nabc\n`);
          },
        },
      },
      error: /malformed Git batch content header/iu,
    },
    {
      name: "truncated batch-content payload",
      operation: (reader) => reader.bytes(oid),
      handlers: {
        content: {
          onRequest(_request, child) {
            child.stdout.end(`${oid} blob 5\nabc`);
          },
        },
      },
      error: /unexpected Git batch EOF/iu,
    },
  ];

  for (const scenario of cases) {
    await t.test(scenario.name, async () => {
      const fake = scriptedGitProcesses(scenario.handlers);
      const reader = new api.GitObjectReader(root, {
        spawnProcess: fake.spawnProcess,
      });
      await assert.rejects(
        () => settlesWithin(scenario.operation(reader)),
        scenario.error
      );
      await settlesWithin(reader.close());
      assert.equal(reader.closed, true);
      assertFakeGitChildrenClosed(fake.children);
    });
  }
});

test("Git batch nonzero exit and stderr flood fail closed after draining both children", async (t) => {
  const root = await temporaryGitRepository(t);
  const cases = [
    {
      name: "nonzero exit with empty stderr",
      handlers: { check: { exitCode: 23 } },
    },
    {
      name: "stderr flood",
      handlers: {
        check: {
          onFinish(child) {
            for (let index = 0; index < 512; index += 1)
              child.stderr.write(Buffer.alloc(4_096, 0x78));
          },
        },
      },
    },
  ];
  for (const scenario of cases) {
    await t.test(scenario.name, async () => {
      const fake = scriptedGitProcesses(scenario.handlers);
      const reader = new api.GitObjectReader(root, {
        spawnProcess: fake.spawnProcess,
      });
      await assert.rejects(
        () => settlesWithin(reader.close()),
        /Git batch process failed/iu
      );
      assert.equal(reader.closed, true);
      assertFakeGitChildrenClosed(fake.children);
    });
  }
});

test("source evidence hashes exact LF/CRLF/EOF bytes and rejects binary snippets", async (t) => {
  const root = await temporaryGitRepository(t);
  const commit = await commitFixture(root);
  const reader = new api.GitObjectReader(root);
  t.after(async () => reader.close());

  const crlfSnippet = Buffer.from("two\r\n", "utf8");
  assert.deepEqual(
    await api.verifySourceEvidence(
      {
        source_sha: commit,
        path: "src/crlf.txt",
        symbol: "two",
        lines: { start: 2, end: 2 },
        snippet_sha256: api.sha256(crlfSnippet),
      },
      reader
    ),
    []
  );
  assert.deepEqual(
    await api.verifySourceEvidence(
      {
        source_sha: commit,
        path: "src/no-final-lf.txt",
        symbol: "last line",
        lines: { start: 1, end: 1 },
        snippet_sha256: api.sha256(Buffer.from("last line")),
      },
      reader
    ),
    []
  );
  const normalizedDigest = await api.verifySourceEvidence(
    {
      source_sha: commit,
      path: "src/crlf.txt",
      symbol: "two",
      lines: { start: 2, end: 2 },
      snippet_sha256: api.sha256(Buffer.from("two\n")),
    },
    reader
  );
  assert.ok(normalizedDigest.some(({ code }) => code === "EVIDENCE_DIGEST"));
  const wrongSymbol = await api.verifySourceEvidence(
    {
      source_sha: commit,
      path: "src/lf.txt",
      symbol: "gamma",
      lines: { start: 1, end: 1 },
      snippet_sha256: api.sha256(Buffer.from("alpha\n")),
    },
    reader
  );
  assert.ok(wrongSymbol.some(({ code }) => code === "EVIDENCE_SYMBOL"));
  const lineBounds = await api.verifySourceEvidence(
    {
      source_sha: commit,
      path: "src/lf.txt",
      symbol: "alpha",
      lines: { start: 0, end: 3 },
      snippet_sha256: "0".repeat(64),
    },
    reader
  );
  assert.ok(lineBounds.some(({ code }) => code === "EVIDENCE_LINES"));
  const missing = await api.verifySourceEvidence(
    {
      source_sha: commit,
      path: "src/missing.txt",
      symbol: "missing",
      lines: { start: 1, end: 1 },
      snippet_sha256: "0".repeat(64),
    },
    reader
  );
  assert.ok(missing.some(({ code }) => code === "EVIDENCE_MISSING"));
  const binary = await api.verifySourceEvidence(
    {
      source_sha: commit,
      path: "src/non-utf8.bin",
      symbol: "f",
      lines: { start: 1, end: 1 },
      snippet_sha256: api.sha256(Buffer.from([0x66, 0x80, 0x6f])),
    },
    reader
  );
  assert.ok(binary.some(({ code }) => code === "EVIDENCE_UTF8"));
});

test("existing-test evidence is Git-verified and cannot satisfy runtime causality", async (t) => {
  const { root } = await temporaryLocalGitClone(t);
  const sourceBytes = git(
    root,
    ["show", `${api.AUDITED_SOURCE_COMMIT}:${api.V1_PATH}`],
    { encoding: null }
  );
  const snippet = sourceBytes.subarray(0, sourceBytes.indexOf(0x0a) + 1);
  const audit = productionShapedAuditFixture();
  const clause = audit.rows[0].clauses[0];
  const existingTest = {
    id: "TST-FR-7.1-001-EX-001",
    path: api.V1_PATH,
    symbol: "{",
    lines: { start: 1, end: 1 },
    source_sha: api.AUDITED_SOURCE_COMMIT,
    snippet_sha256: api.sha256(snippet),
    assertions: ["The immutable test-source snippet is inspected exactly."],
    limitations: ["Test source is not runtime validation evidence."],
  };
  clause.existing_tests = [existingTest];
  refreshAuditDigests(audit);
  const reader = new api.GitObjectReader(root);
  t.after(async () => reader.close());
  assert.deepEqual(
    await api.validateAudit(audit, {
      repoRoot: REPOSITORY_ROOT,
      gitObjects: reader,
    }),
    []
  );

  const wrongPath = structuredClone(audit);
  wrongPath.rows[0].clauses[0].existing_tests[0].path = "missing/test.mjs";
  refreshAuditDigests(wrongPath);
  assert.ok(
    (
      await api.validateAudit(wrongPath, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: reader,
      })
    ).some(
      ({ code, pointer }) =>
        code === "EVIDENCE_MISSING" && pointer.includes("/existing_tests/0")
    )
  );

  const wrongDigest = structuredClone(audit);
  wrongDigest.rows[0].clauses[0].existing_tests[0].snippet_sha256 = "0".repeat(
    64
  );
  refreshAuditDigests(wrongDigest);
  assert.ok(
    (
      await api.validateAudit(wrongDigest, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: reader,
      })
    ).some(
      ({ code, pointer }) =>
        code === "EVIDENCE_DIGEST" && pointer.includes("/existing_tests/0")
    )
  );

  const nonexistentCommit = structuredClone(audit);
  nonexistentCommit.rows[0].clauses[0].existing_tests[0].source_sha =
    "f".repeat(40);
  refreshAuditDigests(nonexistentCommit);
  assert.ok(
    (
      await api.validateAudit(nonexistentCommit, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: reader,
      })
    ).some(({ code }) => code === "EVIDENCE_SUBJECT")
  );
  assert.ok(
    (
      await api.verifySourceEvidence(
        nonexistentCommit.rows[0].clauses[0].existing_tests[0],
        reader
      )
    ).some(({ code }) => code === "EVIDENCE_GIT")
  );

  const duplicate = structuredClone(audit);
  duplicate.rows[1].clauses[0].existing_tests = [structuredClone(existingTest)];
  refreshAuditDigests(duplicate);
  assert.ok(
    (
      await api.validateAudit(duplicate, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
      })
    ).some(({ code }) => code === "EVIDENCE_ID_DUPLICATE")
  );

  for (const evidenceId of ["EV-NOT-FOUND", existingTest.id]) {
    const dangling = structuredClone(audit);
    dangling.rows[0].clauses[0].causality.evidence_ids = [evidenceId];
    refreshAuditDigests(dangling);
    assert.ok(
      (
        await api.validateAudit(dangling, {
          repoRoot: REPOSITORY_ROOT,
          gitObjects: null,
          skipGitEvidence: true,
        })
      ).some(({ code }) => code === "EVIDENCE_ID_DANGLING"),
      evidenceId
    );
  }

  const testSourceCausality = structuredClone(audit);
  const testSourceClause = testSourceCausality.rows[0].clauses[0];
  const testSourceId = "EV-FR-7.1-001-C001-SRC-001";
  testSourceClause.source_evidence = [
    {
      id: testSourceId,
      path: api.V1_PATH,
      symbol: "{",
      lines: { start: 1, end: 1 },
      source_sha: api.AUDITED_SOURCE_COMMIT,
      snippet_sha256: api.sha256(snippet),
      role: "test_source",
      explanation: "Test-source inspection is not runtime causality evidence.",
    },
  ];
  testSourceClause.causality.evidence_ids = [testSourceId];
  refreshAuditDigests(testSourceCausality);
  assert.ok(
    (
      await api.validateAudit(testSourceCausality, {
        repoRoot: REPOSITORY_ROOT,
        gitObjects: null,
        skipGitEvidence: true,
      })
    ).some(({ code }) => code === "EVIDENCE_ID_DANGLING")
  );
});

test("reachability references are clause-local, role-typed, owned, and non-contradictory", async () => {
  const base = productionShapedAuditFixture();
  const validate = async (candidate) =>
    api.validateAudit(refreshAuditDigests(candidate), {
      repoRoot: REPOSITORY_ROOT,
      skipGitEvidence: true,
    });
  assert.deepEqual(await validate(structuredClone(base)), []);

  const scenarios = [
    {
      name: "count-preserving globally valid cross-row entry swap",
      code: "REACHABILITY_ENTRY_REFERENCE",
      mutate(candidate) {
        const first = candidate.rows[0].clauses[0].reachability;
        const second = candidate.rows[1].clauses[0].reachability;
        [first.entry_evidence_ids, second.entry_evidence_ids] = [
          second.entry_evidence_ids,
          first.entry_evidence_ids,
        ];
      },
    },
    {
      name: "wrong source-evidence role",
      code: "REACHABILITY_ENTRY_REFERENCE",
      mutate(candidate) {
        candidate.rows[0].clauses[0].source_evidence[0].role = "implementation";
      },
    },
    {
      name: "existing test cannot be an entry",
      code: "REACHABILITY_ENTRY_REFERENCE",
      mutate(candidate) {
        const clause = candidate.rows[0].clauses[0];
        clause.existing_tests = [
          {
            id: "TST-FR-7.1-001-EX-001",
            path: api.AUDITED_PLAN_PATH,
            symbol: "Matrix Rust SDK",
            lines: { start: 1, end: 1 },
            source_sha: api.AUDITED_SOURCE_COMMIT,
            snippet_sha256:
              "0e778c34a2f5b6da49f87d6b0f14780fa4cd329a57777aeb038daecaf72f466b",
            assertions: ["Synthetic existing test."],
            limitations: ["Not reachability evidence."],
          },
        ];
        clause.reachability.entry_evidence_ids = [clause.existing_tests[0].id];
      },
    },
    {
      name: "source evidence cannot be absence evidence",
      code: "REACHABILITY_ABSENCE_REFERENCE",
      mutate(candidate) {
        const clause = candidate.rows[0].clauses[0];
        clause.reachability.absence_evidence_ids = [
          clause.source_evidence[0].id,
        ];
      },
    },
    {
      name: "reachable cannot carry contradictory absence evidence",
      code: "REACHABILITY_SHAPE",
      mutate(candidate) {
        const clause = candidate.rows[0].clauses[0];
        const absenceId = "EV-FR-7.1-001-C001-ABS-001";
        clause.absence_evidence = [
          {
            id: absenceId,
            source_sha: api.AUDITED_SOURCE_COMMIT,
            expression: "not-present",
            mode: "literal",
            roots: ["src"],
            exclusions: [],
            expected_match_count: 0,
            explanation: "Synthetic absence record.",
          },
        ];
        clause.reachability.absence_evidence_ids = [absenceId];
      },
    },
    {
      name: "duplicate reachability ID",
      code: "REACHABILITY_ID_OWNERSHIP",
      mutate(candidate) {
        candidate.rows[1].clauses[0].reachability.id =
          candidate.rows[0].clauses[0].reachability.id;
      },
    },
    {
      name: "misowned reachability ID",
      code: "REACHABILITY_ID_OWNERSHIP",
      mutate(candidate) {
        candidate.rows[0].clauses[0].reachability.id = "REACH-FR-7.1-002-999";
      },
    },
  ];
  for (const scenario of scenarios) {
    const candidate = structuredClone(base);
    scenario.mutate(candidate);
    const errors = await validate(candidate);
    assert.ok(
      errors.some(({ code }) => code === scenario.code),
      `${scenario.name}: ${JSON.stringify(errors)}`
    );
  }
});

test("absence evidence reruns literal/regex searches at the pinned commit", async (t) => {
  const root = await temporaryGitRepository(t);
  const commit = await commitFixture(root);
  const reader = new api.GitObjectReader(root);
  t.after(async () => reader.close());
  const base = {
    source_sha: commit,
    roots: ["src"],
    exclusions: ["src/non-utf8.bin", "src/link.txt"],
    expected_match_count: 0,
    explanation: "Pinned absence check.",
  };
  assert.deepEqual(
    await api.verifyAbsenceEvidence(
      { ...base, mode: "literal", expression: "not-present" },
      reader
    ),
    []
  );
  const found = await api.verifyAbsenceEvidence(
    { ...base, mode: "regex", expression: "alpha|beta" },
    reader
  );
  assert.ok(found.some(({ code }) => code === "ABSENCE_MATCHES"));
  const unreadable = await api.verifyAbsenceEvidence(
    {
      ...base,
      exclusions: ["src/link.txt"],
      mode: "literal",
      expression: "not-present",
    },
    reader
  );
  assert.ok(unreadable.some(({ code }) => code === "ABSENCE_UTF8"));
  const linked = await api.verifyAbsenceEvidence(
    {
      ...base,
      exclusions: ["src/non-utf8.bin"],
      mode: "literal",
      expression: "not-present",
    },
    reader
  );
  assert.ok(linked.some(({ code }) => code === "ABSENCE_SYMLINK"));

  git(root, [
    "update-index",
    "--add",
    "--cacheinfo",
    "160000",
    commit,
    "vendor/submodule",
  ]);
  git(root, ["commit", "--quiet", "-m", "absence gitlink fixture"]);
  const gitlinkCommit = git(root, ["rev-parse", "HEAD"]);
  const gitlinked = await api.verifyAbsenceEvidence(
    {
      ...base,
      source_sha: gitlinkCommit,
      roots: ["vendor"],
      exclusions: [],
      mode: "literal",
      expression: "not-present",
    },
    reader
  );
  assert.ok(gitlinked.some(({ code }) => code === "ABSENCE_GITLINK"));
});

test("transactional twin writes roll back both files after the second install fails", async (t) => {
  const root = await realpath(
    await mkdtemp(path.join(os.tmpdir(), "synara-r02-e1-transaction-"))
  );
  t.after(async () => rm(root, { recursive: true, force: true }));
  await writeFile(path.join(root, "first.json"), "old-first\n");
  await writeFile(path.join(root, "second.md"), "old-second\n");
  let installs = 0;
  assert.throws(
    () =>
      api.transactionalTwinWrite(
        root,
        [
          ["first.json", Buffer.from("new-first\n")],
          ["second.md", Buffer.from("new-second\n")],
        ],
        {
          renameSync(source, destination) {
            if (source.endsWith(".staged") && ++installs === 2) {
              const error = new Error("injected second install failure");
              error.code = "EIO";
              throw error;
            }
            renameSync(source, destination);
          },
        }
      ),
    /injected/iu
  );
  assert.equal(
    readFileSync(path.join(root, "first.json"), "utf8"),
    "old-first\n"
  );
  assert.equal(
    readFileSync(path.join(root, "second.md"), "utf8"),
    "old-second\n"
  );
  assert.deepEqual((await readdir(root)).sort(), ["first.json", "second.md"]);
});

test("transactional writes reject symlink targets and symlink parent components", async (t) => {
  const root = await realpath(
    await mkdtemp(path.join(os.tmpdir(), "synara-r02-e1-symlink-write-"))
  );
  t.after(async () => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "real"));
  await writeFile(path.join(root, "regular"), "unchanged\n");
  await symlink("regular", path.join(root, "target-link"));
  await symlink("real", path.join(root, "parent-link"));
  assert.throws(
    () =>
      api.transactionalTwinWrite(root, [
        ["target-link", Buffer.from("replace\n")],
      ]),
    /symlink|regular/iu
  );
  assert.throws(
    () =>
      api.transactionalTwinWrite(root, [
        ["parent-link/output", Buffer.from("replace\n")],
      ]),
    /symbolic-link|symlink/iu
  );
  assert.equal(readFileSync(path.join(root, "regular"), "utf8"), "unchanged\n");
  assert.deepEqual(await readdir(path.join(root, "real")), []);
});

test("durable audit identity is tied to the latest unique touching ancestor", async (t) => {
  const root = await temporaryGitRepository(t);
  const introduced = await commitAuditArtifact(root, { version: 1 });
  await writeFile(path.join(root, "unrelated.txt"), "unrelated\n");
  git(root, ["add", "unrelated.txt"]);
  git(root, ["commit", "--quiet", "-m", "unrelated"]);

  const stable = api.deriveDurableAuditIdentity(root);
  assert.equal(stable.introducing_commit_sha, introduced);
  assert.equal(
    stable.file_sha256,
    api.sha256(Buffer.from('{\n  "version": 1\n}\n'))
  );
  assert.equal(
    stable.canonical_semantic_sha256,
    api.sha256(api.canonicalize({ version: 1 }))
  );

  const modified = await commitAuditArtifact(
    root,
    { version: 2 },
    "modify audit"
  );
  const changed = api.deriveDurableAuditIdentity(root);
  assert.equal(changed.introducing_commit_sha, modified);
  assert.notEqual(changed.blob_oid, stable.blob_oid);
});

test("durable audit identity rejects dirty, staged, missing, symlinked, and untracked input", async (t) => {
  await t.test("dirty", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(root, { state: "clean" });
    await writeFile(path.join(root, AUDIT_PATH), '{"state":"dirty"}\n');
    assert.throws(
      () => api.deriveDurableAuditIdentity(root),
      /dirty|worktree/iu
    );
  });
  await t.test("staged", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(root, { state: "clean" });
    await writeFile(path.join(root, AUDIT_PATH), '{"state":"staged"}\n');
    git(root, ["add", AUDIT_PATH]);
    assert.throws(() => api.deriveDurableAuditIdentity(root), /dirty|staged/iu);
  });
  await t.test("missing", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(root, { state: "clean" });
    await rm(path.join(root, AUDIT_PATH));
    assert.throws(() => api.deriveDurableAuditIdentity(root));
  });
  await t.test("symlink", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(root, { state: "clean" });
    await rm(path.join(root, AUDIT_PATH));
    await writeFile(path.join(root, "elsewhere.json"), "{}\n");
    await symlink(
      path.join(root, "elsewhere.json"),
      path.join(root, AUDIT_PATH)
    );
    assert.throws(
      () => api.deriveDurableAuditIdentity(root),
      /symbolic-link|symlink|regular/iu
    );
  });
  await t.test("untracked", async (child) => {
    const root = await temporaryGitRepository(child);
    await mkdir(path.dirname(path.join(root, AUDIT_PATH)), { recursive: true });
    await writeFile(path.join(root, AUDIT_PATH), "{}\n");
    await writeFile(path.join(root, "tracked"), "root\n");
    git(root, ["add", "tracked"]);
    git(root, ["commit", "--quiet", "-m", "root"]);
    assert.throws(
      () => api.deriveDurableAuditIdentity(root),
      /dirty|untracked/iu
    );
  });
  await t.test("conflicted", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(root, { state: "base" });
    const defaultBranch = git(root, ["branch", "--show-current"]);
    git(root, ["checkout", "--quiet", "-b", "conflicting"]);
    await commitAuditArtifact(root, { state: "branch" }, "branch change");
    git(root, ["checkout", "--quiet", defaultBranch]);
    await commitAuditArtifact(root, { state: "main" }, "main change");
    const merge = spawnSync("git", ["merge", "conflicting"], {
      cwd: root,
      encoding: "utf8",
    });
    assert.notEqual(merge.status, 0);
    assert.throws(
      () => api.deriveDurableAuditIdentity(root),
      /dirty|conflict/iu
    );
  });
});

test("durable audit identity rejects incomparable identical introductions", async (t) => {
  const root = await temporaryGitRepository(t);
  await writeFile(path.join(root, "root.txt"), "root\n");
  git(root, ["add", "root.txt"]);
  git(root, ["commit", "--quiet", "-m", "root"]);
  const defaultBranch = git(root, ["branch", "--show-current"]);
  git(root, ["checkout", "--quiet", "-b", "left"]);
  await commitAuditArtifact(root, { identical: true }, "left introduction");
  git(root, ["checkout", "--quiet", defaultBranch]);
  git(root, ["checkout", "--quiet", "-b", "right"]);
  await commitAuditArtifact(root, { identical: true }, "right introduction");
  git(root, [
    "merge",
    "--quiet",
    "--no-ff",
    "left",
    "-m",
    "merge introductions",
  ]);
  assert.throws(
    () => api.deriveDurableAuditIdentity(root),
    /unique|incomparable|ambiguous/iu
  );
});

test("durable audit identity handles rename and deletion/reintroduction histories", async (t) => {
  await t.test("rename into the durable path", async (child) => {
    const root = await temporaryGitRepository(child);
    await writeFile(
      path.join(root, "old-audit.json"),
      '{\n  "renamed": true\n}\n'
    );
    git(root, ["add", "old-audit.json"]);
    git(root, ["commit", "--quiet", "-m", "old audit path"]);
    await mkdir(path.dirname(path.join(root, AUDIT_PATH)), { recursive: true });
    git(root, ["mv", "old-audit.json", AUDIT_PATH]);
    git(root, ["commit", "--quiet", "-m", "rename audit into durable path"]);
    const renamed = git(root, ["rev-parse", "HEAD"]);
    assert.equal(
      api.deriveDurableAuditIdentity(root).introducing_commit_sha,
      renamed
    );
  });

  await t.test("deletion and byte-identical reintroduction", async (child) => {
    const root = await temporaryGitRepository(child);
    await commitAuditArtifact(
      root,
      { reintroduced: true },
      "first introduction"
    );
    await rm(path.join(root, AUDIT_PATH));
    git(root, ["add", "-u", AUDIT_PATH]);
    git(root, ["commit", "--quiet", "-m", "delete audit"]);
    const reintroduced = await commitAuditArtifact(
      root,
      { reintroduced: true },
      "reintroduce audit"
    );
    assert.equal(
      api.deriveDurableAuditIdentity(root).introducing_commit_sha,
      reintroduced
    );
  });
});

test("durable audit identity rejects shallow history", async (t) => {
  const source = await temporaryGitRepository(t);
  await commitAuditArtifact(source, { version: 1 });
  await commitAuditArtifact(source, { version: 2 });
  const parent = await mkdtemp(
    path.join(os.tmpdir(), "synara-r02-e1-shallow-parent-")
  );
  t.after(async () => rm(parent, { recursive: true, force: true }));
  const shallow = path.join(parent, "clone");
  git(parent, [
    "clone",
    "--quiet",
    "--depth",
    "1",
    `file://${source}`,
    shallow,
  ]);
  assert.throws(
    () => api.deriveDurableAuditIdentity(shallow),
    /shallow|complete/iu
  );
});

test("audit production CLIs reject every bypass or artifact-selection flag", () => {
  const cases = [
    [AUDIT_CHECKER, ["--repo-root", "."]],
    [AUDIT_CHECKER, ["--source-commit", "1".repeat(40)]],
    [AUDIT_CHECKER, ["--fixture", "fixture.json"]],
    [AUDIT_CHECKER, ["--json"]],
    [AUDIT_GENERATOR, []],
    [AUDIT_GENERATOR, ["--check", "--write"]],
    [AUDIT_GENERATOR, ["--write", "extra"]],
    [AUDIT_GENERATOR, ["--artifact", "elsewhere.json"]],
  ];
  for (const [script, args] of cases) {
    const result = runNode(script, args);
    assert.equal(
      result.status,
      2,
      `${path.basename(script)} ${args.join(" ")}\n${result.stderr}`
    );
  }
});

test("audit checker fails honestly while the authoritative E2 input is absent", () => {
  const result = runNode(AUDIT_CHECKER);
  assert.equal(result.status, 1, result.stderr);
  assert.doesNotMatch(
    `${result.stdout}\n${result.stderr}`,
    /\/private\/tmp|\/tmp\//u
  );
});

test("all production CLI wrappers and documented modes succeed end to end", async (t) => {
  const { parent, root, auditJson, auditMarkdown } =
    await productionCliRepository(t);
  const environment = {
    GIT_ALLOW_PROTOCOL: "file",
    HTTPS_PROXY: "http://127.0.0.1:1",
    HTTP_PROXY: "http://127.0.0.1:1",
    ALL_PROXY: "http://127.0.0.1:1",
  };
  const invoke = (script, args, label) => {
    assert.equal(
      args.some((argument) =>
        /bypass|fixture|repo-root|source-commit/iu.test(argument)
      ),
      false,
      label
    );
    const result = runNode(
      path.join(root, path.relative(REPOSITORY_ROOT, script)),
      args,
      root,
      environment
    );
    assertCliSuccess(result, label);
    return result;
  };

  invoke(AUDIT_CHECKER, [], "audit checker");
  invoke(AUDIT_GENERATOR, ["--check"], "audit generator --check");

  const auditJsonPath = path.join(root, api.AUDIT_PATH);
  const auditMarkdownPath = path.join(root, api.AUDIT_MARKDOWN_PATH);
  await writeFile(auditMarkdownPath, "stale audit Markdown\n", "utf8");
  invoke(AUDIT_GENERATOR, ["--write"], "audit generator --write repair");
  assert.deepEqual(await readFile(auditJsonPath), auditJson);
  assert.deepEqual(await readFile(auditMarkdownPath), auditMarkdown);
  invoke(AUDIT_CHECKER, [], "audit checker after repair");
  invoke(AUDIT_GENERATOR, ["--write"], "audit generator repeat --write");
  assert.deepEqual(await readFile(auditJsonPath), auditJson);
  assert.deepEqual(await readFile(auditMarkdownPath), auditMarkdown);

  const externalOne = path.join(parent, "migration-output-one");
  const externalTwo = path.join(parent, "migration-output-two");
  await mkdir(externalOne);
  await mkdir(externalTwo);
  invoke(
    MIGRATOR,
    ["--output-dir", externalOne],
    "migrator --output-dir first"
  );
  invoke(
    MIGRATOR,
    ["--output-dir", externalTwo],
    "migrator --output-dir second"
  );
  const externalJsonName = path.basename(api.V1_PATH);
  const externalMarkdownName = path.basename(api.V1_MARKDOWN_PATH);
  const externalOneJson = await readFile(
    path.join(externalOne, externalJsonName)
  );
  const externalOneMarkdown = await readFile(
    path.join(externalOne, externalMarkdownName)
  );
  assert.deepEqual(
    externalOneJson,
    await readFile(path.join(externalTwo, externalJsonName))
  );
  assert.deepEqual(
    externalOneMarkdown,
    await readFile(path.join(externalTwo, externalMarkdownName))
  );
  assert.deepEqual((await readdir(externalOne)).sort(), [
    externalJsonName,
    externalMarkdownName,
  ]);

  invoke(MIGRATOR, ["--write"], "migrator --write");
  assert.deepEqual(
    await readFile(path.join(root, api.V1_PATH)),
    externalOneJson
  );
  assert.deepEqual(
    await readFile(path.join(root, api.V1_MARKDOWN_PATH)),
    externalOneMarkdown
  );
  invoke(MIGRATOR, ["--check"], "migrator --check");
  invoke(TRACEABILITY_CHECKER, [], "v2 checker");
  invoke(TRACEABILITY_GENERATOR, ["--check"], "v2 generator --check");
  const beforeGeneratorWrite = [
    await readFile(path.join(root, api.V1_PATH)),
    await readFile(path.join(root, api.V1_MARKDOWN_PATH)),
  ];
  invoke(TRACEABILITY_GENERATOR, ["--write"], "v2 generator --write");
  assert.deepEqual(
    await readFile(path.join(root, api.V1_PATH)),
    beforeGeneratorWrite[0]
  );
  assert.deepEqual(
    await readFile(path.join(root, api.V1_MARKDOWN_PATH)),
    beforeGeneratorWrite[1]
  );
  invoke(TRACEABILITY_GENERATOR, ["--write"], "v2 generator repeat --write");
  assert.deepEqual(
    await readFile(path.join(root, api.V1_PATH)),
    beforeGeneratorWrite[0]
  );
  assert.deepEqual(
    await readFile(path.join(root, api.V1_MARKDOWN_PATH)),
    beforeGeneratorWrite[1]
  );
  assert.deepEqual(
    (await readdir(root)).filter((name) =>
      name.startsWith(".synara-traceability-write-")
    ),
    []
  );
});

test("runCli exposes stable success, validation, usage, and internal exit codes", async (t) => {
  const { root } = await productionCliRepository(t);
  const streams = () => {
    let stdout = "";
    let stderr = "";
    return {
      stdout: {
        write(value) {
          stdout += value;
        },
      },
      stderr: {
        write(value) {
          stderr += value;
        },
      },
      read() {
        return { stdout, stderr };
      },
    };
  };

  const successStreams = streams();
  assert.equal(
    await api.runCli([], {
      kind: "check-audit",
      repositoryRoot: root,
      stdout: successStreams.stdout,
      stderr: successStreams.stderr,
    }),
    0
  );
  assert.match(successStreams.read().stdout, /PASS/u);
  assert.equal(successStreams.read().stderr, "");

  const validationStreams = streams();
  assert.equal(
    await api.runCli([], {
      kind: "check-v2",
      repositoryRoot: root,
      stdout: validationStreams.stdout,
      stderr: validationStreams.stderr,
    }),
    1
  );

  const usageStreams = streams();
  assert.equal(
    await api.runCli(["--bypass"], {
      kind: "check-audit",
      repositoryRoot: root,
      stdout: usageStreams.stdout,
      stderr: usageStreams.stderr,
    }),
    2
  );

  const internalStreams = streams();
  const injected = new Error("Injected deterministic internal write failure.");
  injected.code = "INTERNAL";
  const internalReader = new api.GitObjectReader(root);
  let internalCode;
  try {
    internalCode = await api.runCli(["--write"], {
      kind: "generate-audit",
      repositoryRoot: root,
      gitObjects: internalReader,
      stdout: internalStreams.stdout,
      stderr: internalStreams.stderr,
      fsOps: {
        writeFileSync() {
          throw injected;
        },
      },
    });
  } finally {
    await internalReader.close();
  }
  assert.equal(internalCode, 3, JSON.stringify(internalStreams.read()));

  for (const output of [
    ...Object.values(validationStreams.read()),
    ...Object.values(usageStreams.read()),
    ...Object.values(internalStreams.read()),
  ])
    assert.doesNotMatch(
      output,
      /\/private\/tmp|\/tmp\/|[A-Za-z]:\\|https?:|ssh:|git@|secret/iu
    );
});

test("strict JSON failures are validation failures, never tooling crashes", async (t) => {
  const root = await temporaryGitRepository(t);
  const tooling = [
    "scripts/check-feature-parity-audit-normalization.mjs",
    "scripts/lib/feature-parity-traceability-v2.mjs",
    "scripts/lib/matrix-rust-governance.mjs",
    "docs/matrix-rust-sdk/schemas/feature-parity-audit-normalization.schema.json",
    "docs/matrix-rust-sdk/schemas/feature-parity-traceability-v2.schema.json",
  ];
  for (const relative of tooling) {
    await mkdir(path.dirname(path.join(root, relative)), { recursive: true });
    await cp(path.join(REPOSITORY_ROOT, relative), path.join(root, relative));
  }
  await mkdir(path.dirname(path.join(root, AUDIT_PATH)), { recursive: true });
  const malformedCases = [
    Buffer.from("\ufeff{}", "utf8"),
    Buffer.from('{"schema_version":"1.0","schema_version":"1.0"}\n'),
    Buffer.from('{"value":9007199254740992}\n'),
    Buffer.from('{"value":-0}\n'),
    Buffer.from('{"value":1.5}\n'),
    Buffer.from('{"value":"\\ud800"}\n'),
  ];
  for (const [index, bytes] of malformedCases.entries()) {
    await writeFile(path.join(root, AUDIT_PATH), bytes);
    const result = runNode(
      path.join(root, "scripts/check-feature-parity-audit-normalization.mjs"),
      [],
      root
    );
    assert.equal(result.status, 1, `case ${index}: ${result.stderr}`);
    assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, /\/tmp\//u);
  }
});

test("temporary repositories and child processes are cleaned after failures", async (t) => {
  const root = await temporaryGitRepository(t);
  await commitFixture(root);
  const reader = new api.GitObjectReader(root);
  await assert.rejects(() => reader.bytes("not-an-object-id"));
  await reader.close();
  assert.notEqual(reader.checkProcess.exitCode, null);
  assert.notEqual(reader.contentProcess.exitCode, null);
  await rm(root, { recursive: true, force: true });
  await assert.rejects(() => stat(root), { code: "ENOENT" });
});
