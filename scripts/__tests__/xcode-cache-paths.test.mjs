import assert from "node:assert/strict";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repository = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const helper = join(repository, "synara-ios/scripts/lib/xcode-cache.sh");
function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), "synara-xcode-cache."));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const project = join(directory, "synara-ios");
  const bin = join(directory, "bin");
  mkdirSync(bin);
  mkdirSync(join(project, "Synara.xcodeproj"), { recursive: true });
  mkdirSync(join(project, "Local"));
  mkdirSync(join(directory, "scripts"));
  copyFileSync(
    join(repository, "scripts/check-xcode-local-package-graph.mjs"),
    join(directory, "scripts/check-xcode-local-package-graph.mjs"),
  );
  writeFileSync(
    join(project, "Synara.xcodeproj/project.pbxproj"),
    "isa = XCLocalSwiftPackageReference; relativePath = Local;",
  );
  for (const [command, body] of Object.entries({
    "xcode-select": 'printf "/Applications/FixtureXcode/Contents/Developer\\n"',
    xcodebuild:
      'printf "Xcode %s\\nBuild version %s\\n" "${FIXTURE_XCODE_MAJOR:-26}" "${FIXTURE_BUILD:-A1}"',
    xcrun:
      'if [ "${FIXTURE_TOOL_FAILURE:-0}" = 1 ]; then exit 71; fi\ncase "$*" in\n"--find swift") printf "%s/swift\\n" "${DEVELOPER_DIR:-/Fixture/Developer}" ;;\n"swift --version") printf "Apple Swift 6.2\\n" ;;\n*) printf "%s\\n" "${FIXTURE_SDK:-26.0}" ;;\nesac',
    xcodegen: 'printf "Version: 2.46.0\\n"',
    uname: 'printf "%s\\n" "${FIXTURE_ARCH:-arm64}"',
    swift: 'printf "%s" "$FIXTURE_MANIFEST"',
  })) {
    const path = join(bin, command);
    writeFileSync(path, `#!/bin/sh\n${body}\n`);
    chmodSync(path, 0o755);
  }
  const env = {
    ...process.env,
    PATH: `${bin}:${process.env.PATH}`,
    FIXTURE_PROJECT: project,
    FIXTURE_HELPER: helper,
    FIXTURE_MANIFEST: '{"dependencies":[]}',
  };
  for (const variable of [
    "SYNARA_IOS_CACHE_ROOT",
    "CLANG_MODULE_CACHE_PATH",
    "SWIFTPM_MODULECACHE_OVERRIDE",
    "IOS_PACKAGE_CACHE_PATH",
    "IOS_CLONED_SOURCE_PACKAGES_DIR_PATH",
    "XCODEGEN_CACHE_PATH",
  ])
    delete env[variable];
  const run = (extra = {}, command = "paths") =>
    spawnSync(
      "bash",
      [
        "-c",
        `
set -euo pipefail
source "$FIXTURE_HELPER"
synara_configure_xcode_cache "$FIXTURE_PROJECT"
if [[ "$FIXTURE_COMMAND" == packages ]]; then
  synara_xcode_package_args "$FIXTURE_PROJECT"
  node -e 'console.log(JSON.stringify(process.argv.slice(1)))' -- "\${SYNARA_XCODE_PACKAGE_ARGS[@]}"
else
  node -e 'console.log(JSON.stringify({root:process.env.SYNARA_XCODE_CACHE_ROOT,key:process.env.SYNARA_XCODE_TOOLCHAIN_KEY,modules:process.env.CLANG_MODULE_CACHE_PATH,compilation:process.env.SYNARA_XCODE_COMPILATION_CACHE_PATH,args:process.argv.slice(1)}))' \${SYNARA_XCODE_COMPILATION_ARGS[@]+"\${SYNARA_XCODE_COMPILATION_ARGS[@]}"}
fi
`,
      ],
      { encoding: "utf8", env: { ...env, ...extra, FIXTURE_COMMAND: command } },
    );
  return { directory, project, run };
}
const parsed = (result) => {
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout);
};

test("keeps local cache identities stable and isolates clones, selected tools, SDKs and architectures", (t) => {
  const context = fixture(t);
  const first = parsed(context.run());
  assert.deepEqual(parsed(context.run()), first);
  assert.match(
    first.root,
    /Library\/Caches\/Synara\/Xcode\/[a-f0-9]{64}\/[a-f0-9]{64}$/,
  );
  const clone = join(context.directory, "another-clone");
  mkdirSync(clone);
  for (const extra of [
    { FIXTURE_PROJECT: clone },
    { FIXTURE_BUILD: "B2" },
    { DEVELOPER_DIR: "/Other/Developer" },
    { FIXTURE_SDK: "26.1" },
    { FIXTURE_ARCH: "x86_64" },
  ])
    assert.notEqual(parsed(context.run(extra)).root, first.root);
  assert.match(first.modules, /ModuleCache\.noindex$/);
  assert.match(first.compilation, /CompilationCache\.noindex$/);
});

