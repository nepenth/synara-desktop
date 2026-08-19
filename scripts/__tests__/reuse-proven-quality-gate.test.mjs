import test from "node:test";
import assert from "node:assert/strict";

import {
  QUALITY_GATE_CHECK_NAME,
  decideProvenQualityGateWithParents,
  isSuccessfulQualityGate,
  secondParentOf,
} from "../reuse-proven-quality-gate.mjs";

const green = {
  name: QUALITY_GATE_CHECK_NAME,
  status: "completed",
  conclusion: "success",
};

test("secondParentOf returns only the incoming merge parent", () => {
  assert.equal(
    secondParentOf(
      "mergeaaaa parentbbbb incomingcc",
      "mergeaaaa"
    ),
    "incomingcc"
  );
  assert.equal(secondParentOf("aaaa parentbbbb", "aaaa"), null);
  assert.equal(secondParentOf("other parentbbbb incomingcc", "aaaa"), null);
});

test("isSuccessfulQualityGate rejects incomplete or failed gates", () => {
  assert.equal(isSuccessfulQualityGate(green), true);
  assert.equal(
    isSuccessfulQualityGate({
      name: QUALITY_GATE_CHECK_NAME,
      status: "in_progress",
      conclusion: null,
    }),
    false
  );
  assert.equal(
    isSuccessfulQualityGate({
      name: QUALITY_GATE_CHECK_NAME,
      status: "completed",
      conclusion: "failure",
    }),
    false
  );
  assert.equal(
    isSuccessfulQualityGate({
      name: "Validate Node desktop runtime",
      status: "completed",
      conclusion: "success",
    }),
    false
  );
});

test("reuses a Quality gate already recorded on the tagged SHA", () => {
  assert.deepEqual(
    decideProvenQualityGateWithParents({
      sha: "tagsha",
      secondParent: "incoming",
      checkRunsBySha: { tagsha: [green], incoming: [] },
    }),
    { reuse: true, provenSha: "tagsha" }
  );
});

test("reuses the incoming PR parent of a merge commit, not first-parent main", () => {
  assert.deepEqual(
    decideProvenQualityGateWithParents({
      sha: "merge",
      secondParent: "prhead",
      checkRunsBySha: {
        merge: [],
        mainparent: [green],
        prhead: [green],
      },
    }),
    { reuse: true, provenSha: "prhead" }
  );
});

test("does not reuse first-parent main when the incoming PR is unproven", () => {
  assert.deepEqual(
    decideProvenQualityGateWithParents({
      sha: "merge",
      secondParent: "prhead",
      checkRunsBySha: {
        merge: [],
        mainparent: [green],
        prhead: [],
      },
    }),
    { reuse: false, provenSha: null }
  );
});

test("runs exact-tag validation when no proven Quality gate exists", () => {
  assert.deepEqual(
    decideProvenQualityGateWithParents({
      sha: "tagsha",
      secondParent: null,
      checkRunsBySha: { tagsha: [] },
    }),
    { reuse: false, provenSha: null }
  );
});
