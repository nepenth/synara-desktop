import assert from "node:assert/strict";
import test from "node:test";

import { planQuickChecks } from "../check-quick.mjs";

const labels = (plan) => plan.clippy.map((step) => step.label);

test("docs and scripts alone lint no Rust and check no renderer files", () => {
  const plan = planQuickChecks(["docs/build-and-release.md", "scripts/ci-scopes.sh"]);
  assert.deepEqual(plan, { clippy: [], prettier: [], eslint: [], typecheck: false });
});

test("only the touched workspace crates are linted", () => {
  assert.deepEqual(labels(planQuickChecks(["crates/synara-nse-core/src/lib.rs"])), ["clippy synara-nse-core"]);
  assert.deepEqual(labels(planQuickChecks(["src-tauri/src/main.rs"])), ["clippy desktop shell"]);
  const core = planQuickChecks(["crates/synara-core/src/lib.rs", "crates/synara-core-bindgen/src/main.rs"]);
  assert.deepEqual(labels(core), [
    "clippy synara-core, synara-core-bindgen",
    "clippy synara-core desktop-only features",
  ]);
  assert.deepEqual(core.clippy[0].args.slice(-3), ["--", "-D", "warnings"]);
  assert.equal(planQuickChecks(["src-tauri/src/main.rs"]).clippy[0].cwd, "src-tauri");
});

test("workspace-wide Rust inputs lint every crate", () => {
  for (const file of ["Cargo.lock", "Cargo.toml", "rust-toolchain.toml", ".cargo/config.toml"]) {
    assert.deepEqual(
      labels(planQuickChecks([file])),
      [
        "clippy synara-core, synara-nse-core, synara-core-bindgen",
        "clippy synara-core desktop-only features",
        "clippy desktop shell",
      ],
      file,
    );
  }
});

test("renderer files get prettier, eslint within its globs, and typecheck for TypeScript", () => {
  const plan = planQuickChecks([
    "synara/src/app/App.tsx",
    "synara/e2e/smoke.spec.ts",
    "synara/scripts/build.mjs",
    "synara/README.md",
    "synara/public/logo.png",
  ]);
  assert.deepEqual(plan.prettier, ["src/app/App.tsx", "e2e/smoke.spec.ts", "scripts/build.mjs", "README.md"]);
  assert.deepEqual(plan.eslint, ["src/app/App.tsx", "e2e/smoke.spec.ts"]);
  assert.equal(plan.typecheck, true);
  assert.equal(planQuickChecks(["synara/README.md"]).typecheck, false);
  assert.equal(planQuickChecks(["synara/tsconfig.json"]).typecheck, true);
});
