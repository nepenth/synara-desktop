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

test("npm download caching rejects branch writers, mutable actions and installed dependencies", () => {
  for (const mutate of [
    (action) =>
      action.replace("if: github.ref == 'refs/heads/main'", "if: always()"),
    (action) =>
      action.replace(
        "actions/cache@caa296126883cff596d87d8935842f9db880ef25",
        "actions/cache@v5",
      ),
    (action) =>
      action.replace(
        "path: ${{ steps.npm-cache.outputs.path }}",
        "path: node_modules",
      ),
    (action) =>
      action.replace("'synara/package-lock.json'", "'wrong/package-lock.json'"),
  ]) {
    assert.equal(
      inspectWorkflowPolicy({
        ...valid,
        nodeSetupAction: mutate(valid.nodeSetupAction),
      }).ok,
      false,
    );
  }
});

test("release seeds require unsigned manual builds with main cache writers", () => {
  for (const mutate of [
    (workflow) => workflow.replace("--no-bundle", "--bundles app"),
    (workflow) =>
      workflow.replace("${{ github.ref == 'refs/heads/main' }}", '"true"'),
    (workflow) => workflow.replace("  workflow_dispatch:", "  push:"),
    (workflow) =>
      workflow.replace(
        "SYNARA_NSE_CORE_APPLE_SLICES: device",
        "SYNARA_NSE_CORE_APPLE_SLICES: all",
      ),
    (workflow) =>
      workflow.replace(
        "    steps:",
        "    env:\n      SIGNING: ${{ secrets.APPLE_ID }}\n    steps:",
      ),
  ]) {
    assert.equal(inspect("build-cache-seed.yml", mutate).ok, false);
  }
});

test("Xcode compiler caches reject broad restore keys, unbounded publication and mutable actions", () => {
  for (const [property, before, after] of [
    ["xcodeSetupAction", "Build version 17F113", "Build version unknown"],
    ["xcodeSetupAction", "${SYNARA_XCODE_TOOLCHAIN_KEY}-", ""],
    ["xcodeSetupAction", "date -u +%G-W%V", "date -u +%s"],
    [
      "xcodeSetupAction",
      "restore-keys: ${{ steps.paths.outputs.cache-prefix }}",
      "restore-keys: xcode-compilation-v1-",
    ],
    [
      "xcodeSetupAction",
      "actions/cache/restore@caa296126883cff596d87d8935842f9db880ef25",
      "actions/cache/restore@v5",
    ],
    ["xcodeSaveAction", "size_kib > 524288", "size_kib > 9999999"],
    ["xcodeSaveAction", "CompilationCache.noindex", "DerivedData"],
    [
      "xcodeSaveAction",
      "if: github.ref == 'refs/heads/main' && steps.budget.outputs.publish == 'true'",
      "if: always()",
    ],
  ]) {
    assert.ok(valid[property].includes(before), before);
    const result = inspectWorkflowPolicy({
      ...valid,
      [property]: valid[property].replace(before, after),
    });
    assert.equal(result.ok, false, `${property}: ${before}`);
  }
});

test("Apple jobs select a pinned Xcode and include it in Rust dependency cache identity", () => {
  for (const filename of [
    "ci.yml",
    "release.yml",
    "ios-skeleton.yml",
    "build-cache-seed.yml",
  ]) {
    for (const [before, after] of [
      [
        "DEVELOPER_DIR: /Applications/Xcode_26.6.app/Contents/Developer",
        "DEVELOPER_DIR: /Applications/Xcode.app/Contents/Developer",
      ],
    ]) {
      const result = inspect(filename, (workflow) =>
        workflow.replace(before, after),
      );
      assert.equal(result.ok, false, `${filename}: ${before}`);
    }
  }
});

