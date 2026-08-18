#!/usr/bin/env node

import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = path.resolve(SCRIPT_DIR, "..");
const STATUS_REL = "docs/matrix-rust-sdk/program-status.json";
const STATUS_MD_REL = "docs/matrix-rust-sdk/program-status.md";
const PLAN_REL = "docs/matrix-rust-sdk-full-replacement-plan.md";
const CURRENT_DOCS = [
  PLAN_REL,
  "docs/matrix-rust-sdk/README.md",
  "docs/matrix-rust-sdk/PROGRESS.md",
  "docs/matrix-rust-sdk/d0-residual-completion.md",
];
const CURRENT_DOC_MARKER = "<!-- matrix-rust-program-status-link -->";
const REMEDIATION_IDS = [
  "R0.1",
  "R0.2",
  "R0.3",
  "R0.4",
  "R0.5",
  "R0.6",
  "R0.7",
  "R0.8",
];
const PHASE_REMEDIATION_BLOCKERS = new Map([
  [0, ["R0.2", "R0.8"]],
  [1, ["R0.1", "R0.3", "R0.8"]],
  [2, ["R0.4", "R0.5", "R0.6", "R0.7", "R0.8"]],
  [3, ["R0.7", "R0.8"]],
  ...Array.from({ length: 11 }, (_, index) => [index + 4, []]),
]);

const KNOWN_TASK_JSON = [
  "swift-rust-version-provenance.json",
  "toolchain-compatibility-report.json",
  "performance-baseline.json",
  "migration-ux-decision.json",
  "p1.6-architectural-guardrails.json",
  "p2.1-matrix-supervisor-actor.json",
  "p2.2-store-paths-keys.json",
  "p2.3-sdk-client-builder.json",
  "p2.4-task-supervision.json",
  "p2.5-diagnostics-health.json",
  "p2.6-destructive-lifecycle.json",
  "p3.1-discovery-login-flow.json",
  "p3.2-password-token-login.json",
];

const VOCABULARIES = {
  artifact_state: new Set([
    "not_started",
    "in_progress",
    "landed",
    "superseded",
  ]),
  integration_state: new Set([
    "not_submitted",
    "pr_open",
    "merged",
    "reverted",
  ]),
  strict_acceptance_state: new Set([
    "not_reviewed",
    "open",
    "accepted",
    "rejected",
  ]),
  phase_gate_state: new Set(["open", "closed"]),
};

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertUnique(values, label) {
  assert(
    new Set(values).size === values.length,
    `${label} contains duplicate IDs`
  );
}

function validateStateAxes(record, label) {
  for (const [field, vocabulary] of Object.entries(VOCABULARIES)) {
    assert(
      vocabulary.has(record[field]),
      `${label}.${field} has invalid value ${JSON.stringify(record[field])}`
    );
  }
}

function phaseForTaskId(id) {
  return Number(id.match(/^P(\d+)\./)?.[1]);
}

function validateTaskStateConsistency(task, label) {
  validateStateAxes(task, label);
  if (task.artifact_state === "not_started") {
    assert(
      task.integration_state === "not_submitted",
      `${label} cannot be submitted before it starts`
    );
    assert(
      task.strict_acceptance_state === "not_reviewed",
      `${label} cannot be reviewed before it starts`
    );
  }
  if (task.integration_state === "merged") {
    assert(
      task.artifact_state === "landed",
      `${label} must be landed when merged`
    );
  }
  if (task.integration_state === "pr_open") {
    assert(
      ["in_progress", "landed"].includes(task.artifact_state),
      `${label} must be in progress or landed while its PR is open`
    );
  }
  if (task.strict_acceptance_state === "accepted") {
    assert(
      task.artifact_state === "landed",
      `${label} must be landed before strict acceptance`
    );
    assert(
      task.integration_state === "merged",
      `${label} must be merged before strict acceptance`
    );
  }
}

