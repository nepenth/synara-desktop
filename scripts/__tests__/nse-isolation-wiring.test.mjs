import test from "node:test";
import assert from "node:assert/strict";
import {
  copyFileSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const checkerPath = "scripts/check-synara-nse-core-isolation.mjs";
const ciBuildPath = "synara-ios/scripts/ci-build.sh";
const productionCheckerPath =
  "scripts/check-synara-nse-core-production-features.mjs";
const invocation =
  'node "$repo_root/scripts/check-synara-nse-core-production-features.mjs"';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "synara-nse-isolation-wiring-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const source = readFileSync(join(repoRoot, checkerPath), "utf8");
  const inputs = [...source.matchAll(/read\(\s*"([^"]+)"\s*,?\s*\)/g)].map(
    (match) => match[1]
  );
  inputs.push(ciBuildPath, productionCheckerPath, checkerPath);
  for (const path of new Set(inputs)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    copyFileSync(join(repoRoot, path), join(root, path));
  }
  return {
    root,
    run: () =>
      spawnSync(process.execPath, [join(root, checkerPath)], {
        encoding: "utf8",
      }),
  };
}

test("the NSE isolation scaffold accepts the wired production checker", (t) => {
  const result = fixture(t).run();
  assert.equal(result.status, 0, result.stderr);
});

test("the NSE isolation scaffold rejects a removed production check", (t) => {
  const isolated = fixture(t);
  const ciBuild = readFileSync(join(isolated.root, ciBuildPath), "utf8");
  assert.ok(ciBuild.includes(invocation));
  writeFileSync(
    join(isolated.root, ciBuildPath),
    ciBuild.replace(invocation, "")
  );
  const result = isolated.run();
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /missing NSE production feature CI invocation/);
});

test("the NSE isolation scaffold rejects a missing production checker", (t) => {
  const isolated = fixture(t);
  rmSync(join(isolated.root, productionCheckerPath));
  const result = isolated.run();
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /check-synara-nse-core-production-features\.mjs/);
});
