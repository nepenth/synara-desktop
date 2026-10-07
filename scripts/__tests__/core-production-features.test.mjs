import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checker = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../check-synara-core-production-features.mjs",
);

function fixture(t, leak) {
  const root = mkdtempSync(join(tmpdir(), "synara-core-feature-check-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const packages = ["core", "desktop", "sdk", "uniffi"];
  writeFileSync(
    join(root, "Cargo.toml"),
    `[workspace]\nmembers = ${JSON.stringify(packages)}\nresolver = "2"\n`,
  );
  for (const pkg of packages) {
    mkdirSync(join(root, pkg, "src"), { recursive: true });
    writeFileSync(join(root, pkg, "src/lib.rs"), "");
  }
  writeFileSync(
    join(root, "sdk/Cargo.toml"),
    `[package]
name = "matrix-sdk"
version = "0.1.0"
edition = "2021"
[features]
automatic-room-key-forwarding = []
experimental-search = []
`,
  );
  writeFileSync(
    join(root, "uniffi/Cargo.toml"),
    `[package]\nname = "uniffi"\nversion = "0.1.0"\nedition = "2021"\n`,
  );
  writeFileSync(
    join(root, "core/Cargo.toml"),
    `[package]
name = "synara-core"
version = "0.1.0"
edition = "2021"
[features]
default = ["full-uniffi"]
full-app = [${leak === "forwarding" ? '"matrix-sdk/automatic-room-key-forwarding"' : ""}]
full-uniffi = ["full-app", "dep:uniffi"${leak === "apple-search" ? ', "matrix-sdk/experimental-search"' : ""}]
[dependencies]
matrix-sdk = { path = "../sdk" }
uniffi = { path = "../uniffi", optional = true }
`,
  );
  writeFileSync(
    join(root, "desktop/Cargo.toml"),
    `[package]
name = "synara"
version = "0.1.0"
edition = "2021"
[dependencies]
synara-core = { path = "../core", default-features = false, features = [${leak === "missing-owners" ? "" : '"full-app"'}] }
${leak === "normal" ? 'uniffi = { path = "../uniffi" }' : ""}
[dev-dependencies]
synara-core = { path = "../core", features = ["full-uniffi"] }
${leak === "build" ? '[build-dependencies]\nuniffi = { path = "../uniffi" }' : ""}
${leak === "windows" ? '[target.\'cfg(target_os = "windows")\'.dependencies]\nuniffi = { path = "../uniffi" }' : ""}
`,
  );
  const manifest = join(root, "Cargo.toml");
  const lock = spawnSync(
    "cargo",
    ["generate-lockfile", "--offline", "--manifest-path", manifest],
    { encoding: "utf8" },
  );
  assert.equal(lock.status, 0, lock.stderr);
  return manifest;
}

const check = (manifest) =>
  spawnSync(process.execPath, [checker, manifest], {
    encoding: "utf8",
    env: {
      ...process.env,
      CARGO_NET_OFFLINE: "true",
      CARGO_TERM_COLOR: "always",
    },
  });

test("separate shipping graphs preserve Apple bindings and exclude dev-only desktop FFI", (t) => {
  const result = check(fixture(t));
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /production feature boundaries passed/);
});
for (const edge of ["normal", "build", "windows"]) {
  test(`desktop UniFFI leakage on ${edge} edge fails`, (t) => {
    const result = check(fixture(t, edge));
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Desktop must not compile Apple UniFFI/);
  });
}
test("desktop must retain full application owners", (t) => {
  const result = check(fixture(t, "missing-owners"));
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Desktop must retain full application owners/);
});
test("shipping graphs reject automatic room-key forwarding", (t) => {
  const result = check(fixture(t, "forwarding"));
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /must not compile automatic room-key forwarding/);
});
test("Apple full Core rejects forwarded desktop-only SDK features", (t) => {
  const result = check(fixture(t, "apple-search"));
  assert.notEqual(result.status, 0);
  assert.match(
    result.stderr,
    /Apple full Core must not compile desktop search/,
  );
});
test("Cargo query failure cannot pass production boundaries", (t) => {
  const manifest = fixture(t);
  writeFileSync(manifest, "invalid manifest");
  const result = check(manifest);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Core production graph query failed/);
});
