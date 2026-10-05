import assert from "node:assert/strict";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = fileURLToPath(new URL("../..", import.meta.url));
const helper = join(repository, "scripts/lib/rust-cache.sh");
const setup = join(repository, "scripts/setup-rust-cache.sh");
const wrapper = join(repository, "scripts/with-rust-cache.sh");
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "synara-rust-cache-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const binary = join(root, "fake kache");
  writeFileSync(
    binary,
    `#!/bin/sh
if [ "$1" = --version ]; then printf '%s\\n' "\${FAKE_KACHE_VERSION:-kache 1.0.0}"; else printf '%s\\n' "$*" >> "$CALL_LOG"; fi
`,
  );
  chmodSync(binary, 0o755);
  const env = {
    ...process.env,
    XDG_DATA_HOME: join(root, "data"),
    XDG_CACHE_HOME: join(root, "cache"),
    SYNARA_KACHE_BIN: binary,
    CALL_LOG: join(root, "calls"),
  };
  for (const name of Object.keys(env)) {
    if (
      name.startsWith("KACHE_") ||
      name.startsWith("CARGO_BUILD_") ||
      name === "RUSTC_WRAPPER" ||
      name === "RUSTC_WORKSPACE_WRAPPER" ||
      name === "SYNARA_RUST_CACHE_ROOT" ||
      name === "SYNARA_RUST_CACHE_MAX_SIZE" ||
      name === "CMAKE_C_COMPILER_LAUNCHER" ||
      name === "CMAKE_CXX_COMPILER_LAUNCHER"
    )
      delete env[name];
  }
  return {
    root,
    binary,
    env,
    run(script, args = [], extra = {}) {
      return spawnSync("bash", [script, ...args], {
        encoding: "utf8",
        cwd: root,
        env: { ...env, ...extra },
      });
    },
  };
}
function configure(f, extra = {}) {
  return f.run(
    "-c",
    [
      `source "$1"; synara_configure_rust_cache "$2" && node -e 'console.log(JSON.stringify(process.env))'`,
      "bash",
      helper,
      repository,
    ],
    extra,
  );
}

test("persistent user cache covers ordinary commands without global Cargo or shell mutation", (t) => {
  const f = fixture(t),
    result = configure(f);
  assert.equal(result.status, 0, result.stderr);
  const env = JSON.parse(result.stdout);
  assert.equal(env.RUSTC_WRAPPER, f.binary);
  assert.equal(env.CARGO_INCREMENTAL, "0");
  assert.equal(env.CMAKE_C_COMPILER_LAUNCHER, f.binary);
  assert.equal(env.CMAKE_CXX_COMPILER_LAUNCHER, f.binary);
  assert.equal(
    env.KACHE_CACHE_DIR,
    join(f.root, "cache/Synara/rust-cache/kache"),
  );
  assert.equal(env.KACHE_RUNTIME_DIR, join(env.KACHE_CACHE_DIR, "run"));
  assert.equal(env.KACHE_CONFIG, join(repository, ".kache.toml"));
  assert.equal(env.KACHE_LOCAL_ONLY, undefined);
  assert.match(readFileSync(env.KACHE_CONFIG, "utf8"), /^local_only = true$/m);
  assert.equal(env.KACHE_CACHE_EXECUTABLES, "1");
  assert.equal(env.KACHE_AUTO_CLEAN_ORPHANED_TARGETS, "0");
  assert.equal(env.KACHE_AUTO_CLEAN_UNUSED_UNITS_DAYS, "0");
  assert.equal(env.KACHE_MAX_SIZE, undefined);
  assert.equal(readFileSync(helper, "utf8").includes(".cargo/config"), false);
});
test("relative compiler installation resolves before Cargo changes directories", (t) => {
  const f = fixture(t);
  const result = configure(f, { SYNARA_KACHE_BIN: "./fake kache" });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    realpathSync(JSON.parse(result.stdout).RUSTC_WRAPPER),
    realpathSync(f.binary),
  );
});
test("CI cache paths, config and budget remain owned by the action", (t) => {
  const f = fixture(t);
  const configured = {
    KACHE_CACHE_DIR: join(f.root, "ci-store"),
    KACHE_RUNTIME_DIR: join(f.root, "ci-runtime"),
    KACHE_CONFIG: join(f.root, "ci.toml"),
    KACHE_MAX_SIZE: "8GiB",
    KACHE_LOCAL_ONLY: "1",
    RUSTC_WRAPPER: f.binary,
  };
  const result = configure(f, configured);
  assert.equal(result.status, 0, result.stderr);
  const env = JSON.parse(result.stdout);
  for (const [key, value] of Object.entries(configured))
    assert.equal(env[key], value);
});
for (const [name, extra] of [
  [
    "unknown compiler wrapper",
    { SYNARA_KACHE_BIN: "", RUSTC_WRAPPER: "/bin/echo" },
  ],
  ["workspace compiler wrapper", { RUSTC_WORKSPACE_WRAPPER: "/bin/echo" }],
  ["Cargo compiler wrapper", { CARGO_BUILD_RUSTC_WRAPPER: "/bin/echo" }],
  ["wrong Kache version", { FAKE_KACHE_VERSION: "kache 0.27.0" }],
  ["disabled Kache", { KACHE_DISABLED: "1" }],
])
  test(`rejects ${name} without silently falling back`, (t) => {
    const result = configure(fixture(t), extra);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /synara-rust-cache/);
  });