export function validateTaskRecord(task, name, canonical) {
  assert(
    !Object.hasOwn(task, "status"),
    `${name} has forbidden ambiguous top-level status`
  );
  assert(
    !Object.hasOwn(task, "merged"),
    `${name} has forbidden ambiguous top-level merged`
  );
  assert(
    !Object.hasOwn(task, "next_phase0"),
    `${name} has forbidden stale next_phase0`
  );
  for (const field of Object.keys(VOCABULARIES)) {
    assert(
      task[field] === canonical[field],
      `${name}.${field} does not match program-status.json`
    );
  }
}

export function validateCurrentDoc(text, relativePath) {
  assert(
    text.includes(CURRENT_DOC_MARKER),
    `${relativePath} is missing ${CURRENT_DOC_MARKER}`
  );
  assert(
    text.includes("program-status.md"),
    `${relativePath} does not link program-status.md`
  );
}

export function validateRenderedStatus(current, status) {
  assert(
    current === renderProgramStatus(status),
    `${STATUS_MD_REL} is stale; run npm run check:matrix-rust-program-status -- --write`
  );
}

export function extractOriginalTaskIds(planText) {
  return [...new Set(planText.match(/P\d+\.\d+/g) ?? [])].sort(
    (left, right) => {
      const [leftPhase, leftTask] = left.slice(1).split(".").map(Number);
      const [rightPhase, rightTask] = right.slice(1).split(".").map(Number);
      return leftPhase - rightPhase || leftTask - rightTask;
    }
  );
}

