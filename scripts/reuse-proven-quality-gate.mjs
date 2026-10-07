import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const QUALITY_GATE_CHECK_NAME = "Quality gate";

export const isSuccessfulQualityGate = (run) =>
  Boolean(
    run &&
      run.name === QUALITY_GATE_CHECK_NAME &&
      run.status === "completed" &&
      run.conclusion === "success"
  );

export const secondParentOf = (revListParentsLine, sha) => {
  const parts = String(revListParentsLine || "")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (parts[0] !== sha || parts.length < 3) {
    return null;
  }
  return parts[2];
};

// A Quality gate that has not finished yet on one of the candidate commits.
export const hasPendingQualityGate = (checkRuns) =>
  (checkRuns || []).some(
    (run) =>
      run &&
      run.name === QUALITY_GATE_CHECK_NAME &&
      run.status !== "completed"
  );

export const decideProvenQualityGate = ({ sha, checkRuns }) => {
  if ((checkRuns || []).some(isSuccessfulQualityGate)) {
    return { reuse: true, provenSha: sha };
  }
  return { reuse: false, provenSha: null };
};

export const decideProvenQualityGateWithParents = ({
  sha,
  secondParent,
  checkRunsBySha,
}) => {
  const head = decideProvenQualityGate({
    sha,
    secondParent,
    checkRuns: checkRunsBySha?.[sha] || [],
  });
  if (head.reuse) {
    return head;
  }
  if (!secondParent) {
    return head;
  }
  const incoming = decideProvenQualityGate({
    sha: secondParent,
    checkRuns: checkRunsBySha?.[secondParent] || [],
  });
  if (incoming.reuse) {
    return incoming;
  }
  return { reuse: false, provenSha: null };
};

const runGit = (args) =>
  execFileSync("git", args, { encoding: "utf8" }).trim();

const fetchCheckRuns = (repo, sha) => {
  const raw = execFileSync(
    "gh",
    ["api", `repos/${repo}/commits/${sha}/check-runs`, "--paginate"],
    { encoding: "utf8" }
  );
  const payloads = raw
    .trim()
    .split(/\n(?=\{)/)
    .map((chunk) => JSON.parse(chunk));
  return payloads.flatMap((payload) => payload.check_runs || []);
};

const writeOutput = (name, value) => {
  const destination = process.env.GITHUB_OUTPUT;
  const line = `${name}=${value}\n`;
  if (destination) {
    appendFileSync(destination, line);
    return;
  }
  process.stdout.write(line);
};

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Release publishes only commits CI already proved. With --require, a missing
// gate fails the release instead of re-running the suite at the tag; a gate
// still running on the tagged commit or the merged PR head is waited for.
const main = async () => {
  const sha = process.env.GITHUB_SHA;
  const repo = process.env.GITHUB_REPOSITORY;
  if (!sha || !repo) {
    throw new Error("GITHUB_SHA and GITHUB_REPOSITORY are required");
  }
  const strict = process.argv.includes("--require");
  const waitMinutes = Number(process.env.SYNARA_QUALITY_GATE_WAIT_MINUTES || 45);
  const deadline = Date.now() + waitMinutes * 60_000;

  const parentsLine = runGit(["rev-list", "--parents", "-n", "1", sha]);
  const secondParent = secondParentOf(parentsLine, sha);
  const candidates = [sha, ...(secondParent ? [secondParent] : [])];

  for (;;) {
    const checkRunsBySha = Object.fromEntries(
      candidates.map((candidate) => [candidate, fetchCheckRuns(repo, candidate)])
    );
    const decision = decideProvenQualityGateWithParents({
      sha,
      secondParent,
      checkRunsBySha,
    });
    if (decision.reuse) {
      writeOutput("reuse", "true");
      writeOutput("proven_sha", decision.provenSha);
      console.log(
        `Reusing proven ${QUALITY_GATE_CHECK_NAME} on ${decision.provenSha}.`
      );
      return;
    }
    const pending = candidates.some((candidate) =>
      hasPendingQualityGate(checkRunsBySha[candidate])
    );
    if (!strict || !pending || Date.now() > deadline) break;
    console.log(`${QUALITY_GATE_CHECK_NAME} still running; checking again in 60s.`);
    await sleep(60_000);
  }

  writeOutput("reuse", "false");
  writeOutput("proven_sha", "");
  const where = `${sha}${secondParent ? ` or incoming parent ${secondParent}` : ""}`;
  if (strict) {
    console.error(
      `No successful CI ${QUALITY_GATE_CHECK_NAME} on ${where}. ` +
        "Merge the release PR after its Quality gate passes, or re-run CI on main, then re-run this release."
    );
    process.exit(1);
  }
  console.log(`No proven ${QUALITY_GATE_CHECK_NAME} on ${where}.`);
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  await main();
}
