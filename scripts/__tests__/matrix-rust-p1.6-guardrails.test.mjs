import assert from "node:assert/strict";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  checkJsSdkImports,
  checkRawMatrixHttp,
  checkSdkTypesInWireModules,
  checkUnversionedIpc,
  findJsSdkImportHits,
  findRawMatrixHttpHits,
  findRustSdkWireHits,
  JS_SDK_ALLOWLIST_REL,
  loadJsSdkAllowlist,
  runGuardrails,
  stripComments,
  toPosix,
} from "../matrix-rust-p1.6-guardrails.mjs";

const REPO_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);
const FIXTURES = path.join(
  REPO_ROOT,
  "scripts/fixtures/matrix-rust-p1.6/prohibited"
);

function makeTempRoot() {
  return mkdtempSync(path.join(tmpdir(), "matrix-rust-p1.6-"));
}

function writeTree(root, files) {
  for (const [rel, content] of Object.entries(files)) {
    const abs = path.join(root, rel);
    mkdirSync(path.dirname(abs), { recursive: true });
    writeFileSync(abs, content, "utf8");
  }
}

function copyFixtureInto(root, fixtureName) {
  const src = path.join(FIXTURES, fixtureName);
  cpSync(src, root, { recursive: true });
}

function collectFiles(root, base = root, acc = []) {
  for (const name of readdirSync(root)) {
    const abs = path.join(root, name);
    if (statSync(abs).isDirectory()) collectFiles(abs, base, acc);
    else acc.push(toPosix(path.relative(base, abs)));
  }
  return acc.sort();
}

