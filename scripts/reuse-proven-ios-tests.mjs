import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

import {
  IOS_INPUTS_HASH_PATTERN,
  IOS_TESTS_JOB_NAME,
  proofStepName,
} from "./ios-test-inputs.mjs";

// Release PRs force the iOS simulator unit lane on every push. A run of the
// same pull request that already passed it on the same input hash
// (scripts/ios-test-inputs.mjs) proves it again, so the job is skipped.
//
// Exit 0 means reuse; every other outcome, including an API error, exits
// non-zero so the caller runs the job. Only runs of ci.yml on this repository
// (never a fork) for the same head branch count, and only a successful
// "iOS simulator tests" job whose own proof step, named with the hash,
// recomputed that hash on its checkout and succeeded.

export const CI_WORKFLOW_PATH = ".github/workflows/ci.yml";
export const MAX_CANDIDATE_RUNS = 30;

export const isTrustedCandidateRun = (run, { repo, headBranch, currentRunId }) =>
  Boolean(
    run &&
      String(run.id) !== String(currentRunId) &&
      run.path === CI_WORKFLOW_PATH &&
      run.repository?.full_name === repo &&
      run.head_repository?.full_name === repo &&
      typeof headBranch === "string" &&
      headBranch !== "" &&
      run.head_branch === headBranch
  );

export const jobProvesInputs = (job, hash) =>
  Boolean(
    job &&
      job.name === IOS_TESTS_JOB_NAME &&
      job.status === "completed" &&
      job.conclusion === "success" &&
      (job.steps || []).some(
        (step) =>
          step.name === proofStepName(hash) &&
          step.status === "completed" &&
          step.conclusion === "success"
      )
  );

// runs: newest first, as the API lists them. jobsByRunId: every attempt's jobs.
export const decideProvenIosTests = ({
  hash,
  repo,
  headBranch,
  currentRunId,
  runs,
  jobsByRunId,
}) => {
  if (!IOS_INPUTS_HASH_PATTERN.test(hash || "") || !repo) {
    return { reuse: false, provenRunId: null };
  }
  for (const run of runs || []) {
    if (!isTrustedCandidateRun(run, { repo, headBranch, currentRunId })) continue;
    if ((jobsByRunId?.[run.id] || []).some((job) => jobProvesInputs(job, hash))) {
      return { reuse: true, provenRunId: run.id, provenRunUrl: run.html_url || null };
    }
  }
  return { reuse: false, provenRunId: null };
};

const ghJson = (path) =>
  execFileSync("gh", ["api", "-X", "GET", path], { encoding: "utf8" });

const parsePages = (raw) =>
  raw
    .trim()
    .split(/\n(?=\{)/)
    .filter(Boolean)
    .map((chunk) => JSON.parse(chunk));

const main = () => {
  const [hash] = process.argv.slice(2);
  const repo = process.env.GITHUB_REPOSITORY;
  const headBranch = process.env.GITHUB_HEAD_REF;
  const currentRunId = process.env.GITHUB_RUN_ID;
  if (!IOS_INPUTS_HASH_PATTERN.test(hash || "") || !repo || !headBranch) {
    console.log("iOS proof lookup: missing hash, repository or head branch; running the job.");
    process.exit(1);
  }

  const query = new URLSearchParams({
    branch: headBranch,
    per_page: String(MAX_CANDIDATE_RUNS),
  });
  const runs =
    JSON.parse(ghJson(`repos/${repo}/actions/workflows/ci.yml/runs?${query}`))
      .workflow_runs || [];
  const jobsByRunId = {};
  for (const run of runs) {
    if (!isTrustedCandidateRun(run, { repo, headBranch, currentRunId })) continue;
    const raw = execFileSync(
      "gh",
      ["api", "-X", "GET", "--paginate", `repos/${repo}/actions/runs/${run.id}/jobs?filter=all&per_page=100`],
      { encoding: "utf8" }
    );
    jobsByRunId[run.id] = parsePages(raw).flatMap((page) => page.jobs || []);
    const decision = decideProvenIosTests({
      hash,
      repo,
      headBranch,
      currentRunId,
      runs: [run],
      jobsByRunId,
    });
    if (decision.reuse) {
      console.log(
        `Reusing ${IOS_TESTS_JOB_NAME} proven on inputs ${hash} by run ${decision.provenRunId}` +
          (decision.provenRunUrl ? ` (${decision.provenRunUrl})` : "") +
          "."
      );
      process.exit(0);
    }
  }
  console.log(`No earlier run of ${headBranch} proved ${IOS_TESTS_JOB_NAME} on inputs ${hash}.`);
  process.exit(1);
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.log(`iOS proof lookup failed (${error.message}); running the job.`);
    process.exit(1);
  }
}
