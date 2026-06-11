import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

const readPkgrel = () => {
  const pkgbuild = readFileSync(path.join(root, "packaging/arch/PKGBUILD"), "utf8");
  const match = pkgbuild.match(/^pkgrel=(\d+)/m);
  assert.ok(match, "PKGBUILD must define pkgrel");
  return Number.parseInt(match[1], 10);
};

test("bump-version usage documents pkgrel options", () => {
  const script = readFileSync(path.join(root, "scripts/bump-version.mjs"), "utf8");
  assert.match(script, /--pkgrel <n>/);
  assert.match(script, /Auto-increment Arch pkgrel/);
  assert.match(script, /Marketing version bumps reset pkgrel to 1/);
});

test("check-version-consistency validates pkgrel", () => {
  const script = readFileSync(path.join(root, "scripts/check-version-consistency.mjs"), "utf8");
  assert.match(script, /archPkgrel/);
  assert.match(script, /positive integer/);
});

test("current Arch pkgrel is a positive integer", () => {
  const pkgrel = readPkgrel();
  assert.ok(Number.isInteger(pkgrel));
  assert.ok(pkgrel >= 1);
});