export function validateProgramStatus(status, planText) {
  assert(status.schema_version === 1, "schema_version must be 1");
  assert(
    status.integration_branch === "feature/matrix-rust-sdk-full-replacement",
    "unexpected integration branch"
  );

  const planIds = extractOriginalTaskIds(planText);
  assert(
    planIds.length === 112,
    `plan must contain 112 original task IDs; found ${planIds.length}`
  );
  assert(
    status.original_plan.task_count === planIds.length,
    "ledger task_count does not match the plan"
  );

  const taskRecords = status.task_records;
  const taskRecordIds = taskRecords.map(({ id }) => id);
  assertUnique(taskRecordIds, "task_records");
  for (const task of taskRecords) {
    assert(
      planIds.includes(task.id),
      `task record ${task.id} is absent from the plan`
    );
    validateTaskStateConsistency(task, `task_records.${task.id}`);
  }

  const landedIds = taskRecords
    .filter(({ artifact_state }) => artifact_state === "landed")
    .map(({ id }) => id);
  const inventoryIds = status.original_plan.landed_task_ids;
  assertUnique(inventoryIds, "landed_task_ids");
  assertUnique(landedIds, "landed_task_ids");
  assert(
    JSON.stringify(inventoryIds) === JSON.stringify(landedIds),
    "landed_task_ids must match landed task_records in plan order"
  );
  assert(
    status.original_plan.landed_task_count === landedIds.length,
    "landed_task_count does not match landed_task_ids"
  );
  assert(
    JSON.stringify(taskRecordIds) ===
      JSON.stringify(planIds.filter((id) => taskRecordIds.includes(id))),
    "task_records must follow original plan order"
  );

  assert(
    status.phase_gates.length === 15,
    "phase_gates must contain phases 0 through 14"
  );
  assertUnique(
    status.phase_gates.map(({ phase }) => phase),
    "phase_gates"
  );
  for (let phase = 0; phase <= 14; phase += 1) {
    const gate = status.phase_gates.find(
      (candidate) => candidate.phase === phase
    );
    assert(gate, `missing Phase ${phase} gate`);
    assert(
      VOCABULARIES.strict_acceptance_state.has(gate.strict_acceptance_state),
      `phase_gates.${phase}.strict_acceptance_state has an invalid value`
    );
    assert(
      VOCABULARIES.phase_gate_state.has(gate.phase_gate_state),
      `phase_gates.${phase}.phase_gate_state has an invalid value`
    );
    const expectedIds = planIds.filter((id) => phaseForTaskId(id) === phase);
    assertUnique(
      gate.planned_task_ids,
      `phase_gates.${phase}.planned_task_ids`
    );
    assertUnique(gate.blocked_by, `phase_gates.${phase}.blocked_by`);
    assert(
      JSON.stringify(gate.planned_task_ids) === JSON.stringify(expectedIds),
      `Phase ${phase} planned_task_ids do not match the plan`
    );
    assert(
      JSON.stringify(gate.blocked_by) ===
        JSON.stringify(PHASE_REMEDIATION_BLOCKERS.get(phase)),
      `Phase ${phase} blocked_by does not match the audited remediation dependencies`
    );
    const closed = gate.phase_gate_state === "closed";
    assert(
      closed === (gate.strict_acceptance_state === "accepted"),
      `Phase ${phase} gate closure and strict acceptance must change together`
    );
    if (closed) {
      for (const blockerId of gate.blocked_by) {
        const blocker = status.remediation_tasks.find(
          (candidate) => candidate.id === blockerId
        );
        assert(
          blocker?.strict_acceptance_state === "accepted",
          `Phase ${phase} cannot close until remediation ${blockerId} is accepted`
        );
      }
      for (const id of expectedIds) {
        const task = taskRecords.find((candidate) => candidate.id === id);
        assert(
          task,
          `Phase ${phase} cannot close until ${id} has a task record`
        );
        assert(
          task.artifact_state === "landed" &&
            task.integration_state === "merged" &&
            task.strict_acceptance_state === "accepted",
          `Phase ${phase} cannot close until ${id} is landed, merged, and accepted`
        );
      }
    }
    for (const task of taskRecords.filter(
      (candidate) => phaseForTaskId(candidate.id) === phase
    )) {
      assert(
        task.phase_gate_state === gate.phase_gate_state,
        `${task.id}.phase_gate_state must match Phase ${phase}`
      );
    }
  }

  const remediationIds = status.remediation_tasks.map(({ id }) => id);
  assertUnique(remediationIds, "remediation_tasks");
  assert(
    JSON.stringify(remediationIds) === JSON.stringify(REMEDIATION_IDS),
    "remediation task IDs must be R0.1 through R0.8"
  );
  for (const task of status.remediation_tasks)
    validateTaskStateConsistency(task, `remediation_tasks.${task.id}`);

  const allRecords = [...taskRecords, ...status.remediation_tasks];
  const validTaskIds = new Set([...planIds, ...remediationIds]);
  const inProgress = allRecords.filter(
    ({ artifact_state }) => artifact_state === "in_progress"
  );
  assert(inProgress.length <= 1, "at most one task may be in progress");
  const {
    active_task: activeTask,
    next_task: nextTask,
    blocked_tasks: blockedTasks,
  } = status.current_execution;
  assert(
    activeTask === null || validTaskIds.has(activeTask),
    "active_task must be null or a valid task ID"
  );
  assert(
    nextTask === null || validTaskIds.has(nextTask),
    "next_task must be null or a valid task ID"
  );
  assert(
    activeTask === null || activeTask !== nextTask,
    "active_task and next_task must differ"
  );
  if (activeTask === null) {
    assert(
      inProgress.length === 0,
      "an in-progress task must be current_execution.active_task"
    );
  } else {
    assert(
      inProgress.length === 1 && inProgress[0].id === activeTask,
      "active_task must be the only in-progress task"
    );
  }

  assert(
    Array.isArray(blockedTasks),
    "current_execution.blocked_tasks must be an array"
  );
  assertUnique(
    blockedTasks.map(({ id }) => id),
    "blocked_tasks"
  );
  for (const blocked of blockedTasks) {
    assert(
      validTaskIds.has(blocked.id),
      `blocked task ${blocked.id} is not a valid task`
    );
    assertUnique(blocked.blocked_by, `blocked_tasks.${blocked.id}.blocked_by`);
    for (const blocker of blocked.blocked_by)
      assert(
        validTaskIds.has(blocker),
        `blocker ${blocker} is not a valid task`
      );
  }
  // Product policy 2026-07-27: residual unaccepted R0 formal gates do not
  // hard-block P3.2 product progress (clean-break re-login approved). When a
  // P3.2 block entry is present, its blockers must be a subset of unaccepted
  // remediations (residual awareness only — not a false phase-gate close).
  const unacceptedRemediations = status.remediation_tasks
    .filter(
      ({ strict_acceptance_state }) => strict_acceptance_state !== "accepted"
    )
    .map(({ id }) => id);
  const p32Block = blockedTasks.find(({ id }) => id === "P3.2");
  if (p32Block) {
    for (const blocker of p32Block.blocked_by) {
      assert(
        unacceptedRemediations.includes(blocker),
        `P3.2 residual blocker ${blocker} must be an unaccepted R0 remediation`
      );
    }
  }

  assert(
    status.product_runtime.dual_backend === false,
    "dual_backend must be false"
  );
  const runtime = status.product_runtime;
  assert(
    ["matrix-js-sdk-only", "matrix-rust-sdk-only"].includes(
      runtime.matrix_client_sdk
    ),
    "invalid matrix_client_sdk state"
  );
  assert(
    [
      "absent",
      "harness-foundation-only",
      "cutover-candidate",
      "sole-production-runtime",
    ].includes(runtime.rust_sdk_state),
    "invalid rust_sdk_state"
  );
  assert(
    ["not_started", "in_progress", "complete"].includes(runtime.cutover_state),
    "invalid cutover_state"
  );
  if (runtime.matrix_client_sdk === "matrix-rust-sdk-only") {
    assert(
      runtime.rust_sdk_state === "sole-production-runtime" &&
        runtime.cutover_state === "complete",
      "Rust-only runtime requires a complete cutover and sole-production-runtime state"
    );
  } else {
    assert(
      runtime.rust_sdk_state !== "sole-production-runtime" &&
        runtime.cutover_state !== "complete",
      "JS-only runtime cannot claim a completed Rust cutover"
    );
  }
  if (runtime.cutover_state === "in_progress") {
    assert(
      runtime.matrix_client_sdk === "matrix-js-sdk-only" &&
        runtime.rust_sdk_state === "cutover-candidate",
      "in-progress cutover requires a non-production Rust candidate while JS remains sole runtime"
    );
    for (let phase = 0; phase <= 10; phase += 1) {
      assert(
        status.phase_gates.find((gate) => gate.phase === phase)
          .phase_gate_state === "closed",
        `cutover cannot start before Phase ${phase} closes`
      );
    }
  }
  if (runtime.cutover_state === "complete") {
    for (let phase = 0; phase <= 11; phase += 1) {
      assert(
        status.phase_gates.find((gate) => gate.phase === phase)
          .phase_gate_state === "closed",
        `cutover cannot complete before Phase ${phase} closes`
      );
    }
  }

  const vertical = status.vertical_execution;
  assert(
    vertical && typeof vertical === "object",
    "vertical_execution is required"
  );
  assert(
    vertical.policy === "full-vertical-delete-per-vertical",
    "vertical_execution.policy must require per-vertical deletion"
  );
  assert(
    ["capability-cutover-in-progress", "between-slices-paused"].includes(
      vertical.integration_product_state
    ),
    "invalid integration_product_state"
  );
  if (vertical.integration_product_state === "capability-cutover-in-progress") {
    assert(
      /^V-[A-Z]+\.\d+(?:-D)?$/.test(vertical.active_slice),
      "vertical_execution.active_slice must be a vertical ID while cutover is in progress"
    );
    assert(
      Number.isInteger(vertical.active_pr) && vertical.active_pr > 0,
      "vertical_execution.active_pr must be a positive PR number while cutover is in progress"
    );
  } else {
    assert(
      vertical.active_slice === null && vertical.active_pr === null,
      "between-slices-paused requires null active_slice and active_pr"
    );
  }
  assertUnique(
    vertical.wired_deletion_open,
    "vertical_execution.wired_deletion_open"
  );
  assertUnique(
    vertical.completed_slices,
    "vertical_execution.completed_slices"
  );
  assertUnique(vertical.next_slices, "vertical_execution.next_slices");
  assertUnique(vertical.held_prs, "vertical_execution.held_prs");
  assert(
    vertical.completion_evidence?.capability_owner_delta === "negative",
    "vertical completion must require a negative capability-owner delta"
  );
  assert(
    vertical.completion_evidence?.global_direct_import_delta ===
      "recorded-non-increasing",
    "vertical completion must record a non-increasing global direct-import delta"
  );
  const importInventory = vertical.matrix_js_sdk_import_inventory;
  assert(
    importInventory.baseline.files >= 0 &&
      importInventory.baseline.import_lines >= 0 &&
      importInventory.current.files >= 0 &&
      importInventory.current.import_lines >= 0,
    "matrix-js-sdk import inventory must be non-negative"
  );
  assert(
    importInventory.current.files <= importInventory.baseline.files &&
      importInventory.current.import_lines <=
        importInventory.baseline.import_lines,
    "matrix-js-sdk current inventory cannot exceed the full-vertical baseline"
  );
}

