/**
 * P1.6 — Architectural CI guardrails for Matrix Rust SDK full replacement.
 *
 * Hard rules (production + contract surfaces only):
 * 1. No matrix_sdk / matrix_sdk_ui / ruma types in DTO or IPC modules.
 * 2. No matrix-js-sdk imports in matrix-ipc / matrix-dto (greenfield contracts).
 * 3. No raw /_matrix/ HTTP in greenfield contract surfaces.
 * 4. No production Client login/sync under src-tauri/src/matrix/ (Client::builder
 *    allowed only under matrix/client_builder/ for P2.3 unauthenticated open;
 *    password/token login APIs allowed only under matrix/auth/ for P3.2;
 *    Client::restore_session allowed only under matrix/lifecycle/ for P3.6;
 *    SyncService::builder allowed only under matrix/sync/ for P4.1).
 * 5. No dual-backend / Matrix backend selector in production runtime sources.
 * 6. Matrix IPC contract surface must remain versioned (protocolVersion / constant).
 * 7. Matrix product Tauri commands in invoke_handler must each have a matching
 *    `allow-matrix-*` permission in `src-tauri/capabilities/main.json`.
 *
 * Product matrix-js-sdk usage outside greenfield zones remains allowed until cutover.
 * Link-smoke (`matrix_sdk_link_smoke`) is outside Zone B and may reference SDK types.
 *
 * Usage:
 *   node scripts/check-matrix-rust-sdk-guardrails.mjs
 *   node scripts/check-matrix-rust-sdk-guardrails.mjs --root /path/to/repo
 *
 * Export `runGuardrails({ root, files })` for unit tests with synthetic trees.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { resolve, join, relative, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const DEFAULT_ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));

/** Paths relative to repo root, POSIX-style. */
const ZONE_TS_CONTRACT = [
  "synara/src/app/features/matrix-ipc/",
  "synara/src/app/features/matrix-dto/",
];

const ZONE_RUST_CONTRACT = [
  "src-tauri/src/matrix/dto/",
  "src-tauri/src/matrix/ipc/",
];

const ZONE_RUST_MATRIX = ["src-tauri/src/matrix/"];

const ZONE_PRODUCTION_SCAN = ["synara/src/", "src-tauri/src/"];

const TEST_PATH_MARKERS = [
  "/__tests__/",
  "/tests.rs",
  "contract_tests.rs",
  // Opt-in disposable-Synapse live proofs are #[cfg(test)] modules only.
  "_synapse_proof.rs",
  ".test.ts",
  ".test.tsx",
  ".test.js",
  ".test.mjs",
  ".spec.ts",
  ".spec.tsx",
];

const DOC_PATH_PREFIXES = [
  "docs/",
  "synara/docs/",
  "synara-ios/docs/",
  "README.md",
  "MODERNIZATION.md",
  "CHANGELOG.md",
];

/**
 * @param {string} source
 * @param {{ blankStrings?: boolean, lang?: "ts"|"rs" }} [opts]
 */
