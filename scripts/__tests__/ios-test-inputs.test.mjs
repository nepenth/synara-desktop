import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  IOS_PROOF_STEP_PREFIX,
  IOS_TESTS_JOB_ID,
  IOS_TEST_INPUT_PATHS,
  appleToolchainSteps,
  extractWorkflowJob,
  hashCommit,
  iosTestInputsHash,
} from "../ios-test-inputs.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const read = (file) => readFileSync(path.join(root, file), "utf8");
const covered = (file) =>
  IOS_TEST_INPUT_PATHS.some((input) => file === input || file.startsWith(`${input}/`));

const workflow = `jobs:
  changes:
    steps:
      - run: true

  ios-tests:
    name: iOS simulator tests
    steps:
      - name: Install Rust 1.96 and Apple targets
        with:
          components: llvm-tools-preview
      - run: scripts/ci-build.sh

  ios-ui-tests:
    steps:
      - run: true
`;

test("extractWorkflowJob returns exactly one job's block", () => {
  assert.equal(
    extractWorkflowJob(workflow, "ios-tests"),
    `  ios-tests:
    name: iOS simulator tests
    steps:
      - name: Install Rust 1.96 and Apple targets
        with:
          components: llvm-tools-preview
      - run: scripts/ci-build.sh`
  );
  assert.equal(extractWorkflowJob(workflow, "missing"), null);
  assert.equal(extractWorkflowJob(`${workflow}  ios-tests:\n    steps: []\n`, "ios-tests"), null);
});

test("appleToolchainSteps matches the NSE isolation checker's steps", () => {
  const steps = appleToolchainSteps(workflow);
  assert.equal(steps.length, 1);
  assert.match(steps[0], /^ {6}- name: Install Rust 1\.96 and Apple targets\n {8}with:\n {10}components: llvm-tools-preview/);
  assert.match(read("scripts/check-synara-nse-core-isolation.mjs"), /Install Rust 1\\\.96 and Apple targets/);
});

test("the hash changes with each input and refuses missing ones", () => {
  const base = { treeListing: "100644 blob aaaa\tCargo.lock", jobBlock: "  ios-tests:", toolchainSteps: ["a"] };
  const hash = iosTestInputsHash(base);
  assert.match(hash, /^[0-9a-f]{64}$/);
  assert.equal(iosTestInputsHash({ ...base }), hash);
  assert.notEqual(iosTestInputsHash({ ...base, treeListing: "100644 blob bbbb\tCargo.lock" }), hash);
  assert.notEqual(iosTestInputsHash({ ...base, jobBlock: "  ios-tests:\n    timeout-minutes: 1" }), hash);
  assert.notEqual(iosTestInputsHash({ ...base, toolchainSteps: ["b"] }), hash);
  assert.throws(() => iosTestInputsHash({ ...base, treeListing: "" }));
  assert.throws(() => iosTestInputsHash({ ...base, jobBlock: null }));
});

