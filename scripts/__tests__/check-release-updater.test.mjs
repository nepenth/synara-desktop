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
  cargoToml: 'tauri-plugin-updater = "2"\ntauri-plugin-process = "2"\n',
  rustLib:
    "tauri_plugin_updater::Builder::new().build(); tauri_plugin_process::init()",
  capabilities: {
    remote: {
      urls: ["http://localhost:*/*"],
    },
    permissions: [
      "core:default",
      "updater:allow-check",
      "updater:allow-download-and-install",
      "process:allow-restart",
    ],
  },
  desktopPackage: {
    dependencies: {
      "@tauri-apps/plugin-updater": "2.11.0",
      "@tauri-apps/plugin-process": "2.3.1",
    },
  },
  releaseWorkflow: `
    - name: Configure release updater channel
      run: node scripts/configure-release-updater.mjs
      env:
        SYNARA_UPDATER_PUBKEY: \${{ vars.SYNARA_UPDATER_PUBKEY }}
        SYNARA_UPDATER_ENDPOINT: \${{ vars.SYNARA_UPDATER_ENDPOINT }}
    env:
      TAURI_SIGNING_PRIVATE_KEY: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
    files: |
      src-tauri/target/universal-apple-darwin/release/bundle/macos/*.sig
      latest.json
    - name: Verify macOS distributable contents
      run: |
        hdiutil attach "$dmg_path" -readonly -nobrowse -mountpoint "$mount_dir"
        codesign --verify --all-architectures --deep --strict --verbose=2 "$mount_dir/Synara.app"
        tar -xzf "$updater_archive" -C "$extract_dir"
        codesign --verify --all-architectures --deep --strict --verbose=2 "$extract_dir/Synara.app"
    updater-metadata:
      needs: [macos]
      steps:
        - uses: actions/download-artifact@v4
        - run: |
            node scripts/generate-release-updater-metadata.mjs \
              --artifacts updater-artifacts \
              --repo "$GITHUB_REPOSITORY" \
              --tag "$GITHUB_REF_NAME" \
              --version "1.2.20" \
              --output latest.json
        - uses: actions/upload-artifact@v4
          with:
            name: gh-release-updater
            path: latest.json
        - uses: softprops/action-gh-release@v3
          with:
            files: |
              release-artifacts/gh-release-updater/latest.json
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

test("release updater gate requires packaged localhost remote capability", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    capabilities: {
      permissions: readyInputs.capabilities.permissions,
    },
    requireEnabled: true,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /packaged localhost webview origin/);
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

test("release updater gate requires workflow updater channel materialization when config is disabled", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    tauriConfig: {
      bundle: {
        createUpdaterArtifacts: false,
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
    requireEnabled: false,
  });

  assert.equal(result.ok, true);
  assert.match(
    result.warnings.join("\n"),
    /configure the release updater channel/,
  );
});

test("release updater gate requires signed update metadata upload", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    releaseWorkflow: `
      env:
        TAURI_SIGNING_PRIVATE_KEY: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
      files: |
        src-tauri/target/release/bundle/appimage/*.sig
        latest.json
    `,
    requireEnabled: true,
  });

  assert.equal(result.ok, false);
  assert.match(
    result.errors.join("\n"),
    /generate and upload signed updater metadata/,
  );
});

test("release updater gate requires updater signature sidecars", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    releaseWorkflow: `
      env:
        TAURI_SIGNING_PRIVATE_KEY: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
      files: |
        latest.json
    `,
    requireEnabled: true,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /signature sidecars/);
});

test("release updater gate requires packaged macOS artifact verification", () => {
  const result = inspectReleaseUpdaterReadiness({
    ...readyInputs,
    releaseWorkflow: readyInputs.releaseWorkflow.replace(
      /- name: Verify macOS distributable contents[\s\S]*?updater-metadata:/,
      "updater-metadata:",
    ),
    requireEnabled: true,
  });

  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /mounted macOS DMG app/);
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
      remote: {
        urls: ["http://localhost:*/*"],
      },
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
