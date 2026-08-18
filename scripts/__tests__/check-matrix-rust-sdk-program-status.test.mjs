import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  extractOriginalTaskIds,
  renderProgramStatus,
  validateCurrentDoc,
  validateProgramStatus,
  validateRenderedStatus,
  validateTaskRecord,
} from "../check-matrix-rust-sdk-program-status.mjs";

const status = JSON.parse(
  await readFile(
    new URL("../../docs/matrix-rust-sdk/program-status.json", import.meta.url),
    "utf8"
  )
);
const plan = await readFile(
  new URL(
    "../../docs/matrix-rust-sdk-full-replacement-plan.md",
    import.meta.url
  ),
  "utf8"
);
const planOrder = extractOriginalTaskIds(plan);

function clone(value) {
  return structuredClone(value);
}

function markAccepted(task) {
  task.artifact_state = "landed";
  task.integration_state = "merged";
  task.strict_acceptance_state = "accepted";
}

/** Dual-track ledgers may already have another remediation in progress. */
function clearInProgressExcept(candidate, exceptId = null) {
  for (const task of [
    ...candidate.task_records,
    ...candidate.remediation_tasks,
  ]) {
    if (task.artifact_state === "in_progress" && task.id !== exceptId) {
      task.artifact_state = "landed";
    }
  }
}

function syncInventory(candidate) {
  const landed = candidate.task_records
    .filter(({ artifact_state: artifactState }) => artifactState === "landed")
    .map(({ id }) => id);
  candidate.original_plan.landed_task_ids = landed;
  candidate.original_plan.landed_task_count = landed.length;
}

/**
 * Optional residual-awareness block only (product policy 2026-07-27):
 * unaccepted R0 formal gates do not hard-block P3.2 product progress.
 * When callers want an explicit residual note, list unaccepted remediations.
 * When empty, product tasks may advance (clean-break re-login).
 */
function syncP32ResidualAwareness(candidate, { forceBlock = false } = {}) {
  const blockers = candidate.remediation_tasks
    .filter(
      ({ strict_acceptance_state: acceptance }) => acceptance !== "accepted"
    )
    .map(({ id }) => id);
  if (forceBlock && blockers.length > 0) {
    candidate.current_execution.blocked_tasks = [
      { id: "P3.2", blocked_by: blockers },
    ];
  } else {
    candidate.current_execution.blocked_tasks = [];
  }
}

/** @deprecated name kept for older call sites — clears hard P3.2 block. */
function syncP32Block(candidate) {
  syncP32ResidualAwareness(candidate, { forceBlock: false });
}

function closePhase(candidate, phase) {
  const gate = candidate.phase_gates.find((item) => item.phase === phase);
  for (const id of gate.planned_task_ids) {
    let task = candidate.task_records.find((item) => item.id === id);
    if (!task) {
      task = {
        id,
        artifact_state: "landed",
        integration_state: "merged",
        strict_acceptance_state: "accepted",
        phase_gate_state: "closed",
      };
      candidate.task_records.push(task);
    }
    markAccepted(task);
    task.phase_gate_state = "closed";
  }
  gate.strict_acceptance_state = "accepted";
  gate.phase_gate_state = "closed";
  candidate.task_records.sort(
    (left, right) => planOrder.indexOf(left.id) - planOrder.indexOf(right.id)
  );
  syncInventory(candidate);
}

test("current ledger matches the 112-task plan", () => {
  assert.equal(planOrder.length, 112);
  assert.doesNotThrow(() => validateProgramStatus(status, plan));
});

test("renderer is deterministic and distinguishes delivery from acceptance", () => {
  const first = renderProgramStatus(status);
  assert.equal(first, renderProgramStatus(clone(status)));
  // Inventory is ledger-driven; must match landed_task_count (not a frozen constant).
  const landed = status.original_plan.landed_task_count;
  assert.match(first, /\d+ \/ 112/);
  assert.match(first, new RegExp(`${landed} / 112`));
  assert.ok(
    landed >= 21,
    "inventory must not regress below post-P3.2 baseline"
  );
  assert.match(first, /landed.*merged.*open.*open/);
  assert.match(first, /[^\n]\n$/);
  assert.doesNotMatch(first, /\n\n$/);
});

