import test from "node:test";
import assert from "node:assert/strict";

import { inspectReleaseUpdaterReadiness } from "../check-release-updater.mjs";

const readyInputs = {
  tauriConfig: {
    bundle: {
      createUpdaterArtifacts: true,
    },
    plugins: {
      updater: {
        pubkey:
          "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        endpoints: ["https://updates.synara.chat/latest.json"],
      },
    },
  },
  cargoToml: 'tauri-plugin-updater = "2"\n',
  rustLib: "tauri_plugin_updater::Builder::new().build()",
  capabilities: {
    permissions: ["core:default", "updater:allow-check"],
  },
  desktopPackage: {
    dependencies: {
      "@tauri-apps/plugin-updater": "2.11.0",
    },
  },
  releaseWorkflow: `
    env:
      TAURI_SIGNING_PRIVATE_KEY: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
    files: |
      src-tauri/target/release/bundle/appimage/*.sig
      latest.json
  `,
};

test("release updater gate accepts complete production updater wiring", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    requireEnabled: true,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("release updater gate fails when release config forces updater artifacts off", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    releaseWorkflow: '{"bundle":{"createUpdaterArtifacts":false}}',
    requireEnabled: true,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /createUpdaterArtifacts to false/);
});

test("non-release check warns instead of failing while updater is intentionally disabled", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    tauriConfig: {
      bundle: {
        createUpdaterArtifacts: false,
      },
    },
    cargoToml: "",
    rustLib: "",
    capabilities: {
      permissions: ["core:default"],
    },
    desktopPackage: {
      dependencies: {},
    },
    releaseWorkflow: '{"bundle":{"createUpdaterArtifacts":false}}',
    requireEnabled: false,
  });

  assert.equal(result.ok, true);
  assert.equal(result.errors.length, 0);
  assert.ok(result.warnings.length >= 8);
});
