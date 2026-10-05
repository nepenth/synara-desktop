import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { spawnSync } from "node:child_process";
import { benchmark } from "../benchmark-rust-cache.mjs";

function fixture(t, extraEnv = {}) {
  const directory = mkdtempSync(join(tmpdir(), "synara-benchmark-fixture-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const root = join(directory, "repo");
  const bin = join(directory, "bin");
  const scratchParent = join(directory, "scratch");
  for (const path of [root, bin, scratchParent, join(root, "target")])
    mkdirSync(path, { recursive: true });
  writeFileSync(join(root, "target", "existing-artifact"), "preserve this");
  writeFileSync(join(root, "Cargo.toml"), "manifest");
  const log = join(directory, "calls.jsonl");
  const mock = `#!/usr/bin/env node
const fs = require("node:fs"), path = require("node:path");
const command = path.basename(process.argv[1]), args = process.argv.slice(2), env = process.env;
const log = (extra = {}) => fs.appendFileSync(env.BENCHMARK_CALL_LOG, JSON.stringify({ command, args, ...extra }) + "\\n");
if (command === "git") console.log(args[0] === "rev-parse" ? "fixture-sha" : "");
else if (command === "rustc") console.log("rustc 1.96.0\\ncommit-hash: rustc-sha");
else if (args[0] === "--version") console.log(command === "kache" ? "kache 0.28.1" : "cargo 1.96.0");
else if (command === "cargo" && args[0] === "fetch") log();
else if (command === "cargo" && args[0] === "build") {
  if (fs.existsSync(env.CARGO_TARGET_DIR)) throw Error("target was not freshly deleted");
  if (env.CARGO_INCREMENTAL !== "0" || env.CARGO_PROFILE_DEV_DEBUG !== "0") throw Error("profile mismatch");
  if (!fs.readFileSync(env.KACHE_CONFIG, "utf8").includes("local_only = true")) throw Error("remote permitted");
  if (env.KACHE_S3_BUCKET || env.KACHE_HOST_CONFIG !== "") throw Error("ambient cache escaped isolation");
  const calls = fs.existsSync(env.BENCHMARK_CALL_LOG) ? fs.readFileSync(env.BENCHMARK_CALL_LOG, "utf8").trim().split("\\n").map(JSON.parse) : [];
  const phase = calls.filter(call => call.command === "cargo" && call.args[0] === "build").length + 1;
  log({ phase, target: env.CARGO_TARGET_DIR, wrapper: env.RUSTC_WRAPPER });
  if (env.FAIL_PHASE === String(phase)) process.exit(7);
  if (env.HANG_PHASE === String(phase)) setInterval(() => {}, 1000);
  if (env.CHANGE_GRAPH_PHASE === String(phase)) fs.appendFileSync(path.join(process.cwd(), "Cargo.toml"), "changed");
  if (env.CHANGE_CONFIG_PHASE === String(phase)) {
    const config = path.join(process.cwd(), ".cargo");
    fs.mkdirSync(config, { recursive: true }); fs.writeFileSync(path.join(config, "config.toml"), "[build]\\njobs = 1\\n");
  }
  if (env.CHANGE_TEST_PHASE === String(phase)) {
    const tests = path.join(process.cwd(), "crates/synara-core/tests");
    fs.mkdirSync(tests, { recursive: true }); fs.writeFileSync(path.join(tests, "new_test.rs"), "test-only");
  }
  fs.mkdirSync(env.CARGO_TARGET_DIR); fs.writeFileSync(path.join(env.CARGO_TARGET_DIR, "artifact"), "built library");
  if (env.RUSTC_WRAPPER) {
    fs.mkdirSync(env.KACHE_CACHE_DIR, { recursive: true });
    fs.writeFileSync(path.join(env.KACHE_CACHE_DIR, "stats"), JSON.stringify({ local_hits: phase === 4 ? 5 : 0, misses: 2 }));
  }
} else if (command === "kache" && args[0] === "stats") {
  const pathToStats = path.join(env.KACHE_CACHE_DIR, "stats");
  console.log(fs.existsSync(pathToStats) ? fs.readFileSync(pathToStats, "utf8") : '{"local_hits":0,"misses":0}');
} else if (command === "kache" && args[0] === "daemon") log();
else throw Error("unexpected command " + command + " " + args.join(" "));
`;
  for (const name of ["cargo", "kache", "git", "rustc"])
    writeFileSync(join(bin, name), mock, { mode: 0o755 });
  return {
    directory,
    root,
    scratchParent,
    output: join(directory, "report.json"),
    cargo: join(bin, "cargo"),
    kache: join(bin, "kache"),
    minFreeBytes: 0,
    env: {
      ...process.env,
      ...extraEnv,
      BENCHMARK_CALL_LOG: log,
      PATH: `${bin}:${process.env.PATH}`,
      GITHUB_STEP_SUMMARY: join(directory, "summary.md"),
      KACHE_S3_BUCKET: "must-be-removed",
    },
    calls: () =>
      existsSync(log)
        ? readFileSync(log, "utf8").trim().split("\n").map(JSON.parse)
        : [],
  };
}

test("compares four actual codegen builds with fresh disposable targets and isolated cache counters", async (t) => {
  const f = fixture(t);
  const report = await benchmark(f);
  assert.equal(report.status, "passed");
  assert.equal(report.verdict, "warm-cache-hits-observed");
  assert.deepEqual(
    report.runs.map((run) => run.name),
    ["baseline-first", "baseline-repeat", "kache-cold", "kache-warm"],
  );
  assert.equal(report.runs[2].statsDelta.misses, 2);
  assert.equal(report.runs[3].statsDelta.local_hits, 5);
  assert.equal(report.runs[3].statsDelta.misses, 0);
  const builds = f
    .calls()
    .filter((call) => call.command === "cargo" && call.args[0] === "build");
  assert.deepEqual(
    builds.map((call) => Boolean(call.wrapper)),
    [false, false, true, true],
  );
  for (const build of builds)
    assert.ok(build.args.includes("--lib") && build.args.includes("--locked"));
  assert.ok(
    f
      .calls()
      .some(
        (call) =>
          call.command === "kache" && call.args.join(" ") === "daemon stop",
      ),
  );
  assert.deepEqual(readdirSync(f.scratchParent), []);
  assert.equal(
    readFileSync(join(f.root, "target/existing-artifact"), "utf8"),
    "preserve this",
  );
  assert.equal(JSON.parse(readFileSync(f.output)).commit, "fixture-sha");
  assert.match(
    readFileSync(f.env.GITHUB_STEP_SUMMARY, "utf8"),
    /Excludes dependency downloads, GitHub cache restore\/save/,
  );
});

test("build failures stop the trial, preserve existing target output, clean private state and publish a failed report", async (t) => {
  const f = fixture(t, { FAIL_PHASE: "3" });
  await assert.rejects(benchmark(f), /failed \(7\)/);
  const report = JSON.parse(readFileSync(f.output));
  assert.equal(report.status, "failed");
  assert.equal(report.runs.length, 2);
  assert.ok(
    f
      .calls()
      .some((call) => call.command === "kache" && call.args[0] === "daemon"),
  );
  assert.deepEqual(readdirSync(f.scratchParent), []);
  assert.equal(
    readFileSync(join(f.root, "target/existing-artifact"), "utf8"),
    "preserve this",
  );
});

test("free-space reserve refuses the experiment before launching a compiler", async (t) => {
  const f = fixture(t);
  await assert.rejects(
    benchmark({ ...f, minFreeBytes: Number.MAX_SAFE_INTEGER }),
    /free-space reserve/,
  );
  assert.equal(JSON.parse(readFileSync(f.output)).status, "failed");
  assert.deepEqual(f.calls(), []);
  assert.deepEqual(readdirSync(f.scratchParent), []);
});

test("interruption terminates the active compiler group and cleans private benchmark state", async (t) => {
  const f = fixture(t, { HANG_PHASE: "1" });
  const result = benchmark(f);
  const rejection = assert.rejects(result, /benchmark interrupted by SIGTERM/);
  let ready = false;
  for (let attempt = 0; attempt < 200; attempt += 1) {
    if (
      f
        .calls()
        .some((call) => call.command === "cargo" && call.args[0] === "build")
    ) {
      ready = true;
      break;
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 20));
  }
  assert.equal(ready, true, "compiler fixture did not start");
  process.emit("SIGTERM");
  await rejection;
  assert.equal(JSON.parse(readFileSync(f.output)).status, "failed");
  assert.deepEqual(readdirSync(f.scratchParent), []);
  assert.ok(existsSync(join(f.root, "target/existing-artifact")));
});

test("rejects measurements when the source graph changes during a trial", async (t) => {
  const f = fixture(t, { CHANGE_GRAPH_PHASE: "2" });
  await assert.rejects(benchmark(f), /source graph changed during measurement/);
  const report = JSON.parse(readFileSync(f.output));
  assert.equal(report.status, "failed");
  assert.equal(report.activeRun, "baseline-repeat");
  assert.equal(report.runs.length, 1);
  assert.deepEqual(readdirSync(f.scratchParent), []);
});

test("integration test edits do not invalidate a production --lib measurement", async (t) => {
  const f = fixture(t, { CHANGE_TEST_PHASE: "2" });
  const report = await benchmark(f);
  assert.equal(report.status, "passed");
  assert.equal(report.runs.length, 4);
  assert.deepEqual(readdirSync(f.scratchParent), []);
});

test("Cargo configuration drift invalidates the production measurement", async (t) => {
  const f = fixture(t, { CHANGE_CONFIG_PHASE: "2" });
  await assert.rejects(benchmark(f), /source graph changed during measurement/);
  const report = JSON.parse(readFileSync(f.output));
  assert.equal(report.status, "failed");
  assert.equal(report.activeRun, "baseline-repeat");
  assert.equal(report.runs.length, 1);
  assert.deepEqual(readdirSync(f.scratchParent), []);
});

test("scratch budget aborts excessive builds and removes only owned scratch", async (t) => {
  const f = fixture(t);
  await assert.rejects(
    benchmark({ ...f, maxBytes: 1 }),
    /scratch storage exceeded/,
  );
  assert.equal(JSON.parse(readFileSync(f.output)).status, "failed");
  assert.deepEqual(readdirSync(f.scratchParent), []);
  assert.ok(existsSync(join(f.root, "target/existing-artifact")));
});

test("benchmark workflow stays manually invoked, version pinned and outside persistent repository caches", () => {
  const workflow = readFileSync(
    new URL(
      "../../.github/workflows/build-cache-benchmark.yml",
      import.meta.url,
    ),
    "utf8",
  );
  assert.match(workflow, /workflow_dispatch:/);
  assert.doesNotMatch(
    workflow,
    /pull_request:|push:|schedule:|Swatinem|actions\/cache@/,
  );
  assert.match(
    workflow,
    /kunobi-ninja\/kache-action@1a33fb2ff51be23eb9e87abeae6edb65be78f71c/,
  );
  for (const input of [
    "github-cache",
    "save-cache",
    "pr-comment",
    "job-summary",
  ])
    assert.match(workflow, new RegExp(`${input}: "false"`));
  assert.match(workflow, /strict: "true"/);
  assert.match(workflow, /version: v0\.28\.1/);
  assert.match(workflow, /max-size: 512MiB/);
  assert.match(workflow, /--max-scratch-mib 3072/);
  assert.doesNotMatch(workflow, /secrets\.|s3-bucket|pull-requests: write/);
});

test("CLI rejects unsafe scratch budgets before creating or launching a build", () => {
  for (const value of ["0", "-1", "NaN", "Infinity", "not-a-number"]) {
    const result = spawnSync(
      process.execPath,
      [
        new URL("../benchmark-rust-cache.mjs", import.meta.url).pathname,
        "--max-scratch-mib",
        value,
      ],
      { encoding: "utf8" },
    );
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /must be a positive finite number/);
    assert.doesNotMatch(result.stderr, /Benchmark:/);
  }
});
