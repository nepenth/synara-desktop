// V-BURN completion validation loop.
//
// Once the burn is declared complete, this check walks the matrix-rust-sdk
// replacement surface and validates implementation + documentation, failing
// fast if any leftover matrix-js-sdk coupling survives:
//
//   1. 0 production + 0 test js-sdk importers (desktop-sdk-usage inventory).
//   2. P1.6 allowlist empty = full ban.
//   3. matrix-js-sdk absent from package.json (deps + devDeps) and lockfile.
//   4. No matrix-js-sdk import/require/dynamic-import in executable source,
//      tooling, or CI outside the explicit detector/fixture allowlist.
//   5. Every native Synapse proof job (reactions, attachments, call-media,
//      polls, rich-messages, threads, two-client receipts) is wired in CI.
//   6. Tracking docs carry the V-BURN reached marker.
//
// Run via `npm run check:matrix-rust-guardrails` (appended to the guardrail
// chain) and standing alone via `node scripts/check-v-burn-complete.mjs`.

import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { resolve, join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));

const findings = [];
let okCount = 0;

function check(name, passes, detail = "") {
  if (passes) {
    okCount += 1;
    console.log(`[v-burn] ok — ${name}`);
  } else {
    findings.push(`${name}${detail ? ` — ${detail}` : ""}`);
    console.error(`[v-burn] VIOLATION — ${name}${detail ? `: ${detail}` : ""}`);
  }
}

const SKIP_DIRS = new Set([
  "node_modules",
  "target",
  ".git",
  "dist",
  "build",
  "coverage",
]);

function walkFiles(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (SKIP_DIRS.has(entry)) continue;
    if (statSync(full).isDirectory()) walkFiles(full, acc);
    else acc.push(full);
  }
  return acc;
}

// --- 1. Inventory: desktop runtime importers ---
const usagePath = join(ROOT, "docs/matrix-rust-sdk/desktop-sdk-usage.json");
if (existsSync(usagePath)) {
  const usage = JSON.parse(readFileSync(usagePath, "utf8"));
  const baseline = usage.summary?.desktopRuntimeBaseline ?? {};
  check(
    "0 production js-sdk importers",
    baseline.productionImportFiles === 0,
    `productionImportFiles=${baseline.productionImportFiles}`
  );
  check(
    "0 test js-sdk importers",
    baseline.testImportFiles === 0,
    `testImportFiles=${baseline.testImportFiles}`
  );
} else {
  check("desktop-sdk-usage inventory exists", false, "missing inventory");
}

// --- 2. Allowlist = empty (full ban) ---
const allowPath = join(ROOT, "docs/matrix-rust-sdk/p1.6-js-sdk-import-allowlist.json");
if (existsSync(allowPath)) {
  const allow = JSON.parse(readFileSync(allowPath, "utf8"));
  check(
    "P1.6 allowlist empty (full ban)",
    allow.pathCount === 0 && Array.isArray(allow.paths) && allow.paths.length === 0,
    `pathCount=${allow.pathCount} paths=${(allow.paths ?? []).length}`
  );
} else {
  check("P1.6 allowlist exists", false, "missing allowlist");
}

// --- 3. package.json / lockfile ---
const pkgPath = join(ROOT, "synara/package.json");
if (existsSync(pkgPath)) {
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  const deps = Object.keys(pkg.dependencies ?? {});
  const devDeps = Object.keys(pkg.devDependencies ?? {});
  check(
    "matrix-js-sdk absent from production dependencies",
    !deps.includes("matrix-js-sdk")
  );
  check(
    "matrix-js-sdk absent from devDependencies",
    !devDeps.includes("matrix-js-sdk")
  );
}
const lockPath = join(ROOT, "synara/package-lock.json");
if (existsSync(lockPath)) {
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  check(
    "matrix-js-sdk absent from lockfile packages",
    !(lock.packages ?? {})["node_modules/matrix-js-sdk"]
  );
}

