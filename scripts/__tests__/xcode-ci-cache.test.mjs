import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const action = readFileSync(
  new URL("../../.github/actions/save-xcode-cache/action.yml", import.meta.url),
  "utf8",
);
const gate = action
  .match(
    /      run: \|\n([\s\S]+?)    - name: Save compiler objects from main/,
  )[1]
  .split("\n")
  .map((line) => line.replace(/^        /, ""))
  .join("\n");

function publicationFixture({
  ref = "refs/heads/main",
  hit = "false",
  size = 128,
  outside = false,
  symlink = false,
} = {}) {
  const root = mkdtempSync(path.join(os.tmpdir(), "synara-xcode-publication-"));
  try {
    const cache = path.join(root, ".xcode-cache", "CompilationCache.noindex");
    const bin = path.join(root, "bin");
    mkdirSync(bin);
    mkdirSync(cache, { recursive: true });
    if (size !== 0)
      writeFileSync(path.join(cache, "compiler-object"), "fixture");
    if (symlink) {
      rmSync(cache, { recursive: true });
      symlinkSync(bin, cache);
    }
    // The exact action shell runs here. Stub only du's allocated-byte report
    // so quota boundary tests never create a half-gigabyte test cache.
    writeFileSync(
      path.join(bin, "du"),
      `#!/bin/sh\nprintf '%s\\t%s\\n' '${size}' "$2"\n`,
      { mode: 0o755 },
    );
    const output = path.join(root, "output");
    const summary = path.join(root, "summary");
    writeFileSync(output, "");
    writeFileSync(summary, "");
    const result = spawnSync("bash", ["-euo", "pipefail", "-c", gate], {
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        GITHUB_WORKSPACE: root,
        GITHUB_REF: ref,
        GITHUB_OUTPUT: output,
        GITHUB_STEP_SUMMARY: summary,
        CACHE_PATH: outside ? bin : cache,
        CACHE_KEY: "xcode-compilation-v1-toolchain-2026-W41",
        CACHE_HIT: hit,
      },
    });
    return {
      ...result,
      output: readFileSync(output, "utf8"),
      summary: readFileSync(summary, "utf8"),
    };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test("main can publish an exact owned CAS at the aggregate size boundary", () => {
  const result = publicationFixture({ size: 524288 });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.output, /publish=true/);
  assert.match(result.summary, /524288 KiB/);
});

for (const [name, options, reason] of [
  ["branches and tags", { ref: "refs/tags/v2.1.44" }, /only read/],
  ["existing weekly snapshots", { hit: "true" }, /already exists/],
  ["empty stores", { size: 0 }, /No compiler objects/],
  ["oversized stores", { size: 524289 }, /exceeds the 512 MiB/],
]) {
  test(`publication skips ${name} without failing a valid build`, () => {
    const result = publicationFixture(options);
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.output, /publish=false/);
    assert.match(result.summary, reason);
  });
}

for (const [name, options] of [
  ["other directories", { outside: true }],
  ["symlinked stores", { symlink: true }],
]) {
  test(`publication rejects ${name}`, () => {
    const result = publicationFixture(options);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Refusing to publish/);
    assert.doesNotMatch(result.output, /publish=true/);
  });
}
