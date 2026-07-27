import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  runGuardrails,
  stripCommentsAndStrings,
} from "../check-matrix-rust-sdk-guardrails.mjs";

const REPO_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);

function writeFixture(root, relPath, content) {
  const abs = path.join(root, relPath);
  mkdirSync(path.dirname(abs), { recursive: true });
  writeFileSync(abs, content, "utf8");
}

function makeTree(files) {
  const root = mkdtempSync(path.join(tmpdir(), "matrix-rust-guardrails-"));
  const list = [];
  for (const [rel, content] of Object.entries(files)) {
    writeFixture(root, rel, content);
    list.push(rel.split(path.sep).join("/"));
  }
  list.sort();
  return { root, files: list };
}

test("stripCommentsAndStrings removes comments and string bodies", () => {
  const src = `
//! No matrix_sdk types here.
use foo::Bar;
// use matrix_sdk::Client;
let s = "matrix_sdk::Client";
/* matrix_sdk::ui */
`;
  const stripped = stripCommentsAndStrings(src, { lang: "rs", blankStrings: true });
  assert.equal(stripped.includes("use matrix_sdk"), false);
  assert.equal(stripped.includes("use foo::Bar"), true);
  // string body + quotes blanked
  assert.equal(stripped.includes("matrix_sdk::Client"), false);
  assert.match(stripped, /let s =\s+;/);

  const keepStrings = stripCommentsAndStrings(src, {
    lang: "rs",
    blankStrings: false,
  });
  assert.equal(keepStrings.includes('let s = "matrix_sdk::Client"'), true);
});

test("clean greenfield contracts pass", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/ipc/version.rs":
      'pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n',
    "src-tauri/src/matrix/ipc/mod.rs":
      "pub mod version;\npub use version::*;\n",
    "src-tauri/src/matrix/dto/mod.rs":
      "pub struct SessionSnapshot;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
    "synara/src/app/features/matrix-ipc/index.ts":
      "export * from './version';\n",
    "synara/src/app/features/matrix-dto/index.ts":
      "export type RoomSummary = { roomId: string };\n",
    "src-tauri/src/lib.rs":
      "tauri::Builder::default().invoke_handler(tauri::generate_handler![desktop::desktop_show])\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, true, formatFail(result));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on matrix_sdk types in DTO module", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/dto/bad.rs":
      "use matrix_sdk::Client;\npub type Bad = Client;\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some((v) => v.rule === "no-sdk-types-in-dto-ipc"),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on matrix-js-sdk import in matrix-ipc", () => {
  const { root, files } = makeTree({
    "synara/src/app/features/matrix-ipc/evil.ts":
      "import { createClient } from 'matrix-js-sdk';\nexport const c = createClient;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some(
        (v) => v.rule === "no-js-sdk-in-greenfield-contracts"
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on raw /_matrix/ HTTP in contract surface", () => {
  const { root, files } = makeTree({
    "synara/src/app/features/matrix-dto/http.ts":
      'export const path = "/_matrix/client/v3/sync";\n',
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some(
        (v) => v.rule === "no-raw-matrix-http-in-contracts"
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on production Client construction under matrix/ outside client_builder/", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/session.rs":
      "fn open() { let _ = Client::builder(); }\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some(
        (v) => v.rule === "no-production-matrix-client-in-matrix-module"
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("allows Client::builder under matrix/client_builder/ but bans login/sync", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/client_builder/open.rs":
      "fn open() { let _ = Client::builder().homeserver_url(\"https://example.org\"); }\n",
    "src-tauri/src/matrix/client_builder/evil.rs":
      "fn login(c: Client) { let _ = c.matrix_auth().login_username(\"a\", \"b\"); }\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false, formatFail(result));
    // login under client_builder must fail
    assert.ok(
      result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("evil.rs")
      ),
      formatFail(result)
    );
    // builder-only construction under client_builder/open.rs must not be the reason
    assert.ok(
      !result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("open.rs")
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("allows password/token login under matrix/auth/ but bans Client::builder and sync", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/auth/login.rs":
      "async fn login(c: &Client) { let _ = c.matrix_auth().login_username(\"a\", \"b\").initial_device_display_name(\"Synara macOS\").await; let _ = c.matrix_auth().login_token(\"t\"); }\n",
    "src-tauri/src/matrix/auth/evil_builder.rs":
      "fn open() { let _ = Client::builder(); }\n",
    "src-tauri/src/matrix/auth/evil_sync.rs":
      "async fn s(c: &Client) { let _ = c.sync_once(Default::default()).await; }\n",
    "src-tauri/src/matrix/supervisor/evil_login.rs":
      "fn login(c: &Client) { let _ = c.matrix_auth().login_token(\"t\"); }\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false, formatFail(result));
    // login under auth/ must be allowed
    assert.ok(
      !result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("auth/login.rs")
      ),
      formatFail(result)
    );
    // builder and sync under auth must fail
    assert.ok(
      result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("evil_builder.rs")
      ),
      formatFail(result)
    );
    assert.ok(
      result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("evil_sync.rs")
      ),
      formatFail(result)
    );
    // login outside auth must fail
    assert.ok(
      result.violations.some(
        (v) =>
          v.rule === "no-production-matrix-client-in-matrix-module" &&
          v.path.includes("evil_login.rs")
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on dual-backend selector pattern", () => {
  const { root, files } = makeTree({
    "synara/src/app/state/matrixBackend.ts":
      "export type MatrixBackend = 'js' | 'rust';\nexport const matrixBackendSelector = 'js';\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some((v) => v.rule === "no-dual-backend-selector"),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on missing MATRIX_IPC_PROTOCOL_VERSION (unversioned IPC)", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/ipc/mod.rs": "pub struct Envelope;\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "// forgot the constant\npub const OTHER: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const OTHER = 1;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some((v) => v.rule === "versioned-matrix-ipc"),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("fails on matrix_ product Tauri command in invoke_handler", () => {
  const { root, files } = makeTree({
    "src-tauri/src/lib.rs": `
fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      desktop::desktop_show,
      matrix_login,
      matrix_sync_status,
    ]);
}
`,
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, false);
    assert.ok(
      result.violations.some(
        (v) => v.rule === "no-matrix-product-tauri-commands"
      ),
      formatFail(result)
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("doc comments mentioning matrix_sdk do not trip SDK-type rule", () => {
  const { root, files } = makeTree({
    "src-tauri/src/matrix/ipc/mod.rs":
      "//! No `matrix_sdk` types, no live supervisor.\n//! No ruma:: types either.\npub const X: u8 = 1;\n",
    "src-tauri/src/matrix/ipc/version.rs":
      "pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;\n",
    "src-tauri/src/matrix/dto/mod.rs":
      "//! Product DTOs — not a matrix_sdk object graph.\npub struct RoomSummary;\n",
    "synara/src/app/features/matrix-ipc/version.ts":
      "/** Not matrix-js-sdk. */\nexport const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;\n",
    "synara/src/app/features/matrix-ipc/index.ts":
      "export * from './version';\n",
  });
  try {
    const result = runGuardrails({ root, files });
    assert.equal(result.ok, true, formatFail(result));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("repository root currently satisfies guardrails", () => {
  const result = runGuardrails({ root: REPO_ROOT });
  assert.equal(
    result.ok,
    true,
    `live repo must pass P1.6 guardrails:\n${formatFail(result)}`
  );
});

function formatFail(result) {
  return (
    result.summary +
    "\n" +
    result.violations
      .map((v) => `  [${v.rule}] ${v.path}:${v.line} — ${v.detail}`)
      .join("\n")
  );
}
