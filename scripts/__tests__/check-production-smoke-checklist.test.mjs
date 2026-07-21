import test from "node:test";
import assert from "node:assert/strict";

import { inspectProductionSmokeChecklist } from "../check-production-smoke-checklist.mjs";

const sections = [
  "## Evidence Rules",
  "## Common Preflight",
  "## macOS Desktop Smoke",
  "## Linux Desktop Smoke",
  "## Timeline Resurrection Smoke",
  "## iOS Tool-Bound Smoke",
  "## Updater Release Smoke",
  "## Signoff Table",
].join("\n");

const commands = [
  "git status --short --branch",
  "git rev-parse --short HEAD",
  "npm run check:versions",
  "npm run check:repo-layout",
  "npm run check:matrix-boundaries",
  "npm run check:quality-gates",
  "npm run check:synapse-harness",
  "npm run check:release-updater",
  "npm run check:release-updater -- --require-enabled",
].join("\n");

const caseIds = [
  "MAC-DESK-001",
  "MAC-DESK-002",
  "MAC-DESK-003",
  "MAC-DESK-004",
  "MAC-DESK-005",
  "MAC-DESK-006",
  "MAC-DESK-007",
  "MAC-DESK-008",
  "MAC-DESK-009",
  "MAC-DESK-010",
  "LINUX-DESK-001",
  "LINUX-DESK-002",
  "LINUX-DESK-003",
  "LINUX-DESK-004",
  "LINUX-DESK-005",
  "LINUX-DESK-006",
  "LINUX-DESK-007",
  "LINUX-DESK-008",
  "TL-001",
  "TL-002",
  "TL-003",
  "TL-004",
  "TL-005",
  "TL-006",
  "TL-007",
  "TL-008",
  "TL-009",
  "TL-010",
  "TL-011",
  "TL-012",
  "IOS-001",
  "IOS-002",
  "IOS-003",
  "IOS-004",
  "UPD-001",
  "UPD-002",
  "UPD-003",
  "UPD-004",
];

const signoffRows = [
  "Common preflight",
  "macOS desktop smoke",
  "Linux desktop smoke",
  "Timeline Resurrection smoke",
  "iOS tool-bound smoke",
  "Updater release smoke",
];

const completeChecklist = [
  sections,
  commands,
  ...caseIds.map((id) => `| ${id} | Area | Criteria | Evidence |`),
  ...signoffRows.map((row) => `| ${row} | Yes | Pending | |`),
].join("\n");

const completeQueue = [
  "docs/production-smoke-checklist.md",
  "| MAC-IOS-001 | P0 | Timeline | Command | Evidence | Pending |",
  "| MAC-IOS-002 | P0 | Timeline | Command | Evidence | Pending |",
  "| MAC-IOS-003 | P0 | Link | Command | Evidence | Pending |",
  "| MAC-IOS-004 | P1 | Release | Command | Evidence | Pending |",
  "| MAC-IOS-005 | P0 | Composer | Command | Evidence | Pending |",
].join("\n");

test("production smoke checklist gate accepts complete coverage", () => {
  const result = inspectProductionSmokeChecklist({
    checklist: completeChecklist,
    macosIosQueue: completeQueue,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("production smoke checklist gate rejects missing case rows", () => {
  const result = inspectProductionSmokeChecklist({
    checklist: completeChecklist.replace("| TL-010 |", "| TL-MISSING |"),
    macosIosQueue: completeQueue,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /TL-010/);
});

test("production smoke checklist gate requires macos ios queue linkage", () => {
  const result = inspectProductionSmokeChecklist({
    checklist: completeChecklist,
    macosIosQueue: completeQueue.replace(
      "docs/production-smoke-checklist.md",
      "docs/other.md"
    ),
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /production-smoke-checklist/);
});
