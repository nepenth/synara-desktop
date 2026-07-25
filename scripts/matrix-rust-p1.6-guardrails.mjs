#!/usr/bin/env node
/**
 * P1.6 — Architectural CI guardrails for Matrix Rust SDK full replacement.
 *
 * Phased checks (must not break the still-JS product during migration):
 *   1. JS SDK imports: hard-ban in matrix-ipc / matrix-dto; freeze new
 *      production importers outside the committed allowlist.
 *   2. Raw Matrix runtime HTTP: ban `/_matrix/` in product client code
 *      outside approved exception paths.
 *   3. Unversioned Matrix IPC: require protocolVersion on wire envelopes
 *      and MATRIX_IPC_PROTOCOL_VERSION in ipc modules.
 *   4. SDK types in DTO/IPC wire modules: ban matrix_sdk / ruma imports
 *      and type paths under src-tauri/src/matrix/{ipc,dto} and TS mirrors.
 *
 * Residual: full ban of matrix-js-sdk across all product paths waits until
 * cutover (plan Phase 11/14). Documented in
 * docs/matrix-rust-sdk/p1.6-architectural-guardrails.md
 *
 * Usage:
 *   node scripts/matrix-rust-p1.6-guardrails.mjs
 *   node scripts/matrix-rust-p1.6-guardrails.mjs --json
 *   node scripts/matrix-rust-p1.6-guardrails.mjs --root /path/to/tree
 */

import { execFileSync } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = path.resolve(SCRIPT_DIR, "..");
const SCHEMA_VERSION = 1;
const TASK_ID = "P1.6";

/** Hard-ban zones for matrix-js-sdk (migration surface). */
export const JS_SDK_HARD_BAN_PREFIXES = [
  "synara/src/app/features/matrix-ipc/",
  "synara/src/app/features/matrix-dto/",
];

/** Product runtime prefixes scanned for raw `/_matrix/` and JS SDK. */
export const PRODUCT_RUNTIME_PREFIXES = [
  "synara/src/",
  "src-tauri/src/",
];

/** Approved product paths that may contain `/_matrix/` literals. */
export const RAW_MATRIX_HTTP_ALLOWLIST = new Map([
  [
    "synara/src/sw.ts",
    "DESKTOP-REST-EXCEPTION-001: service worker injects Matrix auth for media.",
  ],
  [
    "synara/src/app/cs-api.ts",
    "DESKTOP-REST-EXCEPTION-002: login-time homeserver version discovery helper.",
  ],
]);

/** Rust wire modules that must not import matrix_sdk / ruma. */
export const RUST_WIRE_MODULE_PREFIXES = [
  "src-tauri/src/matrix/ipc/",
  "src-tauri/src/matrix/dto/",
];

/** TypeScript wire mirrors. */
export const TS_WIRE_MODULE_PREFIXES = [
  "synara/src/app/features/matrix-ipc/",
  "synara/src/app/features/matrix-dto/",
];

/** Relative path of the committed JS SDK importer allowlist. */
export const JS_SDK_ALLOWLIST_REL =
  "docs/matrix-rust-sdk/p1.6-js-sdk-import-allowlist.json";

const SOURCE_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
  ".rs",
]);

const JS_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
]);

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

export function toPosix(p) {
  return p.split(path.sep).join("/");
}

export function isTestPath(relPath) {
  const base = path.posix.basename(relPath);
  return (
    relPath.includes("/__tests__/") ||
    relPath.includes("/__mocks__/") ||
    relPath.includes("/tests/") ||
    /(^|\/)tests\.rs$/.test(relPath) ||
    /(^|\/)contract_tests\.rs$/.test(relPath) ||
    /\.(test|spec)\.[cm]?[jt]sx?$/.test(base)
  );
}

export function isDocumentationPath(relPath) {
  return (
    relPath.startsWith("docs/") ||
    relPath.startsWith("synara/docs/") ||
    relPath.startsWith("synara-ios/docs/") ||
    relPath === "README.md" ||
    relPath === "MODERNIZATION.md" ||
    relPath === "CHANGELOG.md"
  );
}

