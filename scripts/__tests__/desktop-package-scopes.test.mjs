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
  path.join(root, ".github/workflows/desktop-package-smoke.yml"),
  "utf8",
);
const start = workflow.indexOf("          # Always-on full smoke");
const end = workflow.indexOf("\n  linux-deb:", start);
assert.ok(start > 0 && end > start);
const scopeScript = workflow.slice(start, end).replace(/^          /gm, "");

// Exercise the shipping workflow over real Git diffs, including its early
// metadata/icon branches. A list of expected filter strings is insufficient.
function scopes(files, extraEnv = {}) {
  const cwd = mkdtempSync(path.join(tmpdir(), "synara-package-scope-"));
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
        path.join(cwd, "scripts", script),
      );
    }
    // Icon validation itself has independent tests; this fixture tests scopes.
    writeFileSync(path.join(cwd, "scripts/check-app-icons.mjs"), "");
    git("init", "--quiet");
    git("config", "user.email", "test@example.org");
    git("config", "user.name", "Package scope fixture");
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
    execFileSync("bash", ["-c", scopeScript], {
      cwd,
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        BASE_SHA: base,
        HEAD_SHA: git("rev-parse", "HEAD"),
        GITHUB_OUTPUT: output,
        GITHUB_EVENT_NAME: "pull_request",
        BASE_REF: "main",
        HEAD_REF: "feature/example",
        PR_LABELS: "",
        ...extraEnv,
      },
    });
    return Object.fromEntries(
      readFileSync(output, "utf8")
        .trim()
        .split("\n")
        .map((line) => line.split("=")),
    );
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

for (const file of [
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".cargo/config.toml",
  "crates/synara-core/Cargo.toml",
  "crates/synara-core/src/core.rs",
  "crates/synara-core/src/core/notifications.rs",
  "crates/synara-core/src/app/notifications/decision.rs",
  "crates/synara-core/src/shared_core_ffi/inbox_notifications.rs",
  "crates/synara-nse-core/src/lib.rs",
  "crates/synara-core-bindgen/src/main.rs",
]) {
  test(`${file} triggers desktop package proof`, () => {
    assert.equal(scopes([file]).packages, "true");
  });
}

test("release prose and unrelated documentation retain the cheap path", () => {
  assert.equal(scopes(["docs/releases/v2.1.2.md"]).packages, "false");
  assert.equal(scopes(["docs/example.md"]).packages, "false");
});
test("an icon or release note cannot hide a Core change", () => {
  for (const file of ["src-tauri/icons/icon.png", "docs/releases/v2.1.2.md"]) {
    assert.equal(
      scopes([file, "crates/synara-core/src/core.rs"]).packages,
      "true",
    );
  }
});
test("manual dispatch proves packages even for a prose-only diff", () => {
  assert.equal(
    scopes(["docs/example.md"], { GITHUB_EVENT_NAME: "workflow_dispatch" })
      .packages,
    "true",
  );
});
test("release PR package policy retains its explicit artifact opt-in", () => {
  const env = { HEAD_REF: "release/v2.1.2" };
  assert.equal(scopes(["Cargo.lock"], env).packages, "false");
  assert.equal(
    scopes(["Cargo.lock"], { ...env, PR_LABELS: "needs-package" }).packages,
    "true",
  );
});
test("historical integration branch package policy retains explicit opt-in", () => {
  const env = { BASE_REF: "feature/matrix-rust-sdk-full-replacement" };
  assert.equal(scopes(["Cargo.lock"], env).packages, "false");
  assert.equal(
    scopes(["Cargo.lock"], { ...env, PR_LABELS: "needs-package" }).packages,
    "true",
  );
});