export function renderProgramStatus(status) {
  const taskById = new Map(status.task_records.map((task) => [task.id, task]));
  const phaseRows = status.phase_gates.map((gate) => {
    const recorded = gate.planned_task_ids.filter((id) => taskById.has(id));
    const accepted = recorded.filter(
      (id) => taskById.get(id).strict_acceptance_state === "accepted"
    );
    const blockers =
      gate.blocked_by.length === 0
        ? "None"
        : gate.blocked_by.map((id) => `\`${id}\``).join(", ");
    return `| ${gate.phase} | ${recorded.length}/${gate.planned_task_ids.length} | ${accepted.length}/${gate.planned_task_ids.length} | ${blockers} | \`${gate.strict_acceptance_state}\` | \`${gate.phase_gate_state}\` |`;
  });
  const remediationRows = status.remediation_tasks.map(
    (task) =>
      `| ${task.id} | ${task.title} | \`${task.artifact_state}\` | \`${task.integration_state}\` | \`${task.strict_acceptance_state}\` |`
  );
  const artifactRows = status.task_records.map(
    (task) =>
      `| ${task.id} | \`${task.artifact_state}\` | \`${task.integration_state}\` | \`${task.strict_acceptance_state}\` | \`${task.phase_gate_state}\` |`
  );

  return `${[
    "# Matrix Rust SDK replacement — historical program status",
    "",
    "> Generated from `program-status.json` by `scripts/check-matrix-rust-sdk-program-status.mjs`.",
    "> Do not hand-edit. This is a frozen migration-program snapshot, not current product architecture.",
    "> The replacement has landed; see [the codebase knowledge base](../../CODEBASE_KNOWLEDGE_BASE.md) and [the 2026-08-17 local proof](../shared-native-core/15-2026-08-17-local-proof.md).",
    "",
    `As of: ${status.as_of_date}`,
    "",
    `Integration branch: \`${status.integration_branch}\``,
    "",
    `Audited snapshot: \`${status.audited_snapshot.sha}\``,
    "",
    "## Original-plan foundation queue",
    "",
    `- Active task: **${status.current_execution.active_task ?? "None"}**`,
    `- Next task: **${status.current_execution.next_task ?? "None"}**`,
    `- Blocked tasks: ${
      status.current_execution.blocked_tasks.length === 0
        ? "None"
        : status.current_execution.blocked_tasks
            .map(
              ({ id, blocked_by: blockedBy }) =>
                `**${id}** (by ${blockedBy
                  .map((blocker) => `\`${blocker}\``)
                  .join(", ")})`
            )
            .join("; ")
    }`,
    "",
    "## Original-plan inventory and runtime at this snapshot",
    "",
    `- Landed original task artifacts: **${status.original_plan.landed_task_count} / ${status.original_plan.task_count}**`,
    `- Release/main Matrix client: \`${status.product_runtime.matrix_client_sdk}\``,
    `- Rust SDK state: \`${status.product_runtime.rust_sdk_state}\``,
    `- Dual backend: \`${status.product_runtime.dual_backend}\``,
    `- Cutover state: \`${status.product_runtime.cutover_state}\``,
    "",
    "These fields describe the historical audited snapshot and must not be used as current release/main claims.",
    "",
    "## Full-vertical product execution at this snapshot",
    "",
    `- Policy: \`${status.vertical_execution.policy}\``,
    `- Integration product state: \`${status.vertical_execution.integration_product_state}\``,
    `- Active slice: **${status.vertical_execution.active_slice ?? "None"}**${
      status.vertical_execution.active_pr === null
        ? ""
        : ` (PR #${status.vertical_execution.active_pr})`
    }`,
    `- Wired / deletion open: ${
      status.vertical_execution.wired_deletion_open.length === 0
        ? "None"
        : status.vertical_execution.wired_deletion_open
            .map((id) => `\`${id}\``)
            .join(", ")
    }`,
    `- Completed under full policy: ${
      status.vertical_execution.completed_slices.length === 0
        ? "None"
        : status.vertical_execution.completed_slices
            .map((id) => `\`${id}\``)
            .join(", ")
    }`,
    `- Next slices: ${status.vertical_execution.next_slices
      .map((id) => `\`${id}\``)
      .join(" → ")}`,
    `- Held PRs: ${status.vertical_execution.held_prs
      .map((pr) => `#${pr}`)
      .join(", ")}`,
    `- Completion evidence: negative capability-owner/file deletion delta; global direct-import delta recorded and non-increasing`,
    `- matrix-js-sdk inventory: **${status.vertical_execution.matrix_js_sdk_import_inventory.current.files} files / ${status.vertical_execution.matrix_js_sdk_import_inventory.current.import_lines} import lines current**; baseline **${status.vertical_execution.matrix_js_sdk_import_inventory.baseline.files} / ${status.vertical_execution.matrix_js_sdk_import_inventory.baseline.import_lines}**`,
    "",
    "## Phase gates",
    "",
    "| Phase | Recorded tasks | Accepted tasks | Remediation blockers | Strict acceptance | Gate |",
    "|---:|---:|---:|---|---|---|",
    ...phaseRows,
    "",
    `${
      status.phase_gates.filter(
        ({ phase_gate_state: state }) => state === "closed"
      ).length
    } of 15 strict phase gates are closed.`,
    "",
    "## Mandatory remediation",
    "",
    "| ID | Task | Artifact state | Integration state | Strict acceptance |",
    "|---|---|---|---|---|",
    ...remediationRows,
    "",
    "## Recorded original task artifacts",
    "",
    "`landed` and `merged` describe inventory and Git delivery only. They do not imply strict acceptance.",
    "",
    "| ID | Artifact state | Integration state | Strict acceptance | Phase gate |",
    "|---|---|---|---|---|",
    ...artifactRows,
    "",
  ].join("\n")}`;
}

