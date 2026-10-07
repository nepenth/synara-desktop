import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
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
  process.exit(process.exitCode);
}

console.log(
  `Matrix boundary check passed with ${activeIOSExceptions.size} active iOS exceptions and ` +
    `${activeDesktopExceptions.size} active desktop exceptions.`
);
