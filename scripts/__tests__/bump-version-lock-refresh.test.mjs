import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

test("version bump refreshes the workspace lock without building or changing dependencies", (t) => {
  const fixture = mkdtempSync(path.join(tmpdir(), "synara-version-bump-"));
  t.after(() => rmSync(fixture, { recursive: true, force: true }));
  const write = (relativePath, contents) => {
    const destination = path.join(fixture, relativePath);
    mkdirSync(path.dirname(destination), { recursive: true });
    writeFileSync(destination, contents);
  };
  const copy = (relativePath) => {
    const destination = path.join(fixture, relativePath);
    mkdirSync(path.dirname(destination), { recursive: true });
    copyFileSync(path.join(root, relativePath), destination);
  };
  const read = (relativePath) => readFileSync(path.join(fixture, relativePath), "utf8");
  const run = (command, args) =>
    execFileSync(command, args, {
      cwd: fixture,
      encoding: "utf8",
      env: { ...process.env, CARGO_NET_OFFLINE: "true" },
      stdio: ["ignore", "pipe", "pipe"],
    });

  copy("scripts/bump-version.mjs");
  copy("scripts/check-version-consistency.mjs");
  copy("synara/scripts/update-version.js");
  write("Cargo.toml", '[workspace]\nmembers = ["src-tauri", "tauri"]\nresolver = "2"\n');
  write("src-tauri/Cargo.toml", `[package]
name = "synara"
version = "2.1.42"
edition = "2021"
[dependencies]
tauri = { path = "../tauri", version = "2.11.6" }
`);
  write("src-tauri/src/main.rs", "fn main() {}\n");
  // A cargo check regression would run this build script and fail the bump.
  write("src-tauri/build.rs", 'fn main() { panic!("version bumps must not compile"); }\n');
  write("tauri/Cargo.toml", '[package]\nname = "tauri"\nversion = "2.11.6"\nedition = "2021"\n');
  write("tauri/src/lib.rs", "");
  write("src-tauri/tauri.conf.json", JSON.stringify({ version: "2.1.42" }));
  for (const packageRoot of ["", "synara/"]) {
    const packageJson = {
      name: packageRoot ? "synara-runtime" : "synara-desktop",
      version: "2.1.42",
      type: "module",
      private: true,
      dependencies: { "@tauri-apps/api": "2.11.6" },
      devDependencies: { "@tauri-apps/cli": "2.11.6" },
    };
    write(`${packageRoot}package.json`, JSON.stringify(packageJson));
    write(`${packageRoot}package-lock.json`, JSON.stringify({
      name: packageJson.name,
      version: packageJson.version,
      lockfileVersion: 3,
      packages: { "": packageJson },
    }));
  }
  write("synara-ios/project.yml", 'MARKETING_VERSION: "2.1.42"\nCURRENT_PROJECT_VERSION: "2.1.42"\n');
  write("synara-ios/Synara.xcodeproj/project.pbxproj", "MARKETING_VERSION = 2.1.42;\nCURRENT_PROJECT_VERSION = 2.1.42;\n");
  write("packaging/arch/PKGBUILD", "pkgrel=4\n");

  run("cargo", ["generate-lockfile", "--offline"]);
  const originalLock = read("Cargo.lock");
  run(process.execPath, ["scripts/bump-version.mjs", "2.1.43", "--ios-build", "2.1.43"]);
  assert.equal(
    read("Cargo.lock"),
    originalLock.replace('name = "synara"\nversion = "2.1.42"', 'name = "synara"\nversion = "2.1.43"')
  );
  assert.equal(existsSync(path.join(fixture, "target")), false, "the bump must not produce compiler artifacts");
  assert.match(read("packaging/arch/PKGBUILD"), /^pkgrel=1$/m);
  for (const packageRoot of ["", "synara/"]) {
    assert.equal(JSON.parse(read(`${packageRoot}package.json`)).version, "2.1.43");
    const lock = JSON.parse(read(`${packageRoot}package-lock.json`));
    assert.equal(lock.version, "2.1.43");
    assert.equal(lock.packages[""].version, "2.1.43");
  }
  assert.match(read("synara-ios/project.yml"), /CURRENT_PROJECT_VERSION: "2.1.43"/);
  assert.match(read("synara-ios/Synara.xcodeproj/project.pbxproj"), /CURRENT_PROJECT_VERSION = 2.1.43;/);
  // Re-running an already bumped release is idempotent, including the lock.
  const bumpedLock = read("Cargo.lock");
  run(process.execPath, ["scripts/bump-version.mjs", "2.1.43", "--ios-build", "2.1.43"]);
  assert.equal(read("Cargo.lock"), bumpedLock);
  assert.equal(existsSync(path.join(fixture, "target")), false);
});
