import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  existsSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { spawnSync } from "node:child_process";

// Use real Cargo/rustc, with no registry dependencies. This verifies the
// generator's crate-type override actually emits the archive at its expected
// profile path while ordinary builds avoid static/dynamic link outputs.
test("ordinary rlib and explicit Apple staticlib modes produce the intended artifacts", (t) => {
  const root = mkdtempSync(join(tmpdir(), "synara-staticlib-mode-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "src"));
  writeFileSync(
    join(root, "Cargo.toml"),
    `[package]
name = "synara-staticlib-probe"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["lib"]
[profile.nse-release]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
strip = "debuginfo"
`,
  );
  writeFileSync(
    join(root, "src/lib.rs"),
    '#[no_mangle]\npub extern "C" fn synara_static_archive_probe() -> u32 { 42 }\n',
  );
  const run = (args) => {
    const result = spawnSync("cargo", args, {
      cwd: root,
      encoding: "utf8",
      timeout: 60_000,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: join(root, "target"),
        CARGO_NET_OFFLINE: "true",
      },
    });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  };
  run(["generate-lockfile", "--offline"]);
  const artifacts = (output) =>
    output
      .split("\n")
      .filter(Boolean)
      .map(JSON.parse)
      .filter((event) => event.reason === "compiler-artifact")
      .flatMap((event) => event.filenames);
  const library = artifacts(
    run(["build", "--locked", "--lib", "--message-format=json"]),
  );
  assert.ok(library.some((file) => file.endsWith(".rlib")));
  assert.ok(library.every((file) => !/\.(?:a|dylib|so|dll)$/.test(file)));
  for (const profile of ["release", "nse-release"]) {
    const archive = artifacts(
      run([
        "rustc",
        "--locked",
        "--lib",
        "--crate-type",
        "staticlib",
        "--profile",
        profile,
        "--message-format=json",
      ]),
    );
    assert.ok(archive.some((file) => /\.(?:a|lib)$/.test(file)));
    assert.ok(archive.every((file) => !/\.(?:dylib|so|dll)$/.test(file)));
    assert.ok(
      archive.some(
        (file) =>
          /\.(?:a|lib)$/.test(file) &&
          dirname(file) === join(root, "target", profile) &&
          existsSync(file),
      ),
    );
  }
});
