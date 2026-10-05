import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
  chmodSync,
  readFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checker = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../check-synara-nse-core-production-features.mjs",
);

function fixture(t, leak) {
  const appleTarget =
    leak === "ios-only" || leak === "ios-build"
      ? "'cfg(target_os = \"ios\")'"
      : leak?.startsWith("target:")
        ? JSON.stringify(leak.slice(7))
        : undefined;
  const root = mkdtempSync(join(tmpdir(), "synara-nse-feature-check-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(
    join(root, "Cargo.toml"),
    '[workspace]\nmembers = ["core", "nse"]\nresolver = "2"\n',
  );
  for (const name of ["core", "nse"]) {
    mkdirSync(join(root, name, "src"), { recursive: true });
    writeFileSync(join(root, name, "src/lib.rs"), "");
  }
  writeFileSync(
    join(root, "core/Cargo.toml"),
    `[package]
name = "synara-core"
version = "0.1.0"
edition = "2021"
[features]
default = ["full-uniffi"]
full-uniffi = []
full-app = []
nse-preview = []
`,
  );
  writeFileSync(
    join(root, "nse/Cargo.toml"),
    `[package]
name = "synara-nse-core"
version = "0.1.0"
edition = "2021"
[dependencies]
synara-core = { path = "../core", default-features = false, features = ["nse-preview"${
      leak === "normal"
        ? ', "full-uniffi"'
        : leak === "full-app"
          ? ', "full-app"'
          : ""
    }] }
[dev-dependencies]
synara-core = { path = "../core", features = ["full-uniffi"] }
${
  leak === "build"
    ? '[build-dependencies]\nsynara-core = { path = "../core" }\n'
    : ""
}
${
  appleTarget
    ? `[target.${appleTarget}.${
        leak === "ios-build" ? "build-dependencies" : "dependencies"
      }]\nsynara-core = { path = "../core", features = ["full-uniffi"] }\n`
    : ""
}`,
  );
  const lock = spawnSync(
    "cargo",
    [
      "generate-lockfile",
      "--offline",
      "--manifest-path",
      join(root, "Cargo.toml"),
    ],
    { encoding: "utf8" },
  );
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
  const original = spawnSync(
    "cargo",
    [
      "tree",
      "--offline",
      "--manifest-path",
      manifest,
      "-p",
      "synara-nse-core",
      "-e",
      "features",
    ],
    { encoding: "utf8" },
  );
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

test("full application owners without UniFFI still fail NSE isolation", (t) => {
  const result = check(fixture(t, "full-app"));
  assert.equal(result.status, 1);
  assert.match(result.stderr, /must not enable full application owners/);
});

test("a failed Cargo query cannot pass isolation", (t) => {
  const manifest = fixture(t);
  writeFileSync(manifest, "invalid manifest");
  const result = check(manifest);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /production feature query failed/);
});

for (const target of [
  "ios-only",
  "ios-build",
  "aarch64-apple-ios",
  "aarch64-apple-ios-sim",
  "x86_64-apple-ios",
]) {
  test(`${target} production leakage is rejected even when the macOS graph is narrow`, (t) => {
    const manifest = fixture(
      t,
      target.startsWith("ios-") ? target : `target:${target}`,
    );
    const macOS = spawnSync(
      "cargo",
      [
        "tree",
        "--offline",
        "--manifest-path",
        manifest,
        "-p",
        "synara-nse-core",
        "-e",
        "normal,build,features",
        "-i",
        "synara-core",
        "--target",
        "aarch64-apple-darwin",
      ],
      { encoding: "utf8" },
    );
    assert.equal(macOS.status, 0, macOS.stderr);
    assert.doesNotMatch(macOS.stdout, /synara-core feature "full-uniffi"/);
    const result = check(manifest);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /must not enable the full Core UniFFI feature/);
  });
}

