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

function syncP32Block(candidate) {
  const blockers = candidate.remediation_tasks
    .filter(
      ({ strict_acceptance_state: acceptance }) => acceptance !== "accepted"
    )
    .map(({ id }) => id);
  candidate.current_execution.blocked_tasks =
    blockers.length === 0 ? [] : [{ id: "P3.2", blocked_by: blockers }];
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
  assert.match(first, /20 \/ 112/);
  assert.match(first, /landed.*merged.*open.*open/);
  assert.match(first, /[^\n]\n$/);
  assert.doesNotMatch(first, /\n\n$/);
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

test("accepts R0.1 completion and R0.2 activation without checker edits", () => {
  const future = clone(status);
  markAccepted(future.remediation_tasks[0]);
  clearInProgressExcept(future, "R0.2");
  // Residual R0.2 work (e.g. E2) after E1 may land/merge: model activation as
  // in_progress + pr_open, never in_progress + merged (state machine forbids it).
  future.remediation_tasks[1].artifact_state = "in_progress";
  future.remediation_tasks[1].integration_state = "pr_open";
  future.remediation_tasks[1].strict_acceptance_state = "open";
  future.current_execution.active_task = "R0.2";
  future.current_execution.next_task = "R0.3";
  syncP32Block(future);
  assert.doesNotThrow(() => validateProgramStatus(future, plan));
  assert.match(renderProgramStatus(future), /Active task: \*\*R0\.2\*\*/);
});

test("P3.2 is blocked iff an R0 remediation remains unaccepted", () => {
  const missingBlocker = clone(status);
  missingBlocker.current_execution.blocked_tasks[0].blocked_by.pop();
  assert.throws(
    () => validateProgramStatus(missingBlocker, plan),
    /exactly match/
  );

  const future = clone(status);
  for (const task of future.remediation_tasks) markAccepted(task);
  future.current_execution.active_task = null;
  future.current_execution.next_task = "P3.2";
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
