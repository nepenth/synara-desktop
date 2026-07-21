import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const requirePattern = (errors, value, pattern, message) => {
  if (!pattern.test(value)) errors.push(message);
};

const jobBlock = (workflow, job) => {
  const start = workflow.search(new RegExp(`^  ${job}:`, "m"));
  if (start < 0) return "";
  const remainder = workflow.slice(start);
  const next = remainder.slice(1).search(/^  [a-zA-Z0-9_-]+:/m);
  return next < 0 ? remainder : remainder.slice(0, next + 1);
};

export function inspectQualityGates({
  ciWorkflow,
  iosWorkflow,
  releaseWorkflow,
}) {
  const errors = [];

  requirePattern(
    errors,
    ciWorkflow,
    /ios-tests:[\s\S]*RUN_IOS_TESTS:\s*["']1["']/,
    "CI must execute, not merely build, the iOS test bundles."
  );
  requirePattern(
    errors,
    ciWorkflow,
    /quality-gate:[\s\S]*name:\s*Quality gate[\s\S]*if:\s*always\(\)[\s\S]*needs:\s*\[validate, ios-tests\]/,
    "CI must publish the always-present aggregate Quality gate job."
  );
  requirePattern(
    errors,
    iosWorkflow,
    /workflow_dispatch:[\s\S]*RUN_IOS_TESTS:\s*["']1["']/,
    "The manual iOS diagnostics workflow must run the test suite."
  );

  for (const job of ["exact-tag-desktop-quality", "exact-tag-ios-quality"]) {
    requirePattern(
      errors,
      jobBlock(releaseWorkflow, job),
      /needs:\s*\[validate\]/,
      `Release workflow is missing ${job} at the tagged SHA.`
    );
  }
  requirePattern(
    errors,
    releaseWorkflow,
    /quality-gate:[\s\S]*name:\s*Exact-tag quality gate[\s\S]*needs:\s*\[validate, exact-tag-desktop-quality, exact-tag-ios-quality\]/,
    "Release workflow must aggregate exact-tag desktop and iOS validation."
  );

  for (const job of ["linux-deb", "linux-arch", "macos", "ios-testflight"]) {
    requirePattern(
      errors,
      jobBlock(releaseWorkflow, job),
      /needs:\s*\[quality-gate\]/,
      `Release artifact job ${job} must depend on the exact-tag quality gate.`
    );
  }
  requirePattern(
    errors,
    jobBlock(releaseWorkflow, "publish-gh-release"),
    /environment:\s*[\s\S]*name:\s*production-release/,
    "Release publication must use the protected production-release environment."
  );

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