export function isScriptOrHarnessPath(relPath) {
  return (
    relPath.startsWith("scripts/") ||
    relPath.startsWith("synara/scripts/") ||
    relPath.startsWith("probes/") ||
    relPath.startsWith("integration/") ||
    relPath.startsWith("devAssets/")
  );
}

export function underAnyPrefix(relPath, prefixes) {
  return prefixes.some(
    (prefix) => relPath === prefix.replace(/\/$/, "") || relPath.startsWith(prefix)
  );
}

/**
 * Strip // line comments and /* block comments from source for pattern matching.
 * Keeps string contents approximately (not a full lexer).
 */
export function stripComments(source) {
  let out = "";
  let i = 0;
  const n = source.length;
  let inLine = false;
  let inBlock = false;
  let inSingle = false;
  let inDouble = false;
  let inBacktick = false;
  while (i < n) {
    const c = source[i];
    const next = i + 1 < n ? source[i + 1] : "";
    if (inLine) {
      if (c === "\n") {
        inLine = false;
        out += c;
      }
      i += 1;
      continue;
    }
    if (inBlock) {
      if (c === "*" && next === "/") {
        inBlock = false;
        i += 2;
        out += "  ";
        continue;
      }
      out += c === "\n" ? "\n" : " ";
      i += 1;
      continue;
    }
    if (inSingle) {
      out += c;
      if (c === "\\" && i + 1 < n) {
        out += source[i + 1];
        i += 2;
        continue;
      }
      if (c === "'") inSingle = false;
      i += 1;
      continue;
    }
    if (inDouble) {
      out += c;
      if (c === "\\" && i + 1 < n) {
        out += source[i + 1];
        i += 2;
        continue;
      }
      if (c === '"') inDouble = false;
      i += 1;
      continue;
    }
    if (inBacktick) {
      out += c;
      if (c === "\\" && i + 1 < n) {
        out += source[i + 1];
        i += 2;
        continue;
      }
      if (c === "`") inBacktick = false;
      i += 1;
      continue;
    }
    if (c === "/" && next === "/") {
      inLine = true;
      i += 2;
      continue;
    }
    if (c === "/" && next === "*") {
      inBlock = true;
      i += 2;
      continue;
    }
    if (c === "'") {
      inSingle = true;
      out += c;
      i += 1;
      continue;
    }
    if (c === '"') {
      inDouble = true;
      out += c;
      i += 1;
      continue;
    }
    if (c === "`") {
      inBacktick = true;
      out += c;
      i += 1;
      continue;
    }
    out += c;
    i += 1;
  }
  return out;
}

export function lineHits(source, patterns) {
  const lines = source.split(/\r?\n/);
  const hits = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    for (const pattern of patterns) {
      if (pattern.test(line)) {
        hits.push({ line: index + 1, text: line.trim(), pattern: String(pattern) });
        break;
      }
    }
  }
  return hits;
}

// ---------------------------------------------------------------------------
// File listing
// ---------------------------------------------------------------------------

/**
 * List repository-relative source files under root.
 * Prefer git ls-files when available; fall back to recursive walk.
 * @param {string} root
 * @param {{ includeUntracked?: boolean }} [opts]
 */
export function listRepositoryFiles(root, opts = {}) {
  const { includeUntracked = true } = opts;
  try {
    const tracked = execFileSync("git", ["ls-files"], {
      cwd: root,
      encoding: "utf8",
    })
      .split("\n")
      .filter(Boolean);
    let untracked = [];
    if (includeUntracked) {
      untracked = execFileSync(
        "git",
        ["ls-files", "--others", "--exclude-standard"],
        { cwd: root, encoding: "utf8" }
      )
        .split("\n")
        .filter(Boolean);
    }
    return [...new Set([...tracked, ...untracked])].map(toPosix).sort();
  } catch {
    return walkFiles(root).map((abs) => toPosix(path.relative(root, abs))).sort();
  }
}

function walkFiles(dir, acc = []) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return acc;
  }
  for (const entry of entries) {
    if (entry.name === "node_modules" || entry.name === "target" || entry.name === ".git") {
      continue;
    }
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkFiles(abs, acc);
    } else if (entry.isFile()) {
      acc.push(abs);
    }
  }
  return acc;
}

export function isSourceFile(relPath) {
  const ext = path.posix.extname(relPath);
  return SOURCE_EXTENSIONS.has(ext);
}

