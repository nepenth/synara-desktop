import assert from "node:assert/strict";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const script = fileURLToPath(
  new URL("../generate-xcode-project.mjs", import.meta.url),
);
const outputs = [
  "Synara.xcodeproj/project.pbxproj",
  "Synara.xcodeproj/project.xcworkspace/contents.xcworkspacedata",
  "Synara.xcodeproj/xcshareddata/xcschemes/Synara.xcscheme",
  "Synara/App/Info.plist",
  "SynaraNotificationService/Info.plist",
];

test("Xcode GUI project and generator spec enable native compilation caching", () => {
  const ios = fileURLToPath(new URL("../../synara-ios/", import.meta.url));
  assert.match(
    readFileSync(join(ios, "project.yml"), "utf8"),
    /base:\s*\n\s+COMPILATION_CACHE_ENABLE_CACHING: YES/,
  );
  const project = readFileSync(
    join(ios, "Synara.xcodeproj/project.pbxproj"),
    "utf8",
  );
  for (const configuration of [
    "464FCA1D294DA9ABE961D7DE",
    "B1054016A110974004BA7A1A",
  ]) {
    const start = project.indexOf(`${configuration} /*`);
    assert.notEqual(start, -1);
    const block = project.slice(start, project.indexOf("name =", start));
    assert.match(block, /COMPILATION_CACHE_ENABLE_CACHING = YES;/);
  }
});

function fixture({ real = false } = {}) {
  const directory = mkdtempSync(join(tmpdir(), "synara-xcodegen-cache."));
  const bin = join(directory, "bin");
  const cache = join(directory, "cache", "spec");
  mkdirSync(bin);
  if (!real) {
    const executable = join(bin, "xcodegen");
    writeFileSync(
      executable,
      `#!/usr/bin/env node
const fs = require("node:fs"), path = require("node:path");
if (process.argv.includes("--version")) { console.log(process.env.MOCK_XCODEGEN_VERSION || "Version: 2.45.4"); process.exit(); }
const cache = process.argv[process.argv.indexOf("--cache-path") + 1];
if (process.env.MOCK_XCODEGEN_FAIL === "1") { fs.writeFileSync(cache, "partial"); process.exit(9); }
if (fs.existsSync(cache)) { console.log("Native cache hit"); process.exit(); }
for (const output of ${JSON.stringify(outputs)}) {
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, "generated " + output);
}
fs.writeFileSync(cache, "spec cache");
console.log("Generated");
`,
    );
    chmodSync(executable, 0o755);
  }
  return {
    directory,
    cache,
    run(env = {}) {
      return spawnSync(
        process.execPath,
        [script, "--project-dir", directory, "--cache-path", cache],
        {
          encoding: "utf8",
          env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, ...env },
        },
      );
    },
    cleanup() {
      rmSync(directory, { recursive: true, force: true });
    },
  };
}

function successful(result) {
  assert.equal(result.status, 0, result.stderr + result.stdout);
}

test("unchanged outputs keep project inode/mtime; personal state and remote locks do not invalidate", () => {
  const context = fixture();
  try {
    successful(context.run());
    const project = join(context.directory, outputs[0]);
    const original = statSync(project, { bigint: true });
    const user = join(
      context.directory,
      "Synara.xcodeproj/xcuserdata/person.xcuserdatad",
    );
    const lock = join(
      context.directory,
      "Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved",
    );
    mkdirSync(user, { recursive: true });
    mkdirSync(dirname(lock), { recursive: true });
    writeFileSync(join(user, "state.xcuserstate"), "personal");
    writeFileSync(lock, "reviewed remote revisions");
    const result = context.run();
    successful(result);
    assert.match(result.stdout, /Native cache hit/);
    const current = statSync(project, { bigint: true });
    assert.equal(current.ino, original.ino);
    assert.equal(current.mtimeNs, original.mtimeNs);
    assert.equal(readFileSync(lock, "utf8"), "reviewed remote revisions");
  } finally {
    context.cleanup();
  }
});

test("changed project, missing scheme/plist, corrupt state and generator changes force generation", () => {
  const context = fixture();
  try {
    successful(context.run());
    for (const output of outputs) {
      const file = join(context.directory, output);
      writeFileSync(file, "another checked-out project version");
      const result = context.run();
      successful(result);
      assert.match(result.stdout, /Generated/);
      assert.equal(readFileSync(file, "utf8"), `generated ${output}`);
      rmSync(file);
      successful(context.run());
      assert.ok(existsSync(file));
    }
    writeFileSync(`${context.cache}.outputs.json`, "corrupt");
    assert.match(context.run().stdout, /Generated/);
    const result = context.run({ MOCK_XCODEGEN_VERSION: "Version: 2.46.0" });
    successful(result);
    assert.match(result.stdout, /Generated/);
  } finally {
    context.cleanup();
  }
});

test("failed generation clears partial native cache and retries normally", () => {
  const context = fixture();
  try {
    assert.equal(context.run({ MOCK_XCODEGEN_FAIL: "1" }).status, 9);
    assert.ok(!existsSync(context.cache));
    assert.ok(!existsSync(`${context.cache}.outputs.json`));
    successful(context.run());
  } finally {
    context.cleanup();
  }
});

const installed =
  spawnSync("xcodegen", ["--version"], { encoding: "utf8" }).status === 0;
test(
  "real XcodeGen preserves identical project and responds to source/spec/output changes",
  { skip: !installed },
  () => {
    const context = fixture({ real: true });
    try {
      for (const directory of ["Synara/App", "SynaraNotificationService"])
        mkdirSync(join(context.directory, directory), { recursive: true });
      const spec = `name: Synara
targets:
  Synara:
    type: application
    platform: iOS
    sources: [Synara]
    info:
      path: Synara/App/Info.plist
      properties:
        CFBundleDisplayName: Synara
  SynaraNotificationService:
    type: app-extension
    platform: iOS
    sources: [SynaraNotificationService]
    info:
      path: SynaraNotificationService/Info.plist
schemes:
  Synara:
    build:
      targets:
        Synara: all
`;
      writeFileSync(join(context.directory, "project.yml"), spec);
      writeFileSync(
        join(context.directory, "Synara/App/App.swift"),
        "import Foundation\n",
      );
      successful(context.run());
      // The first generation creates new plist source names; let the native
      // source-list cache observe them before measuring a stable repeat.
      successful(context.run());
      const project = join(context.directory, outputs[0]);
      const original = statSync(project, { bigint: true });
      successful(context.run());
      const unchanged = statSync(project, { bigint: true });
      assert.equal(unchanged.ino, original.ino);
      assert.equal(unchanged.mtimeNs, original.mtimeNs);
      writeFileSync(
        join(context.directory, "Synara/App/Added.swift"),
        "import Foundation\n",
      );
      successful(context.run());
      assert.match(readFileSync(project, "utf8"), /Added.swift in Sources/);
      writeFileSync(
        join(context.directory, "project.yml"),
        spec.replace(
          "CFBundleDisplayName: Synara",
          "CFBundleDisplayName: Changed",
        ),
      );
      successful(context.run());
      assert.match(
        readFileSync(join(context.directory, outputs[3]), "utf8"),
        /Changed/,
      );
      for (const output of [outputs[2], outputs[3]]) {
        rmSync(join(context.directory, output));
        successful(context.run());
        assert.ok(existsSync(join(context.directory, output)));
      }
    } finally {
      context.cleanup();
    }
  },
);