// --- 4. No js-sdk coupling in executable source/tooling/CI ---
// Detector/guardrail/fixture files legitimately contain the literal for
// ban-detection tests; docs record history. Everything else must be clean.
const literal = "matrix-js-sdk";
const allowedDetectorSuffixes = [
  "inventory-matrix-sdk-usage.mjs",
  "inventory-matrix-sdk-usage.test.mjs",
  "matrix-rust-p1.6-guardrails.mjs",
  "matrix-rust-p1.6-guardrails.test.mjs",
  "check-matrix-rust-sdk-guardrails.mjs",
  "check-matrix-rust-sdk-guardrails.test.mjs",
  "check-matrix-boundaries.mjs",
  "check-matrix-boundaries.test.mjs",
];
const fixtureDirs = [join(ROOT, "scripts/fixtures/")];
const docDirs = [join(ROOT, "docs/")];
const scanRoots = [
  join(ROOT, "synara/src"),
  join(ROOT, "synara/scripts"),
  join(ROOT, "src-tauri/src"),
  join(ROOT, "src-tauri"),
  join(ROOT, "scripts"),
  join(ROOT, ".github/workflows"),
];

const coupling = [];
for (const rootDir of scanRoots) {
  if (!existsSync(rootDir)) continue;
  for (const file of walkFiles(rootDir)) {
    if (extname(file) === ".json" && file.endsWith("Cargo.lock")) continue;
    if (
      docDirs.some((d) => file.startsWith(d)) ||
      fixtureDirs.some((d) => file.startsWith(d))
    ) {
      continue;
    }
    if (allowedDetectorSuffixes.some((s) => file.endsWith(s))) continue;
    const base = file.split("/").pop() ?? "";
    if (base.startsWith(".")) continue;
    let text;
    try {
      text = readFileSync(file, "utf8");
    } catch {
      continue;
    }
    // Flag only actual import/require statements (static, type-only, require,
    // dynamic, and /subpath). Comments, absence-guard assertions, and docs are
    // not coupling.
    const importPattern =
      /(?:from\s+['"]matrix-js-sdk|require\(\s*['"]matrix-js-sdk|import\(\s*['"]matrix-js-sdk)/;
    for (const line of text.split(/\r?\n/)) {
      if (importPattern.test(line)) {
        coupling.push(`${file.replace(ROOT + "/", "")}: ${line.trim().slice(0, 90)}`);
        break;
      }
    }
  }
}
// src-tauri.json/package-lock.json + Cargo manifests do not contain the string;
// only report actual coupling (the allowlist already skipped detectors/docs).
check(
  "no matrix-js-sdk coupling in source/tooling/CI (outside detectors/fixtures/docs)",
  coupling.length === 0,
  coupling.slice(0, 6).join(", ")
);

// --- 5. Native Synapse proof family wired in CI ---
const ciPath = join(ROOT, ".github/workflows/ci.yml");
if (existsSync(ciPath)) {
  const ci = readFileSync(ciPath, "utf8");
  const requiredProofJobs = [
    "synapse-native-reactions",
    "synapse-native-attachments",
    "synapse-native-call-media",
    "synapse-native-polls",
    "synapse-native-rich-messages",
    "synapse-native-threads",
    "synapse-native-receipts",
  ];
  for (const job of requiredProofJobs) {
    check(`CI native proof job ${job} present`, ci.includes(`  ${job}:\n`));
  }
} else {
  check("CI workflow exists", false, "missing ci.yml");
}

// --- 6. Tracking docs carry V-BURN reached marker ---
const docs = [
  "docs/matrix-rust-sdk/PROGRESS.md",
  "docs/matrix-rust-sdk/SCOREBOARD.md",
  "docs/matrix-rust-sdk/v-burn-importer-taxonomy.md",
];
for (const doc of docs) {
  const full = join(ROOT, doc);
  check(
    `${doc} records V-BURN reached`,
    existsSync(full) && /V-BURN/i.test(readFileSync(full, "utf8")),
    "missing file or V-BURN marker"
  );
}

console.log(`\n[v-burn] ${okCount} checks ok, ${findings.length} violation(s).`);
if (findings.length > 0) {
  console.error("[v-burn] V-BURN completion validation FAILED.");
  process.exit(1);
}
console.log("[v-burn] V-BURN completion validation passed.");
