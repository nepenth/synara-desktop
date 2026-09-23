import test from "node:test";
import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checker = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../check-synara-nse-core-archive-exports.sh"
);

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "synara-nse-archive-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function writeExecutable(path, contents) {
  writeFileSync(path, contents);
  chmodSync(path, 0o755);
}

function check(t, { archiveContents, nmScript, extraArgs = [] }) {
  const root = fixture(t);
  const archive = join(root, "libsynara_nse_core.a");
  if (archiveContents !== undefined) {
    writeFileSync(archive, archiveContents);
  }
  const env = { ...process.env };
  if (nmScript !== undefined) {
    const nm = join(root, "nm");
    writeExecutable(nm, nmScript);
    env.SYNARA_NSE_ARCHIVE_NM = nm;
  }
  return spawnSync(
    "bash",
    [checker, ...(archiveContents === undefined ? extraArgs : [archive])],
    {
      encoding: "utf8",
      env,
    }
  );
}

const llvm22Mismatch = `#!/usr/bin/env bash
echo "nm: error: $1(synara_nse_core.rcgu.o): Unknown attribute kind (105) (Producer: 'LLVM22.1.2-rust-1.96.1-stable' Reader: 'LLVM APPLE_1_2100.1.1.101_0')" >&2
exit 1
`;

test("readable nm output without full Core exports passes", (t) => {
  const result = check(t, {
    archiveContents: "clean nse archive",
    nmScript: `#!/usr/bin/env bash
echo "_uniffi_synara_nse_core_fn_resolve"
exit 0
`,
  });
  assert.equal(result.status, 0, result.stderr);
});

test("readable nm output with full Core exports fails", (t) => {
  const result = check(t, {
    archiveContents: "clean nse archive",
    nmScript: `#!/usr/bin/env bash
echo "_uniffi_synara_core_fn_login"
exit 0
`,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});

test("a full Core export after large nm output still fails", (t) => {
  const result = check(t, {
    archiveContents: "archive with many symbols",
    nmScript: `#!${process.execPath}
const { writeSync } = require("node:fs");
const chunk = "0000 T _uniffi_synara_nse_core_resolve\\n".repeat(1024);
for (let index = 0; index < 64; index += 1) writeSync(1, chunk);
writeSync(1, "0000 T _uniffi_synara_core_forbidden\\n");
`,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});

test("LLVM22 rustc 1.96 nm mismatch without full Core bytes is not a compile failure", (t) => {
  const result = check(t, {
    archiveContents: "rustc 1.96 nse archive without core uniffi",
    nmScript: llvm22Mismatch,
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stderr, /cannot read rustc 1\.96\/LLVM22 objects/);
});

test("LLVM22 rustc 1.96 nm mismatch still rejects full Core bytes", (t) => {
  const result = check(t, {
    archiveContents: "leaked _uniffi_synara_core_fn_login export",
    nmScript: llvm22Mismatch,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden full Core exports/);
});

test("an unrelated nm failure still fails closed", (t) => {
  const result = check(t, {
    archiveContents: "truncated archive",
    nmScript: `#!/usr/bin/env bash
echo "nm: error: $1: The file was not recognized as a valid object file" >&2
exit 1
`,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /archive symbol inspection failed/);
  assert.doesNotMatch(result.stderr, /cannot read rustc 1\.96\/LLVM22 objects/);
});

test("a missing archive fails before nm", (t) => {
  const result = check(t, { extraArgs: ["/tmp/synara-missing-nse-archive.a"] });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /archive is missing/);
});

test("an empty archive fails before nm", (t) => {
  const result = check(t, {
    archiveContents: "",
    nmScript: `#!/usr/bin/env bash
echo "should not run"
exit 0
`,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /archive is empty/);
});

test("no archives is an inspection failure", () => {
  const result = spawnSync("bash", [checker], { encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Usage:/);
});
