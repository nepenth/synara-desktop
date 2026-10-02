import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { runInNewContext } from "node:vm";

const root = path.resolve(import.meta.dirname, "../..");
const workflow = readFileSync(
  path.join(root, ".github/workflows/ci.yml"),
  "utf8"
);
const unitJob = workflow.slice(
  workflow.indexOf("  ios-tests:"),
  workflow.indexOf("  ios-ui-tests:")
);
const scalar = (name) => {
  const expression = unitJob.match(
    new RegExp(`^          ${name}: \\$\\{\\{ (.*) \\}\\}$`, "m")
  )?.[1];
  assert.ok(expression, `${name} uses an explicit workflow expression`);
  return expression;
};
// These shipping expressions use the boolean/string subset whose &&/||/==
// semantics match JavaScript. Evaluate the actual YAML scalars across events
// and typed dispatch inputs rather than copying their implementation.
function options(event, inputs = {}) {
  const evaluate = (name) =>
    runInNewContext(
      scalar(name),
      {
        github: { event_name: event },
        inputs,
      },
      { timeout: 100 }
    );
  return {
    targets: evaluate("targets"),
    slices: evaluate("SYNARA_CORE_APPLE_SLICES"),
    device: evaluate("CHECK_IOS_DEVICE_RELEASE"),
  };
}
const defaults = {
  targets: "aarch64-apple-ios-sim",
  slices: "simulator-arm64",
  device: "0",
};
const allTargets =
  "aarch64-apple-ios,aarch64-apple-ios-sim,x86_64-apple-ios,aarch64-apple-darwin";

test("manual Apple inputs retain economical typed defaults", () => {
  assert.match(
    workflow,
    /apple_slices:\n(?: {8}.*\n)*? {8}type: choice\n {8}options: \[simulator-arm64, all\]\n {8}default: simulator-arm64/
  );
  assert.match(
    workflow,
    /check_ios_device_release:\n(?: {8}.*\n)*? {8}type: boolean\n {8}default: false/
  );
  assert.deepEqual(
    options("workflow_dispatch", {
      apple_slices: "simulator-arm64",
      check_ios_device_release: false,
    }),
    defaults
  );
});
test("all-slice dispatch compiles every full-Core and NSE Apple slice without device Release by default", () => {
  assert.deepEqual(
    options("workflow_dispatch", {
      apple_slices: "all",
      check_ios_device_release: false,
    }),
    {
      targets: allTargets,
      slices: "all",
      device: "0",
    }
  );
});
test("device Release opt-in installs all required targets and uses the exact enabled flag", () => {
  for (const apple_slices of ["simulator-arm64", "all"]) {
    assert.deepEqual(
      options("workflow_dispatch", {
        apple_slices,
        check_ios_device_release: true,
      }),
      {
        targets: allTargets,
        slices: "all",
        device: "1",
      }
    );
  }
});
test("dispatch inputs cannot enable expensive Apple work on other events", () => {
  for (const event of ["pull_request", "push", "schedule"]) {
    assert.deepEqual(
      options(event, { apple_slices: "all", check_ios_device_release: true }),
      defaults
    );
  }
});
test("manual opt-ins remain confined to the unsigned unit lane and bounded by its budget", () => {
  assert.match(unitJob, /timeout-minutes: 120/);
  assert.match(unitJob, /RUN_IOS_TESTS: "1"\n {10}IOS_TEST_SUITE: unit/);
  for (const name of ["ios-ui-tests", "ios-compile"]) {
    const start = workflow.indexOf(`  ${name}:`);
    const job = workflow.slice(start).split(/\n  [a-z][a-z-]+:/, 1)[0];
    assert.doesNotMatch(
      job,
      /inputs\.apple_slices|inputs\.check_ios_device_release|CHECK_IOS_DEVICE_RELEASE:/
    );
    assert.match(job, /SYNARA_CORE_APPLE_SLICES: simulator-arm64/);
  }
});
