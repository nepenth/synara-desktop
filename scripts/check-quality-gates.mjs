import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const indentation = (line) => line.length - line.trimStart().length;
const unquote = (value) => value.trim().replace(/^(['"])(.*)\1$/, "$2");

function parseJobs(workflow) {
  const jobs = new Map();
  let inJobs = false;
  let currentJob;

  for (const line of workflow.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const indent = indentation(line);
    if (indent === 0) {
      if (trimmed === "jobs:") {
        inJobs = true;
        currentJob = undefined;
        continue;
      }
      if (inJobs) break;
      continue;
    }
    if (!inJobs) continue;

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

function getPropertyBlock(lines, property, indent) {
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (indentation(line) !== indent) continue;
    const match = line.trim().match(/^([A-Za-z0-9_-]+):(?:\s*(.*))?$/);
    if (!match || match[1] !== property) continue;

    const children = [];
    for (
      let childIndex = index + 1;
      childIndex < lines.length;
      childIndex += 1
    ) {
      const child = lines[childIndex];
      if (child.trim() && indentation(child) <= indent) break;
      children.push(child);
    }
    return { inline: match[2] ?? "", children };
  }
  return undefined;
}

const getScalar = (lines, property, indent) => {
  const inline = getPropertyBlock(lines, property, indent)?.inline;
  return inline === undefined ? undefined : unquote(inline);
};

function getList(lines, property, indent) {
  const block = getPropertyBlock(lines, property, indent);
  if (!block) return undefined;
  if (block.inline) {
    const inline = block.inline.trim();
    if (inline.startsWith("[") && inline.endsWith("]")) {
      const body = inline.slice(1, -1).trim();
      return body ? body.split(",").map((item) => unquote(item)) : [];
    }
    return [unquote(inline)];
  }

  const values = [];
  for (const line of block.children) {
    const match =
      indentation(line) === indent + 2 ? line.trim().match(/^-\s+(.+)$/) : null;
    if (match) values.push(unquote(match[1]));
  }
  return values;
}

function getNestedScalar(lines, property, childProperty, indent) {
  const block = getPropertyBlock(lines, property, indent);
  return block
    ? getScalar(block.children, childProperty, indent + 2)
    : undefined;
}

function parseSteps(jobLines) {
  const block = getPropertyBlock(jobLines, "steps", 4);
  if (!block) return [];

  const steps = [];
  let currentStep;
  for (const line of block.children) {
    const entry =
      indentation(line) === 6
        ? line.trim().match(/^-\s*(.*)$/)?.[1]
        : undefined;
    if (entry !== undefined) {
      currentStep = [];
      steps.push(currentStep);
      if (entry) currentStep.push(`${" ".repeat(8)}${entry}`);
      continue;
    }
    currentStep?.push(line);
  }
  return steps;
}

function getStepRun(step) {
  const block = getPropertyBlock(step, "run", 8);
  if (!block) return "";
  if (block.inline && !["|", ">", "|-", ">-"].includes(block.inline)) {
    return unquote(block.inline);
  }
  return block.children.map((line) => line.trim()).join("\n");
}

function getStepEnvironment(step) {
  const block = getPropertyBlock(step, "env", 8);
  const environment = new Map();
  if (!block) return environment;
  for (const line of block.children) {
    if (indentation(line) !== 10) continue;
    const match = line.trim().match(/^([A-Za-z_][A-Za-z0-9_]*):\s*(.+)$/);
    if (match) environment.set(match[1], unquote(match[2]));
  }
  return environment;
}

const sameList = (actual, expected) =>
  Array.isArray(actual) &&
  actual.length === expected.length &&
  [...actual]
    .sort()
    .every((value, index) => value === [...expected].sort()[index]);

const executableLines = (run) =>
  run
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));

function hasIosTestBuildStep(jobLines) {
  return parseSteps(jobLines).some((step) => {
    const runLines = executableLines(getStepRun(step));
    return (
      runLines.includes("scripts/ci-build.sh") &&
      getScalar(step, "working-directory", 8) === "synara-ios" &&
      getStepEnvironment(step).get("RUN_IOS_TESTS") === "1"
    );
  });
}

function aggregateGateError(jobLines, expectedName, expectedNeeds) {
  if (!jobLines) return "job is missing";
  if (getScalar(jobLines, "name", 4) !== expectedName) {
    return `job name must be ${expectedName}`;
  }
  if (getScalar(jobLines, "if", 4) !== "always()") {
    return "job must use if: always()";
  }
  if (!sameList(getList(jobLines, "needs", 4), expectedNeeds)) {
    return `job needs must be exactly [${expectedNeeds.join(", ")}]`;
  }

  for (const step of parseSteps(jobLines)) {
    const environment = getStepEnvironment(step);
    const variables = expectedNeeds.map((need) => {
      const expected = `\${{ needs.${need}.result }}`;
      return [...environment.entries()].find(
        ([, value]) => value === expected
      )?.[0];
    });
    if (variables.some((variable) => !variable)) continue;

    const run = executableLines(getStepRun(step))
      .join(" ")
      .replace(/\s+/g, " ");
    const condition = variables
      .map((variable) => `"$${variable}" != "success"`)
      .join(" || ");
    const conditionStart = run.indexOf(`if [[ ${condition} ]]; then`);
    if (conditionStart < 0) continue;
    const conditionEnd = run.indexOf(" fi", conditionStart);
    const failureBody =
      conditionEnd < 0 ? "" : run.slice(conditionStart, conditionEnd);
    if (/\bexit\s+1\b/.test(failureBody)) return undefined;
  }

  return "job must explicitly exit 1 unless every required job result is success";
}

export function inspectQualityGates({
  ciWorkflow,
  iosWorkflow,
  releaseWorkflow,
  releaseDocs = "",
}) {
  const errors = [];
  const ciJobs = parseJobs(ciWorkflow);
  const iosJobs = parseJobs(iosWorkflow);
  const releaseJobs = parseJobs(releaseWorkflow);

  if (!hasIosTestBuildStep(ciJobs.get("ios-tests"))) {
    errors.push(
      "CI iOS validation must invoke synara-ios/scripts/ci-build.sh with RUN_IOS_TESTS=1 in the same step."
    );
  }
  if (!hasIosTestBuildStep(iosJobs.get("test"))) {
    errors.push(
      "Manual iOS diagnostics must invoke synara-ios/scripts/ci-build.sh with RUN_IOS_TESTS=1 in the same step."
    );
  }
  if (!hasIosTestBuildStep(releaseJobs.get("exact-tag-ios-quality"))) {
    errors.push(
      "Exact-tag iOS validation must invoke synara-ios/scripts/ci-build.sh with RUN_IOS_TESTS=1 in the same step."
    );
  }

  const ciAggregateError = aggregateGateError(
    ciJobs.get("quality-gate"),
    "Quality gate",
    ["validate", "ios-tests"]
  );
  if (ciAggregateError) errors.push(`CI aggregate ${ciAggregateError}.`);

  for (const job of ["exact-tag-desktop-quality", "exact-tag-ios-quality"]) {
    if (
      !sameList(getList(releaseJobs.get(job) ?? [], "needs", 4), ["validate"])
    ) {
      errors.push(`Release workflow ${job} needs must be exactly [validate].`);
    }
  }

  const releaseAggregateError = aggregateGateError(
    releaseJobs.get("quality-gate"),
    "Exact-tag quality gate",
    ["validate", "exact-tag-desktop-quality", "exact-tag-ios-quality"]
  );
  if (releaseAggregateError) {
    errors.push(`Release aggregate ${releaseAggregateError}.`);
  }

  for (const job of ["linux-deb", "linux-arch", "macos", "ios-testflight"]) {
    if (
      !sameList(getList(releaseJobs.get(job) ?? [], "needs", 4), [
        "quality-gate",
      ])
    ) {
      errors.push(
        `Release artifact job ${job} needs must be exactly [quality-gate] at job scope.`
      );
    }
  }

  const publishJob = releaseJobs.get("publish-gh-release") ?? [];
  if (
    getNestedScalar(publishJob, "environment", "name", 4) !==
    "production-release"
  ) {
    errors.push(
      "Release publication must use the protected production-release environment."
    );
  }

  if (
    !/Do not configure[\s\S]{0,200}production-release[\s\S]{0,200}status checks/i.test(
      releaseDocs
    ) ||
    !/do\s+not\s+run on tag refs/i.test(releaseDocs) ||
    !/required human reviewers/i.test(releaseDocs) ||
    !/exact-tag[^\n]*jobs/i.test(releaseDocs)
  ) {
    errors.push(
      "Release documentation must forbid non-tag CI status checks on production-release and prescribe human reviewers plus exact-tag jobs."
    );
  }

  return { ok: errors.length === 0, errors };
}

function main() {
  const result = inspectQualityGates({
    ciWorkflow: readFileSync(
      path.join(root, ".github/workflows/ci.yml"),
      "utf8"
    ),
    iosWorkflow: readFileSync(
      path.join(root, ".github/workflows/ios-skeleton.yml"),
      "utf8"
    ),
    releaseWorkflow: readFileSync(
      path.join(root, ".github/workflows/release.yml"),
      "utf8"
    ),
    releaseDocs: readFileSync(
      path.join(root, "docs/build-and-release.md"),
      "utf8"
    ),
  });

  for (const error of result.errors) console.error(`[quality-gates] ${error}`);
  if (!result.ok) process.exit(1);
  console.log("[quality-gates] CI and exact-tag release gates are complete.");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
