import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";
import { fileURLToPath } from "node:url";

const runtimeRoot = fileURLToPath(new URL("../../synara/", import.meta.url));
const requireRuntime = createRequire(new URL("../../synara/package.json", import.meta.url));
const { ESLint } = requireRuntime("eslint");
const eslint = new ESLint({ cwd: runtimeRoot });

// Negative fixtures prove the compatibility bridge executes the configured
// plugin rules, rather than merely making the configuration load successfully.
for (const [rule, source] of [
  ["react/no-direct-mutation-state", "import React from 'react'; export class Bad extends React.Component { update() { this.state.value = 1; } render() { return null; } }"],
  ["jsx-a11y/alt-text", "export const Bad = () => <img />;"],
  ["import/no-extraneous-dependencies", "import semver from 'semver'; export const value = semver;"],
  ["react-hooks/rules-of-hooks", "import { useState } from 'react'; export function Bad({ active }) { if (active) { useState(0); } return null; }"],
]) {
  test(`ESLint 10 retains ${rule}`, async () => {
    // An existing source path lets React's version detector resolve the runtime
    // package. lintText checks fixture bytes without writing to this file.
    const results = await eslint.lintText(source, { filePath: "src/index.tsx" });
    assert.ok(results.flatMap((result) => result.messages).some((message) => message.ruleId === rule));
  });
}