test("respects explicit cache and module roots and opts into a bounded local native compiler cache", (t) => {
  const context = fixture(t);
  const root = join(context.directory, "custom cache");
  const actual = parsed(
    context.run({
      SYNARA_IOS_CACHE_ROOT: root,
      CLANG_MODULE_CACHE_PATH: "/explicit modules",
      SYNARA_IOS_COMPILATION_CACHE_LIMIT: "256M",
      SYNARA_IOS_CACHE_REMARKS: "YES",
    }),
  );
  assert.equal(actual.root, root);
  assert.equal(actual.modules, "/explicit modules");
  assert.ok(
    actual.args.includes(
      `COMPILATION_CACHE_CAS_PATH=${root}/CompilationCache.noindex`,
    ),
  );
  for (const flag of [
    "COMPILATION_CACHE_ENABLE_CACHING=YES",
    "COMPILATION_CACHE_LIMIT_SIZE=256M",
    "COMPILATION_CACHE_ENABLE_DIAGNOSTIC_REMARKS=YES",
    "COMPILATION_CACHE_ENABLE_PLUGIN=NO",
    "COMPILATION_CACHE_REMOTE_SERVICE_PATH=",
  ])
    assert.ok(actual.args.includes(flag));
});

test("retains old local Xcode compatibility and fails closed when selected tools cannot be inspected", (t) => {
  const context = fixture(t);
  assert.deepEqual(parsed(context.run({ FIXTURE_XCODE_MAJOR: "16" })).args, []);
  assert.equal(context.run({ FIXTURE_TOOL_FAILURE: "1" }).status, 71);
});

test("all-local graphs use shared download/checkouts without requesting a nonexistent lock", (t) => {
  const context = fixture(t);
  const args = parsed(
    context.run(
      {
        IOS_PACKAGE_CACHE_PATH: "/package downloads",
        IOS_CLONED_SOURCE_PACKAGES_DIR_PATH: "/package checkouts",
      },
      "packages",
    ),
  );
  assert.deepEqual(args, [
    "-packageCachePath",
    "/package downloads",
    "-clonedSourcePackagesDirPath",
    "/package checkouts",
    "-scmProvider",
    "system",
  ]);
});

test("remote dependencies require the reviewed lock and forbid automatic re-resolution", (t) => {
  const context = fixture(t);
  const env = { FIXTURE_MANIFEST: '{"dependencies":[{"sourceControl":[{}]}]}' };
  const missing = context.run(env, "packages");
  assert.equal(missing.status, 1);
  assert.match(missing.stderr, /committed Swift package lock/);
  const directory = join(
    context.project,
    "Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm",
  );
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, "Package.resolved"), '{"version":3,"pins":[]}');
  const args = parsed(context.run(env, "packages"));
  for (const flag of [
    "-onlyUsePackageVersionsFromResolvedFile",
    "-disableAutomaticPackageResolution",
    "-skipPackageUpdates",
  ])
    assert.ok(args.includes(flag));
});

test("stale all-local locks fail without deleting or rewriting reviewed files", (t) => {
  const context = fixture(t);
  const directory = join(
    context.project,
    "Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm",
  );
  mkdirSync(directory, { recursive: true });
  const lock = join(directory, "Package.resolved");
  writeFileSync(lock, "reviewed bytes");
  const result = context.run({}, "packages");
  assert.equal(result.status, 1);
  assert.match(result.stderr, /stale Swift package lock/);
  assert.equal(readFileSync(lock, "utf8"), "reviewed bytes");
});

test("the UI runner reuses its prepared products and preserves automatic signing", (t) => {
  const context = fixture(t);
  const scripts = join(context.project, "scripts");
  mkdirSync(join(scripts, "lib"), { recursive: true });
  copyFileSync(helper, join(scripts, "lib/xcode-cache.sh"));
  const runner = join(scripts, "run-ui-tests.sh");
  copyFileSync(join(repository, "synara-ios/scripts/run-ui-tests.sh"), runner);
  const log = join(context.directory, "xcode.jsonl");
  writeFileSync(
    join(context.directory, "bin/xcodebuild"),
    `#!/bin/sh
if [ "$1" = -version ]; then printf 'Xcode 26.0\\nBuild version A1\\n'; exit 0; fi
node -e 'require("node:fs").appendFileSync(process.argv[1],JSON.stringify(process.argv.slice(2))+"\\n")' -- "$FIXTURE_LOG" "$@"
`,
  );
  const result = spawnSync("bash", [runner], {
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${join(context.directory, "bin")}:${process.env.PATH}`,
      FIXTURE_MANIFEST: '{"dependencies":[]}',
      FIXTURE_LOG: log,
      SYNARA_IOS_CACHE_ROOT: join(context.directory, "cache root"),
      SYNARA_UI_TEST_SHARD: "auth-and-rooms",
      SYNARA_UI_TEST_SIGNING_MODE: "automatic",
      SYNARA_IOS_DEVELOPMENT_TEAM: "TEAM",
    },
  });
  assert.equal(result.status, 0, result.stderr);
  const calls = readFileSync(log, "utf8").trim().split("\n").map(JSON.parse);
  assert.equal(calls.length, 2);
  assert.ok(calls[0].includes("build-for-testing"));
  assert.ok(calls[1].includes("test-without-building"));
  const derived = calls[0][calls[0].indexOf("-derivedDataPath") + 1];
  assert.match(derived, /cache root\/DerivedData\/ui-tests$/);
  for (const args of calls) {
    assert.equal(args[args.indexOf("-derivedDataPath") + 1], derived);
    assert.ok(args.includes("CODE_SIGN_STYLE=Automatic"));
    assert.ok(args.includes("DEVELOPMENT_TEAM=TEAM"));
    assert.ok(!args.includes("CODE_SIGNING_ALLOWED=NO"));
    assert.ok(!args.includes("-onlyUsePackageVersionsFromResolvedFile"));
    assert.ok(args.includes("COMPILATION_CACHE_ENABLE_CACHING=YES"));
  }
});