function scanFixture(fixtureName, { allowlist = new Set() } = {}) {
  const root = makeTempRoot();
  try {
    copyFixtureInto(root, fixtureName);
    const files = collectFiles(root);
    return runGuardrails({
      root,
      files,
      allowlist,
      readFile: (rel) => readFileSync(path.join(root, rel), "utf8"),
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

// ---------------------------------------------------------------------------
// Unit helpers
// ---------------------------------------------------------------------------

test("stripComments removes line and block comments but keeps strings", () => {
  const src = `
// use matrix_sdk::Client;
/* use ruma::OwnedRoomId; */
const s = "use matrix_sdk::Client";
use matrix_sdk::Room;
`;
  const stripped = stripComments(src);
  assert.match(stripped, /use matrix_sdk::Room/);
  assert.match(stripped, /"use matrix_sdk::Client"/);
  assert.doesNotMatch(stripped, /OwnedRoomId/);
  assert.equal(
    findRustSdkWireHits(src).some(
      (h) => h.text.includes("Client") && h.line < 3
    ),
    false
  );
  assert.ok(
    findRustSdkWireHits(src).some((h) => h.text.includes("matrix_sdk::Room"))
  );
});

test("findJsSdkImportHits detects from/require/import()", () => {
  assert.ok(
    findJsSdkImportHits(`import { Room } from 'matrix-js-sdk';`).length > 0
  );
  assert.ok(
    findJsSdkImportHits(`import x from "matrix-js-sdk/lib/crypto-api";`)
      .length > 0
  );
  assert.ok(
    findJsSdkImportHits(`const m = require('matrix-js-sdk');`).length > 0
  );
  assert.ok(
    findJsSdkImportHits(`const m = await import('matrix-js-sdk');`).length > 0
  );
  assert.equal(
    findJsSdkImportHits(`// import { Room } from 'matrix-js-sdk';`).length,
    0
  );
  assert.equal(findJsSdkImportHits(`const x = 'not-matrix-js-sdk';`).length, 0);
});

test("findRawMatrixHttpHits detects path literals", () => {
  assert.ok(
    findRawMatrixHttpHits(`fetch(base + '/_matrix/client/versions')`).length > 0
  );
  assert.equal(
    findRawMatrixHttpHits(`// '/_matrix/client/versions'`).length,
    0
  );
});

test("findRustSdkWireHits detects use and type paths", () => {
  assert.ok(findRustSdkWireHits(`use matrix_sdk::Client;\n`).length > 0);
  assert.ok(findRustSdkWireHits(`use ruma::OwnedUserId;\n`).length > 0);
  assert.ok(
    findRustSdkWireHits(`fn f() -> matrix_sdk::Room { todo!() }\n`).length > 0
  );
  assert.equal(
    findRustSdkWireHits(`// No matrix_sdk types on the wire.\n`).length,
    0
  );
});

// ---------------------------------------------------------------------------
// Real integration tree must PASS
// ---------------------------------------------------------------------------

test("runGuardrails passes on current repository tree", () => {
  const result = runGuardrails({ root: REPO_ROOT });
  if (!result.ok) {
    console.error(JSON.stringify(result.findings, null, 2));
  }
  assert.equal(result.ok, true);
  assert.equal(result.findingCount, 0);
  assert.ok(result.allowlistSize >= 143);
});

test("committed JS SDK allowlist loads and has expected size", () => {
  const allow = loadJsSdkAllowlist(REPO_ROOT);
  assert.ok(allow.size >= 143);
  assert.equal(
    allow.has("synara/src/app/features/room/RoomTimeline.tsx"),
    false
  );
  assert.ok(allow.has("synara/src/app/features/room/RoomView.tsx"));
  assert.equal(
    allow.has("synara/src/app/features/matrix-ipc/envelope.ts"),
    false
  );
  assert.ok(existsSync(path.join(REPO_ROOT, JS_SDK_ALLOWLIST_REL)));
});

// ---------------------------------------------------------------------------
// Prohibited fixtures must FAIL
// ---------------------------------------------------------------------------

test("fixture: js-sdk in matrix-ipc hard-ban zone fails", () => {
  const result = scanFixture("js-sdk-in-matrix-ipc");
  assert.equal(result.ok, false);
  assert.ok(
    result.findings.some((f) => f.rule === "js-sdk-hard-ban-zone"),
    `expected js-sdk-hard-ban-zone, got ${result.findings.map((f) => f.rule)}`
  );
  assert.ok(result.findings.some((f) => f.path.includes("leakyImport.ts")));
});

test("fixture: new production js-sdk importer outside allowlist fails", () => {
  const result = scanFixture("js-sdk-new-file", { allowlist: new Set() });
  assert.equal(result.ok, false);
  assert.ok(
    result.findings.some((f) => f.rule === "js-sdk-new-importer"),
    `expected js-sdk-new-importer, got ${result.findings.map((f) => f.rule)}`
  );
});

test("fixture: raw /_matrix/ HTTP outside exceptions fails (TS + Rust)", () => {
  const result = scanFixture("raw-matrix-http");
  assert.equal(result.ok, false);
  const raw = result.findings.filter((f) => f.rule === "raw-matrix-http");
  assert.ok(
    raw.length >= 2,
    `expected >=2 raw-matrix-http findings, got ${raw.length}`
  );
  assert.ok(raw.some((f) => f.path.endsWith("rawHttp.ts")));
  assert.ok(raw.some((f) => f.path.endsWith("raw_http.rs")));
});

test("fixture: unversioned IPC envelope fails", () => {
  const result = scanFixture("unversioned-ipc");
  assert.equal(result.ok, false);
  assert.ok(
    result.findings.some(
      (f) =>
        f.rule === "unversioned-ipc-shape" ||
        f.rule === "unversioned-ipc-envelope" ||
        f.rule === "unversioned-ipc-missing-const"
    ),
    `expected unversioned-ipc-* finding, got ${result.findings.map(
      (f) => f.rule
    )}`
  );
});

test("fixture: matrix_sdk in DTO wire module fails", () => {
  const result = scanFixture("sdk-types-in-dto");
  assert.equal(result.ok, false);
  assert.ok(
    result.findings.some((f) => f.rule === "sdk-types-in-wire-rust"),
    `expected sdk-types-in-wire-rust, got ${result.findings.map((f) => f.rule)}`
  );
  assert.ok(result.findings.some((f) => f.path.includes("dto/leaky.rs")));
});

test("fixture: ruma in IPC wire module fails", () => {
  const result = scanFixture("sdk-types-in-ipc");
  assert.equal(result.ok, false);
  assert.ok(
    result.findings.some((f) => f.rule === "sdk-types-in-wire-rust"),
    `expected sdk-types-in-wire-rust, got ${result.findings.map((f) => f.rule)}`
  );
  assert.ok(result.findings.some((f) => f.path.includes("ipc/leaky.rs")));
});

// ---------------------------------------------------------------------------
// Allowlist / exceptions
// ---------------------------------------------------------------------------

test("allowlisted importer does not fail; same path without allowlist fails", () => {
  const root = makeTempRoot();
  try {
    const rel = "synara/src/app/hooks/useSomething.ts";
    writeTree(root, {
      [rel]: `import { Room } from 'matrix-js-sdk';\nexport const x = 1;\n`,
    });
    const files = [rel];
    const readFile = (p) => readFileSync(path.join(root, p), "utf8");

    const blocked = runGuardrails({
      root,
      files,
      allowlist: new Set(),
      readFile,
    });
    assert.equal(blocked.ok, false);
    assert.ok(blocked.findings.some((f) => f.rule === "js-sdk-new-importer"));

    const allowed = runGuardrails({
      root,
      files,
      allowlist: new Set([rel]),
      readFile,
    });
    assert.equal(allowed.ok, true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("approved raw HTTP exceptions pass when scanned alone", () => {
  const root = makeTempRoot();
  try {
    writeTree(root, {
      "synara/src/sw.ts": `const MEDIA_PATHS = ['/_matrix/client/v1/media/download'];\n`,
      "synara/src/app/cs-api.ts": `const u = base + '/_matrix/client/versions';\n`,
    });
    const result = runGuardrails({
      root,
      files: collectFiles(root),
      allowlist: new Set(),
      readFile: (rel) => readFileSync(path.join(root, rel), "utf8"),
    });
    assert.equal(result.ok, true, JSON.stringify(result.findings));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("individual checkers are exportable and composable", () => {
  const findings = checkJsSdkImports({
    root: REPO_ROOT,
    files: ["synara/src/app/features/matrix-ipc/bad.ts"],
    allowlist: new Set(),
    readFile: () => `import { Room } from 'matrix-js-sdk';\n`,
  });
  assert.ok(findings.some((f) => f.rule === "js-sdk-hard-ban-zone"));

  const raw = checkRawMatrixHttp({
    root: REPO_ROOT,
    files: ["synara/src/app/utils/evil.ts"],
    readFile: () => `fetch('/_matrix/client/v3/sync');\n`,
  });
  assert.ok(raw.some((f) => f.rule === "raw-matrix-http"));

  const sdk = checkSdkTypesInWireModules({
    root: REPO_ROOT,
    files: ["src-tauri/src/matrix/dto/x.rs"],
    readFile: () => `use matrix_sdk::Client;\n`,
  });
  assert.ok(sdk.some((f) => f.rule === "sdk-types-in-wire-rust"));

  const ipc = checkUnversionedIpc({
    root: REPO_ROOT,
    files: ["src-tauri/src/matrix/ipc/envelope.rs"],
    readFile: () =>
      `pub struct MatrixIpcEnvelope { pub session_generation: u64, pub sequence: u64, pub kind: String }\n`,
  });
  assert.ok(ipc.length > 0);
});
