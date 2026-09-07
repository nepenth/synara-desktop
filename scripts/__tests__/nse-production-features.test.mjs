import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checker = resolve(dirname(fileURLToPath(import.meta.url)), "../check-synara-nse-core-production-features.mjs");

function fixture(t, leak) {
  const root = mkdtempSync(join(tmpdir(), "synara-nse-feature-check-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(join(root, "Cargo.toml"), '[workspace]\nmembers = ["core", "nse"]\nresolver = "2"\n');
  for (const name of ["core", "nse"]) {
    mkdirSync(join(root, name, "src"), { recursive: true });
    writeFileSync(join(root, name, "src/lib.rs"), "");
  }
  writeFileSync(join(root, "core/Cargo.toml"), `[package]
name = "synara-core"
version = "0.1.0"
edition = "2021"
[features]
default = ["full-uniffi"]
full-uniffi = []
nse-preview = []
`);
  writeFileSync(join(root, "nse/Cargo.toml"), `[package]
name = "synara-nse-core"
version = "0.1.0"
edition = "2021"
[dependencies]
synara-core = { path = "../core", default-features = false, features = ["nse-preview"${leak === "normal" ? ', "full-uniffi"' : ""}] }
[dev-dependencies]
synara-core = { path = "../core", features = ["full-uniffi"] }
${leak === "build" ? '[build-dependencies]\nsynara-core = { path = "../core" }\n' : ""}`);
  const lock = spawnSync("cargo", ["generate-lockfile", "--offline", "--manifest-path", join(root, "Cargo.toml")], { encoding: "utf8" });
  assert.equal(lock.status, 0, lock.stderr);
  return join(root, "Cargo.toml");
}

function check(manifest) {
  return spawnSync(process.execPath, [checker, manifest], {
    encoding: "utf8",
    env: { ...process.env, CARGO_NET_OFFLINE: "true" },
  });
}

test("dev-only full Core fixture does not contaminate the production graph", (t) => {
  const manifest = fixture(t);
  const original = spawnSync("cargo", ["tree", "--offline", "--manifest-path", manifest, "-p", "synara-nse-core", "-e", "features"], { encoding: "utf8" });
  assert.equal(original.status, 0, original.stderr);
  assert.match(original.stdout, /synara-core feature "full-uniffi"/);
  const result = check(manifest);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /production feature isolation passed/);
});

for (const edge of ["normal", "build"]) {
  test(`full Core feature on a ${edge} dependency still fails isolation`, (t) => {
    const result = check(fixture(t, edge));
    assert.equal(result.status, 1);
    assert.match(result.stderr, /must not enable the full Core UniFFI feature/);
  });
}

test("a failed Cargo query cannot pass isolation", (t) => {
  const manifest = fixture(t);
  writeFileSync(manifest, "invalid manifest");
  const result = check(manifest);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /production feature query failed/);
});