// ---------------------------------------------------------------------------
// Guardrail 1 — JS SDK imports
// ---------------------------------------------------------------------------

const JS_SDK_IMPORT_PATTERNS = [
  /from\s+['"]matrix-js-sdk(?:\/[^'"]*)?['"]/,
  /import\s+['"]matrix-js-sdk(?:\/[^'"]*)?['"]/,
  /require\(\s*['"]matrix-js-sdk(?:\/[^'"]*)?['"]\s*\)/,
  /import\(\s*['"]matrix-js-sdk(?:\/[^'"]*)?['"]\s*\)/,
];

export function findJsSdkImportHits(source) {
  const stripped = stripComments(source);
  return lineHits(stripped, JS_SDK_IMPORT_PATTERNS);
}

/**
 * Load committed allowlist of production files permitted to import matrix-js-sdk.
 * @param {string} root
 * @param {string} [relPath]
 * @returns {Set<string>}
 */
export function loadJsSdkAllowlist(root, relPath = JS_SDK_ALLOWLIST_REL) {
  const abs = path.join(root, relPath);
  if (!existsSync(abs)) {
    return new Set();
  }
  const raw = JSON.parse(readFileSync(abs, "utf8"));
  const paths = Array.isArray(raw) ? raw : raw.paths || [];
  return new Set(paths.map(toPosix));
}

/**
 * @param {{ root: string, files: string[], allowlist: Set<string>, readFile: (rel: string) => string }} ctx
 */
export function checkJsSdkImports(ctx) {
  const findings = [];
  for (const rel of ctx.files) {
    if (!JS_EXTENSIONS.has(path.posix.extname(rel))) continue;
    if (isTestPath(rel)) continue;
    if (isDocumentationPath(rel) || isScriptOrHarnessPath(rel)) continue;
    if (!rel.startsWith("synara/src/")) continue;

    let source;
    try {
      source = ctx.readFile(rel);
    } catch {
      continue;
    }
    const hits = findJsSdkImportHits(source);
    if (hits.length === 0) continue;

    if (underAnyPrefix(rel, JS_SDK_HARD_BAN_PREFIXES)) {
      findings.push({
        rule: "js-sdk-hard-ban-zone",
        path: rel,
        message:
          "matrix-js-sdk import in migration hard-ban zone (matrix-ipc / matrix-dto)",
        hits,
      });
      continue;
    }

    // Production freeze: new importers outside allowlist fail.
    if (!ctx.allowlist.has(rel)) {
      findings.push({
        rule: "js-sdk-new-importer",
        path: rel,
        message:
          "matrix-js-sdk import in production file not on P1.6 allowlist (new importer)",
        hits,
      });
    }
  }
  return findings;
}

// ---------------------------------------------------------------------------
// Guardrail 2 — raw Matrix runtime HTTP
// ---------------------------------------------------------------------------

