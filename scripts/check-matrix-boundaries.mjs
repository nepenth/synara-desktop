import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";


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

// Product iOS uses SynaraCore; historical REST exceptions are retired.
const IOS_ALLOWED_DIRECT_MATRIX_PATHS = new Map();

const DESKTOP_ALLOWED_DIRECT_MATRIX_PATHS = new Map([
  [
    "synara/src/app/cs-api.ts",
    "DESKTOP-REST-EXCEPTION-002: login-time homeserver version discovery helper.",
  ],
  [
    "synara/src/app/components/uia-stages/recaptchaFallback.ts",
    "DESKTOP-REST-EXCEPTION-003: UIA reCAPTCHA fallback page URL, opened in the system browser; the app sends no request.",
  ],
  [
    "src-tauri/src/csp_tests.rs",
    "DESKTOP-REST-EXCEPTION-004: test-only CSP assertions on the scoped loopback discovery sources; no request.",
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

// Rust reaches Matrix through matrix-sdk. Raw HTTP (`reqwest`, the SDK's
// `http_client()`, or a hand-built "/_matrix/" path) outside test code needs a
// named exception with its reason.
const RUST_ALLOWED_RAW_HTTP_PATHS = new Map([
  [
    "crates/synara-core/src/app/media/bounded.rs",
    "RUST-RAW-HTTP-001: size-capped media download/thumbnail through client.http_client() with v1/v3 fallback; the SDK media API has no byte cap.",
  ],
  [
    "crates/synara-core/src/app/client_builder/open.rs",
    "RUST-RAW-HTTP-002: build_auth_probe_client builds the redirect-free, timeout-bounded reqwest client handed to the SDK for pre-session login/registration probes; it sends nothing itself.",
  ],
]);

const RUST_RAW_HTTP_PATTERNS = [
  /\breqwest\b/,
  /\.http_client\(\)/,
  /"[^"]*\/_matrix\/[^"]*"/,
];

const rustFiles = repositoryFiles.filter(
  (path) =>
    (path.startsWith("crates/") || path.startsWith("src-tauri/")) &&
    path.endsWith(".rs")
);

// Files compiled only for tests: integration tests and benches, and modules
// declared behind `#[cfg(test)]` in their parent (`mod foo;` → foo.rs or
// foo/mod.rs, including everything below foo/).
function isRustTestPath(path) {
  return /(^|\/)(tests|benches)\//.test(path);
}

const rustTestModulePrefixes = new Set();
for (const file of rustFiles) {
  const lines = readFileSync(resolve(root, file), "utf8").split(/\r?\n/);
  const dir = file.slice(0, file.lastIndexOf("/") + 1);
  const stem = file.slice(file.lastIndexOf("/") + 1, -".rs".length);
  // A module file `foo.rs` keeps its children in `foo/`; `mod.rs`, `lib.rs`
  // and `main.rs` keep them beside themselves.
  const childDir = ["mod", "lib", "main"].includes(stem) ? dir : `${dir}${stem}/`;
  let pendingTestAttr = false;
  for (const raw of lines) {
    const line = raw.trim();
    if (/^#\[cfg\((all\()?test\b/.test(line)) {
      pendingTestAttr = true;
      continue;
    }
    if (pendingTestAttr && line.startsWith("#[")) {
      const pathAttr = line.match(/^#\[path\s*=\s*"([^"]+)"\]/);
      if (pathAttr) {
        // Outside an inline module, `#[path]` is relative to this file's
        // directory.
        rustTestModulePrefixes.add(posix.normalize(`${dir}${pathAttr[1]}`));
      }
      continue;
    }
    if (pendingTestAttr) {
      const decl = line.match(/^(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z0-9_]+)\s*;/);
      if (decl) {
        rustTestModulePrefixes.add(`${childDir}${decl[1]}.rs`);
        rustTestModulePrefixes.add(`${childDir}${decl[1]}/`);
      }
    }
    pendingTestAttr = false;
  }
}

function isRustTestModuleFile(path) {
  for (const prefix of rustTestModulePrefixes) {
    if (path === prefix || (prefix.endsWith("/") && path.startsWith(prefix))) {
      return true;
    }
  }
  return false;
}

// Line hits outside `#[cfg(test)]` items and outside comments. The item after
// the attribute is skipped to its closing brace, or to its `;`.
function rustProductionHits(path, patterns) {
  const lines = readFileSync(resolve(root, path), "utf8").split(/\r?\n/);
  const hits = [];
  let skipping = false;
  let pendingTestAttr = false;
  let depth = 0;
  let opened = false;
  for (const [index, raw] of lines.entries()) {
    const line = raw.trim();
    if (!skipping && /^#\[cfg\((all\()?test\b/.test(line)) {
      pendingTestAttr = true;
      continue;
    }
    if (pendingTestAttr && line.startsWith("#[")) continue;
    if (pendingTestAttr) {
      pendingTestAttr = false;
      skipping = true;
      depth = 0;
      opened = false;
    }
    if (skipping) {
      const code = line.replace(/"(?:\\.|[^"\\])*"/g, '""').replace(/\/\/.*$/, "");
      for (const char of code) {
        if (char === "{") {
          depth += 1;
          opened = true;
        } else if (char === "}") {
          depth -= 1;
        }
      }
      if ((opened && depth <= 0) || (!opened && code.endsWith(";"))) {
        skipping = false;
      }
      continue;
    }
    if (line.startsWith("//")) continue;
    if (patterns.some((pattern) => pattern.test(line))) {
      hits.push({ line: index + 1, text: line });
    }
  }
  return hits;
}

const activeRustExceptions = new Set();
for (const file of rustFiles) {
  if (isRustTestPath(file) || isRustTestModuleFile(file)) continue;
  const hits = rustProductionHits(file, RUST_RAW_HTTP_PATTERNS);
  if (hits.length === 0) continue;
  if (RUST_ALLOWED_RAW_HTTP_PATHS.has(file)) {
    activeRustExceptions.add(file);
    continue;
  }
  fail(
    `${file} sends raw Matrix HTTP outside matrix-sdk (reqwest, http_client() or a "/_matrix/" path): ` +
      hits.map((hit) => `${hit.line}`).join(", ")
  );
}
for (const file of RUST_ALLOWED_RAW_HTTP_PATHS.keys()) {
  if (!activeRustExceptions.has(file)) {
    fail(`${file} is a Rust raw-HTTP exception with no remaining raw HTTP; remove the exception.`);
  }
}

// The JS Matrix SDK and widget API were retired by the Rust migration. Keep
// them out of every lockfile and every source import.
const RETIRED_PACKAGES = ["matrix-js-sdk", "matrix-widget-api"];
const lockfiles = repositoryFiles.filter((path) =>
  /(^|\/)package-lock\.json$/.test(path)
);
for (const lockfile of lockfiles) {
  const { packages = {} } = JSON.parse(
    readFileSync(resolve(root, lockfile), "utf8")
  );
  for (const name of RETIRED_PACKAGES) {
    if (Object.keys(packages).some((key) => key.endsWith(`node_modules/${name}`))) {
      fail(`${lockfile} still resolves retired package ${name}`);
    }
  }
}
const importPattern = new RegExp(
  `(?:from\\s+|import\\s*\\(\\s*|require\\s*\\(\\s*)['"](?:${RETIRED_PACKAGES.join("|")})(?:/[^'"]*)?['"]`
);
for (const file of repositoryFiles) {
  if (!/\.(ts|tsx|js|jsx|mjs|cjs)$/.test(file) || isDocumentation(file)) continue;
  if (file.startsWith("scripts/")) continue;
  const hits = lineHits(file, [importPattern]);
  if (hits.length > 0) {
    fail(
      `${file} imports a retired Matrix JS package: ` +
        hits.map((hit) => `${hit.line}`).join(", ")
    );
  }
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
  console.error("\nApproved Rust raw-HTTP exceptions:");
  for (const [path, reason] of RUST_ALLOWED_RAW_HTTP_PATHS) {
    console.error(`- ${path}: ${reason}`);
  }
  process.exit(process.exitCode);
}

console.log(
  `Matrix boundary check passed with ${activeIOSExceptions.size} active iOS exceptions and ` +
    `${activeDesktopExceptions.size} active desktop exceptions and ` +
    `${activeRustExceptions.size} active Rust raw-HTTP exceptions.`
);
