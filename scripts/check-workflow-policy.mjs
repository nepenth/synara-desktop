import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const indentation = (line) => line.length - line.trimStart().length;
const integrationBranch = "feature/matrix-rust-sdk-full-replacement";
const cancellableValidationWorkflows = ["ci.yml", "desktop-package-smoke.yml"];

function parseJobs(workflow) {
  const jobs = new Map();
  let inJobs = false;
  let currentJob;

  for (const line of workflow.split(/\r?\n/)) {
    const trimmed = line.trim();
    const indent = indentation(line);
    if (indent === 0) {
      if (trimmed === "jobs:") {
        inJobs = true;
        continue;
      }
      if (inJobs && trimmed) break;
      continue;
    }
    if (!inJobs || !trimmed || trimmed.startsWith("#")) continue;

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

function topLevelBlock(workflow, property) {
  const lines = workflow.split(/\r?\n/);
  const start = lines.findIndex(
    (line) => indentation(line) === 0 && line.trim() === `${property}:`,
  );
  if (start < 0) return [];

  const block = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && indentation(line) === 0) break;
    block.push(line);
  }
  return block;
}

const jobScalar = (lines, property) => {
  const prefix = `${property}:`;
  const line = lines.find(
    (candidate) =>
      indentation(candidate) === 4 && candidate.trim().startsWith(prefix),
  );
  return line?.trim().slice(prefix.length).trim();
};

function hasJobScopedSecret(lines) {
  const envIndex = lines.findIndex(
    (line) => indentation(line) === 4 && line.trim() === "env:",
  );
  if (envIndex < 0) return false;

  for (let index = envIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && indentation(line) <= 4) break;
    if (line.includes("${{ secrets.")) return true;
  }
  return false;
}

function pullRequestBlock(workflow) {
  return (
    workflow.match(
      /^  pull_request:\s*\n([\s\S]*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:|^[A-Za-z_][A-Za-z0-9_-]*:)/m,
    )?.[1] ?? ""
  );
}

function hasIntegrationPullRequestTarget(workflow) {
  return pullRequestBlock(workflow).includes(`"${integrationBranch}"`);
}

// Cache readers must share the writer's build family and target paths. Keeping
// PR/tag jobs read-only preserves main's reusable entries within the 10 GB cap.
function inspectRustCachePolicy(workflows, errors) {
  const mainWriter = "${{ github.ref == 'refs/heads/main' }}";
  const manualMainWriter =
    "${{ github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main' }}";
  const simulatorWriter =
    "${{ github.ref == 'refs/heads/main' && (github.event_name != 'workflow_dispatch' || (inputs.apple_slices != 'all' && !inputs.check_ios_device_release)) }}";
  const readers = '"false"';
  const contracts = [
    ["ci.yml", "validate-rust", "validate-rust-desktop", mainWriter],
    ...[
      "reactions",
      "attachments",
      "polls",
      "rich-messages",
      "threads",
      "receipts",
    ].map((proof) => [
      "ci.yml",
      `synapse-native-${proof}`,
      "validate-rust-desktop",
      readers,
    ]),
    [
      "release.yml",
      "exact-tag-desktop-quality",
      "validate-rust-desktop",
      readers,
    ],
    [
      "ci.yml",
      "ios-tests",
      "ci-synara-core-apple-simulator-arm64",
      simulatorWriter,
    ],
    ["ci.yml", "ios-ui-tests", "ci-synara-core-apple-simulator-arm64", readers],
    ["ci.yml", "ios-compile", "ci-synara-core-apple-simulator-arm64", readers],
    [
      "release.yml",
      "exact-tag-ios-quality",
      "ci-synara-core-apple-simulator-arm64",
      readers,
    ],
    [
      "ios-skeleton.yml",
      "test",
      "ci-synara-core-apple-simulator-arm64",
      readers,
    ],
    [
      "desktop-package-smoke.yml",
      "linux-deb",
      "release-linux-deb",
      manualMainWriter,
    ],
    ["release.yml", "linux-deb", "release-linux-deb", readers],
    [
      "macos-signed-build.yml",
      "macos-signed-build",
      "release-macos",
      manualMainWriter,
    ],
    ["release.yml", "macos", "release-macos", readers],
    ["build-cache-seed.yml", "macos-universal", "release-macos", mainWriter],
    [
      "build-cache-seed.yml",
      "ios-device",
      "release-synara-core-apple-device",
      mainWriter,
    ],
    [
      "desktop-package-smoke.yml",
      "linux-arch",
      "desktop-registry-arch",
      manualMainWriter,
      true,
    ],
    ["release.yml", "linux-arch", "desktop-registry-arch", readers, true],
    [
      "desktop-package-smoke.yml",
      "macos-app",
      "desktop-registry-macos-host",
      manualMainWriter,
      true,
    ],
    [
      "release.yml",
      "ios-testflight-upload",
      "release-synara-core-apple-device",
      readers,
    ],
  ];
  for (const [filename, jobName, family, save, registryOnly] of contracts) {
    const job = parseJobs(workflows[filename] ?? "").get(jobName) ?? [];
    const cacheSteps = job
      .join("\n")
      .split(/^      - /m)
      .filter((step) => /uses: Swatinem\/rust-cache@/.test(step));
    const label = `${filename} ${jobName}`;
    if (cacheSteps.length !== 1) {
      errors.push(
        `${label} must declare exactly one Rust cache for ${family}.`,
      );
      continue;
    }
    const step = cacheSteps[0];
    const input = (key) =>
      step.match(new RegExp(`^ {10}${key}: (.*)$`, "m"))?.[1];
    if (input("shared-key") !== family || input("save-if") !== save) {
      errors.push(
        `${label} must use cache family ${family} with save-if: ${save}.`,
      );
    }
    if (registryOnly && input("cache-targets") !== "false") {
      errors.push(
        `${label} must cache only the Cargo registry (cache-targets: false).`,
      );
    }
    if (!registryOnly && input("cache-targets") === "false") {
      errors.push(`${label} must restore compiled Rust dependencies.`);
    }
    if (family.includes("synara-core-apple")) {
      if (
        !step.includes(". -> target/synara-core-apple") ||
        !step.includes(". -> target/synara-core-bindgen") ||
        input("env-vars") !== "DEVELOPER_DIR"
      ) {
        errors.push(
          `${label} must cache the Apple archives and isolated host bindgen target directories with the selected Xcode in cache identity.`,
        );
      }
    } else if (!step.includes(". -> target")) {
      errors.push(`${label} must use the root workspace target directory.`);
    }
  }
}