test("preserves historical full-vertical execution and deletion policy", () => {
  const rendered = renderProgramStatus(status);
  const currentInventory =
    status.vertical_execution.matrix_js_sdk_import_inventory.current;
  assert.match(rendered, /Full-vertical product execution at this snapshot/);
  assert.match(rendered, /full-vertical-delete-per-vertical/);
  assert.match(rendered, /Wired \/ deletion open/);
  assert.match(
    rendered,
    new RegExp(
      `${currentInventory.files} files / ${currentInventory.import_lines} import lines current`
    )
  );
  assert.match(rendered, /negative capability-owner\/file deletion delta/);
  assert.match(
    rendered,
    /Integration product state: `between-slices-paused`/
  );
  assert.ok(
    rendered.includes(
      `Active slice: **${status.vertical_execution.active_slice ?? "None"}**${
        status.vertical_execution.active_pr === null ? "" : ` (PR #${status.vertical_execution.active_pr})`
      }`
    )
  );
  assert.match(
    rendered,
    /Completed under full policy: .*`V-AUTH\.1`.*`V-ROOMS\.5`/
  );

  const wrongPolicy = clone(status);
  wrongPolicy.vertical_execution.policy = "bulk-delete-at-end";
  assert.throws(
    () => validateProgramStatus(wrongPolicy, plan),
    /must require per-vertical deletion/
  );

  const wrongDeltaRule = clone(status);
  wrongDeltaRule.vertical_execution.completion_evidence.capability_owner_delta =
    "optional";
  assert.throws(
    () => validateProgramStatus(wrongDeltaRule, plan),
    /negative capability-owner delta/
  );

  const duplicateResidual = clone(status);
  duplicateResidual.vertical_execution.next_slices.push("V-AUTH.2");
  assert.throws(
    () => validateProgramStatus(duplicateResidual, plan),
    /duplicate IDs/
  );

  const importerRegression = clone(status);
  importerRegression.vertical_execution.matrix_js_sdk_import_inventory.current.files =
    importerRegression.vertical_execution.matrix_js_sdk_import_inventory
      .baseline.files + 1;
  assert.throws(
    () => validateProgramStatus(importerRegression, plan),
    /cannot exceed the full-vertical baseline/
  );

  const pausedWithActiveWork = clone(status);
  pausedWithActiveWork.vertical_execution.integration_product_state =
    "between-slices-paused";
  pausedWithActiveWork.vertical_execution.active_slice = "V-AUTH.1";
  pausedWithActiveWork.vertical_execution.active_pr = 237;
  assert.throws(
    () => validateProgramStatus(pausedWithActiveWork, plan),
    /between-slices-paused requires null active_slice and active_pr/
  );

  const resumedWithoutActiveWork = clone(status);
  resumedWithoutActiveWork.vertical_execution.integration_product_state =
    "capability-cutover-in-progress";
  resumedWithoutActiveWork.vertical_execution.active_slice = null;
  resumedWithoutActiveWork.vertical_execution.active_pr = null;
  assert.throws(
    () => validateProgramStatus(resumedWithoutActiveWork, plan),
    /active_slice must be a vertical ID while cutover is in progress/
  );
});

test("rejects invalid vocabulary, duplicate IDs, and inventory drift", () => {
  const badVocabulary = clone(status);
  badVocabulary.task_records[0].strict_acceptance_state = "complete-ish";
  assert.throws(
    () => validateProgramStatus(badVocabulary, plan),
    /invalid value/
  );

  const duplicate = clone(status);
  duplicate.original_plan.landed_task_ids[19] = "P0.1";
  assert.throws(() => validateProgramStatus(duplicate, plan), /duplicate IDs/);

  const countDrift = clone(status);
  countDrift.original_plan.landed_task_count -= 1;
  assert.throws(
    () => validateProgramStatus(countDrift, plan),
    /does not match/
  );
});

test("rejects phase closure until tasks and audited remediations are accepted", () => {
  const premature = clone(status);
  premature.phase_gates[1].phase_gate_state = "closed";
  premature.phase_gates[1].strict_acceptance_state = "accepted";
  assert.throws(
    () => validateProgramStatus(premature, plan),
    /cannot close until/
  );

  const valid = clone(status);
  closePhase(valid, 0);
  assert.throws(
    () => validateProgramStatus(valid, plan),
    /cannot close until remediation R0\.2/
  );
  for (const id of ["R0.2", "R0.8"]) {
    markAccepted(valid.remediation_tasks.find((task) => task.id === id));
  }
  clearInProgressExcept(valid, null);
  valid.current_execution.active_task = null;
  valid.current_execution.next_task = "R0.3";
  syncP32Block(valid);
  assert.doesNotThrow(() => validateProgramStatus(valid, plan));
});

test("accepts residual R0 activation without checker edits", () => {
  // Product-first: residual formal R0 work may still be in progress while
  // product tasks (P3.2+) advance. Simulate R0.7 residual as the active task.
  const future = clone(status);
  clearInProgressExcept(future, "R0.7");
  const r07 = future.remediation_tasks.find((task) => task.id === "R0.7");
  r07.artifact_state = "in_progress";
  r07.integration_state = "pr_open";
  r07.strict_acceptance_state = "open";
  future.current_execution.active_task = "R0.7";
  future.current_execution.next_task = "P3.3";
  future.current_execution.blocked_tasks = [];
  assert.doesNotThrow(() => validateProgramStatus(future, plan));
  assert.match(renderProgramStatus(future), /Active task: \*\*R0\.7\*\*/);
});

test("residual R0 gates do not hard-block P3.2; optional residual notes stay consistent", () => {
  // Current ledger: unaccepted remediations remain, but blocked_tasks is empty
  // so product P3.x work is not hard-gated (policy 2026-07-27).
  assert.ok(
    status.remediation_tasks.some(
      (task) => task.strict_acceptance_state !== "accepted"
    )
  );
  assert.deepEqual(status.current_execution.blocked_tasks, []);
  assert.doesNotThrow(() => validateProgramStatus(status, plan));
  assert.match(renderProgramStatus(status), /Blocked tasks: None/);

  // Optional residual-awareness block is allowed when blockers are unaccepted.
  const residualNote = clone(status);
  residualNote.current_execution.blocked_tasks = [
    { id: "P3.2", blocked_by: ["R0.7", "R0.8"] },
  ];
  assert.doesNotThrow(() => validateProgramStatus(residualNote, plan));

  // Accepted remediations cannot appear as residual blockers.
  const badBlocker = clone(status);
  badBlocker.current_execution.blocked_tasks = [
    { id: "P3.2", blocked_by: ["R0.1"] },
  ];
  assert.throws(
    () => validateProgramStatus(badBlocker, plan),
    /unaccepted R0 remediation/
  );

  // Clearing all remediations still leaves product free to proceed.
  const future = clone(status);
  for (const task of future.remediation_tasks) markAccepted(task);
  future.current_execution.active_task = null;
  future.current_execution.next_task = "P3.3";
  syncP32Block(future);
  assert.doesNotThrow(() => validateProgramStatus(future, plan));
  assert.match(renderProgramStatus(future), /Blocked tasks: None/);
});

test("rejects premature cutover and accepts the allowed final runtime transition", () => {
  const premature = clone(status);
  premature.product_runtime.matrix_client_sdk = "matrix-rust-sdk-only";
  premature.product_runtime.rust_sdk_state = "sole-production-runtime";
  premature.product_runtime.cutover_state = "complete";
  assert.throws(
    () => validateProgramStatus(premature, plan),
    /cannot complete before Phase 0/
  );

  const future = clone(status);
  for (const task of future.remediation_tasks) markAccepted(task);
  future.current_execution.active_task = null;
  future.current_execution.next_task = "P3.2";
  syncP32Block(future);
  for (let phase = 0; phase <= 11; phase += 1) closePhase(future, phase);
  future.product_runtime.matrix_client_sdk = "matrix-rust-sdk-only";
  future.product_runtime.rust_sdk_state = "sole-production-runtime";
  future.product_runtime.cutover_state = "complete";
  assert.doesNotThrow(() => validateProgramStatus(future, plan));
});

test("rejects plan task-count drift", () => {
  assert.throws(
    () => validateProgramStatus(status, plan.replace("P14.6", "P14-final")),
    /112 original task IDs/
  );
});

test("rejects stale generic task metadata", () => {
  const canonical = status.task_records.find(({ id }) => id === "P2.1");
  const valid = { taskId: "P2.1", ...canonical };
  assert.doesNotThrow(() => validateTaskRecord(valid, "p2.1.json", canonical));
  for (const staleField of ["status", "merged", "next_phase0"]) {
    assert.throws(
      () =>
        validateTaskRecord(
          { ...valid, [staleField]: "stale" },
          "p2.1.json",
          canonical
        ),
      /forbidden/
    );
  }
});

test("rejects current-doc marker and generated Markdown drift", () => {
  const currentDoc =
    "<!-- matrix-rust-program-status-link -->\n[status](program-status.md)\n";
  assert.doesNotThrow(() => validateCurrentDoc(currentDoc, "current.md"));
  assert.throws(
    () => validateCurrentDoc("[status](program-status.md)", "current.md"),
    /missing/
  );
  assert.throws(
    () =>
      validateCurrentDoc(
        "<!-- matrix-rust-program-status-link -->",
        "current.md"
      ),
    /does not link/
  );

  const rendered = renderProgramStatus(status);
  assert.doesNotThrow(() => validateRenderedStatus(rendered, status));
  assert.throws(
    () => validateRenderedStatus(`${rendered}\n`, status),
    /is stale/
  );
});