export function stripCommentsAndStrings(source, opts = {}) {
  const blankStrings = opts.blankStrings !== false;
  const lang = opts.lang ?? "ts";
  // Replace comments (and optionally string/char literals) with spaces.
  let out = "";
  let i = 0;
  const n = source.length;
  while (i < n) {
    const c = source[i];
    const next = source[i + 1];

    // Line comment
    if (c === "/" && next === "/") {
      out += "  ";
      i += 2;
      while (i < n && source[i] !== "\n") {
        out += source[i] === "\t" ? "\t" : " ";
        i += 1;
      }
      continue;
    }

    // Block comment
    if (c === "/" && next === "*") {
      out += "  ";
      i += 2;
      while (i < n - 1 && !(source[i] === "*" && source[i + 1] === "/")) {
        out += source[i] === "\n" ? "\n" : source[i] === "\t" ? "\t" : " ";
        i += 1;
      }
      if (i < n - 1) {
        out += "  ";
        i += 2;
      }
      continue;
    }

    // Rust lifetimes ('static, 'a) and raw identifiers (r#type) are code, not
    // string delimiters. Mis-parsing them blanks generate_handler bodies and
    // creates false negatives for product-command ACL checks.
    if (lang === "rs" && c === "'" && next && /[A-Za-z_]/.test(next)) {
      out += c;
      i += 1;
      while (i < n && /[A-Za-z0-9_]/.test(source[i])) {
        out += source[i];
        i += 1;
      }
      continue;
    }
    if (lang === "rs" && c === "r" && next === "#") {
      let j = i + 1;
      let hashes = 0;
      while (j < n && source[j] === "#") {
        hashes += 1;
        j += 1;
      }
      if (!(j < n && source[j] === '"')) {
        // Raw identifier: r#foo — keep as code.
        out += c;
        i += 1;
        continue;
      }
    }

    // Strings — blank only when blankStrings is true (import/use checks).
    // Keep string contents for /_matrix/ path detection.
    if (
      blankStrings &&
      (c === '"' ||
        c === "'" ||
        (lang === "rs" && c === "r" && (next === '"' || next === "#")))
    ) {
      // raw string (Rust) simplified: r"..." or r#"..."#
      if (lang === "rs" && c === "r") {
        let j = i + 1;
        let hashes = 0;
        while (j < n && source[j] === "#") {
          hashes += 1;
          j += 1;
        }
        if (j < n && source[j] === '"') {
          const close = `"${"#".repeat(hashes)}`;
          out += " ".repeat(j - i + 1);
          i = j + 1;
          while (i < n) {
            if (source.startsWith(close, i)) {
              out += " ".repeat(close.length);
              i += close.length;
              break;
            }
            out += source[i] === "\n" ? "\n" : " ";
            i += 1;
          }
          continue;
        }
        // Not a raw string after all (should be unreachable after raw-ident
        // handling above); emit as code.
        out += c;
        i += 1;
        continue;
      }

      const quote = c;
      out += " ";
      i += 1;
      while (i < n) {
        if (source[i] === "\\") {
          out += "  ";
          i += 2;
          continue;
        }
        if (source[i] === quote) {
          out += " ";
          i += 1;
          break;
        }
        out += source[i] === "\n" ? "\n" : " ";
        i += 1;
      }
      continue;
    }

    // Template literals (TS) — blank only when blankStrings
    if (blankStrings && lang === "ts" && c === "`") {
      out += " ";
      i += 1;
      while (i < n) {
        if (source[i] === "\\") {
          out += "  ";
          i += 2;
          continue;
        }
        if (source[i] === "`") {
          out += " ";
          i += 1;
          break;
        }
        out += source[i] === "\n" ? "\n" : " ";
        i += 1;
      }
      continue;
    }

    out += c;
    i += 1;
  }
  return out;
}

function toPosix(p) {
  return p.split(sep).join("/");
}

function isDocPath(rel) {
  return DOC_PATH_PREFIXES.some((p) => rel === p || rel.startsWith(p));
}

function isTestPath(rel) {
  return TEST_PATH_MARKERS.some((m) => rel.includes(m));
}

function inAnyZone(rel, zones) {
  return zones.some((z) => rel === z.slice(0, -1) || rel.startsWith(z));
}

/**
 * Cutover-phase product commands are allowed only when the main webview
 * capability ACL grants the matching `allow-matrix-*` permission.
 * `allow-matrix-timeline-open` → `matrix_timeline_open`.
 */
function loadAllowedMatrixProductCommands(root) {
  const rel = "src-tauri/capabilities/main.json";
  const abs = join(root, rel);
  if (!existsSync(abs)) return new Set();
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(abs, "utf8"));
  } catch {
    return new Set();
  }
  const allowed = new Set();
  for (const perm of parsed.permissions ?? []) {
    if (typeof perm !== "string" || !perm.startsWith("allow-matrix-")) continue;
    const slug = perm.slice("allow-matrix-".length);
    if (!slug) continue;
    allowed.add(`matrix_${slug.replaceAll("-", "_")}`);
  }
  return allowed;
}

function listRepoFiles(root) {
  if (existsSync(join(root, ".git"))) {
    try {
      const tracked = execFileSync("git", ["ls-files"], {
        cwd: root,
        encoding: "utf8",
      })
        .split("\n")
        .filter(Boolean);
      const untracked = execFileSync(
        "git",
        ["ls-files", "--others", "--exclude-standard"],
        { cwd: root, encoding: "utf8" }
      )
        .split("\n")
        .filter(Boolean);
      return [...new Set([...tracked, ...untracked])].map(toPosix).sort();
    } catch {
      // fall through to walk
    }
  }
  return walkFiles(root, root);
}

