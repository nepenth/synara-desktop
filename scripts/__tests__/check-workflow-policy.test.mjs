import assert from "node:assert/strict";
import test from "node:test";

import {
  inspectWorkflowPolicy,
  loadWorkflowPolicyInputs,
} from "../check-workflow-policy.mjs";

const valid = loadWorkflowPolicyInputs();
const integrationBranch = "feature/matrix-rust-sdk-full-replacement";
const validationWorkflows = ["ci.yml", "desktop-package-smoke.yml"];

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
    workflow.replace("    timeout-minutes: 60\n", "")
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

test("requires task PR validation on the Matrix Rust integration branch", () => {
  for (const workflowName of validationWorkflows) {
    const result = inspect(workflowName, (workflow) =>
      workflow.replace(`, "${integrationBranch}"`, "")
    );
    assert.equal(result.ok, false, workflowName);
    assert.match(
      result.errors.join("\n"),
      /must validate pull requests targeting/,
      workflowName
    );
  }
});

test("requires event- and pull-request-specific cancellable validation lanes", () => {
  for (const workflowName of validationWorkflows) {
    const noEvent = inspect(workflowName, (workflow) =>
      workflow.replace("-${{ github.event_name }}", "")
    );
    assert.match(
      noEvent.errors.join("\n"),
      /must separate workflow event types/,
      workflowName
    );

    const noPullRequest = inspect(workflowName, (workflow) =>
      workflow.replace(
        "${{ github.event.pull_request.number || github.ref }}",
        "${{ github.ref }}"
      )
    );
    assert.match(
      noPullRequest.errors.join("\n"),
      /must isolate each pull request/,
      workflowName
    );

    const noCancellation = inspect(workflowName, (workflow) =>
      workflow.replace("cancel-in-progress: true", "cancel-in-progress: false")
    );
    assert.match(
      noCancellation.errors.join("\n"),
      /must cancel obsolete runs only within the same event/,
      workflowName
    );
  }
});

test("rejects an unstable or skippable package gate", () => {
  const result = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace(
      `  pull_request:\n    branches: [main, "${integrationBranch}", "release/**"]`,
      `  pull_request:\n    branches: [main, "${integrationBranch}", "release/**"]\n    paths: ["src-tauri/**"]`
    )
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /stable aggregate check/);
});

test("rejects weakened package change detection", () => {
  const missingPath = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace("            src-tauri \\\n", "")
  );
  assert.match(missingPath.errors.join("\n"), /must retain the src-tauri path/);

  const noDiff = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace(
      'git diff --quiet "$BASE_SHA" "$HEAD_SHA" --',
      "git diff --quiet --"
    )
  );
  assert.match(noDiff.errors.join("\n"), /PR diff-based package change/);
});

test("requires strict desktop-shell and shared-workspace Rust gates in CI", () => {
  for (const [command, expected] of [
    ["cargo fmt --check", /strict Rust formatting/],
    ["cargo clippy --locked --all-targets -- -D warnings", /strict Rust lint/],
    ["cargo fmt --all -- --check", /shared workspace formatting/],
    [
      "cargo clippy --locked --workspace --all-targets -- -D warnings",
      /shared workspace lint/,
    ],
    ["cargo check --locked --workspace", /shared workspace check/],
    ["cargo test --locked --workspace", /shared workspace tests/],
  ]) {
    const result = inspect("ci.yml", (workflow) =>
      workflow.replace(command, "cargo --version")
    );
    assert.equal(result.ok, false, command);
    assert.match(result.errors.join("\n"), expected, command);
  }
});

test("requires exact-tag shared Rust workspace validation", () => {
  const result = inspect("release.yml", (workflow) =>
    workflow.replace(
      "          cargo test --locked --workspace",
      "          cargo --version"
    )
  );
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /Exact-tag desktop quality must validate the shared Rust workspace/
  );
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

test("requires immutable version guards before validation and publication", () => {
  const missingValidationGuard = inspect("release.yml", (workflow) =>
    workflow.replace(
      "      - name: Require a fresh incremented release version\n        env:\n          GH_TOKEN: ${{ github.token }}\n        run: node scripts/assert-release-version.mjs\n\n",
      ""
    )
  );
  assert.match(
    missingValidationGuard.errors.join("\n"),
    /immutable release-version guard/
  );

  const misplacedPublishGuard = inspect("release.yml", (workflow) =>
    workflow
      .replace(
        "      - name: Recheck immutable release version before publication\n        run: node scripts/assert-release-version.mjs\n\n",
        ""
      )
      .replace(
        "      - name: Create fixed pacman repository release\n",
        "      - name: Recheck immutable release version before publication\n        run: node scripts/assert-release-version.mjs\n\n      - name: Create fixed pacman repository release\n"
      )
  );
  assert.match(
    misplacedPublishGuard.errors.join("\n"),
    /must run immediately before/
  );
});

test("rejects a retained alternate semantic-release publisher", () => {
  const result = inspectWorkflowPolicy({
    ...valid,
    runtimePackage: `${valid.runtimePackage}\n@semantic-release/github`,
  });
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /alternate semantic-release publisher/
  );
});
