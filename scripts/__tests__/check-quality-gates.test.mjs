import test from "node:test";
import assert from "node:assert/strict";

import { inspectQualityGates } from "../check-quality-gates.mjs";

const ciWorkflow = `
  ios-tests:
    env:
      RUN_IOS_TESTS: "1"
  quality-gate:
    name: Quality gate
    if: always()
    needs: [validate, ios-tests]
`;
const iosWorkflow = `
on:
  workflow_dispatch:
jobs:
  test:
    env:
      RUN_IOS_TESTS: "1"
`;
const releaseWorkflow = `
  exact-tag-desktop-quality:
    needs: [validate]
  exact-tag-ios-quality:
    needs: [validate]
  quality-gate:
    name: Exact-tag quality gate
    needs: [validate, exact-tag-desktop-quality, exact-tag-ios-quality]
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

test("accepts complete CI and exact-tag release gates", () => {
  const result = inspectQualityGates({
    ciWorkflow,
    iosWorkflow,
    releaseWorkflow,
  });
  assert.deepEqual(result, { ok: true, errors: [] });
});

test("rejects an iOS build-only CI workflow", () => {
  const result = inspectQualityGates({
    ciWorkflow: ciWorkflow.replace('RUN_IOS_TESTS: "1"', 'RUN_IOS_TESTS: "0"'),
    iosWorkflow,
    releaseWorkflow,
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /execute.*iOS test/i);
});

test("rejects an artifact that bypasses exact-tag validation", () => {
  const result = inspectQualityGates({
    ciWorkflow,
    iosWorkflow,
    releaseWorkflow: releaseWorkflow.replace(
      "  macos:\n    needs: [quality-gate]",
      "  macos:\n    needs: [validate]"
    ),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /macos/);
});