function walkFiles(root, dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "target" || name === ".git")
      continue;
    const abs = join(dir, name);
    const st = statSync(abs);
    if (st.isDirectory()) out.push(...walkFiles(root, abs));
    else out.push(toPosix(relative(root, abs)));
  }
  return out.sort();
}

/**
 * @typedef {{ rule: string, path: string, line: number, detail: string }} Violation
 */

/**
 * @param {string} text
 * @param {RegExp[]} patterns
 * @returns {{ line: number, text: string, pattern: string }[]}
 */
function findHits(text, patterns) {
  const lines = text.split(/\r?\n/);
  const hits = [];
  for (let i = 0; i < lines.length; i += 1) {
    for (const pattern of patterns) {
      if (pattern.test(lines[i])) {
        hits.push({
          line: i + 1,
          text: lines[i].trim().slice(0, 160),
          pattern: pattern.source,
        });
      }
    }
  }
  return hits;
}

const RUST_SDK_TYPE_PATTERNS = [
  /\buse\s+matrix_sdk\b/,
  /\buse\s+matrix_sdk_ui\b/,
  /\buse\s+ruma\b/,
  /\bmatrix_sdk\s*::/,
  /\bmatrix_sdk_ui\s*::/,
  /\bruma\s*::/,
  /\bextern\s+crate\s+matrix_sdk\b/,
];

const TS_JS_SDK_IMPORT_PATTERNS = [
  /\bfrom\s+['"]matrix-js-sdk(?:\/[^'"]*)?['"]/,
  /\bimport\s*\(\s*['"]matrix-js-sdk(?:\/[^'"]*)?['"]\s*\)/,
  /\brequire\s*\(\s*['"]matrix-js-sdk(?:\/[^'"]*)?['"]\s*\)/,
];

