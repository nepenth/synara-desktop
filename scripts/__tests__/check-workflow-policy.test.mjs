import assert from "node:assert/strict";
import test from "node:test";

import {
  inspectWorkflowPolicy,
  loadWorkflowPolicyInputs,
} from "../check-workflow-policy.mjs";

const valid = loadWorkflowPolicyInputs();

const inspect = (workflowName, transform) =>
  inspectWorkflowPolicy({
    ...valid,
    workflows: {
      ...valid.workflows,
      [workflowName]: transform(valid.workflows[workflowName]),
    },
  });

test("accepts the repository workflow policy", () => {
  assert.deepEqual(inspectWorkflowPolicy(valid), { ok: true, errors: [] });
});

test("rejects mutable action references", () => {
  const result = inspect("ci.yml", (workflow) =>
    workflow.replace(/actions\/checkout@[0-9a-f]{40}/, "actions/checkout@main")
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /full commit SHA/);
});

test("rejects missing least-privilege permissions and timeouts", () => {
  const noPermissions = inspect("ios-skeleton.yml", (workflow) =>
    workflow.replace("permissions:\n  contents: read\n\n", "")
  );
  assert.match(noPermissions.errors.join("\n"), /contents: read/);

  const noTimeout = inspect("ios-skeleton.yml", (workflow) =>
    workflow.replace("    timeout-minutes: 45\n", "")
  );
  assert.match(noTimeout.errors.join("\n"), /must have a 1-120 minute timeout/);
});

test("rejects secrets exposed to an entire job", () => {
  const result = inspect("macos-signed-build.yml", (workflow) =>
    workflow.replace(
      "    steps:\n",
      "    env:\n      LEAKED: ${{ secrets.APPLE_ID }}\n    steps:\n"
    )
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /scope secrets/);
});

test("rejects an unstable or skippable package gate", () => {
  const result = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace(
      '  pull_request:\n    branches: [main, "release/**"]',
      '  pull_request:\n    branches: [main, "release/**"]\n    paths: ["src-tauri/**"]'
    )
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /stable aggregate check/);
});

test("rejects per-tag production release concurrency", () => {
  const result = inspect("release.yml", (workflow) =>
    workflow.replace("group: production-release", "group: ${{ github.ref }}")
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /serialized concurrency lane/);
});

test("rejects ungrouped dependency update fan-out", () => {
  const result = inspectWorkflowPolicy({
    ...valid,
    dependabot: valid.dependabot.replace("      github-actions-updates:\n", ""),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /grouped github-actions-updates/);
});
