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
  const flowList = [block.inline, ...block.children.map((line) => line.trim())]
    .join(" ")
    .trim();
  if (flowList.startsWith("[") && flowList.endsWith("]")) {
    const body = flowList.slice(1, -1).trim();
    return body
      ? body
          .split(",")
          .map((item) => unquote(item))
          .filter(Boolean)
      : [];
  }
  if (block.inline) {
    return [unquote(block.inline)];
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

const SHELL_CONTROL_FLOW =
  /(?:^|\s)(?:if|then|elif|else|fi|for|while|until|case|esac|do|done|select|function|exit|return|true|false)(?:\s|;|$)|&&|\|\||[;|<>(){}]|`|\$\(|\\$/;

const hasUnconditionalCommand = (runLines, command) =>
  runLines.includes(command) &&
  runLines.every((line) => !SHELL_CONTROL_FLOW.test(line));

function hasIosTestBuildStep(jobLines) {
  return parseSteps(jobLines).some((step) => {
    const runLines = executableLines(getStepRun(step));
    return (
      hasUnconditionalCommand(runLines, "scripts/ci-build.sh") &&
      getScalar(step, "working-directory", 8) === "synara-ios" &&
      getStepEnvironment(step).get("RUN_IOS_TESTS") === "1"
    );
  });
}

const REQUIRED_APPLE_RUST_TARGETS = ["aarch64-apple-ios"];

function hasAppleRustToolchainStep(jobLines) {
  return parseSteps(jobLines ?? []).some((step) => {
    const uses = getScalar(step, "uses", 8) ?? "";
    if (!uses.startsWith("dtolnay/rust-toolchain@")) return false;
    if (getNestedScalar(step, "with", "toolchain", 8) !== "1.93") return false;
    const targets = (getNestedScalar(step, "with", "targets", 8) ?? "")
      .split(",")
      .map((target) => target.trim())
      .filter(Boolean);
    return REQUIRED_APPLE_RUST_TARGETS.every((target) =>
      targets.includes(target)
    );
  });
}

function hasSynaraCoreGenerateStep(jobLines) {
  return parseSteps(jobLines ?? []).some((step) => {
    const runLines = executableLines(getStepRun(step));
    return (
      hasUnconditionalCommand(
        runLines,
        "scripts/generate-synara-core-swift.sh"
      ) &&
      getScalar(step, "if", 8) === undefined &&
      getScalar(step, "continue-on-error", 8) === undefined
    );
  });
}

const PROVEN_QUALITY_GATE_REUSE_IF = "steps.reuse.outputs.reuse != 'true'";

function allowsProvenQualityGateReuse(step) {
  const condition = getScalar(step, "if", 8);
  return condition === undefined || condition === PROVEN_QUALITY_GATE_REUSE_IF;
}

function hasProvenQualityGateReuseStep(jobLines) {
  return parseSteps(jobLines ?? []).some((step) => {
    const runLines = executableLines(getStepRun(step));
    return (
      getScalar(step, "id", 8) === "reuse" &&
      hasUnconditionalCommand(
        runLines,
        "node scripts/reuse-proven-quality-gate.mjs"
      ) &&
      getScalar(step, "if", 8) === undefined &&
      getScalar(step, "continue-on-error", 8) === undefined
    );
  });
}

function hasRequiredCommandStep(jobLines, command, workingDirectory) {
  return parseSteps(jobLines ?? []).some((step) => {
    const runLines = executableLines(getStepRun(step));
    return (
      hasUnconditionalCommand(runLines, command) &&
      getScalar(step, "working-directory", 8) === workingDirectory &&
      allowsProvenQualityGateReuse(step) &&
      getScalar(step, "continue-on-error", 8) === undefined
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

    const runLines = executableLines(getStepRun(step));
    const condition = variables
      .map((variable) => `"$${variable}" != "success"`)
      .join(" || ");
    const conditionStart = runLines.indexOf(`if [[ ${condition} ]]; then`);
    if (conditionStart !== 0) continue;
    const conditionEnd = runLines.indexOf("fi", conditionStart + 1);
    if (conditionEnd !== runLines.length - 1) continue;
    const failureBody = runLines.slice(conditionStart + 1, conditionEnd);
    if (failureBody.at(-1) !== "exit 1") continue;
    const diagnostics = failureBody.slice(0, -1);
    if (diagnostics.every((line) => /^echo\s+"[^"]*"\s+>&2$/.test(line))) {
      return undefined;
    }
  }

  return "job must explicitly exit 1 unless every required job result is success";
}

/**
 * CI quality gate after path filters: needs include `changes` plus heavy jobs.
 * Desktop validation may be a single `validate` job or split
 * `validate-rust` + `validate-frontend` (parallel wall-clock speedup).
 * `changes` must be success; heavy jobs may be success or skipped.
 */
function pathFilteredCiAggregateError(jobLines) {
  if (!jobLines) return "job is missing";
  if (getScalar(jobLines, "name", 4) !== "Quality gate") {
    return "job name must be Quality gate";
  }
  if (getScalar(jobLines, "if", 4) !== "always()") {
    return "job must use if: always()";
  }
  const expectedNeedsMonolith = [
    "changes",
    "validate",
    "ios-tests",
    "synapse-native-reactions",
    "synapse-native-attachments",
    "synapse-native-polls",
    "synapse-native-rich-messages",
    "synapse-native-threads",
    "synapse-native-receipts",
  ];
  const expectedNeedsSplit = [
    "changes",
    "validate-rust",
    "validate-frontend",
    "ios-tests",
    "synapse-native-reactions",
    "synapse-native-attachments",
    "synapse-native-polls",
    "synapse-native-rich-messages",
    "synapse-native-threads",
    "synapse-native-receipts",
  ];
  const needs = getList(jobLines, "needs", 4);
  const splitDesktop = sameList(needs, expectedNeedsSplit);
  const monolithDesktop = sameList(needs, expectedNeedsMonolith);
  if (!splitDesktop && !monolithDesktop) {
    return `job needs must be exactly [${expectedNeedsSplit.join(
      ", "
    )}] (or legacy monolith [${expectedNeedsMonolith.join(", ")}])`;
  }

  for (const step of parseSteps(jobLines)) {
    const environment = getStepEnvironment(step);
    const changesVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.changes.result }}"
    )?.[0];
    const desktopVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.validate.result }}"
    )?.[0];
    const desktopRustVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.validate-rust.result }}"
    )?.[0];
    const desktopFrontendVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.validate-frontend.result }}"
    )?.[0];
    const iosVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.ios-tests.result }}"
    )?.[0];
    const synapseNativeReactionsVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.synapse-native-reactions.result }}"
    )?.[0];
    const synapseNativeAttachmentsVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.synapse-native-attachments.result }}"
    )?.[0];
    const synapseNativePollsVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.synapse-native-polls.result }}"
    )?.[0];
    const synapseNativeRichMessagesVar = [...environment.entries()].find(
      ([, value]) =>
        value === "${{ needs.synapse-native-rich-messages.result }}"
    )?.[0];
    const synapseNativeThreadsVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.synapse-native-threads.result }}"
    )?.[0];
    const synapseNativeReceiptsVar = [...environment.entries()].find(
      ([, value]) => value === "${{ needs.synapse-native-receipts.result }}"
    )?.[0];
    const desktopOk = splitDesktop
      ? Boolean(desktopRustVar && desktopFrontendVar)
      : Boolean(desktopVar);
    if (
      !changesVar ||
      !desktopOk ||
      !iosVar ||
      !synapseNativeReactionsVar ||
      !synapseNativeAttachmentsVar ||
      !synapseNativePollsVar ||
      !synapseNativeRichMessagesVar ||
      !synapseNativeThreadsVar ||
      !synapseNativeReceiptsVar
    ) {
      continue;
    }

    const runLines = executableLines(getStepRun(step));
    const runText = runLines.join("\n");

    // changes must hard-fail unless success.
    const changesGuard = runLines.findIndex(
      (line) =>
        line.includes(`"$${changesVar}" != "success"`) ||
        line.includes(`"$${changesVar}" == "success"`)
    );
    if (changesGuard < 0) continue;
    const changesExit = runLines
      .slice(changesGuard)
      .findIndex((line) => line === "exit 1");
    if (changesExit < 0) continue;

    // Path-filtered heavy jobs: success|skipped accepted via case arm.
    if (!runText.includes("success|skipped")) continue;

    // Each heavy result variable must be referenced in an ok()/case path.
    const desktopRefsOk = splitDesktop
      ? runText.includes(`"$${desktopRustVar}"`) &&
        runText.includes(`"$${desktopFrontendVar}"`)
      : runText.includes(`"$${desktopVar}"`);
    if (
      !desktopRefsOk ||
      !runText.includes(`"$${iosVar}"`) ||
      !runText.includes(`"$${synapseNativeReactionsVar}"`) ||
      !runText.includes(`"$${synapseNativeAttachmentsVar}"`) ||
      !runText.includes(`"$${synapseNativePollsVar}"`) ||
      !runText.includes(`"$${synapseNativeRichMessagesVar}"`) ||
      !runText.includes(`"$${synapseNativeThreadsVar}"`) ||
      !runText.includes(`"$${synapseNativeReceiptsVar}"`)
    ) {
      continue;
    }

    // Aggregate fail flag must force a bare exit 1 (not quoted/echoed/short-circuited).
    const failExit = runLines.findIndex(
      (line, idx) =>
        line === "exit 1" &&
        idx > changesGuard + changesExit &&
        runLines[idx - 1]?.includes("fail")
    );
    // Also accept: `if [[ "$fail" -ne 0 ]]; then` then `exit 1`
    const failBlock = runLines.findIndex((line) =>
      /\[\[\s*"\$fail"\s*-ne\s*0\s*\]\]/.test(line)
    );
    if (failBlock >= 0) {
      const body = runLines.slice(failBlock + 1);
      const end = body.indexOf("fi");
      if (end > 0 && body.slice(0, end).includes("exit 1")) {
        return undefined;
      }
    }
    if (failExit >= 0) return undefined;
  }

  return "job must require changes=success, allow success|skipped for validate/ios/native-synapse, and exit 1 on failure";
}

export function inspectQualityGates({
  ciWorkflow,
  iosWorkflow,
  releaseWorkflow,
  releaseDocs = "",
  rootPackage = "",
}) {
  const errors = [];
  const ciJobs = parseJobs(ciWorkflow);
  const iosJobs = parseJobs(iosWorkflow);
  const releaseJobs = parseJobs(releaseWorkflow);

  let packageScripts = {};
  try {
    packageScripts = JSON.parse(rootPackage).scripts ?? {};
  } catch {
    errors.push("Root package metadata must be valid JSON.");
  }

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

  // Desktop Node gates may live on monolithic `validate` or split `validate-frontend`.
  const ciDesktopNodeJob =
    ciJobs.get("validate-frontend") ?? ciJobs.get("validate");
  for (const [jobLines, label] of [
    [ciDesktopNodeJob, "CI desktop validation"],
    [
      releaseJobs.get("exact-tag-desktop-quality"),
      "Exact-tag desktop validation",
    ],
  ]) {
    for (const command of [
      "npx playwright install --with-deps chromium",
      "npm run typecheck",
      "npm run test:browser:timeline",
      "npm run check:security",
    ]) {
      if (!hasRequiredCommandStep(jobLines, command, "synara")) {
        errors.push(`${label} must execute ${command} in synara.`);
      }
    }
  }

  for (const [jobLines, label] of [
    [ciDesktopNodeJob, "CI desktop validation"],
    [
      releaseJobs.get("exact-tag-desktop-quality"),
      "Exact-tag desktop validation",
    ],
  ]) {
    if (!hasRequiredCommandStep(jobLines, "npm run check:release-updater")) {
      errors.push(
        `${label} must execute npm run check:release-updater at repository root.`
      );
    }
  }

  const ciAggregateError = pathFilteredCiAggregateError(
    ciJobs.get("quality-gate")
  );
  if (ciAggregateError) errors.push(`CI aggregate ${ciAggregateError}.`);

  for (const job of ["exact-tag-desktop-quality", "exact-tag-ios-quality"]) {
    if (
      !sameList(getList(releaseJobs.get(job) ?? [], "needs", 4), ["validate"])
    ) {
      errors.push(`Release workflow ${job} needs must be exactly [validate].`);
    }
    if (!hasProvenQualityGateReuseStep(releaseJobs.get(job))) {
      errors.push(
        `Release workflow ${job} must reuse a proven Quality gate via scripts/reuse-proven-quality-gate.mjs before rerunning exact-tag work.`
      );
    }
  }

  const exactDesktop = releaseJobs.get("exact-tag-desktop-quality") ?? [];
  for (const [command, workingDirectory] of [
    ["npm run check:repo-layout", undefined],
    ["npm run check:versions", undefined],
    ["npm run check:matrix-boundaries", undefined],
    ["npm run check:quality-gates", undefined],
    ["npm run check:synapse-harness", undefined],
    ["npm run check:production-smoke", undefined],
    ["node --test scripts/__tests__/*.test.mjs", undefined],
    ["cargo check --locked", "src-tauri"],
    ["cargo test --locked", "src-tauri"],
    ["npm run typecheck", "synara"],
    ["npm run typecheck:modernization", "synara"],
    ["npm run test:modernization", "synara"],
    ["npm run test:browser:timeline", "synara"],
    ["npm run check:eslint", "synara"],
    ["npm run check:prettier", "synara"],
    ["npm run check:security", "synara"],
  ]) {
    if (!hasRequiredCommandStep(exactDesktop, command, workingDirectory)) {
      errors.push(
        `Exact-tag desktop validation must execute ${command}${
          workingDirectory ? ` in ${workingDirectory}` : " at repository root"
        }.`
      );
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

  for (const job of [
    "linux-deb",
    "linux-arch",
    "macos",
    "ios-testflight-upload",
  ]) {
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

  if (
    !sameList(getList(releaseJobs.get("ios-testflight") ?? [], "needs", 4), [
      "ios-testflight-upload",
    ])
  ) {
    errors.push(
      "TestFlight verification must depend only on the successful upload job so failed-job retries cannot duplicate the upload."
    );
  }

  if (
    !sameList(getList(releaseJobs.get("updater-metadata") ?? [], "needs", 4), [
      "macos",
    ])
  ) {
    errors.push(
      "Release workflow updater-metadata needs must be exactly [macos]."
    );
  }

  const publishJob = releaseJobs.get("publish-gh-release") ?? [];
  if (
    !sameList(getList(publishJob, "needs", 4), [
      "linux-deb",
      "linux-arch",
      "macos",
      "updater-metadata",
    ])
  ) {
    errors.push(
      "Release publication needs must include exactly the desktop artifacts and updater metadata job."
    );
  }
  if (getList(publishJob, "needs", 4).includes("ios-testflight")) {
    errors.push(
      "GitHub Release must not wait on TestFlight promotion."
    );
  }
  if (
    getNestedScalar(publishJob, "environment", "name", 4) !==
    "production-release"
  ) {
    errors.push(
      "Release publication must use the protected production-release environment."
    );
  }

  if (/^\s{2}workflow_dispatch:/m.test(releaseWorkflow)) {
    errors.push(
      "Release workflow must be tag-only; workflow_dispatch cannot safely select an explicit release tag."
    );
  }
  if (/inputs\.force_internal_testflight/.test(releaseWorkflow)) {
    errors.push(
      "TestFlight internal-only control must use the repository variable, not a workflow-dispatch input."
    );
  }
  const testflightUpload = releaseJobs.get("ios-testflight-upload") ?? [];
  const testflight = releaseJobs.get("ios-testflight") ?? [];
  const testflightUploadSteps = parseSteps(testflightUpload);
  const testflightSteps = parseSteps(testflight);
  const internalOnlyValues = testflightUploadSteps.flatMap((step) => [
    getStepEnvironment(step).get("SYNARA_TESTFLIGHT_INTERNAL_ONLY"),
  ]);
  if (
    !internalOnlyValues.includes(
      "${{ vars.SYNARA_TESTFLIGHT_INTERNAL_ONLY || 'true' }}"
    )
  ) {
    errors.push(
      "TestFlight internal-only control must default the SYNARA_TESTFLIGHT_INTERNAL_ONLY repository variable to true."
    );
  }

  const testflightTimeout = Number(getScalar(testflight, "timeout-minutes", 4));
  if (
    !Number.isInteger(testflightTimeout) ||
    testflightTimeout < 50 ||
    testflightTimeout > 120
  ) {
    errors.push(
      "TestFlight job timeout-minutes must be between 50 and 120 so Apple processing is bounded but observable."
    );
  }

  const uploadTimeout = Number(
    getScalar(testflightUpload, "timeout-minutes", 4)
  );
  if (
    !Number.isInteger(uploadTimeout) ||
    uploadTimeout < 90 ||
    uploadTimeout > 120
  ) {
    errors.push(
      "TestFlight upload timeout-minutes must be between 90 and 120 so UniFFI generate plus archive can finish."
    );
  }

  if (!hasAppleRustToolchainStep(testflightUpload)) {
    errors.push(
      "TestFlight upload must install Rust 1.93 with aarch64-apple-ios before archive."
    );
  }

  const generateStepIndex = testflightUploadSteps.findIndex((step) =>
    hasUnconditionalCommand(
      executableLines(getStepRun(step)),
      "scripts/generate-synara-core-swift.sh"
    )
  );
  if (
    generateStepIndex < 0 ||
    !hasSynaraCoreGenerateStep(testflightUpload)
  ) {
    errors.push(
      "TestFlight upload must generate SynaraCore with scripts/generate-synara-core-swift.sh before xcodebuild archive."
    );
  }

  const uploadStepIndex = testflightUploadSteps.findIndex((step) =>
    hasUnconditionalCommand(
      executableLines(getStepRun(step)),
      "synara-ios/scripts/upload-testflight-internal.sh"
    )
  );
  const uploadStep = testflightUploadSteps[uploadStepIndex] ?? [];
  if (
    uploadStepIndex < 0 ||
    getScalar(uploadStep, "id", 8) !== "upload_ios" ||
    getScalar(uploadStep, "if", 8) !== undefined ||
    getScalar(uploadStep, "continue-on-error", 8) !== undefined ||
    getStepEnvironment(uploadStep).get("SYNARA_IOS_DIAGNOSTICS_DIR") !==
      "${{ runner.temp }}/synara-ios-testflight-diagnostics"
  ) {
    errors.push(
      "TestFlight upload must be an unconditional upload_ios step that preserves distribution diagnostics."
    );
  }

  if (
    generateStepIndex >= 0 &&
    uploadStepIndex >= 0 &&
    generateStepIndex >= uploadStepIndex
  ) {
    errors.push(
      "TestFlight upload must generate SynaraCore with scripts/generate-synara-core-swift.sh before xcodebuild archive."
    );
  }

  if (
    getScalar(testflightUpload, "outputs", 4) !== "" ||
    getNestedScalar(testflightUpload, "outputs", "marketing_version", 4) !==
      "${{ steps.upload_ios.outputs.marketing_version }}" ||
    getNestedScalar(testflightUpload, "outputs", "build_number", 4) !==
      "${{ steps.upload_ios.outputs.build_number }}"
  ) {
    errors.push(
      "TestFlight upload job must expose the exact uploaded marketing version and build number."
    );
  }

  const uploadDiagnosticsStepIndex = testflightUploadSteps.findIndex(
    (step) =>
      getScalar(step, "uses", 8)?.startsWith("actions/upload-artifact@") &&
      getNestedScalar(step, "with", "path", 8) ===
        "${{ runner.temp }}/synara-ios-testflight-diagnostics"
  );
  const uploadDiagnosticsStep =
    testflightUploadSteps[uploadDiagnosticsStepIndex] ?? [];
  if (
    uploadDiagnosticsStepIndex <= uploadStepIndex ||
    getScalar(uploadDiagnosticsStep, "if", 8) !== "always()" ||
    getNestedScalar(uploadDiagnosticsStep, "with", "retention-days", 8) !== "30"
  ) {
    errors.push(
      "TestFlight upload diagnostics must be preserved with if: always() and 30-day retention."
    );
  }

  const verifierStepIndex = testflightSteps.findIndex((step) =>
    hasUnconditionalCommand(
      executableLines(getStepRun(step)),
      "node synara-ios/scripts/promote-testflight-internal.mjs"
    )
  );
  const verifierStep = testflightSteps[verifierStepIndex] ?? [];
  const verifierEnvironment = getStepEnvironment(verifierStep);
  if (
    verifierStepIndex < 0 ||
    getScalar(verifierStep, "if", 8) !== undefined ||
    getScalar(verifierStep, "continue-on-error", 8) !== undefined ||
    verifierEnvironment.get("SYNARA_IOS_MARKETING_VERSION") !==
      "${{ needs.ios-testflight-upload.outputs.marketing_version }}" ||
    verifierEnvironment.get("SYNARA_IOS_BUILD_NUMBER") !==
      "${{ needs.ios-testflight-upload.outputs.build_number }}" ||
    verifierEnvironment.get("SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS") !==
      "${{ vars.SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS }}" ||
    verifierEnvironment.get("SYNARA_IOS_DIAGNOSTICS_DIR") !==
      "${{ runner.temp }}/synara-ios-testflight-diagnostics"
  ) {
    errors.push(
      "TestFlight release must verify and promote the exact uploaded version/build to configured internal groups."
    );
  }

  const diagnosticsStepIndex = testflightSteps.findIndex(
    (step) =>
      getScalar(step, "uses", 8)?.startsWith("actions/upload-artifact@") &&
      getNestedScalar(step, "with", "path", 8) ===
        "${{ runner.temp }}/synara-ios-testflight-diagnostics"
  );
  const diagnosticsStep = testflightSteps[diagnosticsStepIndex] ?? [];
  if (
    diagnosticsStepIndex <= verifierStepIndex ||
    getScalar(diagnosticsStep, "if", 8) !== "always()" ||
    getNestedScalar(diagnosticsStep, "with", "retention-days", 8) !== "30"
  ) {
    errors.push(
      "TestFlight processing diagnostics must be uploaded after verification with if: always() and 30-day retention."
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
    rootPackage: readFileSync(path.join(root, "package.json"), "utf8"),
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