export function inspectWorkflowPolicy({
  workflows,
  dependabot,
  runtimePackage = "",
  nodeSetupAction = "",
  xcodeSetupAction = "",
  xcodeSaveAction = "",
}) {
  const errors = [];

  for (const [name, action] of [
    ["setup-xcode-cache", xcodeSetupAction],
    ["save-xcode-cache", xcodeSaveAction],
  ]) {
    for (const reference of action.matchAll(/^\s*uses:\s*([^\s#]+)/gm)) {
      if (!/^[^@\s]+@[0-9a-f]{40}$/.test(reference[1]))
        errors.push(`${name} actions must use a full commit SHA.`);
    }
  }
  if (
    !xcodeSetupAction.includes(
      "expected_xcode=$'Xcode 26.6\\nBuild version 17F113'",
    ) ||
    !xcodeSetupAction.includes('"$actual_xcode" != "$expected_xcode"') ||
    !xcodeSetupAction.includes(
      "source synara-ios/scripts/lib/xcode-cache.sh",
    ) ||
    !xcodeSetupAction.includes(
      "xcode-compilation-v1-${RUNNER_OS}-${RUNNER_ARCH}-${SYNARA_XCODE_TOOLCHAIN_KEY}-",
    ) ||
    !xcodeSetupAction.includes("date -u +%G-W%V") ||
    !xcodeSetupAction.includes("SYNARA_IOS_COMPILATION_CACHE_LIMIT=256M") ||
    !xcodeSetupAction.includes("actions/cache/restore@") ||
    !xcodeSetupAction.includes("path: ${{ steps.paths.outputs.cache-path }}") ||
    !xcodeSetupAction.includes(
      "restore-keys: ${{ steps.paths.outputs.cache-prefix }}",
    ) ||
    /actions\/cache(?:\/save)?@/.test(xcodeSetupAction)
  ) {
    errors.push(
      "Xcode cache setup must restore only compiler objects using an exact toolchain prefix and weekly bounded snapshots.",
    );
  }
  if (
    !xcodeSaveAction.includes(
      'expected_path="$GITHUB_WORKSPACE/.xcode-cache/CompilationCache.noindex"',
    ) ||
    !xcodeSaveAction.includes(
      '"$CACHE_PATH" != "$expected_path" || -L "$CACHE_PATH"',
    ) ||
    !xcodeSaveAction.includes("size_kib > 524288") ||
    !xcodeSaveAction.includes("size_kib == 0") ||
    !xcodeSaveAction.includes('"$CACHE_HIT" == true') ||
    !xcodeSaveAction.includes(
      "if: github.ref == 'refs/heads/main' && steps.budget.outputs.publish == 'true'",
    ) ||
    !xcodeSaveAction.includes("actions/cache/save@") ||
    !xcodeSaveAction.includes("path: ${{ inputs.cache-path }}")
  ) {
    errors.push(
      "Xcode compiler cache publication must be nonempty, at most 512 MiB, scoped to the owned CAS directory and main only.",
    );
  }
  const swiftJobs = [
    ["ci.yml", "ios-tests"],
    ["ci.yml", "ios-ui-tests"],
    ["ci.yml", "ios-compile"],
    ["ios-skeleton.yml", "test"],
    ["release.yml", "exact-tag-ios-quality"],
    ["release.yml", "ios-testflight-upload"],
  ];
  for (const [filename, jobName] of swiftJobs) {
    const job = (parseJobs(workflows[filename] ?? "").get(jobName) ?? []).join(
      "\n",
    );
    if (job.split("uses: ./.github/actions/setup-xcode-cache").length !== 2)
      errors.push(
        `${filename} ${jobName} must restore the shared Xcode compiler cache exactly once.`,
      );
  }
  for (const [filename, jobName] of [
    ...swiftJobs,
    ["build-cache-seed.yml", "ios-device"],
  ]) {
    const job = (parseJobs(workflows[filename] ?? "").get(jobName) ?? []).join(
      "\n",
    );
    if (
      !job.includes(
        "DEVELOPER_DIR: /Applications/Xcode_26.6.app/Contents/Developer",
      )
    )
      errors.push(
        `${filename} ${jobName} must select the pinned Xcode 26.6 toolchain before Apple build or cache work.`,
      );
  }
  for (const [filename, workflow] of Object.entries(workflows)) {
    for (const [jobName, lines] of parseJobs(workflow)) {
      const writers = lines
        .join("\n")
        .split(/^      - /m)
        .filter((step) =>
          step.includes("uses: ./.github/actions/save-xcode-cache"),
        );
      if (writers.length && (filename !== "ci.yml" || jobName !== "ios-tests"))
        errors.push(
          `${filename} ${jobName} cannot publish the shared Xcode compiler cache.`,
        );
      if (
        filename === "ci.yml" &&
        jobName === "ios-tests" &&
        (writers.length !== 1 ||
          !writers[0].includes("if: github.ref == 'refs/heads/main'"))
      )
        errors.push(
          "Only the successful main iOS unit lane may publish Xcode compiler objects.",
        );
    }
  }

  if (
    !nodeSetupAction.includes(
      "if: github.ref == 'refs/heads/main'\n      uses: actions/cache@",
    ) ||
    !nodeSetupAction.includes(
      "if: github.ref != 'refs/heads/main'\n      uses: actions/cache/restore@",
    ) ||
    nodeSetupAction.split(
      "hashFiles('package-lock.json', 'synara/package-lock.json')",
    ).length !== 3 ||
    !nodeSetupAction.includes("path: ${{ steps.npm-cache.outputs.path }}") ||
    nodeSetupAction.includes("node_modules")
  ) {
    errors.push(
      "Shared Node setup must cache npm downloads from both lockfiles with only main writers.",
    );
  }
  for (const reference of nodeSetupAction.matchAll(/^\s*uses:\s*([^\s#]+)/gm)) {
    if (!/^[^@\s]+@[0-9a-f]{40}$/.test(reference[1])) {
      errors.push("Shared Node setup actions must use a full commit SHA.");
    }
  }

  for (const [filename, workflow] of Object.entries(workflows).sort()) {
    const permissions = topLevelBlock(workflow, "permissions");
    if (!permissions.some((line) => /^  contents:\s*read\s*$/.test(line))) {
      errors.push(
        `${filename} must declare top-level contents: read permissions.`,
      );
    }
    if (topLevelBlock(workflow, "concurrency").length === 0) {
      errors.push(`${filename} must declare workflow-level concurrency.`);
    }

    if (cancellableValidationWorkflows.includes(filename)) {
      if (!hasIntegrationPullRequestTarget(workflow)) {
        errors.push(
          `${filename} must validate pull requests targeting ${integrationBranch}.`,
        );
      }

      const concurrency = topLevelBlock(workflow, "concurrency")
        .map((line) => line.trim())
        .filter(Boolean);
      const group = concurrency.find((line) => line.startsWith("group:")) ?? "";
      if (
        !group.includes("github.head_ref") ||
        !group.includes("github.ref_name")
      ) {
        errors.push(
          `${filename} concurrency must share one cancellable lane per branch for push and pull_request.`,
        );
      }
      if (!concurrency.includes("cancel-in-progress: true")) {
        errors.push(
          `${filename} must cancel obsolete runs within the same branch lane.`,
        );
      }
    }

    for (const match of workflow.matchAll(/^\s*uses:\s*([^\s#]+)/gm)) {
      const reference = match[1];
      if (reference.startsWith("./")) continue;
      if (!/^[^@\s]+@[0-9a-f]{40}$/.test(reference)) {
        errors.push(
          `${filename} action ${reference} must use a full commit SHA.`,
        );
      }
    }

    if (/runs-on:\s*macos-latest/.test(workflow)) {
      errors.push(`${filename} must pin macOS runner generations.`);
    }

    for (const [jobName, jobLines] of parseJobs(workflow)) {
      const timeout = Number(jobScalar(jobLines, "timeout-minutes"));
      if (!Number.isInteger(timeout) || timeout < 1 || timeout > 120) {
        errors.push(
          `${filename} job ${jobName} must have a 1-120 minute timeout.`,
        );
      }
      if (hasJobScopedSecret(jobLines)) {
        errors.push(
          `${filename} job ${jobName} must scope secrets to only the steps that consume them.`,
        );
      }
    }
  }

  const packageWorkflow = workflows["desktop-package-smoke.yml"] ?? "";
  const packageJobs = parseJobs(packageWorkflow);
  const packageChanges = packageJobs.get("changes") ?? [];
  const packageGate = packageJobs.get("package-gate") ?? [];
  if (/^\s{4}paths:/m.test(pullRequestBlock(packageWorkflow))) {
    errors.push(
      "Desktop package smoke must emit its stable aggregate check for every pull request.",
    );
  }
  if (
    jobScalar(packageGate, "name") !== "Desktop package gate" ||
    (jobScalar(packageGate, "if") !== "always()" &&
      jobScalar(packageGate, "if") !== "always() && !cancelled()") ||
    !packageGate
      .join("\n")
      .includes("needs: [changes, linux-deb, linux-arch, macos-app]")
  ) {
    errors.push(
      "Desktop package smoke must retain an always-running aggregate package gate.",
    );
  }
  const packageChangeContract = packageChanges.join("\n");
  const packageDiffStart = packageChangeContract.indexOf(
    'git diff --quiet "$BASE_SHA" "$HEAD_SHA" --',
  );
  const packageDiffEnd =
    packageDiffStart >= 0
      ? packageChangeContract.indexOf("; then", packageDiffStart)
      : -1;
  const packageDiffContract =
    packageDiffStart >= 0 && packageDiffEnd >= 0
      ? packageChangeContract.slice(packageDiffStart, packageDiffEnd)
      : "";
  for (const pathRoot of [
    "scripts",
    "packaging/arch",
    "src-tauri",
    "synara",
    "crates",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    ".github/actions",
  ]) {
    if (!packageDiffContract.includes(pathRoot)) {
      errors.push(
        `Desktop package change detection must retain the ${pathRoot} path.`,
      );
    }
  }
  if (
    !packageChangeContract.includes(
      'git diff --quiet "$BASE_SHA" "$HEAD_SHA" --',
    ) ||
    !packageChangeContract.includes('echo "packages=false"') ||
    !packageChangeContract.includes('echo "packages=true"')
  ) {
    errors.push(
      "Desktop package smoke must retain PR diff-based package change detection.",
    );
  }

  // Rust gates may live in the monolithic `validate` job or the split
  // `validate-rust` job (parallel with `validate-frontend` for wall-clock speed).
  const ciJobs = parseJobs(workflows["ci.yml"] ?? "");
  const ciValidateRust = [
    ...(ciJobs.get("validate") ?? []),
    ...(ciJobs.get("validate-rust") ?? []),
  ];
  const ciValidationContract = ciValidateRust.join("\n");
  for (const [label, command] of [
    [
      "Core shipping features",
      "node scripts/check-synara-core-production-features.mjs",
    ],
    [
      "NSE shipping features",
      "node scripts/check-synara-nse-core-production-features.mjs",
    ],
    ["formatting", "cargo fmt --check"],
    ["lint", "cargo clippy --locked --all-targets -- -D warnings"],
    ["shared workspace formatting", "cargo fmt --all -- --check"],
    [
      "shared workspace lint",
      "cargo clippy --locked -p synara-core -p synara-nse-core -p synara-core-bindgen --all-targets -- -D warnings",
    ],
    [
      "shared workspace check",
      "cargo check --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
    ],
    [
      "shared workspace tests",
      "cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
    ],
  ]) {
    if (!ciValidationContract.includes(command)) {
      errors.push(`CI must retain strict Rust ${label}: ${command}.`);
    }
  }

  const releaseWorkflow = workflows["release.yml"] ?? "";
  const releaseConcurrency = topLevelBlock(releaseWorkflow, "concurrency")
    .map((line) => line.trim())
    .filter(Boolean);
  if (
    !releaseConcurrency.includes("group: production-release") ||
    !releaseConcurrency.includes("cancel-in-progress: false")
  ) {
    errors.push(
      "Production release tags must share a non-cancelling serialized concurrency lane.",
    );
  }

  const releaseVersionGuard = "node scripts/assert-release-version.mjs";
  const guardCount = releaseWorkflow.split(releaseVersionGuard).length - 1;
  const releaseJobs = parseJobs(releaseWorkflow);
  const releaseValidate = (releaseJobs.get("validate") ?? []).join("\n");
  const exactTagDesktopQuality = (
    releaseJobs.get("exact-tag-desktop-quality") ?? []
  ).join("\n");
  for (const [label, contract] of [
    ["CI", (ciJobs.get("rust-dependency-audit") ?? []).join("\n")],
    ["Exact-tag", exactTagDesktopQuality],
  ]) {
    if (
      !contract.includes("tool: cargo-audit@0.22.2") ||
      !contract.includes("checksum: true") ||
      !contract.includes("fallback: none") ||
      !contract.includes("run: cargo audit") ||
      contract.includes("cargo install cargo-audit")
    ) {
      errors.push(
        `${label} Rust audit must use the pinned checksum-verified binary and remain required.`,
      );
    }
  }
  for (const command of [
    "node scripts/check-synara-core-production-features.mjs",
    "node scripts/check-synara-nse-core-production-features.mjs",
    "cargo fmt --all -- --check",
    "cargo clippy --locked -p synara-core -p synara-nse-core -p synara-core-bindgen --all-targets -- -D warnings",
    "cargo check --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
    "cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
  ]) {
    if (!exactTagDesktopQuality.includes(command)) {
      errors.push(
        `Exact-tag desktop quality must validate the shared Rust workspace: ${command}.`,
      );
    }
  }
  const releasePublish = (releaseJobs.get("publish-gh-release") ?? []).join(
    "\n",
  );
  const validationTagCheck = releaseValidate.indexOf(
    "Require tag to match the shared version",
  );
  const validationGuard = releaseValidate.indexOf(releaseVersionGuard);
  if (
    guardCount !== 2 ||
    validationGuard < 0 ||
    validationTagCheck < 0 ||
    validationGuard < validationTagCheck
  ) {
    errors.push(
      "Production release validation must run the immutable release-version guard exactly once after the exact tag check and before builds.",
    );
  }
  if (!releasePublish.includes(releaseVersionGuard)) {
    errors.push(
      "Production release publication must recheck the immutable release-version guard.",
    );
  }
  const guardBeforePublish = releasePublish.indexOf(releaseVersionGuard);
  const ghReleasePublish = releasePublish.indexOf(
    "softprops/action-gh-release",
  );
  const directlyPrecedesGhRelease =
    /Recheck immutable release version before publication\n        run: node scripts\/assert-release-version\.mjs\n      - name: Create GitHub Release with all client artifacts/.test(
      releasePublish,
    );
  if (
    guardBeforePublish < 0 ||
    ghReleasePublish < 0 ||
    guardBeforePublish > ghReleasePublish ||
    !directlyPrecedesGhRelease
  ) {
    errors.push(
      "Production release version guard must run immediately before the mutating GitHub release action.",
    );
  }
  if (
    !releaseValidate.includes("GH_TOKEN: ${{ github.token }}") ||
    !releasePublish.includes("GH_TOKEN: ${{ github.token }}") ||
    !releasePublish.includes("fetch-depth: 0")
  ) {
    errors.push(
      "Production release version guard must have ledger access and an immutable full-history tag checkout.",
    );
  }
  if (
    typeof runtimePackage !== "string" ||
    /semantic-release/i.test(runtimePackage)
  ) {
    errors.push(
      "Runtime package must not retain an alternate semantic-release publisher.",
    );
  }

  const signedBuild = parseJobs(workflows["macos-signed-build.yml"] ?? "").get(
    "macos-signed-build",
  );
  if (
    !signedBuild
      ?.join("\n")
      .match(/environment:\s*\n\s+name:\s*production-release/)
  ) {
    errors.push(
      "Manual macOS signing must use the protected production-release environment.",
    );
  }

  for (const group of [
    "npm-updates",
    "github-actions-updates",
    "workspace-rust-updates",
  ]) {
    const groupedLane = new RegExp(
      `^ {6}${group}:\\n(?:(?: {8,}.*)?\\n)*? {8}patterns: \\["\\*"\\]$`,
      "m",
    );
    if (!groupedLane.test(dependabot)) {
      errors.push(`Dependabot must retain the grouped ${group} update lane.`);
    }
  }

  const cargoUpdates = dependabot
    .split(/^  - package-ecosystem:/m)
    .slice(1)
    .filter((entry) => /^\s*cargo\s*$/m.test(entry.split("\n", 1)[0]));
  if (
    cargoUpdates.length !== 1 ||
    !/^    directory: \/\s*$/m.test(cargoUpdates[0] ?? "")
  ) {
    errors.push(
      "Dependabot must maintain exactly one Cargo update lane at the root workspace.",
    );
  }

  for (const [filename, workflow] of Object.entries(workflows)) {
    if (
      /src-tauri\/(?:target\/|Cargo\.lock)|src-tauri\s*->\s*target|workspaces:\s*src-tauri\b/m.test(
        workflow,
      )
    ) {
      errors.push(
        `${filename} must use the root Cargo lockfile, workspace cache, and target output paths.`,
      );
    }
  }

  inspectRustCachePolicy(workflows, errors);

  const seeds = workflows["build-cache-seed.yml"] ?? "";
  if (
    !seeds.includes("  workflow_dispatch:") ||
    /secrets\.|production-release|notariz|testflight-upload|schedule:/.test(
      seeds,
    ) ||
    !seeds.includes("--target universal-apple-darwin --no-bundle") ||
    !seeds.includes("SYNARA_CORE_APPLE_SLICES: device") ||
    !seeds.includes("SYNARA_NSE_CORE_APPLE_SLICES: device")
  ) {
    errors.push(
      "Release cache seeds must be manual unsigned universal/device builds without signing or publication.",
    );
  }

  return { ok: errors.length === 0, errors };
}

export function loadWorkflowPolicyInputs(repositoryRoot = root) {
  const workflowDirectory = path.join(repositoryRoot, ".github", "workflows");
  const workflows = Object.fromEntries(
    readdirSync(workflowDirectory)
      .filter((filename) => /\.ya?ml$/.test(filename))
      .sort()
      .map((filename) => [
        filename,
        readFileSync(path.join(workflowDirectory, filename), "utf8"),
      ]),
  );
  return {
    workflows,
    dependabot: readFileSync(
      path.join(repositoryRoot, ".github", "dependabot.yml"),
      "utf8",
    ),
    runtimePackage: readFileSync(
      path.join(repositoryRoot, "synara", "package.json"),
      "utf8",
    ),
    nodeSetupAction: readFileSync(
      path.join(repositoryRoot, ".github/actions/setup-node/action.yml"),
      "utf8",
    ),
    xcodeSetupAction: readFileSync(
      path.join(repositoryRoot, ".github/actions/setup-xcode-cache/action.yml"),
      "utf8",
    ),
    xcodeSaveAction: readFileSync(
      path.join(repositoryRoot, ".github/actions/save-xcode-cache/action.yml"),
      "utf8",
    ),
  };
}

function main() {
  const result = inspectWorkflowPolicy(loadWorkflowPolicyInputs());
  for (const error of result.errors)
    console.error(`[workflow-policy] ${error}`);
  if (!result.ok) process.exit(1);
  console.log(
    "[workflow-policy] permissions, timeouts, action pins, integration CI, concurrency, Rust quality, release secret scope, and package gates are valid.",
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main();
}
