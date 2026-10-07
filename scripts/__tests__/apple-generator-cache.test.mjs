import assert from "node:assert/strict";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

function fixture(t, { bounded, override }) {
  const root = mkdtempSync(join(tmpdir(), "synara-apple-cache-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const bin = join(root, "bin");
  mkdirSync(bin);
  mkdirSync(join(root, "scripts/lib"), { recursive: true });
  for (const name of [
    "generate-synara-core-swift.sh",
    "generate-synara-nse-core-swift.sh",
    "lib/publish-generated-apple-pair.sh",
    "lib/compare-generated-apple-pair.mjs",
    "lib/rust-cache.sh",
  ]) {
    copyFileSync(join(repoRoot, "scripts", name), join(root, "scripts", name));
  }
  // Existing graph/LLVM tests prove the real guards. Record their ordering here
  // while exercising the actual generators and transactional publication.
  writeFileSync(
    join(root, "scripts/check-synara-nse-core-production-features.mjs"),
    'import fs from "node:fs"; fs.appendFileSync(process.env.CACHE_LOG, JSON.stringify({ command: "preflight" }) + "\\n");\n',
  );
  writeFileSync(
    join(root, "scripts/lib/rust-llvm-symbols.sh"),
    "resolve_rust_llvm_nm() { return 0; }\n",
  );
  const checker = join(
    root,
    "scripts/check-synara-nse-core-archive-exports.sh",
  );
  writeFileSync(
    checker,
    '#!/bin/sh\nfor archive in "$@"; do test -s "$archive" || exit 1; done\n',
  );
  chmodSync(checker, 0o755);

  const mock = `#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
const command = path.basename(process.argv[1], ".mjs");
const args = process.argv.slice(2);
const targetDir = process.env.CARGO_TARGET_DIR;
const log = (extra = {}) => fs.appendFileSync(process.env.CACHE_LOG,
  JSON.stringify({ command, args, targetDir, wrapper: process.env.RUSTC_WRAPPER, incremental: process.env.CARGO_INCREMENTAL, ...extra }) + "\\n");
const value = (name) => args[args.indexOf(name) + 1];
if (command === "kache") console.log("kache 1.0.0");
else if (command === "uname") console.log("Darwin");
else if (command === "rustup") console.log([
  "aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios", "aarch64-apple-darwin",
].join("\\n"));
else if (command === "cargo" && args[0] === "rustc") {
  const previous = fs.existsSync(process.env.LAST_APPLE_TARGET)
    ? fs.readFileSync(process.env.LAST_APPLE_TARGET, "utf8") : "";
  // Bounded builds must remove each target before starting the next one.
  if (process.env.SYNARA_APPLE_SPACE_BOUNDED === "1" && previous && fs.existsSync(previous)) {
    throw new Error("previous bounded target survived: " + previous);
  }
  fs.writeFileSync(process.env.LAST_APPLE_TARGET, targetDir);
  log();
  const profile = args.includes("--profile") ? value("--profile") : "release";
  const output = path.join(targetDir, value("--target"), profile);
  fs.mkdirSync(output, { recursive: true });
  fs.writeFileSync(path.join(output, "lib" + value("--package").replaceAll("-", "_") + ".a"), "archive");
} else if (command === "cargo" && args[0] === "run") {
  const marker = path.join(targetDir, "host-generator-reuse");
  const reused = fs.existsSync(marker);
  log({ reused });
  fs.mkdirSync(targetDir, { recursive: true });
  fs.writeFileSync(marker, "preserved");
  const output = value("--out-dir");
  // Library mode names the binding after --crate; the input is the archive.
  if (!args.includes("--crate") || !value("generate").endsWith(".a")) {
    throw new Error("Apple generators must run UniFFI in library mode");
  }
  const name = value("--crate");
  fs.mkdirSync(output, { recursive: true });
  for (const [suffix, contents] of [[".swift", "generated"], ["FFI.h", "header"], ["FFI.modulemap", "module"]]) {
    fs.writeFileSync(path.join(output, name + suffix), contents);
  }
} else if (command === "xcodebuild") {
  log();
  const output = value("-output");
  fs.mkdirSync(output, { recursive: true });
  fs.writeFileSync(path.join(output, "Info.plist"), "framework");
  for (let i = 0; i < args.length; i++) if (args[i] === "-library") {
    if (!fs.existsSync(args[i + 1])) throw new Error("missing staged archive");
    const slice = path.join(output, "slice-" + i);
    fs.mkdirSync(slice);
    fs.copyFileSync(args[i + 1], path.join(slice, path.basename(args[i + 1])));
  }
} else if (command === "xcrun" && args[0] === "lipo") {
  log(); fs.writeFileSync(value("-output"), "fat-archive");
} else if (command === "xcrun") console.log("/mock-sdk");
else throw new Error("unexpected mock invocation: " + command + " " + args.join(" "));
`;
  // Each command is a separate .mjs entry so Node treats it as an ES module.
  for (const command of [
    "uname",
    "rustup",
    "cargo",
    "xcrun",
    "xcodebuild",
    "kache",
  ]) {
    const file = join(bin, `${command}.mjs`);
    writeFileSync(file, mock);
    chmodSync(file, 0o755);
    writeFileSync(join(bin, command), `#!/bin/sh\nexec node "${file}" "$@"\n`);
    chmodSync(join(bin, command), 0o755);
  }
  const log = join(root, "calls.jsonl");
  const bindgenDir = override
    ? join(root, "custom-host-cache")
    : join(root, "target/synara-core-bindgen");
  const env = {
    ...process.env,
    PATH: `${bin}:${process.env.PATH}`,
    SYNARA_KACHE_BIN: join(bin, "kache"),
    RUSTC_WRAPPER: "",
    RUSTC_WORKSPACE_WRAPPER: "",
    CARGO_BUILD_RUSTC_WRAPPER: "",
    CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER: "",
    KACHE_CACHE_DIR: join(root, "compiler-cache"),
    KACHE_RUNTIME_DIR: join(root, "compiler-cache/run"),
    CACHE_LOG: log,
    LAST_APPLE_TARGET: join(root, "last-apple-target"),
    SYNARA_APPLE_SPACE_BOUNDED: bounded ? "1" : "0",
    SYNARA_CORE_APPLE_SLICES: "all",
    SYNARA_NSE_CORE_APPLE_SLICES: "all",
  };
  for (const name of [
    "SYNARA_CORE_APPLE_TARGET_DIR",
    "SYNARA_NSE_CORE_APPLE_TARGET_DIR",
    "SYNARA_CORE_APPLE_SPACE_BOUNDED",
    "SYNARA_NSE_CORE_APPLE_SPACE_BOUNDED",
    "SYNARA_APPLE_BINDGEN_TARGET_DIR",
  ])
    delete env[name];
  if (override) env.SYNARA_APPLE_BINDGEN_TARGET_DIR = bindgenDir;
  return {
    root,
    bindgenDir,
    run(name) {
      const result = spawnSync("bash", [join(root, "scripts", name)], {
        env,
        encoding: "utf8",
      });
      assert.equal(result.status, 0, result.stderr);
      return result;
    },
    calls: () => readFileSync(log, "utf8").trim().split("\n").map(JSON.parse),
  };
}

for (const bounded of [false, true]) {
  for (const override of [false, true]) {
    test(`Apple generators reuse ${override ? "overridden" : "default"} host cache in ${bounded ? "bounded" : "ordinary"} mode`, (t) => {
      const f = fixture(t, { bounded, override });
      f.run("generate-synara-core-swift.sh");
      const coreOutputs = [
        "Sources/SynaraCore/Generated/synara_core.swift",
        "Artifacts/SynaraCore.xcframework",
        "Artifacts/SynaraCore.xcframework/Info.plist",
      ];
      const outputMetadata = () =>
        coreOutputs.map((output) => {
          const info = lstatSync(
            join(f.root, "synara-ios/SynaraCore", output),
            { bigint: true },
          );
          return { inode: info.ino, mtime: info.mtimeNs };
        });
      const originalOutputs = outputMetadata();
      f.run("generate-synara-nse-core-swift.sh");
      assert.match(
        f.run("generate-synara-core-swift.sh").stdout,
        /generated pair unchanged/,
      );
      assert.deepEqual(outputMetadata(), originalOutputs);
      const calls = f.calls();
      for (const call of calls.filter((call) => call.command === "cargo")) {
        assert.equal(call.wrapper, join(f.root, "bin/kache"));
        assert.equal(call.incremental, "0");
      }
      assert.equal(calls[0].command, "preflight");
      const hostRuns = calls.filter(
        (call) => call.command === "cargo" && call.args[0] === "run",
      );
      assert.deepEqual(
        hostRuns.map((call) => call.reused),
        [false, true, true],
      );
      for (const call of hostRuns) {
        assert.equal(call.targetDir, f.bindgenDir);
        assert.ok(call.args.includes("--locked"));
        assert.equal(
          call.args[call.args.indexOf("--package") + 1],
          "synara-core-bindgen",
        );
        assert.equal(
          call.args[call.args.indexOf("--manifest-path") + 1],
          join(f.root, "Cargo.toml"),
        );
      }
      assert.equal(
        readFileSync(join(f.bindgenDir, "host-generator-reuse"), "utf8"),
        "preserved",
      );
      const builds = calls.filter(
        (call) => call.command === "cargo" && call.args[0] === "rustc",
      );
      assert.equal(builds.length, 11); // Core four slices, NSE three, Core rerun four.
      for (const call of builds) {
        assert.ok(call.args.includes("--lib"));
        assert.equal(
          call.args[call.args.indexOf("--crate-type") + 1],
          "staticlib",
        );
        assert.notEqual(call.targetDir, f.bindgenDir);
        assert.equal(existsSync(call.targetDir), !bounded);
      }
      for (const name of ["SynaraCore", "SynaraNseCore"]) {
        assert.ok(
          existsSync(
            join(
              f.root,
              "synara-ios",
              name,
              "Artifacts",
              `${name}.xcframework/Info.plist`,
            ),
          ),
        );
      }
    });
  }
}
