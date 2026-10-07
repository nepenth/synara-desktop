import assert from "node:assert/strict";
import test from "node:test";

import { inspectWorkflow, inspectWorkflows } from "../check-workflows.mjs";

const valid = `name: Example
on: push
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-22.04
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - uses: ./.github/actions/setup-node
      - name: Use a step-scoped secret
        env:
          TOKEN: \${{ secrets.TOKEN }}
        run: echo ok
`;

test("the repository workflows pass", () => {
  assert.deepEqual(inspectWorkflows(), []);
});

test("a valid workflow passes", () => {
  assert.deepEqual(inspectWorkflow("x.yml", valid), []);
});

test("rejects tag-pinned actions", () => {
  const errors = inspectWorkflow(
    "x.yml",
    valid.replace(/checkout@[0-9a-f]{40}/, "checkout@v7"),
  );
  assert.match(errors.join("\n"), /not pinned to a commit SHA/);
});

test("rejects missing permissions and timeouts", () => {
  const errors = inspectWorkflow(
    "x.yml",
    valid.replace("permissions:\n  contents: read\n", "").replace("    timeout-minutes: 5\n", ""),
  );
  assert.match(errors.join("\n"), /missing top-level permissions/);
  assert.match(errors.join("\n"), /job build has no timeout-minutes/);
});

test("rejects job-level and workflow-level secrets", () => {
  const jobLevel = valid.replace(
    "    timeout-minutes: 5\n",
    "    timeout-minutes: 5\n    env:\n      TOKEN: ${{ secrets.TOKEN }}\n",
  );
  assert.match(inspectWorkflow("x.yml", jobLevel).join("\n"), /secret referenced outside a step/);
  const workflowLevel = valid.replace("jobs:\n", "env:\n  TOKEN: ${{ secrets.TOKEN }}\njobs:\n");
  assert.match(inspectWorkflow("x.yml", workflowLevel).join("\n"), /secret referenced outside a step/);
});