test("only the main iOS unit lane writes Swift compiler snapshots; other lanes remain readers", () => {
  const noWriterGuard = inspect("ci.yml", (workflow) =>
    workflow.replace(
      "if: github.ref == 'refs/heads/main'\n        uses: ./.github/actions/save-xcode-cache",
      "if: always()\n        uses: ./.github/actions/save-xcode-cache",
    ),
  );
  assert.equal(noWriterGuard.ok, false);
  for (const filename of ["ios-skeleton.yml", "release.yml"]) {
    const unexpectedWriter = inspect(filename, (workflow) =>
      workflow.replace(
        "uses: ./.github/actions/setup-xcode-cache",
        "uses: ./.github/actions/save-xcode-cache",
      ),
    );
    assert.equal(unexpectedWriter.ok, false, filename);
  }
});

test("Rust audits retain exact versions, checksum verification and exact-tag coverage", () => {
  for (const filename of ["ci.yml", "release.yml"]) {
    for (const mutate of [
      (workflow) =>
        workflow.replace("tool: cargo-audit@0.22.2", "tool: cargo-audit"),
      (workflow) => workflow.replace("checksum: true", "checksum: false"),
      (workflow) =>
        workflow.replace("fallback: none", "fallback: cargo-install"),
      (workflow) =>
        workflow.replace("run: cargo audit", "run: echo skipped audit"),
    ]) {
      assert.equal(inspect(filename, mutate).ok, false);
    }
  }
});

test("rejects mutable action references", () => {
  const result = inspect("ci.yml", (workflow) =>
    workflow.replace(/actions\/checkout@[0-9a-f]{40}/, "actions/checkout@main"),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /full commit SHA/);
});

test("rejects missing least-privilege permissions and timeouts", () => {
  const noPermissions = inspect("ios-skeleton.yml", (workflow) =>
    workflow.replace("permissions:\n  contents: read\n\n", ""),
  );
  assert.match(noPermissions.errors.join("\n"), /contents: read/);

  const noTimeout = inspect("ios-skeleton.yml", (workflow) =>
    workflow.replace("    timeout-minutes: 90\n", ""),
  );
  assert.match(noTimeout.errors.join("\n"), /must have a 1-120 minute timeout/);
});

test("rejects secrets exposed to an entire job", () => {
  const result = inspect("macos-signed-build.yml", (workflow) =>
    workflow.replace(
      "    steps:\n",
      "    env:\n      LEAKED: ${{ secrets.APPLE_ID }}\n    steps:\n",
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /scope secrets/);
});

test("requires task PR validation on the Matrix Rust integration branch", () => {
  for (const workflowName of validationWorkflows) {
    const result = inspect(workflowName, (workflow) =>
      workflow.replace(`"${integrationBranch}"`, ""),
    );
    assert.equal(result.ok, false, workflowName);
    assert.match(
      result.errors.join("\n"),
      /must validate pull requests targeting/,
      workflowName,
    );
  }
});

test("requires a cancellable validation lane per branch", () => {
  for (const workflowName of validationWorkflows) {
    const noBranchLane = inspect(workflowName, (workflow) =>
      workflow.replace(
        "${{ github.head_ref || github.ref_name }}",
        "${{ github.ref }}",
      ),
    );
    assert.match(
      noBranchLane.errors.join("\n"),
      /share one cancellable lane per branch/,
      workflowName,
    );

    const noCancellation = inspect(workflowName, (workflow) =>
      workflow.replace("cancel-in-progress: true", "cancel-in-progress: false"),
    );
    assert.match(
      noCancellation.errors.join("\n"),
      /must cancel obsolete runs within the same branch lane/,
      workflowName,
    );
  }
});

test("rejects an unstable or skippable package gate", () => {
  const result = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace(
      `  pull_request:\n    branches: [main, "${integrationBranch}", "release/**"]`,
      `  pull_request:\n    branches: [main, "${integrationBranch}", "release/**"]\n    paths: ["src-tauri/**"]`,
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /stable aggregate check/);
});

test("rejects weakened package change detection", () => {
  const missingPath = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace("            src-tauri \\\n", ""),
  );
  assert.match(missingPath.errors.join("\n"), /must retain the src-tauri path/);

  const noDiff = inspect("desktop-package-smoke.yml", (workflow) =>
    workflow.replace(
      'git diff --quiet "$BASE_SHA" "$HEAD_SHA" --',
      "git diff --quiet --",
    ),
  );
  assert.match(noDiff.errors.join("\n"), /PR diff-based package change/);
});

