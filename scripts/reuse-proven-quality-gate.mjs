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

const main = () => {
  const sha = process.env.GITHUB_SHA;
  const repo = process.env.GITHUB_REPOSITORY;
  if (!sha || !repo) {
    throw new Error("GITHUB_SHA and GITHUB_REPOSITORY are required");
  }

  const parentsLine = runGit(["rev-list", "--parents", "-n", "1", sha]);
  const secondParent = secondParentOf(parentsLine, sha);
  const checkRunsBySha = {
    [sha]: fetchCheckRuns(repo, sha),
  };
  if (secondParent) {
    checkRunsBySha[secondParent] = fetchCheckRuns(repo, secondParent);
  }

  const decision = decideProvenQualityGateWithParents({
    sha,
    secondParent,
    checkRunsBySha,
  });

  writeOutput("reuse", decision.reuse ? "true" : "false");
  writeOutput("proven_sha", decision.provenSha || "");
  if (decision.reuse) {
    console.log(
      `Reusing proven ${QUALITY_GATE_CHECK_NAME} on ${decision.provenSha}.`
    );
    return;
  }
  console.log(
    `No proven ${QUALITY_GATE_CHECK_NAME} on ${sha}${
      secondParent ? ` or incoming parent ${secondParent}` : ""
    }; running exact-tag validation.`
  );
};

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