test("hashCommit tracks committed iOS inputs and ignores everything else", () => {
  const cwd = mkdtempSync(path.join(tmpdir(), "synara-ios-inputs-"));
  const git = (...args) =>
    execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  const write = (file, text) => {
    mkdirSync(path.dirname(path.join(cwd, file)), { recursive: true });
    writeFileSync(path.join(cwd, file), text);
  };
  const commit = () => {
    git("add", ".");
    git("-c", "core.hooksPath=/dev/null", "commit", "--quiet", "--allow-empty", "-m", "change");
    return hashCommit("HEAD", cwd);
  };
  try {
    git("init", "--quiet");
    git("config", "user.email", "test@example.org");
    git("config", "user.name", "iOS inputs fixture");
    write(".github/workflows/ci.yml", workflow);
    write(".github/workflows/release.yml", "jobs:\n  build:\n    steps: []\n");
    write("synara-ios/project.yml", "name: Synara\n");
    write("Cargo.lock", "# lock\n");
    write("docs/readme.md", "docs\n");
    const first = commit();

    write("docs/readme.md", "other docs\n");
    write("synara/src/app.ts", "export {};\n");
    write(".github/workflows/release.yml", "jobs:\n  build:\n    timeout-minutes: 5\n    steps: []\n");
    assert.equal(commit(), first, "docs, renderer and unrelated release edits keep the hash");

    write("untracked-only.txt", "x");
    write("synara-ios/project.yml", "name: Edited but not committed\n");
    assert.equal(hashCommit("HEAD", cwd), first, "the working tree does not count");
    git("checkout", "--quiet", "--", "synara-ios/project.yml");

    write(".github/workflows/ci.yml", workflow.replace("  ios-ui-tests:\n    steps:", "  ios-ui-tests:\n    timeout-minutes: 9\n    steps:"));
    assert.equal(commit(), first, "another job's edit keeps the hash");

    for (const [file, text] of [
      ["synara-ios/project.yml", "name: Synara 2\n"],
      ["Cargo.lock", "# lock 2\n"],
      ["crates/synara-core/src/lib.rs", "pub fn f() {}\n"],
      ["scripts/generate-synara-core-swift.sh", "#!/bin/sh\n"],
      [".github/workflows/ci.yml", workflow.replace("ci-build.sh", "ci-build.sh --verbose")],
      [".github/workflows/release.yml", "jobs:\n  ios:\n    steps:\n      - name: Install Rust 1.96 and Apple targets\n        with: {}\n"],
    ]) {
      const before = hashCommit("HEAD", cwd);
      write(file, text);
      assert.notEqual(commit(), before, `${file} changes the hash`);
    }

    git("rm", "--quiet", ".github/workflows/ci.yml");
    git("-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "drop workflow");
    assert.throws(() => hashCommit("HEAD", cwd), "a missing workflow fails closed");
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
});

test("every input path exists in this repository", () => {
  for (const input of IOS_TEST_INPUT_PATHS) {
    assert.ok(existsSync(path.join(root, input)), `${input} is listed but missing`);
  }
});

// Every repository script the job, ci-build.sh and the scripts they call name
// must be a hash input, or editing it would not invalidate an earlier proof.
test("every script the iOS simulator job reaches is a hash input", () => {
  const job = extractWorkflowJob(read(".github/workflows/ci.yml"), IOS_TESTS_JOB_ID);
  assert.ok(job, "ci.yml has one ios-tests job");
  assert.ok(job.includes(`name: ${IOS_PROOF_STEP_PREFIX} \${{ needs.changes.outputs.ios_inputs_hash }}`));

  const seen = new Set();
  const pending = [];
  // Comment lines name scripts that are not run.
  const code = (text) =>
    text
      .split("\n")
      .filter((line) => !/^\s*(?:#|\/\/|\*)/.test(line))
      .join("\n");
  const visit = (text) => {
    for (const [reference] of code(text).matchAll(/(?:\.github\/actions|scripts|synara-ios\/scripts)\/[A-Za-z0-9_./-]+\.(?:sh|mjs|js|toml)|\.github\/actions\/[A-Za-z0-9_-]+/g)) {
      if (!seen.has(reference)) {
        seen.add(reference);
        pending.push(reference);
      }
    }
  };
  visit(job);
  for (const relative of [
    "synara-ios/scripts/ci-build.sh",
    "synara-ios/scripts/lib/xcode-cache.sh",
    "synara-ios/scripts/lib/klipy-api-key.sh",
  ]) {
    pending.push(relative);
  }
  // Relative `./lib/...` imports inside the node checkers.
  const relativeImports = (file, text) =>
    [...text.matchAll(/from "(\.\/[^"]+)"/g)].map(([, spec]) =>
      path.posix.normalize(path.posix.join(path.posix.dirname(file), spec))
    );
  while (pending.length > 0) {
    const reference = pending.pop();
    // ci-build.sh and its siblings name some scripts relative to synara-ios.
    const file =
      existsSync(path.join(root, reference)) || !existsSync(path.join(root, "synara-ios", reference))
        ? reference
        : `synara-ios/${reference}`;
    if (file.startsWith("scripts/__tests__/")) {
      assert.ok(covered(file), `${file} is read by the job but is not a hash input`);
      continue;
    }
    if (!existsSync(path.join(root, file))) continue;
    assert.ok(covered(file), `${file} is reached by the iOS simulator job but is not a hash input`);
    const full = path.join(root, file);
    let text;
    try {
      text = readFileSync(full, "utf8");
    } catch {
      // A composite action directory: read its action.yml.
      text = readFileSync(path.join(full, "action.yml"), "utf8");
    }
    visit(text);
    for (const imported of relativeImports(file, text)) {
      if (!seen.has(imported)) {
        seen.add(imported);
        pending.push(imported);
      }
    }
  }
  assert.ok(seen.has("scripts/generate-synara-core-swift.sh"));
  assert.ok(seen.has("scripts/lib/ffi-surface.mjs"));
  assert.ok(seen.has(".github/actions/setup-xcode-cache"));
  assert.ok(seen.has("scripts/check-xcode-local-package-graph.mjs"));
});
