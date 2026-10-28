import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import path from "node:path";
import test from "node:test";

import { IOS_TESTS_JOB_NAME, proofStepName } from "../ios-test-inputs.mjs";
import {
  CI_WORKFLOW_PATH,
  decideProvenIosTests,
  isTrustedCandidateRun,
  jobProvesInputs,
} from "../reuse-proven-ios-tests.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const repo = "nepenth/synara-desktop";
const headBranch = "release/v2.6.0";
const hash = "a".repeat(64);
const otherHash = "b".repeat(64);

const run = (id, overrides = {}) => ({
  id,
  path: CI_WORKFLOW_PATH,
  head_branch: headBranch,
  repository: { full_name: repo },
  head_repository: { full_name: repo },
  html_url: `https://github.com/${repo}/actions/runs/${id}`,
  ...overrides,
});
const provenJob = (proofHash = hash, overrides = {}) => ({
  name: IOS_TESTS_JOB_NAME,
  status: "completed",
  conclusion: "success",
  steps: [
    { name: "Build and run unsigned simulator tests", status: "completed", conclusion: "success" },
    { name: proofStepName(proofHash), status: "completed", conclusion: "success" },
  ],
  ...overrides,
});
const decide = (runs, jobsByRunId, overrides = {}) =>
  decideProvenIosTests({ hash, repo, headBranch, currentRunId: 99, runs, jobsByRunId, ...overrides });

test("an earlier run of the same PR that proved the same hash is reused", () => {
  assert.deepEqual(decide([run(2), run(1)], { 1: [provenJob()] }), {
    reuse: true,
    provenRunId: 1,
    provenRunUrl: `https://github.com/${repo}/actions/runs/1`,
  });
});

test("a different hash, or no proof step, runs the job", () => {
  assert.equal(decide([run(1)], { 1: [provenJob(otherHash)] }).reuse, false);
  assert.equal(decide([run(1)], { 1: [provenJob(hash, { steps: [] })] }).reuse, false);
  assert.equal(decide([run(1)], {}).reuse, false);
  assert.equal(decide([], {}).reuse, false);
});

test("only a completed, successful job and proof step count", () => {
  for (const job of [
    provenJob(hash, { conclusion: "failure" }),
    provenJob(hash, { conclusion: "cancelled" }),
    provenJob(hash, { status: "in_progress", conclusion: null }),
    provenJob(hash, { name: "iOS simulator UI tests" }),
    provenJob(hash, { name: "iOS compile gate" }),
    provenJob(hash, {
      steps: [{ name: proofStepName(hash), status: "completed", conclusion: "skipped" }],
    }),
    provenJob(hash, {
      steps: [{ name: proofStepName(hash), status: "in_progress", conclusion: null }],
    }),
  ]) {
    assert.equal(jobProvesInputs(job, hash), false, JSON.stringify(job));
  }
  assert.equal(jobProvesInputs(provenJob(), hash), true);
});

test("an earlier attempt's proof counts even if a later attempt failed", () => {
  const failedRerun = provenJob(hash, { conclusion: "failure" });
  assert.equal(decide([run(1)], { 1: [failedRerun, provenJob()] }).reuse, true);
});

test("forks, other branches, other workflows and this run are never trusted", () => {
  const context = { repo, headBranch, currentRunId: 99 };
  for (const candidate of [
    run(1, { head_repository: { full_name: "someone/synara-desktop" } }),
    run(1, { head_repository: null }),
    run(1, { repository: { full_name: "someone/synara-desktop" } }),
    run(1, { head_branch: "main" }),
    run(1, { path: ".github/workflows/release.yml" }),
    run(99),
  ]) {
    assert.equal(isTrustedCandidateRun(candidate, context), false, JSON.stringify(candidate));
    assert.equal(decide([candidate], { [candidate.id]: [provenJob()] }).reuse, false);
  }
  assert.equal(isTrustedCandidateRun(run(1), context), true);
  assert.equal(isTrustedCandidateRun(run(1), { ...context, headBranch: "" }), false);
});

test("a missing or malformed hash, or a missing repository, never reuses", () => {
  for (const overrides of [{ hash: "" }, { hash: "abc" }, { hash: hash.toUpperCase() }, { repo: "" }]) {
    assert.equal(decide([run(1)], { 1: [provenJob()] }, overrides).reuse, false);
  }
});

test("the CLI fails closed without a hash or a head branch", () => {
  for (const [args, env] of [
    [[], { GITHUB_REPOSITORY: repo, GITHUB_HEAD_REF: headBranch }],
    [[hash], { GITHUB_REPOSITORY: repo, GITHUB_HEAD_REF: "" }],
    [[hash], { GITHUB_REPOSITORY: "", GITHUB_HEAD_REF: headBranch }],
  ]) {
    assert.throws(
      () =>
        execFileSync(process.execPath, [path.join(root, "scripts/reuse-proven-ios-tests.mjs"), ...args], {
          env: { ...process.env, ...env, PATH: "" },
          stdio: "pipe",
        }),
      (error) => error.status === 1
    );
  }
});

test("the CLI fails closed when the GitHub API is unreachable", () => {
  assert.throws(
    () =>
      execFileSync(process.execPath, [path.join(root, "scripts/reuse-proven-ios-tests.mjs"), hash], {
        // No gh on PATH: the lookup throws and the job runs.
        env: { ...process.env, GITHUB_REPOSITORY: repo, GITHUB_HEAD_REF: headBranch, PATH: "" },
        stdio: "pipe",
      }),
    (error) => error.status === 1
  );
});
