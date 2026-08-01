import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const indentation = (line) => line.length - line.trimStart().length;
const integrationBranch = "feature/matrix-rust-sdk-full-replacement";
const cancellableValidationWorkflows = [
  "ci.yml",
  "desktop-package-smoke.yml",
];

function parseJobs(workflow) {
  const jobs = new Map();
  let inJobs = false;
  let currentJob;

  for (const line of workflow.split(/\r?\n/)) {
    const trimmed = line.trim();
    const indent = indentation(line);
    if (indent === 0) {
      if (trimmed === "jobs:") {
        inJobs = true;
        continue;
      }
      if (inJobs && trimmed) break;
      continue;
    }
    if (!inJobs || !trimmed || trimmed.startsWith("#")) continue;

    const jobMatch = indent === 2 ? trimmed.match(/^([A-Za-z0-9_-]+):$/) : null;
    if (jobMatch) {
      currentJob = [];
      jobs.set(jobMatch[1], currentJob);
      continue;
    }
    currentJob?.push(line);
  }

  return jobs;
}

function topLevelBlock(workflow, property) {
  const lines = workflow.split(/\r?\n/);
  const start = lines.findIndex(
    (line) => indentation(line) === 0 && line.trim() === `${property}:`
  );
  if (start < 0) return [];

  const block = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && indentation(line) === 0) break;
    block.push(line);
  }
  return block;
}

const jobScalar = (lines, property) => {
  const prefix = `${property}:`;
  const line = lines.find(
    (candidate) =>
      indentation(candidate) === 4 && candidate.trim().startsWith(prefix)
  );
  return line?.trim().slice(prefix.length).trim();
};

function hasJobScopedSecret(lines) {
  const envIndex = lines.findIndex(
    (line) => indentation(line) === 4 && line.trim() === "env:"
  );
  if (envIndex < 0) return false;

  for (let index = envIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && indentation(line) <= 4) break;
    if (line.includes("${{ secrets.")) return true;
  }
  return false;
}

function pullRequestBlock(workflow) {
  return (
    workflow.match(
      /^  pull_request:\s*\n([\s\S]*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|^[A-Za-z_][A-Za-z0-9_-]*:)/m
    )?.[1] ?? ""
  );
}

function hasIntegrationPullRequestTarget(workflow) {
  return pullRequestBlock(workflow).includes(`"${integrationBranch}"`);
}

