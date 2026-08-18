import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  extractLocalMarkdownLinks,
  findBrokenLocalLinks,
  findSensitiveDocumentation,
} from "../check-documentation-hygiene.mjs";

test("sensitive documentation detects private material and machine identifiers", () => {
  const cases = [
    "-----BEGIN PRIVATE KEY-----",
    "/Users/alice/project/file.txt",
    "C:\\Users\\alice\\project\\file.txt",
    "/var/folders/ab/private/file",
    "https://kb.whyland.com/internal",
    "matrix.whyland.com",
    "login as testuser2",
    "run this on spark-3",
    `ghp_${"a".repeat(36)}`,
    `syt_${"a".repeat(24)}`,
    `AKIA${"A".repeat(16)}`,
    `AIza${"a".repeat(32)}`,
    `sk-proj-${"a".repeat(32)}`,
    `xoxb-${"a".repeat(24)}`,
  ];
  for (const value of cases) {
    assert.ok(findSensitiveDocumentation(value).length > 0, value);
  }
});

test("sensitive documentation allows explicit generic examples", () => {
  assert.deepEqual(findSensitiveDocumentation("/Users/example/project"), []);
  assert.deepEqual(
    findSensitiveDocumentation("/Users/runner/work/project"),
    []
  );
  assert.deepEqual(findSensitiveDocumentation("AuthKey_ABC123DEFG.p8"), []);
});

test("markdown link extraction ignores remote and fragment links", () => {
  assert.deepEqual(
    extractLocalMarkdownLinks(
      "[local](docs/README.md) [remote](https://example.com) [anchor](#section)"
    ).map(({ target }) => target),
    ["docs/README.md"]
  );
});

test("local link validation reports missing and escaping targets", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-doc-check-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, "docs"));
  await writeFile(path.join(root, "README.md"), "ok\n");

  const diagnostics = findBrokenLocalLinks({
    root,
    file: "docs/index.md",
    text: "[ok](../README.md) [missing](missing.md) [escape](../../outside.md)",
  });
  assert.equal(diagnostics.length, 2);
  assert.match(diagnostics[0].message, /broken local link/u);
  assert.match(diagnostics[1].message, /escapes the repository/u);
});