for (const feature of [
  "x509-identity",
  "experimental-x509-identity-verification",
  "rust-x509-verifier-impl",
]) {
  test(`upstream-only ${feature} is rejected in the forward feature graph`, (t) => {
    const manifest = fixture(t);
    const root = dirname(manifest);
    mkdirSync(join(root, "crypto/src"), { recursive: true });
    writeFileSync(join(root, "crypto/src/lib.rs"), "");
    writeFileSync(
      join(root, "crypto/Cargo.toml"),
      `[package]
name = "matrix-sdk-crypto"
version = "0.1.0"
edition = "2021"
[features]
${feature} = []
`,
    );
    writeFileSync(
      manifest,
      readFileSync(manifest, "utf8").replace(
        '"core", "nse"',
        '"core", "nse", "crypto"',
      ),
    );
    const core = join(root, "core/Cargo.toml");
    writeFileSync(
      core,
      readFileSync(core, "utf8") +
        `
[dependencies]
matrix-sdk-crypto = { path = "../crypto", features = ["${feature}"] }
`,
    );
    const lock = spawnSync(
      "cargo",
      ["generate-lockfile", "--offline", "--manifest-path", manifest],
      { encoding: "utf8" },
    );
    assert.equal(lock.status, 0, lock.stderr);
    const inverse = spawnSync(
      "cargo",
      [
        "tree",
        "--locked",
        "--manifest-path",
        manifest,
        "-p",
        "synara-nse-core",
        "-e",
        "normal,build,features",
        "-i",
        "synara-core",
        "--target",
        "aarch64-apple-ios",
      ],
      { encoding: "utf8", env: { ...process.env, CARGO_TERM_COLOR: "never" } },
    );
    assert.equal(inverse.status, 0, inverse.stderr);
    assert.doesNotMatch(inverse.stdout, new RegExp(`feature "${feature}"`));
    const result = check(manifest);
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /must not enable X\.509 identity verification/);
  });
}

test("every Cargo query overrides inherited color before matching feature nodes", (t) => {
  const root = mkdtempSync(join(tmpdir(), "synara-nse-color-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const cargo = join(root, "cargo");
  const log = join(root, "calls");
  writeFileSync(
    cargo,
    String.raw`#!/bin/sh
printf '%s\n' "$CARGO_TERM_COLOR" >> "$QUERY_LOG"
if [ "$CARGO_TERM_COLOR" != never ]; then
  printf 'matrix-sdk-crypto feature "\033[31mexperimental-x509-identity-verification\033[0m"\n'
  exit 0
fi
case "$*" in
  *"-i synara-core"*) printf 'synara-core feature "nse-preview"\n' ;;
  *"normal,build,features"*)
    if [ "$QUERY_LEAK" = 1 ]; then printf 'matrix-sdk-crypto feature "experimental-x509-identity-verification"\n'; fi ;;
  *) printf 'synara-core v0.1.0 (/fixture/x509-identity)\n' ;;
esac
`,
  );
  chmodSync(cargo, 0o755);
  const env = {
    ...process.env,
    PATH: `${root}:${process.env.PATH}`,
    CARGO_TERM_COLOR: "always",
    QUERY_LOG: log,
  };
  const clean = spawnSync(process.execPath, [checker, "fixture.toml"], {
    encoding: "utf8",
    env,
  });
  assert.equal(clean.status, 0, clean.stderr);
  assert.deepEqual(
    readFileSync(log, "utf8").trim().split("\n"),
    Array(12).fill("never"),
  );
  const leaking = spawnSync(process.execPath, [checker, "fixture.toml"], {
    encoding: "utf8",
    env: { ...env, QUERY_LEAK: "1" },
  });
  assert.equal(leaking.status, 1, leaking.stderr);
  assert.match(leaking.stderr, /must not enable X\.509 identity verification/);
});

// Root-feature forwarding may not emit the upstream feature node in a forward
// tree; the guard must inspect effective package feature sets as well.
test("forwarded upstream-only room-key forwarding cannot bypass NSE isolation", (t) => {
  const manifest = fixture(t);
  const root = dirname(manifest);
  mkdirSync(join(root, "crypto/src"), { recursive: true });
  writeFileSync(join(root, "crypto/src/lib.rs"), "");
  writeFileSync(
    join(root, "crypto/Cargo.toml"),
    `[package]
name = "matrix-sdk-crypto"
version = "0.1.0"
edition = "2021"
[features]
automatic-room-key-forwarding = []
`,
  );
  writeFileSync(
    manifest,
    readFileSync(manifest, "utf8").replace(
      '"core", "nse"',
      '"core", "nse", "crypto"',
    ),
  );
  const core = join(root, "core/Cargo.toml");
  writeFileSync(
    core,
    readFileSync(core, "utf8").replace(
      "nse-preview = []",
      'nse-preview = ["matrix-sdk-crypto/automatic-room-key-forwarding"]',
    ) +
      `
[dependencies]
matrix-sdk-crypto = { path = "../crypto" }
`,
  );
  const lock = spawnSync(
    "cargo",
    ["generate-lockfile", "--offline", "--manifest-path", manifest],
    { encoding: "utf8" },
  );
  assert.equal(lock.status, 0, lock.stderr);
  const result = check(manifest);
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /must not compile automatic-room-key-forwarding/);
});