test("requires strict desktop-shell and shared-workspace Rust gates in CI", () => {
  for (const [command, expected] of [
    ["cargo fmt --check", /strict Rust formatting/],
    ["cargo clippy --locked --all-targets -- -D warnings", /strict Rust lint/],
    ["cargo fmt --all -- --check", /shared workspace formatting/],
    [
      "cargo clippy --locked -p synara-core -p synara-nse-core -p synara-core-bindgen --all-targets -- -D warnings",
      /shared workspace lint/,
    ],
    [
      "cargo check --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
      /shared workspace check/,
    ],
    [
      "cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
      /shared workspace tests/,
    ],
  ]) {
    const result = inspect("ci.yml", (workflow) =>
      workflow.replace(command, "cargo --version"),
    );
    assert.equal(result.ok, false, command);
    assert.match(result.errors.join("\n"), expected, command);
  }
});

test("requires exact-tag shared Rust workspace validation", () => {
  const result = inspect("release.yml", (workflow) =>
    workflow.replace(
      "          cargo test --locked -p synara-core -p synara-nse-core -p synara-core-bindgen",
      "          cargo --version",
    ),
  );
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /Exact-tag desktop quality must validate the shared Rust workspace/,
  );
});

test("rejects per-tag production release concurrency", () => {
  const result = inspect("release.yml", (workflow) =>
    workflow.replace("group: production-release", "group: ${{ github.ref }}"),
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

test("Cargo dependency updates use exactly one root workspace lane", () => {
  for (const dependabot of [
    valid.dependabot.replace(
      "  - package-ecosystem: cargo\n    directory: /",
      "  - package-ecosystem: cargo\n    directory: /src-tauri",
    ),
    `${valid.dependabot}\n  - package-ecosystem: cargo\n    directory: /src-tauri\n`,
  ]) {
    const result = inspectWorkflowPolicy({ ...valid, dependabot });
    assert.equal(result.ok, false);
    assert.match(
      result.errors.join("\n"),
      /exactly one Cargo update lane at the root/,
    );
  }
});

test("package publication and Rust caches use canonical workspace paths", () => {
  for (const [name, original, replacement] of [
    [
      "desktop-package-smoke.yml",
      "target/release/bundle/deb/",
      "src-tauri/target/release/bundle/deb/",
    ],
    [
      "release.yml",
      "target/universal-apple-darwin/release/bundle/",
      "src-tauri/target/universal-apple-darwin/release/bundle/",
    ],
    ["ci.yml", "Cargo.lock", "src-tauri/Cargo.lock"],
  ]) {
    const result = inspect(name, (workflow) =>
      workflow.replace(original, replacement),
    );
    assert.equal(result.ok, false, name);
    assert.match(
      result.errors.join("\n"),
      /root Cargo lockfile, workspace cache, and target output paths/,
    );
  }
});

test("requires immutable version guards before validation and publication", () => {
  const missingValidationGuard = inspect("release.yml", (workflow) =>
    workflow.replace(
      "      - name: Require a fresh incremented release version\n        env:\n          GH_TOKEN: ${{ github.token }}\n        run: node scripts/assert-release-version.mjs\n\n",
      "",
    ),
  );
  assert.match(
    missingValidationGuard.errors.join("\n"),
    /immutable release-version guard/,
  );

  const misplacedPublishGuard = inspect("release.yml", (workflow) =>
    workflow
      .replace(
        "      - name: Recheck immutable release version before publication\n        run: node scripts/assert-release-version.mjs\n\n",
        "",
      )
      .replace(
        "      - name: Create fixed pacman repository release\n",
        "      - name: Recheck immutable release version before publication\n        run: node scripts/assert-release-version.mjs\n\n      - name: Create fixed pacman repository release\n",
      ),
  );
  assert.match(
    misplacedPublishGuard.errors.join("\n"),
    /must run immediately before/,
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
    /alternate semantic-release publisher/,
  );
});

function editJob(workflow, job, transform) {
  // The next job boundary, rather than an end-of-line match, scopes mutations.
  const start = workflow.indexOf(`  ${job}:\n`);
  assert.ok(start >= 0);
  const suffix = workflow.slice(start);
  const next = suffix.slice(1).search(/^  [a-z][a-z-]*:\n/m);
  const end = next < 0 ? workflow.length : start + next + 1;
  return (
    workflow.slice(0, start) +
    transform(workflow.slice(start, end)) +
    workflow.slice(end)
  );
}

test("native proofs restore the main validation cache without freezing narrower outputs", () => {
  const result = inspect("ci.yml", (workflow) =>
    editJob(workflow, "synapse-native-polls", (job) =>
      job.replace('save-cache: "false"', 'save-cache: "true"'),
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /synapse-native-polls.*save-cache/);
});

test("release tags cannot replace the main release cache writer", () => {
  const result = inspect("release.yml", (workflow) =>
    editJob(workflow, "linux-deb", (job) =>
      job.replace('save-cache: "false"', 'save-cache: "true"'),
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /linux-deb.*save-cache/);
});

test("manual all-slice Apple builds cannot inflate the normal simulator cache", () => {
  const result = inspect("ci.yml", (workflow) =>
    editJob(workflow, "ios-tests", (job) =>
      job.replace(
        /^ {10}save-cache: .*$/m,
        "          save-cache: ${{ github.ref == 'refs/heads/main' }}",
      ),
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /ios-tests.*save-cache/);
});

test("Rust lanes cannot reintroduce target-directory cache configuration", () => {
  for (const jobName of ["linux-arch", "macos-app"]) {
    const result = inspect("desktop-package-smoke.yml", (workflow) =>
      editJob(workflow, jobName, (job) =>
        job.replace(
          "          family:",
          "          workspaces: . -> target\n          family:",
        ),
      ),
    );
    assert.equal(result.ok, false);
    assert.match(result.errors.join("\n"), /without target archives/);
  }
});

test("Rust lanes cannot bypass the shared compiler cache action", () => {
  const result = inspect("ios-skeleton.yml", (workflow) =>
    workflow.replace(
      "uses: ./.github/actions/setup-rust-cache",
      "uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6",
    ),
  );
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /must use Kache/);
});

test("cache readers must keep the writer's canonical build family", () => {
  const result = inspect("ci.yml", (workflow) =>
    editJob(workflow, "synapse-native-threads", (job) =>
      job.replace("family: validate-rust-desktop", "family: threads-only"),
    ),
  );
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /synapse-native-threads.*cache family/,
  );
});

test("Kache setup rejects unsafe publication, stale immutable snapshots and compiler cache bypasses", () => {
  for (const [property, before, after] of [
    [
      "rustLocalCache",
      "SYNARA_KACHE_VERSION=1.0.0",
      "SYNARA_KACHE_VERSION=1.1.0",
    ],
    [
      "rustCacheIdentity",
      "source scripts/lib/rust-cache.sh",
      "# version omitted",
    ],
    [
      "rustSetupAction",
      "kunobi-ninja/kache-action@1a33fb2ff51be23eb9e87abeae6edb65be78f71c",
      "kunobi-ninja/kache-action@main",
    ],
    ["rustSetupAction", "version: v1.0.0", "version: latest"],
    [
      "rustSetupAction",
      "node scripts/start-ci-rust-cache.mjs",
      "echo daemon-skipped",
    ],
    [
      "rustSetupAction",
      "SYNARA_CACHE_MAX_SIZE: ${{ inputs.max-size }}",
      "SYNARA_CACHE_MAX_SIZE: 100GiB",
    ],
    [
      "rustSetupAction",
      "hashFiles('Cargo.lock')",
      "hashFiles('**/Cargo.lock')",
    ],
    [
      "rustSetupAction",
      "cargo fetch --locked --manifest-path Cargo.toml",
      "cargo fetch --manifest-path Cargo.toml",
    ],
    ["rustSetupAction", 'github-cache: "false"', 'github-cache: "true"'],
    ["rustSetupAction", 'save-cache: "false"', 'save-cache: "true"'],
    [
      "rustSetupAction",
      'cache-executables: "true"',
      'cache-executables: "false"',
    ],
    ["rustSetupAction", 'cache-c-cpp: "true"', 'cache-c-cpp: "false"'],
    ["rustSetupAction", 'pr-comment: "false"', 'pr-comment: "true"'],
    ["rustSetupAction", 'strict: "true"', 'strict: "false"'],
    [
      "rustSetupAction",
      "if: inputs.save-cache == 'true' && github.ref == 'refs/heads/main'",
      "if: always()",
    ],
    ["rustSetupAction", "if: inputs.save-cache != 'true'", "if: always()"],
    ["rustSetupAction", "-${{ github.sha }}", ""],
    [
      "rustSetupAction",
      "restore-keys: ${{ steps.identity.outputs.compiler-prefix }}-",
      "restore-keys: synara-kache-v1-",
    ],
    [
      "rustSetupAction",
      "path: ${{ github.workspace }}/.kache-cache",
      "path: target",
    ],
    ["rustSetupAction", "~/.cargo/git/db", "~/.cargo/registry/src"],
    [
      "rustCacheIdentity",
      'echo "CARGO_INCREMENTAL=0"',
      'echo "CARGO_INCREMENTAL=1"',
    ],
    [
      "rustCacheIdentity",
      '"${GITHUB_REF:-}" == refs/heads/main',
      '"${GITHUB_REF:-}" != refs/heads/main',
    ],
    ["rustCacheIdentity", "${SYNARA_CACHE_FAMILY}-", ""],
    [
      "rustCacheIdentity",
      "xcrun --sdk iphoneos --show-sdk-version",
      "echo unknown-sdk",
    ],
    ["rustCacheIdentity", 'echo "KACHE_HOST_CONFIG="', 'echo "IGNORED="'],
  ]) {
    assert.ok(valid[property].includes(before), `${property}: ${before}`);
    const result = inspectWorkflowPolicy({
      ...valid,
      [property]: valid[property].replace(before, after),
    });
    assert.equal(result.ok, false, `${property}: ${before}`);
  }
});

test("Kache daemon cleanup runs before custom compiler snapshot publication", () => {
  const marker = "    - name: Install pinned Kache compiler wrapper";
  const index = valid.rustSetupAction.indexOf(marker);
  assert.ok(index >= 0);
  const prefix = valid.rustSetupAction.slice(0, index);
  const split = prefix.indexOf("    - name:");
  const action =
    prefix.slice(0, split) +
    valid.rustSetupAction.slice(index) +
    prefix.slice(split);
  const result = inspectWorkflowPolicy({ ...valid, rustSetupAction: action });
  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /daemon shutdown precedes snapshot publication/,
  );
});

test("only permitted main writers eagerly fill the whole Cargo download snapshot", () => {
  const fetchStep = valid.rustSetupAction
    .split(/^    - /m)
    .find((step) =>
      step.includes("run: cargo fetch --locked --manifest-path Cargo.toml"),
    );
  assert.ok(fetchStep);
  for (const changed of [
    fetchStep.replace(
      "if: inputs.save-cache == 'true' && github.ref == 'refs/heads/main'",
      "if: always()",
    ),
    fetchStep.replace(
      "cargo fetch --locked --manifest-path Cargo.toml",
      "echo fetch-disabled",
    ),
  ]) {
    const result = inspectWorkflowPolicy({
      ...valid,
      rustSetupAction: valid.rustSetupAction.replace(fetchStep, changed),
    });
    assert.equal(result.ok, false);
    assert.match(
      result.errors.join("\n"),
      /fetch the complete locked workspace/,
    );
  }
});
