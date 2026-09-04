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
const workflow = readFileSync(
  path.join(root, ".github/workflows/ci.yml"),
  "utf8"
);
const start = workflow.indexOf("          set -euo pipefail");
const end = workflow.indexOf("\n  validate-rust:", start);
assert.ok(start > 0 && end > start);
const scopeScript = workflow.slice(start, end).replace(/^          /gm, "");

// Execute the actual workflow shell over real git diffs. These assertions
// cover the output gates, not merely the spelling of path filters.
function scopes(files, extraEnv = {}) {
  const cwd = mkdtempSync(path.join(tmpdir(), "synara-ci-scope-"));
  const git = (...args) =>
    execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  try {
    mkdirSync(path.join(cwd, "scripts"));
    for (const script of ["ci-icon-only.mjs", "ci-metadata-only.mjs"]) {
      copyFileSync(
        path.join(root, "scripts", script),
        path.join(cwd, "scripts", script)
      );
    }
    // Version consistency is orthogonal to the scope decision under test.
    writeFileSync(path.join(cwd, "scripts/check-version-consistency.mjs"), "");
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
      "change"
    );
    const output = path.join(cwd, "outputs");
    execFileSync("bash", ["-c", scopeScript], {
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
    return Object.fromEntries(
      readFileSync(output, "utf8")
        .trim()
        .split("\n")
        .map((line) => line.split("="))
    );
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

for (const file of [
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  "src-tauri/Cargo.lock",
  ".cargo/config.toml",
]) {
  test(`${file} alone runs Rust compile and tests`, () => {
    const result = scopes([file]);
    assert.equal(result.validate_rust, "true");
    assert.equal(result.validate_rust_tests, "true");
  });
}
for (const file of [
  "package-lock.json",
  "synara/package.json",
  "synara/src/app/pages/auth/AuthFooter.tsx",
  "devAssets/index.html",
  "packaging/arch/PKGBUILD",
  ".github/workflows/release.yml",
]) {
  test(`${file} alone runs frontend validation`, () => {
    assert.equal(scopes([file]).validate_frontend, "true");
  });
}
test("release PRs into main run iOS gates even for notes-only diffs", () => {
  const result = scopes(["docs/releases/v2.1.2.md"], {
    GITHUB_HEAD_REF: "release/v2.1.2",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "true");
});
test("release pushes run iOS gates even for notes-only diffs", () => {
  const result = scopes(["docs/releases/v2.1.2.md"], {
    EVENT_NAME: "push",
    GITHUB_REF_NAME: "release/v2.1.2",
  });
  assert.equal(result.ios, "true");
  assert.equal(result.ios_ui, "true");
});
test("explicit iOS opt-in cannot be skipped as release metadata", () => {
  assert.equal(
    scopes(["synara-ios/project.yml"], { PR_LABELS: "needs-ios" }).ios,
    "true"
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
    "true"
  );
  assert.equal(
    scopes(["src-tauri/icons/icon.png", ".github/workflows/ci.yml"])
      .validate_rust,
    "true"
  );
});