async function parseJsonFile(filePath) {
  const text = await readFile(filePath, "utf8");
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(
      `${path.relative(DEFAULT_ROOT, filePath)} is invalid JSON: ${
        error.message
      }`
    );
  }
}

async function validateRepository(root, write) {
  const status = await parseJsonFile(path.join(root, STATUS_REL));
  const planText = await readFile(path.join(root, PLAN_REL), "utf8");
  validateProgramStatus(status, planText);

  const docsDir = path.join(root, "docs/matrix-rust-sdk");
  const jsonFiles = (await readdir(docsDir)).filter((name) =>
    name.endsWith(".json")
  );
  for (const name of jsonFiles) await parseJsonFile(path.join(docsDir, name));

  const artifactById = new Map(
    status.task_records.map((artifact) => [artifact.id, artifact])
  );
  for (const name of KNOWN_TASK_JSON) {
    const task = await parseJsonFile(path.join(docsDir, name));
    const id = task.task_id ?? task.taskId;
    const canonical = artifactById.get(id);
    assert(canonical, `${name} task ID ${id} is absent from task_records`);
    validateTaskRecord(task, name, canonical);
  }

  for (const relativePath of CURRENT_DOCS) {
    const text = await readFile(path.join(root, relativePath), "utf8");
    validateCurrentDoc(text, relativePath);
  }

  const rendered = renderProgramStatus(status);
  const mdPath = path.join(root, STATUS_MD_REL);
  if (write) {
    await writeFile(mdPath, rendered, "utf8");
  } else {
    const current = await readFile(mdPath, "utf8");
    validateRenderedStatus(current, status);
  }
}

async function main() {
  const write = process.argv.includes("--write");
  const rootArgIndex = process.argv.indexOf("--root");
  const root =
    rootArgIndex >= 0
      ? path.resolve(process.argv[rootArgIndex + 1])
      : DEFAULT_ROOT;
  await validateRepository(root, write);
  process.stdout.write(
    `Matrix Rust SDK program status ${write ? "generated" : "valid"}.\n`
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
