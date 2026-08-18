import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  formatViolations,
  runGuardrails,
} from "./check-matrix-rust-sdk-guardrails.mjs";
import { runGuardrails as runP16AllowlistGuardrails } from "./matrix-rust-p1.6-guardrails.mjs";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));

const trackedFiles = execFileSync("git", ["ls-files"], {
  cwd: root,
  encoding: "utf8",
})
  .split("\n")
  .filter(Boolean);

const untrackedFiles = execFileSync(
  "git",
  ["ls-files", "--others", "--exclude-standard"],
  {
    cwd: root,
    encoding: "utf8",
  }
)
  .split("\n")
  .filter(Boolean);

// `git ls-files` retains unstaged deletions. Boundary checks should inspect the
// working tree that will be validated, not try to read paths already removed.
const repositoryFiles = [
  ...new Set([...trackedFiles, ...untrackedFiles]),
].filter((path) => existsSync(resolve(root, path)));

const IOS_ALLOWED_DIRECT_MATRIX_PATHS = new Map([
  [
    "synara-ios/Synara/Services/MatrixRustSDKService.swift",
    "IOS-REST-EXCEPTION-006: device display-name patch pending SDK device-display-name support.",
  ],
  [
    "synara-ios/Synara/Services/RoomReadMarkerService.swift",
    "IOS-REST-EXCEPTION-007: room read-marker account-data lookup pending SDK read-receipt/account-data support.",
  ],
]);

const DESKTOP_ALLOWED_DIRECT_MATRIX_PATHS = new Map([
  [
    "synara/src/sw.ts",
    "DESKTOP-REST-EXCEPTION-001: service worker injects Matrix auth for media requests in the Tauri WebView runtime.",
  ],
  [
    "synara/src/app/cs-api.ts",
    "DESKTOP-REST-EXCEPTION-002: login-time homeserver version discovery helper.",
  ],
  [
    "src-tauri/src/matrix/auth/http_transport.rs",
    "DESKTOP-REST-EXCEPTION-003 / R0.7-CS-API-001: read-only well-known + login-types listing (no credentials; no dual-backend).",
  ],
]);

const DOCS_ALLOWLIST = [
  "docs/",
  "synara/docs/",
  "synara-ios/docs/",
  "README.md",
  "MODERNIZATION.md",
  "CHANGELOG.md",
];

function isDocumentation(path) {
  return DOCS_ALLOWLIST.some(
    (prefix) => path === prefix || path.startsWith(prefix)
  );
}

function lineHits(path, patterns) {
  const text = readFileSync(resolve(root, path), "utf8");
  const lines = text.split(/\r?\n/);
  const hits = [];
  for (const [index, line] of lines.entries()) {
    if (patterns.some((pattern) => pattern.test(line))) {
      hits.push({ line: index + 1, text: line.trim() });
    }
  }
  return hits;
}

function fail(message) {
  console.error(`[matrix-boundaries] ${message}`);
  process.exitCode = 1;
}

const iosFiles = repositoryFiles.filter(
  (path) =>
    path.startsWith("synara-ios/Synara/") &&
    path.endsWith(".swift") &&
    !path.includes("/SynaraTests/") &&
    !path.includes("/SynaraUITests/")
);

const activeIOSExceptions = new Set();
for (const file of iosFiles) {
  const hits = lineHits(file, [
    /URLSession\.shared/,
    /appendPathComponent\("_matrix"\)/,
    /\/_matrix\//,
  ]);
  if (hits.length === 0) continue;
  if (IOS_ALLOWED_DIRECT_MATRIX_PATHS.has(file)) {
    activeIOSExceptions.add(file);
    continue;
  }
  fail(
    `${file} contains direct Matrix networking outside an approved SDK boundary: ` +
      hits.map((hit) => `${hit.line}`).join(", ")
  );
}

const desktopFiles = repositoryFiles.filter(
  (path) =>
    (path.startsWith("synara/src/") || path.startsWith("src-tauri/src/")) &&
    /\.(ts|tsx|js|jsx|rs)$/.test(path) &&
    !path.includes("/__tests__/")
);

const activeDesktopExceptions = new Set();
for (const file of desktopFiles) {
  const hits = lineHits(file, [/\/_matrix\//]);
  if (hits.length === 0) continue;
  if (DESKTOP_ALLOWED_DIRECT_MATRIX_PATHS.has(file)) {
    activeDesktopExceptions.add(file);
    continue;
  }
  if (isDocumentation(file)) continue;
  fail(
    `${file} contains direct Matrix REST endpoint usage outside the desktop Matrix boundary: ` +
      hits.map((hit) => `${hit.line}`).join(", ")
  );
}

if (process.exitCode) {
  console.error("\nApproved iOS exceptions:");
  for (const [path, reason] of IOS_ALLOWED_DIRECT_MATRIX_PATHS) {
    console.error(`- ${path}: ${reason}`);
  }
  console.error("\nApproved desktop exceptions:");
  for (const [path, reason] of DESKTOP_ALLOWED_DIRECT_MATRIX_PATHS) {
    console.error(`- ${path}: ${reason}`);
  }
  process.exit(process.exitCode);
}

console.log(
  `Matrix boundary check passed with ${activeIOSExceptions.size} active iOS exceptions and ` +
    `${activeDesktopExceptions.size} active desktop exceptions.`
);

// P1.6 — Matrix Rust SDK replacement architectural guardrails.
// (1) JS SDK allowlist freeze + wire-module bans + raw HTTP + versioned IPC
const p16 = runP16AllowlistGuardrails({ root, files: repositoryFiles });
if (!p16.ok) {
  console.error(
    `[matrix-boundaries] P1.6 allowlist/wire guardrails failed with ${p16.findingCount} finding(s)`
  );
  for (const f of p16.findings) {
    console.error(`  - [${f.rule}] ${f.path}: ${f.message}`);
  }
  console.error(
    "\nSee docs/matrix-rust-sdk/p1.6-architectural-guardrails.md for rules."
  );
  process.exit(1);
}
console.log(
  `[matrix-boundaries] P1.6 allowlist/wire guardrails passed (${p16.fileCount} files; allowlist ${p16.allowlistSize}).`
);

// (2) Dual-backend ban, no production Client under matrix/, no matrix_* Tauri cmds
const rustGuardrails = runGuardrails({ root, files: repositoryFiles });
if (!rustGuardrails.ok) {
  console.error(`[matrix-boundaries] ${rustGuardrails.summary}`);
  console.error(formatViolations(rustGuardrails.violations));
  console.error(
    "\nSee docs/matrix-rust-sdk/p1.6-architectural-guardrails.md for rules."
  );
  process.exit(1);
}
console.log(`[matrix-boundaries] ${rustGuardrails.summary}`);
