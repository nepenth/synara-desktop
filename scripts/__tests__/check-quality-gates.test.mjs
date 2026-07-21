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
    steps:
      - run: npx playwright install --with-deps chromium
        working-directory: synara
      - run: npm run test:browser:timeline
        working-directory: synara
      - run: npm run check:security
        working-directory: synara
  ios-tests:
    runs-on: macos-latest
${iosBuildStep}
  synapse-integration:
    name: Synapse two-client integration
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - run: npm ci
        working-directory: synara
      - run: scripts/synapse-integration.sh up
      - run: npm run test:synapse-integration
        env:
          SYNARA_RECEIPT_MODE: both
      - if: always()
        run: scripts/synapse-integration.sh reset
  quality-gate:
    name: Quality gate
    if: always()
    needs: [validate, ios-tests, synapse-integration]
    runs-on: ubuntu-latest
    steps:
      - name: Require every client validation job
        env:
          DESKTOP_RESULT: \${{ needs.validate.result }}
          IOS_RESULT: \${{ needs.ios-tests.result }}
          SYNAPSE_RESULT: \${{ needs.synapse-integration.result }}
        run: |
          if [[ "$DESKTOP_RESULT" != "success" || "$IOS_RESULT" != "success" || "$SYNAPSE_RESULT" != "success" ]]; then
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
    steps:
      - run: npx playwright install --with-deps chromium
        working-directory: synara
      - run: |
          npm run check:repo-layout
          npm run check:versions
          npm run check:matrix-boundaries
          npm run check:quality-gates
          npm run check:synapse-harness
          npm run check:production-smoke
          node --test scripts/__tests__/*.test.mjs
      - run: |
          cargo check --locked
          cargo test --locked
        working-directory: src-tauri
      - run: |
          npm run typecheck:modernization
          npm run test:modernization
          npm run test:browser:timeline
          npm run check:eslint
          npm run check:prettier
          npm run check:security
        working-directory: synara
  exact-tag-ios-quality:
    needs: [validate]
    runs-on: macos-latest
${iosBuildStep}
  exact-tag-synapse-integration:
    name: Exact-tag Synapse two-client integration
    needs: [validate]
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - run: npm ci
        working-directory: synara
      - run: scripts/synapse-integration.sh up
      - run: npm run test:synapse-integration
        env:
          SYNARA_RECEIPT_MODE: both
      - if: always()
        run: scripts/synapse-integration.sh reset
  quality-gate:
    name: Exact-tag quality gate
    if: always()
    needs: [validate, exact-tag-desktop-quality, exact-tag-ios-quality, exact-tag-synapse-integration]
    runs-on: ubuntu-latest
    steps:
      - name: Require full validation at the tagged SHA
        env:
          TAG_RESULT: \${{ needs.validate.result }}
          DESKTOP_RESULT: \${{ needs.exact-tag-desktop-quality.result }}
          IOS_RESULT: \${{ needs.exact-tag-ios-quality.result }}
          SYNAPSE_RESULT: \${{ needs.exact-tag-synapse-integration.result }}
        run: |
          if [[ "$TAG_RESULT" != "success" || "$DESKTOP_RESULT" != "success" || "$IOS_RESULT" != "success" || "$SYNAPSE_RESULT" != "success" ]]; then
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
    steps:
      - run: synara-ios/scripts/upload-testflight-internal.sh
        env:
          SYNARA_TESTFLIGHT_INTERNAL_ONLY: \${{ vars.SYNARA_TESTFLIGHT_INTERNAL_ONLY || 'true' }}
  updater-metadata:
    needs: [macos]
  publish-gh-release:
    needs: [linux-deb, linux-arch, macos, updater-metadata, ios-testflight]
    environment:
      name: production-release
`;

const releaseDocs = `
Do not configure the production-release environment with status checks that do
not run on tag refs. Use required human reviewers and exact-tag validation jobs.
`;

const rootPackage = JSON.stringify({
  scripts: {
    "test:synapse-integration":
      "SYNARA_RUN_SYNAPSE_INTEGRATION=1 npm --prefix synara exec -- vite-node --config synara/scripts/vite-node.integration.config.mjs synara/scripts/run-synapse-two-client-integration.mjs",
  },
});

const inspect = (overrides = {}) =>
  inspectQualityGates({
    ciWorkflow,
    iosWorkflow,
    releaseWorkflow,
    releaseDocs,
    rootPackage,
    ...overrides,
  });

test("accepts complete CI and exact-tag release gates", () => {
  assert.deepEqual(inspect(), { ok: true, errors: [] });
});

test("rejects missing real-layout timeline execution", () => {
  for (const [override, workflow] of [
    ["ciWorkflow", ciWorkflow],
    ["releaseWorkflow", releaseWorkflow],
  ]) {
    for (const command of [
      "npx playwright install --with-deps chromium",
      "npm run test:browser:timeline",
      "npm run check:security",
    ]) {
      const result = inspect({
        [override]: workflow.replace(command, `echo ${command}`),
      });
      assert.equal(result.ok, false, `${override}: ${command}`);
      assert.match(
        result.errors.join("\n"),
        /desktop validation.*must execute/i
      );
    }
  }
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

test("rejects quoted, echoed, and short-circuited aggregate failures", () => {
  for (const replacement of [
    'echo "exit 1"',
    "exit 1 || true",
    "false && exit 1",
  ]) {
    const result = inspect({
      ciWorkflow: ciWorkflow.replace(
        "            exit 1",
        `            ${replacement}`
      ),
    });
    assert.equal(result.ok, false, replacement);
    assert.match(result.errors.join("\n"), /explicitly exit 1/i);
  }

  for (const prefix of ["exit 0", "true"]) {
    const result = inspect({
      ciWorkflow: ciWorkflow.replace(
        '          if [[ "$DESKTOP_RESULT"',
        `          ${prefix}\n          if [[ "$DESKTOP_RESULT"`
      ),
    });
    assert.equal(result.ok, false, prefix);
    assert.match(result.errors.join("\n"), /explicitly exit 1/i);
  }
});

test("rejects missing and no-op Synapse integration execution", () => {
  for (const workflow of [
    ciWorkflow.replace("  synapse-integration:", "  removed-synapse-job:"),
    ciWorkflow.replace(
      "      - run: npm run test:synapse-integration",
      "      - run: echo npm run test:synapse-integration"
    ),
    ciWorkflow.replace(
      "      - if: always()\n        run: scripts/synapse-integration.sh reset",
      "      - run: scripts/synapse-integration.sh reset"
    ),
  ]) {
    const result = inspect({ ciWorkflow: workflow });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("\n"), /Synapse integration/i);
  }

  const noOpPackage = inspect({
    rootPackage: JSON.stringify({
      scripts: { "test:synapse-integration": "echo skipped" },
    }),
  });
  assert.equal(noOpPackage.ok, false);
  assert.match(noOpPackage.errors.join("\n"), /pinned two-client runner/i);

  for (const workflow of [
    releaseWorkflow.replace(
      "  exact-tag-synapse-integration:",
      "  removed-exact-tag-synapse-integration:"
    ),
    releaseWorkflow.replace(
      "      - run: npm run test:synapse-integration\n        env:",
      "      - run: echo npm run test:synapse-integration\n        env:"
    ),
  ]) {
    const result = inspect({ releaseWorkflow: workflow });
    assert.equal(result.ok, false);
    assert.match(result.errors.join("\n"), /Exact-tag Synapse integration/i);
  }
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
      "needs: [validate, exact-tag-desktop-quality, exact-tag-ios-quality, exact-tag-synapse-integration]",
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

test("rejects missing exact-tag desktop validation commands", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow.replace(
      "          cargo test --locked",
      "          echo cargo test --locked"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /must execute cargo test --locked/i);
});

test("rejects incomplete publication and updater dependencies", () => {
  for (const workflow of [
    releaseWorkflow.replace(
      "needs: [linux-deb, linux-arch, macos, updater-metadata, ios-testflight]",
      "needs: [linux-deb, linux-arch, macos, ios-testflight]"
    ),
    releaseWorkflow.replace(
      "  updater-metadata:\n    needs: [macos]",
      "  updater-metadata:\n    needs: [quality-gate]"
    ),
  ]) {
    const result = inspect({ releaseWorkflow: workflow });
    assert.equal(result.ok, false);
    assert.match(
      result.errors.join("\n"),
      /publication needs|updater-metadata needs/i
    );
  }
});

test("rejects manual release dispatch and dispatch-input TestFlight control", () => {
  const result = inspect({
    releaseWorkflow: releaseWorkflow
      .replace("jobs:", "on:\n  workflow_dispatch:\njobs:")
      .replace(
        "vars.SYNARA_TESTFLIGHT_INTERNAL_ONLY || 'true'",
        "inputs.force_internal_testflight || vars.SYNARA_TESTFLIGHT_INTERNAL_ONLY || 'true'"
      ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /tag-only/i);
  assert.match(result.errors.join("\n"), /repository variable/i);
});

test("rejects release documentation that recommends unavailable CI checks", () => {
  const result = inspect({
    releaseDocs: "Require ordinary CI on production-release.",
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /documentation.*forbid/i);
});