test("CMake launchers preserve explicit compiler choices and empty opt-outs", (t) => {
  const result = configure(fixture(t), {
    CMAKE_C_COMPILER_LAUNCHER: "",
    CMAKE_CXX_COMPILER_LAUNCHER: "user-launcher",
    CC: "target-clang",
    CXX: "target-clang++",
  });
  assert.equal(result.status, 0, result.stderr);
  const env = JSON.parse(result.stdout);
  assert.equal(env.CMAKE_C_COMPILER_LAUNCHER, "");
  assert.equal(env.CMAKE_CXX_COMPILER_LAUNCHER, "user-launcher");
  assert.equal(env.CC, "target-clang");
  assert.equal(env.CXX, "target-clang++");
});
test("shell environment output survives whitespace and executes no setup installation", (t) => {
  const f = fixture(t);
  const result = f.run(setup, ["--env"]);
  assert.equal(result.status, 0, result.stderr);
  const restored = f.run("-c", [
    `${result.stdout}\nprintf '%s' "$RUSTC_WRAPPER"`,
  ]);
  assert.equal(restored.stdout, f.binary);
});
test("command wrapper preserves arguments and subprocess exit status", (t) => {
  const f = fixture(t);
  const result = f.run(wrapper, [
    "bash",
    "-c",
    "printf '%s' \"$1\"; exit 17",
    "bash",
    "argument with spaces",
  ]);
  assert.equal(result.status, 17);
  assert.equal(result.stdout, "argument with spaces");
});
test("status uses the documented stats and cleanup-preview commands", (t) => {
  const f = fixture(t);
  const result = f.run(setup, ["--status"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    readFileSync(f.env.CALL_LOG, "utf8"),
    "stats\nclean --stale 14d --dry-run\n",
  );
});
test("cleanup previews stale targets unless --yes is explicitly requested", (t) => {
  const f = fixture(t);
  const result = f.run(setup, ["--clean"]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(
    readFileSync(f.env.CALL_LOG, "utf8"),
    "clean --stale 14d --dry-run\n",
  );
  const applied = f.run(setup, ["--clean", "--yes"]);
  assert.equal(applied.status, 0, applied.stderr);
  assert.match(
    readFileSync(f.env.CALL_LOG, "utf8"),
    /clean --stale 14d --yes\n$/,
  );
});
test("installer checks the pinned SHA256 before extraction or execution", (t) => {
  const f = fixture(t),
    bin = join(f.root, "bin");
  mkdirSync(bin);
  for (const [name, body] of Object.entries({
    curl: 'while [ "$1" != --output ]; do shift; done; printf corrupt > "$2"',
    tar: 'printf extracted > "$CALL_LOG"',
  })) {
    const path = join(bin, name);
    writeFileSync(path, `#!/bin/sh\n${body}\n`);
    chmodSync(path, 0o755);
  }
  const result = f.run(setup, ["--install"], { PATH: `${bin}:${f.env.PATH}` });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /SHA256 verification failed/);
});
