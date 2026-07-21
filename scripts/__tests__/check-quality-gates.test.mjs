import test from "node:test";
import assert from "node:assert/strict";

import { inspectQualityGates } from "../check-quality-gates.mjs";

const iosBuildStep = `
    steps:
      - name: Build and run unsigned simulator tests
        run: scripts/ci-build.sh
        working-directory: synara-ios
        env:
          RUN_IOS_TESTS: "1"
`;

const ciWorkflow = `
jobs:
  validate:
    runs-on: ubuntu-latest
  ios-tests:
    runs-on: macos-latest
${iosBuildStep}
  quality-gate:
    name: Quality gate
    if: always()
    needs: [validate, ios-tests]
    runs-on: ubuntu-latest
    steps:
      - name: Require every client validation job
        env:
          DESKTOP_RESULT: \${{ needs.validate.result }}
          IOS_RESULT: \${{ needs.ios-tests.result }}
        run: |
          if [[ "$DESKTOP_RESULT" != "success" || "$IOS_RESULT" != "success" ]]; then
            exit 1
          fi
`;

const iosWorkflow = `
on:
  workflow_dispatch:
jobs:
  test:
    runs-on: macos-latest
${iosBuildStep}
`;

const releaseWorkflow = `
jobs:
  validate:
    runs-on: ubuntu-latest
  exact-tag-desktop-quality:
    needs: [validate]
    runs-on: ubuntu-latest
  exact-tag-ios-quality:
    needs: [validate]
    runs-on: macos-latest
${iosBuildStep}
  quality-gate:
    name: Exact-tag quality gate
    if: always()
    needs: [validate, exact-tag-desktop-quality, exact-tag-ios-quality]
    runs-on: ubuntu-latest
    steps:
      - name: Require full validation at the tagged SHA
        env:
          TAG_RESULT: \${{ needs.validate.result }}
          DESKTOP_RESULT: \${{ needs.exact-tag-desktop-quality.result }}
          IOS_RESULT: \${{ needs.exact-tag-ios-quality.result }}
        run: |
          if [[ "$TAG_RESULT" != "success" || "$DESKTOP_RESULT" != "success" || "$IOS_RESULT" != "success" ]]; then
            exit 1
          fi
  linux-deb:
    needs: [quality-gate]
  linux-arch:
    needs: [quality-gate]
  macos:
    needs: [quality-gate]
  ios-testflight:
    needs: [quality-gate]
  publish-gh-release:
    environment:
      name: production-release
`;

const releaseDocs = `
Do not configure the production-release environment with status checks that do
not run on tag refs. Use required human reviewers and exact-tag validation jobs.
`;

const inspect = (overrides = {}) =>
  inspectQualityGates({
    ciWorkflow,
    iosWorkflow,
    releaseWorkflow,
    releaseDocs,
    ...overrides,
  });

test("accepts complete CI and exact-tag release gates", () => {
  assert.deepEqual(inspect(), { ok: true, errors: [] });
});

test("rejects an iOS build step with only a decoy script reference", () => {
  const result = inspect({
    ciWorkflow: ciWorkflow.replace(
      "run: scripts/ci-build.sh",
      "name: scripts/ci-build.sh decoy\n        run: echo build-only"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /CI iOS validation.*invoke/i);
});

test("rejects an iOS test flag detached from the build-script step", () => {
  const result = inspect({
    iosWorkflow: iosWorkflow
      .replace('RUN_IOS_TESTS: "1"', 'RUN_IOS_TESTS: "0"')
      .replace(
        "runs-on: macos-latest",
        'runs-on: macos-latest\n    env:\n      RUN_IOS_TESTS: "1"'
      ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /Manual iOS diagnostics.*same step/i);
});

test("rejects an exact-tag iOS job that does not invoke the test script", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "run: scripts/ci-build.sh",
      "run: echo build-only"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /Exact-tag iOS validation.*invoke/i);
});

test("rejects an always aggregate that never fails", () => {
  const result = inspect({
    ciWorkflow: ciWorkflow.replace(
      "            exit 1",
      "            echo ignored"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /explicitly exit 1/i);
});

test("rejects a no-op exact-tag release aggregate", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "            exit 1",
      "            true"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /Release aggregate.*explicitly exit 1/i
  );
});

test("rejects an aggregate without if always", () => {
  const result = inspect({
    ciWorkflow: ciWorkflow.replace("    if: always()", "    if: success()"),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /if: always/);
});

test("rejects aggregate needs that are not exact", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "needs: [validate, exact-tag-desktop-quality, exact-tag-ios-quality]",
      "needs: [validate, exact-tag-desktop-quality]"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /needs must be exactly/);
});

test("rejects an artifact dependency decoy outside job scope", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "  macos:\n    needs: [quality-gate]",
      '  macos:\n    needs: [validate]\n    steps:\n      - run: echo "needs: [quality-gate]"'
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /artifact job macos.*job scope/i);
});

test("rejects exact-tag validation that depends on extra jobs", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "  exact-tag-desktop-quality:\n    needs: [validate]",
      "  exact-tag-desktop-quality:\n    needs: [validate, quality-gate]"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /desktop-quality needs must be exactly/
  );
});

test("rejects release documentation that recommends unavailable CI checks", () => {
  const result = inspect({
    releaseDocs: "Require ordinary CI on production-release.",
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /documentation.*forbid/i);
});