export function inspectWorkflowPolicy({ workflows, dependabot }) {
  const errors = [];

  for (const [filename, workflow] of Object.entries(workflows).sort()) {
    const permissions = topLevelBlock(workflow, "permissions");
    if (!permissions.some((line) => /^  contents:\s*read\s*$/.test(line))) {
      errors.push(
        `${filename} must declare top-level contents: read permissions.`
      );
    }
    if (topLevelBlock(workflow, "concurrency").length === 0) {
      errors.push(`${filename} must declare workflow-level concurrency.`);
    }

    if (cancellableValidationWorkflows.includes(filename)) {
      if (!hasIntegrationPullRequestTarget(workflow)) {
        errors.push(
          `${filename} must validate pull requests targeting ${integrationBranch}.`
        );
      }

      const concurrency = topLevelBlock(workflow, "concurrency")
        .map((line) => line.trim())
        .filter(Boolean);
      const group = concurrency.find((line) => line.startsWith("group:")) ?? "";
      if (!group.includes("github.event_name")) {
        errors.push(
          `${filename} concurrency must separate workflow event types.`
        );
      }
      if (
        !group.includes("github.event.pull_request.number") ||
        !group.includes("github.ref")
      ) {
        errors.push(
          `${filename} concurrency must isolate each pull request and retain a ref fallback.`
        );
      }
      if (!concurrency.includes("cancel-in-progress: true")) {
        errors.push(
          `${filename} must cancel obsolete runs only within the same event and pull-request/ref lane.`
        );
      }
    }

    for (const match of workflow.matchAll(/^\s*uses:\s*([^\s#]+)/gm)) {
      const reference = match[1];
      if (reference.startsWith("./")) continue;
      if (!/^[^@\s]+@[0-9a-f]{40}$/.test(reference)) {
        errors.push(
          `${filename} action ${reference} must use a full commit SHA.`
        );
      }
    }

    if (/runs-on:\s*macos-latest/.test(workflow)) {
      errors.push(`${filename} must pin macOS runner generations.`);
    }

    for (const [jobName, jobLines] of parseJobs(workflow)) {
      const timeout = Number(jobScalar(jobLines, "timeout-minutes"));
      if (!Number.isInteger(timeout) || timeout < 1 || timeout > 120) {
        errors.push(
          `${filename} job ${jobName} must have a 1-120 minute timeout.`
        );
      }
      if (hasJobScopedSecret(jobLines)) {
        errors.push(
          `${filename} job ${jobName} must scope secrets to only the steps that consume them.`
        );
      }
    }
  }

  const packageWorkflow = workflows["desktop-package-smoke.yml"] ?? "";
  const packageJobs = parseJobs(packageWorkflow);
  const packageChanges = packageJobs.get("changes") ?? [];
  const packageGate = packageJobs.get("package-gate") ?? [];
  if (/^\s{4}paths:/m.test(pullRequestBlock(packageWorkflow))) {
    errors.push(
      "Desktop package smoke must emit its stable aggregate check for every pull request."
    );
  }
  if (
    jobScalar(packageGate, "name") !== "Desktop package gate" ||
    jobScalar(packageGate, "if") !== "always()" ||
    !packageGate
      .join("\n")
      .includes("needs: [changes, linux-deb, linux-arch, macos-app]")
  ) {
    errors.push(
      "Desktop package smoke must retain an always-running aggregate package gate."
    );
  }
  const packageChangeContract = packageChanges.join("\n");
  for (const pathRoot of ["scripts", "packaging/arch", "src-tauri", "synara"]) {
    if (!packageChangeContract.includes(pathRoot)) {
      errors.push(
        `Desktop package change detection must retain the ${pathRoot} path.`
      );
    }
  }
  if (
    !packageChangeContract.includes(
      'git diff --quiet "$BASE_SHA" "$HEAD_SHA" --'
    ) ||
    !packageChangeContract.includes('echo "packages=false"') ||
    !packageChangeContract.includes('echo "packages=true"')
  ) {
    errors.push(
      "Desktop package smoke must retain PR diff-based package change detection."
    );
  }

  // Rust gates may live in the monolithic `validate` job or the split
  // `validate-rust` job (parallel with `validate-frontend` for wall-clock speed).
  const ciJobs = parseJobs(workflows["ci.yml"] ?? "");
  const ciValidateRust = [
    ...(ciJobs.get("validate") ?? []),
    ...(ciJobs.get("validate-rust") ?? []),
  ];
  const ciValidationContract = ciValidateRust.join("\n");
  for (const [label, command] of [
    ["formatting", "cargo fmt --check"],
    ["lint", "cargo clippy --locked --all-targets -- -D warnings"],
  ]) {
    if (!ciValidationContract.includes(command)) {
      errors.push(`CI must retain strict Rust ${label}: ${command}.`);
    }
  }

  const releaseConcurrency = topLevelBlock(
    workflows["release.yml"] ?? "",
    "concurrency"
  )
    .map((line) => line.trim())
    .filter(Boolean);
  if (
    !releaseConcurrency.includes("group: production-release") ||
    !releaseConcurrency.includes("cancel-in-progress: false")
  ) {
    errors.push(
      "Production release tags must share a non-cancelling serialized concurrency lane."
    );
  }

  const signedBuild = parseJobs(workflows["macos-signed-build.yml"] ?? "").get(
    "macos-signed-build"
  );
  if (
    !signedBuild
      ?.join("\n")
      .match(/environment:\s*\n\s+name:\s*production-release/)
  ) {
    errors.push(
      "Manual macOS signing must use the protected production-release environment."
    );
  }

  for (const group of [
    "npm-updates",
    "github-actions-updates",
    "rust-updates",
  ]) {
    const groupedLane = new RegExp(
      `^ {6}${group}:\\n(?:(?: {8,}.*)?\\n)*? {8}patterns: \\["\\*"\\]$`,
      "m"
    );
    if (!groupedLane.test(dependabot)) {
      errors.push(`Dependabot must retain the grouped ${group} update lane.`);
    }
  }

  return { ok: errors.length === 0, errors };
}

export function loadWorkflowPolicyInputs(repositoryRoot = root) {
  const workflowDirectory = path.join(repositoryRoot, ".github", "workflows");
  const workflows = Object.fromEntries(
    readdirSync(workflowDirectory)
      .filter((filename) => /\.ya?ml$/.test(filename))
      .sort()
      .map((filename) => [
        filename,
        readFileSync(path.join(workflowDirectory, filename), "utf8"),
      ])
  );
  return {
    workflows,
    dependabot: readFileSync(
      path.join(repositoryRoot, ".github", "dependabot.yml"),
      "utf8"
    ),
  };
}

function main() {
  const result = inspectWorkflowPolicy(loadWorkflowPolicyInputs());
  for (const error of result.errors)
    console.error(`[workflow-policy] ${error}`);
  if (!result.ok) process.exit(1);
  console.log(
    "[workflow-policy] permissions, timeouts, action pins, integration CI, concurrency, Rust quality, release secret scope, and package gates are valid."
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
