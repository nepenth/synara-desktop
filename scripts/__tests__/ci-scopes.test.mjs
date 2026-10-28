import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "../..");

// Execute the actual scope script over real git diffs. These assertions
// cover the output gates, not merely the spelling of path filters.
// With `reuseExit`, the workflows are committed and the proof lookup is a stub
// that exits with that code and records the hash it was asked about.
function scopes(files, extraEnv = {}, { reuseExit } = {}) {
  const cwd = mkdtempSync(path.join(tmpdir(), "synara-ci-scope-"));
  const git = (...args) =>
    execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  try {
    mkdirSync(path.join(cwd, "scripts"));
    for (const script of [
      "ci-icon-only.mjs",
      "ci-metadata-only.mjs",
      "ci-scopes.sh",
      "ios-test-inputs.mjs",
    ]) {
      copyFileSync(
        path.join(root, "scripts", script),
        path.join(cwd, "scripts", script),
      );
    }
    // Version consistency is orthogonal to the scope decision under test.
    writeFileSync(path.join(cwd, "scripts/check-version-consistency.mjs"), "");
    const lookupLog = path.join(cwd, "lookup.log");
    if (reuseExit !== undefined) {
      mkdirSync(path.join(cwd, ".github/workflows"), { recursive: true });
      for (const workflow of ["ci.yml", "release.yml"]) {
        copyFileSync(
          path.join(root, ".github/workflows", workflow),
          path.join(cwd, ".github/workflows", workflow),
        );
      }
      writeFileSync(
        path.join(cwd, "scripts/reuse-proven-ios-tests.mjs"),
        `import { appendFileSync } from "node:fs";\n` +
          `appendFileSync(${JSON.stringify(lookupLog)}, process.argv[2] + "\\n");\n` +
          `process.exit(${reuseExit});\n`,
      );
      extraEnv = { IOS_PROOF_LOOKUP: "1", ...extraEnv };
    }
    git("init", "--quiet");
    git("config", "user.email", "test@example.org");
    git("config", "user.name", "CI scope fixture");
    git("add", ".");
    git("-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "base");
    const base = git("rev-parse", "HEAD");
    for (const file of files) {
      mkdirSync(path.dirname(path.join(cwd, file)), { recursive: true });
      writeFileSync(path.join(cwd, file), "changed\n");
    }
    git("add", ".");
    git(
      "-c",
      "core.hooksPath=/dev/null",
      "commit",
      "--quiet",
      "--allow-empty",
      "-m",
      "change",
    );
    const output = path.join(cwd, "outputs");
    execFileSync("bash", ["scripts/ci-scopes.sh"], {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        BASE_SHA: base,
        HEAD_SHA: git("rev-parse", "HEAD"),
        GITHUB_OUTPUT: output,
        EVENT_NAME: "pull_request",
        PR_LABELS: "",
        GITHUB_BASE_REF: "main",
        GITHUB_HEAD_REF: "feature/example",
        GITHUB_REF_NAME: "1/merge",
        ...extraEnv,
      },
    });
    const result = Object.fromEntries(
      readFileSync(output, "utf8")
        .trim()
        .split("\n")
        .map((line) => line.split("=")),
    );
    let lookups = [];
    try {
      lookups = readFileSync(lookupLog, "utf8").trim().split("\n").filter(Boolean);
    } catch {}
    Object.defineProperty(result, "lookups", { value: lookups });
    return result;
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

for (const file of [
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".cargo/config.toml",
]) {
  test(`${file} alone runs Rust lint and tests`, () => {
    assert.equal(scopes([file]).validate_rust, "true");
  });
}
for (const file of [
  "package-lock.json",
  "synara/package.json",
  "synara/src/app/pages/auth/AuthFooter.tsx",
  "devAssets/index.html",
  "packaging/arch/PKGBUILD",
  ".github/workflows/release.yml",
  ".github/actions/setup-node/action.yml",
]) {
  test(`${file} alone runs frontend validation`, () => {
    assert.equal(scopes([file]).validate_frontend, "true");
  });
}
test("release PRs run iOS unit tests even for notes-only diffs; UI tests stay nightly", () => {
  const result = scopes(["docs/releases/v2.1.2.md"], {
    GITHUB_HEAD_REF: "release/v2.1.2",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "false");
});
test("explicit iOS opt-in cannot be skipped as release metadata", () => {
  assert.equal(
    scopes(["synara-ios/project.yml"], { PR_LABELS: "needs-ios" }).ios,
    "true",
  );
  const result = scopes(["docs/releases/v2.1.2.md"], {
    PR_LABELS: "needs-ios-ui",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "true");
});
test("ordinary prose-only changes retain the cheap path", () => {
  const result = scopes(["docs/releases/v2.1.2.md"]);
  assert.equal(result.validate_rust, "false");
  assert.equal(result.validate_frontend, "false");
  assert.equal(result.ios, "false");
});
test("adding an icon cannot hide a workflow or dependency edit", () => {
  assert.equal(
    scopes(["src-tauri/icons/icon.png", "package-lock.json"]).validate_frontend,
    "true",
  );
  assert.equal(
    scopes(["src-tauri/icons/icon.png", ".github/workflows/ci.yml"])
      .validate_rust,
    "true",
  );
});

test("iOS path changes on an unlabeled feature PR run the compile gate only", () => {
  for (const file of [
    "synara-ios/Synara/App/SynaraApp.swift",
    "crates/synara-core/src/shared_core_ffi/session_lifecycle.rs",
    "crates/synara-core/src/ffi.rs",
    "crates/synara-core/src/core.rs",
    "crates/synara-core/src/core/notifications.rs",
    "crates/synara-core/src/app/notifications/decision.rs",
    "crates/synara-core/src/shared_core_ffi/inbox_notifications.rs",
    "crates/synara-core/Cargo.toml",
    "crates/synara-nse-core/src/lib.rs",
    "crates/synara-core-bindgen/Cargo.toml",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo/config.toml",
    "scripts/generate-synara-core-swift.sh",
    "scripts/generate-xcode-project.mjs",
    ".github/actions/setup-xcode-cache/action.yml",
    ".github/actions/save-xcode-cache/action.yml",
  ]) {
    const result = scopes([file]);
    assert.equal(result.ios, "false", `${file}: simulator lane stays skipped`);
    assert.equal(result.ios_ui, "false");
    assert.equal(result.ios_compile, "true", `${file}: compile gate runs`);
  }
});
test("Swift cache and project generation changes run the Apple unit gate on main", () => {
  for (const file of [
    "scripts/generate-xcode-project.mjs",
    ".github/actions/setup-xcode-cache/action.yml",
    ".github/actions/save-xcode-cache/action.yml",
    "synara-ios/scripts/lib/xcode-cache.sh",
  ]) {
    const result = scopes([file], {
      EVENT_NAME: "push",
      GITHUB_REF_NAME: "main",
    });
    assert.equal(result.ios, "true", file);
    assert.equal(result.ios_ui, "false", file);
    assert.equal(result.ios_compile, "false", file);
  }
});
test("the compile gate never runs alongside or instead of a scheduled simulator lane", () => {
  // Labeled opt-in: full lane, compile gate redundant.
  const labeled = scopes(["synara-ios/project.yml"], {
    PR_LABELS: "needs-ios",
  });
  assert.equal(labeled.ios, "true");
  assert.equal(labeled.ios_compile, "false");
  // Release PR: full lane.
  const release = scopes(["synara-ios/project.yml"], {
    GITHUB_HEAD_REF: "release/v2.1.2",
  });
  assert.equal(release.ios, "true");
  assert.equal(release.ios_compile, "false");
  // Push to main with iOS paths: unit lane already runs.
  const mainPush = scopes(["synara-ios/project.yml"], {
    EVENT_NAME: "push",
    GITHUB_REF_NAME: "main",
  });
  assert.equal(mainPush.ios, "true");
  assert.equal(mainPush.ios_ui, "false");
  assert.equal(mainPush.ios_compile, "false");
  // No iOS paths at all: nothing Apple-side runs.
  for (const result of [
    scopes(["docs/releases/v2.1.2.md"]),
    scopes(["synara/src/app/pages/auth/AuthFooter.tsx"]),
  ]) {
    assert.equal(result.ios, "false");
    assert.equal(result.ios_compile, "false");
  }
  // Full-scope events already run everything.
  assert.equal(
    scopes(["synara-ios/project.yml"], { EVENT_NAME: "workflow_dispatch" })
      .ios_compile,
    "false",
  );
});
test("NSE production feature guard changes run the unit gate on main", () => {
  const result = scopes(
    ["scripts/check-synara-nse-core-production-features.mjs"],
    {
      EVENT_NAME: "push",
      GITHUB_REF_NAME: "main",
    },
  );
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "false");
});
test("NSE archive export guard changes run the unit gate on main", () => {
  const result = scopes(["scripts/check-synara-nse-core-archive-exports.sh"], {
    EVENT_NAME: "push",
    GITHUB_REF_NAME: "main",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "false");
});

test("shared Core changes on main run Apple unit tests without enabling UI tests", () => {
  for (const file of [
    "crates/synara-core/src/app/notifications/decision.rs",
    "crates/synara-core/src/shared_core_ffi/inbox_notifications.rs",
    "crates/synara-core/src/core.rs",
  ]) {
    const result = scopes([file], {
      EVENT_NAME: "push",
      GITHUB_REF_NAME: "main",
    });
    assert.equal(result.ios, "true", file);
    assert.equal(result.ios_ui, "false", file);
    assert.equal(result.ios_compile, "false", file);
  }
});
test("scheduled and manual full runs retain both Apple test lanes", () => {
  for (const env of [
    { EVENT_NAME: "schedule" },
    { EVENT_NAME: "workflow_dispatch" },
  ]) {
    const result = scopes(["crates/synara-core/src/core.rs"], env);
    assert.equal(result.ios, "true");
    assert.equal(result.ios_ui, "true");
    assert.equal(result.ios_compile, "false");
  }
});
test("a main push with missing diff metadata still keeps UI tests opt-in", () => {
  const result = scopes(["crates/synara-core/src/core.rs"], {
    EVENT_NAME: "push",
    GITHUB_REF_NAME: "main",
    BASE_SHA: "",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "false");
});

test("shared Core sources run the live Synapse proofs; renderer-only changes do not", () => {
  for (const file of [
    "crates/synara-core/src/app/timeline/live.rs",
    "crates/synara-core/src/app/send/room_queue.rs",
    "src-tauri/src/matrix/send/live_synapse_proof/tests.rs",
    "src-tauri/src/bridge/send_text.rs",
    "integration/synapse/homeserver.yaml",
    "Cargo.lock",
  ]) {
    assert.equal(scopes([file]).synapse, "true", file);
  }
  for (const file of [
    "synara/src/app/pages/auth/AuthFooter.tsx",
    "docs/releases/v2.1.2.md",
    "synara-ios/Synara/App/SynaraApp.swift",
  ]) {
    assert.equal(scopes([file]).synapse, "false", file);
  }
});

test("package builds and cache seeding run nightly or on request only", () => {
  assert.equal(scopes(["src-tauri/src/lib.rs"]).packages, "false");
  assert.equal(
    scopes(["src-tauri/src/lib.rs"], { EVENT_NAME: "push", GITHUB_REF_NAME: "main" }).packages,
    "false",
  );
  assert.equal(scopes([], { EVENT_NAME: "schedule" }).packages, "true");
  assert.equal(scopes([], { EVENT_NAME: "workflow_dispatch" }).packages, "false");
  assert.equal(
    scopes([], { EVENT_NAME: "workflow_dispatch", PACKAGES_INPUT: "true" }).packages,
    "true",
  );
});

test("a release PR skips iOS simulator tests an earlier run proved on the same inputs", () => {
  const result = scopes(
    ["docs/releases/v2.1.2.md", "synara-ios/Synara/App.swift"],
    { GITHUB_HEAD_REF: "release/v2.1.2" },
    { reuseExit: 0 },
  );
  assert.equal(result.ios, "false");
  assert.equal(result.ios_reused, "true");
  // The proof built the same inputs, so the compile gate is not added back.
  assert.equal(result.ios_compile, "false");
  assert.match(result.ios_inputs_hash, /^[0-9a-f]{64}$/);
  assert.deepEqual(result.lookups, [result.ios_inputs_hash]);
});
test("a release PR without a proof runs iOS simulator tests with the hash to record", () => {
  const result = scopes(
    ["docs/releases/v2.1.2.md"],
    { GITHUB_HEAD_REF: "release/v2.1.2" },
    { reuseExit: 1 },
  );
  assert.equal(result.ios, "true");
  assert.equal(result.ios_reused, "false");
  assert.match(result.ios_inputs_hash, /^[0-9a-f]{64}$/);
});
test("nightly and main pushes hash the iOS inputs but never reuse a proof", () => {
  for (const env of [
    { EVENT_NAME: "schedule" },
    { EVENT_NAME: "push", GITHUB_BASE_REF: "", GITHUB_HEAD_REF: "" },
  ]) {
    const result = scopes(["synara-ios/project.yml"], env, { reuseExit: 0 });
    assert.equal(result.ios, "true");
    assert.equal(result.ios_reused, "false");
    assert.match(result.ios_inputs_hash, /^[0-9a-f]{64}$/);
    assert.deepEqual(result.lookups, []);
  }
});
test("scopes that do not schedule iOS tests neither hash nor look up a proof", () => {
  const result = scopes(["synara/src/app/pages/auth/AuthFooter.tsx"], {}, { reuseExit: 0 });
  assert.equal(result.ios, "false");
  assert.equal(result.ios_reused, "false");
  assert.equal(result.ios_inputs_hash, "");
  assert.deepEqual(result.lookups, []);
});
test("hash inputs outside the old hand-kept list still schedule the iOS compile gate", () => {
  for (const file of [
    "scripts/check-xcode-local-package-graph.mjs",
    "scripts/lib/ffi-surface.mjs",
    "src-tauri/Cargo.toml",
  ]) {
    assert.equal(scopes([file]).ios_compile, "true", file);
  }
});