const RAW_MATRIX_HTTP_PATTERNS = [
  /\/_matrix\//,
  /\bappendPathComponent\s*\(\s*["']_matrix["']\s*\)/,
];

/** Construction APIs allowed only under `matrix/client_builder/` (P2.3). */
const CLIENT_BUILDER_ONLY_PATTERNS = [
  /\bClient\s*::\s*builder\b/,
  // Exclude HTTP helpers such as reqwest::Client::new().
  /(?<!reqwest::)\bClient\s*::\s*new\b/,
];

/**
 * Password/token login APIs allowed only under `matrix/auth/` (P3.2).
 * Still banned under client_builder/ and every other matrix/ module.
 */
const AUTH_LOGIN_ONLY_PATTERNS = [
  /\.login_username\b/,
  /\.login_token\b/,
  /\.matrix_auth\s*\(/,
];

/** Sync/join APIs forbidden under production `matrix/` (non-test). */
const PRODUCTION_SYNC_PATTERNS = [
  /\.sync_once\b/,
  /\.sync_with_result_callback\b/,
  /\bRoom\s*::\s*join\b/,
];

/**
 * Session restore API allowed only under `matrix/lifecycle/` (P3.6).
 * Still banned under client_builder/, auth/, sync/, and every other matrix/ module.
 */
const RESTORE_SESSION_ONLY_PATTERNS = [/\.restore_session\b/];

/**
 * SyncService construction allowed only under `matrix/sync/` (P4.1).
 * Still banned under client_builder/, auth/, lifecycle/, and every other matrix/ module.
 */
const SYNC_SERVICE_ONLY_PATTERNS = [/\bSyncService\s*::\s*builder\b/];

/** Full ban set for non-allowlisted matrix modules. */
const PRODUCTION_CLIENT_PATTERNS = [
  ...CLIENT_BUILDER_ONLY_PATTERNS,
  ...AUTH_LOGIN_ONLY_PATTERNS,
  ...RESTORE_SESSION_ONLY_PATTERNS,
  ...SYNC_SERVICE_ONLY_PATTERNS,
  ...PRODUCTION_SYNC_PATTERNS,
];

/** Patterns still banned inside client_builder/ (may construct Client only). */
const CLIENT_BUILDER_BANNED_PATTERNS = [
  ...AUTH_LOGIN_ONLY_PATTERNS,
  ...RESTORE_SESSION_ONLY_PATTERNS,
  ...SYNC_SERVICE_ONLY_PATTERNS,
  ...PRODUCTION_SYNC_PATTERNS,
];

/** Patterns still banned inside auth/ (may login; may not construct Client, restore, or sync). */
const AUTH_BANNED_PATTERNS = [
  ...CLIENT_BUILDER_ONLY_PATTERNS,
  ...RESTORE_SESSION_ONLY_PATTERNS,
  ...SYNC_SERVICE_ONLY_PATTERNS,
  ...PRODUCTION_SYNC_PATTERNS,
];

/**
 * Patterns still banned inside lifecycle/ (may restore_session; may not construct
 * Client, login, SyncService, or Client::sync_once).
 */
const LIFECYCLE_BANNED_PATTERNS = [
  ...CLIENT_BUILDER_ONLY_PATTERNS,
  ...AUTH_LOGIN_ONLY_PATTERNS,
  ...SYNC_SERVICE_ONLY_PATTERNS,
  ...PRODUCTION_SYNC_PATTERNS,
];

/**
 * Patterns still banned inside sync/ (may SyncService::builder; may not construct
 * Client, login, restore_session, or Client::sync_once).
 */
const SYNC_BANNED_PATTERNS = [
  ...CLIENT_BUILDER_ONLY_PATTERNS,
  ...AUTH_LOGIN_ONLY_PATTERNS,
  ...RESTORE_SESSION_ONLY_PATTERNS,
  ...PRODUCTION_SYNC_PATTERNS,
];

const ZONE_CLIENT_BUILDER_ALLOW = ["src-tauri/src/matrix/client_builder/"];
const ZONE_AUTH_LOGIN_ALLOW = ["src-tauri/src/matrix/auth/"];
const ZONE_LIFECYCLE_RESTORE_ALLOW = ["src-tauri/src/matrix/lifecycle/"];
const ZONE_SYNC_SERVICE_ALLOW = ["src-tauri/src/matrix/sync/"];

const DUAL_BACKEND_PATTERNS = [
  /\bMatrixBackend\b/,
  /\bmatrixBackendSelector\b/,
  /\bselectMatrixBackend\b/,
  /\bdualBackend\b/,
  /\bdual_backend\b/,
  /\buseMatrixJsSdk\b/,
  /\bMATRIX_BACKEND_JS\b/,
  /\bMATRIX_BACKEND_RUST\b/,
];

/**
 * @param {{ root: string, files?: string[] }} opts
 * @returns {{ ok: boolean, violations: Violation[], summary: string }}
 */
export function runGuardrails(opts) {
  const root = opts.root;
  const files = (opts.files ?? listRepoFiles(root)).map(toPosix);
  /** @type {Violation[]} */
  const violations = [];

  const add = (rule, path, line, detail) => {
    violations.push({ rule, path, line, detail });
  };

  // --- Zone B: Rust DTO/IPC — no SDK types ---
  for (const rel of files) {
    if (!inAnyZone(rel, ZONE_RUST_CONTRACT)) continue;
    if (!rel.endsWith(".rs")) continue;
    const abs = join(root, rel);
    if (!existsSync(abs)) continue;
    const raw = readFileSync(abs, "utf8");
    const code = stripCommentsAndStrings(raw, {
      lang: "rs",
      blankStrings: true,
    });
    for (const hit of findHits(code, RUST_SDK_TYPE_PATTERNS)) {
      add(
        "no-sdk-types-in-dto-ipc",
        rel,
        hit.line,
        `SDK/Ruma type reference forbidden in DTO/IPC: ${hit.text}`
      );
    }
  }

  // --- Zone A: TS contract — no matrix-js-sdk, no raw /_matrix/ ---
  for (const rel of files) {
    if (!inAnyZone(rel, ZONE_TS_CONTRACT)) continue;
    if (!/\.(ts|tsx|js|jsx)$/.test(rel)) continue;
    // Still scan tests for matrix-js-sdk imports (they must not depend on JS SDK either)
    const abs = join(root, rel);
    if (!existsSync(abs)) continue;
    const raw = readFileSync(abs, "utf8");
    // Keep string contents: import paths and /_matrix/ live in string literals.
    const code = stripCommentsAndStrings(raw, {
      lang: "ts",
      blankStrings: false,
    });
    for (const hit of findHits(code, TS_JS_SDK_IMPORT_PATTERNS)) {
      add(
        "no-js-sdk-in-greenfield-contracts",
        rel,
        hit.line,
        `matrix-js-sdk import forbidden in matrix-ipc/matrix-dto: ${hit.text}`
      );
    }
    if (!isTestPath(rel)) {
      for (const hit of findHits(code, RAW_MATRIX_HTTP_PATTERNS)) {
        add(
          "no-raw-matrix-http-in-contracts",
          rel,
          hit.line,
          `raw /_matrix/ HTTP forbidden in contract modules: ${hit.text}`
        );
      }
    }
  }

  // --- Zone C: src-tauri/src/matrix —
  //     Client::builder only under matrix/client_builder/ (P2.3);
  //     login_username/login_token/matrix_auth only under matrix/auth/ (P3.2);
  //     restore_session only under matrix/lifecycle/ (P3.6);
  //     SyncService::builder only under matrix/sync/ (P4.1);
  //     sync_once banned under non-test matrix modules. ---
  for (const rel of files) {
    if (!inAnyZone(rel, ZONE_RUST_MATRIX)) continue;
    if (!rel.endsWith(".rs")) continue;
    if (isTestPath(rel)) continue;
    const abs = join(root, rel);
    if (!existsSync(abs)) continue;
    const raw = readFileSync(abs, "utf8");
    const code = stripCommentsAndStrings(raw, {
      lang: "rs",
      blankStrings: true,
    });
    const inClientBuilder = inAnyZone(rel, ZONE_CLIENT_BUILDER_ALLOW);
    const inAuthLogin = inAnyZone(rel, ZONE_AUTH_LOGIN_ALLOW);
    const inLifecycleRestore = inAnyZone(rel, ZONE_LIFECYCLE_RESTORE_ALLOW);
    const inSyncService = inAnyZone(rel, ZONE_SYNC_SERVICE_ALLOW);
    let patterns;
    let detailPrefix;
    if (inClientBuilder) {
      patterns = CLIENT_BUILDER_BANNED_PATTERNS;
      detailPrefix =
        "login/sync/session API forbidden under matrix/ (builder module may only construct unauthenticated Client)";
    } else if (inAuthLogin) {
      patterns = AUTH_BANNED_PATTERNS;
      detailPrefix =
        "Client construction/sync/session-restore forbidden under matrix/auth/ (auth may only password/token login)";
    } else if (inLifecycleRestore) {
      patterns = LIFECYCLE_BANNED_PATTERNS;
      detailPrefix =
        "Client construction/login/SyncService/sync_once forbidden under matrix/lifecycle/ (lifecycle may only restore_session for P3.6)";
    } else if (inSyncService) {
      patterns = SYNC_BANNED_PATTERNS;
      detailPrefix =
        "Client construction/login/restore_session/sync_once forbidden under matrix/sync/ (sync may only SyncService::builder for P4.1)";
    } else {
      patterns = PRODUCTION_CLIENT_PATTERNS;
      detailPrefix =
        "production Client/login/restore/sync API forbidden under matrix/ outside client_builder/, auth/, lifecycle/, and sync/";
    }
    for (const hit of findHits(code, patterns)) {
      add(
        "no-production-matrix-client-in-matrix-module",
        rel,
        hit.line,
        `${detailPrefix}: ${hit.text}`
      );
    }
  }

  // --- Dual-backend / selector ban in production sources ---
  for (const rel of files) {
    if (!inAnyZone(rel, ZONE_PRODUCTION_SCAN)) continue;
    if (isDocPath(rel) || isTestPath(rel)) continue;
    if (!/\.(ts|tsx|js|jsx|rs)$/.test(rel)) continue;
    const abs = join(root, rel);
    if (!existsSync(abs)) continue;
    const raw = readFileSync(abs, "utf8");
    const lang = rel.endsWith(".rs") ? "rs" : "ts";
    const code = stripCommentsAndStrings(raw, { lang, blankStrings: true });
    for (const hit of findHits(code, DUAL_BACKEND_PATTERNS)) {
      add(
        "no-dual-backend-selector",
        rel,
        hit.line,
        `dual-backend / selector pattern forbidden (sole owner = Rust SDK at cutover): ${hit.text}`
      );
    }
  }

  // --- Versioned IPC: constants must exist in both mirrors ---
  const rustVersion = files.find(
    (f) => f === "src-tauri/src/matrix/ipc/version.rs"
  );
  const tsVersion = files.find(
    (f) => f === "synara/src/app/features/matrix-ipc/version.ts"
  );
  if (rustVersion) {
    const text = readFileSync(join(root, rustVersion), "utf8");
    if (!/MATRIX_IPC_PROTOCOL_VERSION\s*[:=]/.test(text)) {
      add(
        "versioned-matrix-ipc",
        rustVersion,
        1,
        "MATRIX_IPC_PROTOCOL_VERSION constant missing (unversioned IPC forbidden)"
      );
    }
  } else {
    // Only require if matrix ipc tree exists
    if (files.some((f) => f.startsWith("src-tauri/src/matrix/ipc/"))) {
      add(
        "versioned-matrix-ipc",
        "src-tauri/src/matrix/ipc/version.rs",
        1,
        "IPC version module missing"
      );
    }
  }
  if (tsVersion) {
    const text = readFileSync(join(root, tsVersion), "utf8");
    if (!/MATRIX_IPC_PROTOCOL_VERSION\s*=/.test(text)) {
      add(
        "versioned-matrix-ipc",
        tsVersion,
        1,
        "MATRIX_IPC_PROTOCOL_VERSION constant missing (unversioned IPC forbidden)"
      );
    }
  } else if (
    files.some((f) => f.startsWith("synara/src/app/features/matrix-ipc/"))
  ) {
    add(
      "versioned-matrix-ipc",
      "synara/src/app/features/matrix-ipc/version.ts",
      1,
      "IPC version module missing"
    );
  }

  // --- Matrix product Tauri commands must be ACL-registered ---
  const libRs = "src-tauri/src/lib.rs";
  if (files.includes(libRs) || existsSync(join(root, libRs))) {
    const text = readFileSync(join(root, libRs), "utf8");
    const code = stripCommentsAndStrings(text, {
      lang: "rs",
      blankStrings: true,
    });
    // Match generate_handler![...] body for matrix_ product commands
    const handlerMatch = code.match(/generate_handler!\s*\[([\s\S]*?)\]/);
    if (handlerMatch) {
      const body = handlerMatch[1];
      const cmdHits = body.match(/\bmatrix_[A-Za-z0-9_]+/g) ?? [];
      const allowed = loadAllowedMatrixProductCommands(root);
      for (const cmd of cmdHits) {
        if (allowed.has(cmd)) continue;
        add(
          "no-matrix-product-tauri-commands",
          libRs,
          1,
          `Matrix product Tauri command '${cmd}' registered in invoke_handler without matching allow-matrix-* capability ACL`
        );
      }
    }
  }

  // --- Deepening direct-client access: forbid mx.http / client.http in greenfield ---
  for (const rel of files) {
    if (!inAnyZone(rel, ZONE_TS_CONTRACT)) continue;
    if (!/\.(ts|tsx)$/.test(rel) || isTestPath(rel)) continue;
    const abs = join(root, rel);
    if (!existsSync(abs)) continue;
    const code = stripCommentsAndStrings(readFileSync(abs, "utf8"), {
      lang: "ts",
      blankStrings: true,
    });
    for (const hit of findHits(code, [
      /\bmx\.http\b/,
      /\bclient\.http\b/,
      /\bgetHttpUriForMxc\b/,
      /\bMatrixClient\b/,
    ])) {
      add(
        "no-deepening-direct-client-in-contracts",
        rel,
        hit.line,
        `direct matrix-js-sdk client access forbidden in contract modules: ${hit.text}`
      );
    }
  }

  const ok = violations.length === 0;
  const summary = ok
    ? `Matrix Rust SDK guardrails passed (${files.length} files scanned).`
    : `Matrix Rust SDK guardrails failed with ${violations.length} violation(s).`;

  return { ok, violations, summary };
}

export function formatViolations(violations) {
  return violations
    .map((v) => `  [${v.rule}] ${v.path}:${v.line} — ${v.detail}`)
    .join("\n");
}

function main(argv) {
  let root = DEFAULT_ROOT;
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--root" && argv[i + 1]) {
      root = resolve(argv[i + 1]);
      i += 1;
    }
  }

  const result = runGuardrails({ root });
  if (result.ok) {
    console.log(`[matrix-rust-guardrails] ${result.summary}`);
    process.exit(0);
  }

  console.error(`[matrix-rust-guardrails] ${result.summary}`);
  console.error(formatViolations(result.violations));
  console.error(
    "\nSee docs/matrix-rust-sdk/p1.6-architectural-guardrails.md for rules."
  );
  process.exit(1);
}

const isMain =
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url;

if (isMain) {
  main(process.argv.slice(2));
}