const RAW_MATRIX_HTTP_PATTERNS = [/(['"`])\/_matrix\//, /\/_matrix\//];

export function findRawMatrixHttpHits(source) {
  const stripped = stripComments(source);
  // Prefer string-literal style; still catch path concatenations with /_matrix/
  return lineHits(stripped, [/\/_matrix\//]);
}

/**
 * @param {{ root: string, files: string[], readFile: (rel: string) => string }} ctx
 */
export function checkRawMatrixHttp(ctx) {
  const findings = [];
  for (const rel of ctx.files) {
    if (!isSourceFile(rel)) continue;
    if (isTestPath(rel)) continue;
    if (isDocumentationPath(rel) || isScriptOrHarnessPath(rel)) continue;
    if (!underAnyPrefix(rel, PRODUCT_RUNTIME_PREFIXES)) continue;
    // iOS has its own exceptions via check-matrix-boundaries; this P1.6 check
    // focuses on desktop product runtime (synara + src-tauri).

    if (RAW_MATRIX_HTTP_ALLOWLIST.has(rel)) continue;

    let source;
    try {
      source = ctx.readFile(rel);
    } catch {
      continue;
    }
    const hits = findRawMatrixHttpHits(source);
    if (hits.length === 0) continue;

    findings.push({
      rule: "raw-matrix-http",
      path: rel,
      message:
        "raw /_matrix/ usage outside approved product exception paths (use Matrix Rust SDK; exceptions: sw.ts, cs-api.ts)",
      hits,
    });
  }
  return findings;
}

// ---------------------------------------------------------------------------
// Guardrail 3 — unversioned Matrix IPC
// ---------------------------------------------------------------------------

/**
 * Ensure wire IPC modules declare version constants and envelope fields.
 * @param {{ root: string, files: string[], readFile: (rel: string) => string }} ctx
 */
export function checkUnversionedIpc(ctx) {
  const findings = [];

  const rustVersion = "src-tauri/src/matrix/ipc/version.rs";
  const tsVersion = "synara/src/app/features/matrix-ipc/version.ts";
  const rustEnvelope = "src-tauri/src/matrix/ipc/envelope.rs";
  const tsEnvelope = "synara/src/app/features/matrix-ipc/envelope.ts";

  const required = [
    {
      path: rustVersion,
      patterns: [/MATRIX_IPC_PROTOCOL_VERSION/],
      rule: "unversioned-ipc-missing-const",
      message: "Rust IPC version module must define MATRIX_IPC_PROTOCOL_VERSION",
    },
    {
      path: tsVersion,
      patterns: [/MATRIX_IPC_PROTOCOL_VERSION/],
      rule: "unversioned-ipc-missing-const",
      message: "TS IPC version module must define MATRIX_IPC_PROTOCOL_VERSION",
    },
    {
      path: rustEnvelope,
      patterns: [/protocol_version/, /MATRIX_IPC_PROTOCOL_VERSION/],
      rule: "unversioned-ipc-envelope",
      message:
        "Rust MatrixIpcEnvelope must carry protocol_version / MATRIX_IPC_PROTOCOL_VERSION",
    },
    {
      path: tsEnvelope,
      patterns: [/protocolVersion/],
      rule: "unversioned-ipc-envelope",
      message: "TS MatrixIpcEnvelope must include protocolVersion",
    },
  ];

  for (const req of required) {
    if (!ctx.files.includes(req.path) && !existsSync(path.join(ctx.root, req.path))) {
      // Only enforce when the module tree exists (real repo or full fixture).
      const treePresent =
        ctx.files.some((f) => f.startsWith("src-tauri/src/matrix/ipc/")) ||
        ctx.files.some((f) => f.startsWith("synara/src/app/features/matrix-ipc/"));
      if (!treePresent) continue;
      // If specific side of tree is present, require matching files.
      const rustSide = ctx.files.some((f) => f.startsWith("src-tauri/src/matrix/ipc/"));
      const tsSide = ctx.files.some((f) =>
        f.startsWith("synara/src/app/features/matrix-ipc/")
      );
      if (req.path.startsWith("src-tauri/") && !rustSide) continue;
      if (req.path.startsWith("synara/") && !tsSide) continue;

      findings.push({
        rule: req.rule,
        path: req.path,
        message: `${req.message} (file missing)`,
        hits: [],
      });
      continue;
    }

    let source;
    try {
      source = ctx.readFile(req.path);
    } catch {
      findings.push({
        rule: req.rule,
        path: req.path,
        message: `${req.message} (unreadable)`,
        hits: [],
      });
      continue;
    }
    const stripped = stripComments(source);
    const missing = req.patterns.filter((p) => !p.test(stripped));
    if (missing.length > 0) {
      findings.push({
        rule: req.rule,
        path: req.path,
        message: req.message,
        hits: missing.map((p) => ({
          line: 0,
          text: `missing pattern ${p}`,
          pattern: String(p),
        })),
      });
    }
  }

  // Reject envelopes that look like Matrix IPC wire shapes without protocolVersion.
  // Scans wire modules for object/struct definitions that include sessionGeneration/
  // session_generation + sequence + kind but omit protocol version fields.
  for (const rel of ctx.files) {
    if (!underAnyPrefix(rel, [...RUST_WIRE_MODULE_PREFIXES, ...TS_WIRE_MODULE_PREFIXES])) {
      continue;
    }
    if (isTestPath(rel)) continue;
    let source;
    try {
      source = ctx.readFile(rel);
    } catch {
      continue;
    }
    const stripped = stripComments(source);
    const looksLikeEnvelope =
      (/\bsession_generation\b/.test(stripped) || /\bsessionGeneration\b/.test(stripped)) &&
      /\bsequence\b/.test(stripped) &&
      (/\bkind\b/.test(stripped) || /MatrixIpcMessage/.test(stripped));
    if (!looksLikeEnvelope) continue;
    const hasProtocol =
      /\bprotocol_version\b/.test(stripped) || /\bprotocolVersion\b/.test(stripped);
    if (!hasProtocol) {
      findings.push({
        rule: "unversioned-ipc-shape",
        path: rel,
        message:
          "IPC wire shape includes sessionGeneration/sequence but omits protocolVersion/protocol_version",
        hits: [],
      });
    }
  }

  return findings;
}

// ---------------------------------------------------------------------------
// Guardrail 4 — SDK types in DTO / IPC wire modules
// ---------------------------------------------------------------------------

/** Patterns that indicate Rust SDK / Ruma leakage into wire modules (after comment strip). */
const RUST_SDK_WIRE_PATTERNS = [
  /^\s*use\s+matrix_sdk(\b|::)/m,
  /^\s*use\s+matrix_sdk_ui(\b|::)/m,
  /^\s*use\s+matrix_sdk_base(\b|::)/m,
  /^\s*use\s+ruma(\b|::)/m,
  /^\s*extern\s+crate\s+matrix_sdk\b/m,
  /^\s*extern\s+crate\s+ruma\b/m,
  // type paths not in comments (comment-stripped)
  /(?<![\w])matrix_sdk\s*::/,
  /(?<![\w])matrix_sdk_ui\s*::/,
  /(?<![\w])matrix_sdk_base\s*::/,
  /(?<![\w])ruma\s*::/,
];

export function findRustSdkWireHits(source) {
  const stripped = stripComments(source);
  const hits = [];
  const lines = stripped.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    for (const pattern of RUST_SDK_WIRE_PATTERNS) {
      // Per-line test: convert multi-line-aware patterns to single-line
      const single = new RegExp(pattern.source.replace(/^\^\\s\*/, "^\\s*").replace(/m$/, ""), "");
      // Simpler: test the full stripped text for use/extern, and per-line for ::
      void single;
    }
  }
  // Line-oriented patterns for imports / type paths
  const linePatterns = [
    /^\s*use\s+matrix_sdk(\b|::)/,
    /^\s*use\s+matrix_sdk_ui(\b|::)/,
    /^\s*use\s+matrix_sdk_base(\b|::)/,
    /^\s*use\s+ruma(\b|::)/,
    /^\s*extern\s+crate\s+matrix_sdk\b/,
    /^\s*extern\s+crate\s+ruma\b/,
    /(?<![\w])matrix_sdk\s*::/,
    /(?<![\w])matrix_sdk_ui\s*::/,
    /(?<![\w])matrix_sdk_base\s*::/,
    /(?<![\w])ruma\s*::/,
  ];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    for (const pattern of linePatterns) {
      if (pattern.test(line)) {
        hits.push({ line: index + 1, text: line.trim(), pattern: String(pattern) });
        break;
      }
    }
  }
  return hits;
}

/**
 * @param {{ root: string, files: string[], readFile: (rel: string) => string }} ctx
 */
export function checkSdkTypesInWireModules(ctx) {
  const findings = [];

  for (const rel of ctx.files) {
    if (isTestPath(rel)) continue;

    // Rust wire modules
    if (rel.endsWith(".rs") && underAnyPrefix(rel, RUST_WIRE_MODULE_PREFIXES)) {
      let source;
      try {
        source = ctx.readFile(rel);
      } catch {
        continue;
      }
      const hits = findRustSdkWireHits(source);
      if (hits.length > 0) {
        findings.push({
          rule: "sdk-types-in-wire-rust",
          path: rel,
          message:
            "matrix_sdk / ruma type or import in DTO/IPC wire module (use Synara-owned types only)",
          hits,
        });
      }
    }

    // TS wire modules — matrix-js-sdk already hard-banned; also ban direct
    // references that re-export SDK types without import (rare). Covered by
    // hard-ban + new-importer for matrix-js-sdk.
    if (
      JS_EXTENSIONS.has(path.posix.extname(rel)) &&
      underAnyPrefix(rel, TS_WIRE_MODULE_PREFIXES)
    ) {
      let source;
      try {
        source = ctx.readFile(rel);
      } catch {
        continue;
      }
      const hits = findJsSdkImportHits(source);
      if (hits.length > 0) {
        findings.push({
          rule: "sdk-types-in-wire-ts",
          path: rel,
          message:
            "matrix-js-sdk import in TypeScript DTO/IPC wire mirror (Synara-owned types only)",
          hits,
        });
      }
    }
  }

  return findings;
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/**
 * Run all P1.6 guardrails against a repository root.
 *
 * @param {{
 *   root?: string,
 *   files?: string[],
 *   allowlist?: Set<string> | string[],
 *   allowlistPath?: string,
 *   readFile?: (rel: string) => string,
 * }} [options]
 */
export function runGuardrails(options = {}) {
  const root = options.root ? path.resolve(options.root) : DEFAULT_ROOT;
  const files = (options.files || listRepositoryFiles(root)).map(toPosix);
  const allowlist =
    options.allowlist instanceof Set
      ? options.allowlist
      : Array.isArray(options.allowlist)
        ? new Set(options.allowlist.map(toPosix))
        : loadJsSdkAllowlist(root, options.allowlistPath || JS_SDK_ALLOWLIST_REL);

  const readFile =
    options.readFile ||
    ((rel) => readFileSync(path.join(root, rel), "utf8"));

  const ctx = { root, files, allowlist, readFile };

  const findings = [
    ...checkJsSdkImports(ctx),
    ...checkRawMatrixHttp(ctx),
    ...checkUnversionedIpc(ctx),
    ...checkSdkTypesInWireModules(ctx),
  ];

  // Deduplicate js-sdk hard-ban vs sdk-types-in-wire-ts for same path
  const seen = new Set();
  const deduped = [];
  for (const f of findings) {
    const key = `${f.rule}|${f.path}|${f.message}`;
    if (seen.has(key)) continue;
    seen.add(key);
    deduped.push(f);
  }

  const byRule = {};
  for (const f of deduped) {
    byRule[f.rule] = (byRule[f.rule] || 0) + 1;
  }

  return {
    schemaVersion: SCHEMA_VERSION,
    taskId: TASK_ID,
    ok: deduped.length === 0,
    root,
    fileCount: files.length,
    allowlistSize: allowlist.size,
    findingCount: deduped.length,
    byRule,
    findings: deduped,
  };
}

function formatReport(result) {
  const lines = [];
  lines.push(
    `[matrix-rust-p1.6] ${result.ok ? "PASS" : "FAIL"} — ${result.findingCount} finding(s); scanned ${result.fileCount} file(s); allowlist ${result.allowlistSize}`
  );
  if (!result.ok) {
    for (const f of result.findings) {
      const hitLoc =
        f.hits && f.hits.length
          ? ` @ lines ${f.hits.map((h) => h.line).filter((n) => n > 0).join(",") || "?"}`
          : "";
      lines.push(`  - [${f.rule}] ${f.path}${hitLoc}: ${f.message}`);
      if (f.hits) {
        for (const h of f.hits.slice(0, 3)) {
          if (h.line > 0 && h.text) {
            lines.push(`      ${h.line}: ${h.text.slice(0, 120)}`);
          }
        }
      }
    }
  }
  return lines.join("\n");
}

function parseArgs(argv) {
  const opts = { json: false, root: undefined };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === "--json") opts.json = true;
    else if (a === "--root") {
      opts.root = argv[i + 1];
      i += 1;
    } else if (a === "--help" || a === "-h") {
      opts.help = true;
    }
  }
  return opts;
}

function main(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  if (opts.help) {
    console.log(`Usage: node scripts/matrix-rust-p1.6-guardrails.mjs [--json] [--root DIR]`);
    process.exit(0);
  }
  const result = runGuardrails({ root: opts.root });
  if (opts.json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(formatReport(result));
  }
  process.exit(result.ok ? 0 : 1);
}

const isMain =
  process.argv[1] &&
  pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (isMain) {
  main();
}
