import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const REQUIRED_SECTIONS = [
  "## Evidence Rules",
  "## Common Preflight",
  "## macOS Desktop Smoke",
  "## Linux Desktop Smoke",
  "## Timeline Resurrection Smoke",
  "## iOS Tool-Bound Smoke",
  "## Updater Release Smoke",
  "## Signoff Table",
];

const REQUIRED_COMMANDS = [
  "git status --short --branch",
  "git rev-parse --short HEAD",
  "npm run check:versions",
  "npm run check:repo-layout",
  "npm run check:matrix-boundaries",
  "npm run check:release-updater",
  "npm run check:release-updater -- --require-enabled",
];

const REQUIRED_CASE_IDS = [
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
  "IOS-001",
  "IOS-002",
  "IOS-003",
  "IOS-004",
  "UPD-001",
  "UPD-002",
  "UPD-003",
  "UPD-004",
];

const REQUIRED_SIGNOFF_ROWS = [
  "Common preflight",
  "macOS desktop smoke",
  "Linux desktop smoke",
  "Timeline Resurrection smoke",
  "iOS tool-bound smoke",
  "Updater release smoke",
];

const REQUIRED_IOS_QUEUE_IDS = [
  "MAC-IOS-001",
  "MAC-IOS-002",
  "MAC-IOS-003",
  "MAC-IOS-004",
  "MAC-IOS-005",
];

const escapeRegExp = (value) =>
  value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const hasMarkdownTableRow = (text, firstCell) =>
  new RegExp(`^\\|\\s*${escapeRegExp(firstCell)}\\s*\\|`, "m").test(text);

export function inspectProductionSmokeChecklist({
  checklist,
  macosIosQueue,
}) {
  const errors = [];

  for (const section of REQUIRED_SECTIONS) {
    if (!checklist.includes(section)) {
      errors.push(`Missing required smoke checklist section: ${section}`);
    }
  }

  for (const command of REQUIRED_COMMANDS) {
    if (!checklist.includes(command)) {
      errors.push(`Missing required smoke checklist command: ${command}`);
    }
  }

  for (const caseId of REQUIRED_CASE_IDS) {
    if (!hasMarkdownTableRow(checklist, caseId)) {
      errors.push(`Missing required smoke case row: ${caseId}`);
    }
  }

  for (const row of REQUIRED_SIGNOFF_ROWS) {
    if (!hasMarkdownTableRow(checklist, row)) {
      errors.push(`Missing required signoff row: ${row}`);
    }
  }

  if (!macosIosQueue.includes("docs/production-smoke-checklist.md")) {
    errors.push(
      "MACOS_IOS_VALIDATION_QUEUE.md must link to docs/production-smoke-checklist.md."
    );
  }

  for (const queueId of REQUIRED_IOS_QUEUE_IDS) {
    if (!hasMarkdownTableRow(macosIosQueue, queueId)) {
      errors.push(`Missing required macOS/iOS queue row: ${queueId}`);
    }
  }

  return {
    ok: errors.length === 0,
    errors,
  };
}

function main() {
  const result = inspectProductionSmokeChecklist({
    checklist: readFileSync(
      path.join(root, "docs/production-smoke-checklist.md"),
      "utf8"
    ),
    macosIosQueue: readFileSync(
      path.join(root, "MACOS_IOS_VALIDATION_QUEUE.md"),
      "utf8"
    ),
  });

  for (const error of result.errors) {
    console.error(`[production-smoke] ${error}`);
  }

  if (!result.ok) {
    console.error("[production-smoke] checklist gate failed.");
    process.exit(1);
  }

  console.log("[production-smoke] checklist coverage is complete.");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
