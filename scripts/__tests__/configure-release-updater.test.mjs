import test from "node:test";
import assert from "node:assert/strict";

import { buildReleaseUpdaterConfig } from "../configure-release-updater.mjs";
import { inspectReleaseUpdaterReadiness } from "../check-release-updater.mjs";

const baseConfig = {
  bundle: {
    active: true,
    createUpdaterArtifacts: false,
  },
  productName: "Synara",
};

const pubkey = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

test("release updater config enables artifacts and sets updater channel", () => {
  const config = buildReleaseUpdaterConfig({
    baseConfig,
    pubkey,
    endpoint: "https://updates.synara.chat/latest.json",
  });

  assert.equal(config.bundle.createUpdaterArtifacts, true);
  assert.equal(config.plugins.updater.pubkey, pubkey);
  assert.deepEqual(config.plugins.updater.endpoints, [
    "https://updates.synara.chat/latest.json",
  ]);
  assert.equal(config.productName, "Synara");
});

test("release updater config derives GitHub latest endpoint from repository", () => {
  const config = buildReleaseUpdaterConfig({
    baseConfig,
    pubkey,
    repository: "nepenth/synara-desktop",
  });

  assert.deepEqual(config.plugins.updater.endpoints, [
    "https://github.com/nepenth/synara-desktop/releases/latest/download/latest.json",
  ]);
});

test("release updater config satisfies strict release readiness inspection", () => {
  const tauriConfig = buildReleaseUpdaterConfig({
    baseConfig,
    pubkey,
    repository: "nepenth/synara-desktop",
  });

  const result = inspectReleaseUpdaterReadiness({
    tauriConfig,
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
        "@tauri-apps/plugin-updater": "2.10.1",
        "@tauri-apps/plugin-process": "2.3.1",
      },
    },
    releaseWorkflow: `
      - name: Configure release updater channel
        run: node scripts/configure-release-updater.mjs
        env:
          SYNARA_UPDATER_PUBKEY: \${{ vars.SYNARA_UPDATER_PUBKEY }}
          SYNARA_UPDATER_ENDPOINT: \${{ vars.SYNARA_UPDATER_ENDPOINT }}
      - name: Build macOS universal release packages
        run: npm run tauri build -- --target universal-apple-darwin
        env:
          APPLE_ID: \${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: \${{ secrets.APPLE_APP_SPECIFIC_PASSWORD }}
          APPLE_TEAM_ID: \${{ secrets.APPLE_TEAM_ID }}
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
    requireEnabled: true,
  });

  assert.equal(result.ok, true);
  assert.deepEqual(result.errors, []);
});

test("release updater config rejects missing or placeholder public keys", () => {
  assert.throws(
    () =>
      buildReleaseUpdaterConfig({
        baseConfig,
        pubkey: "CHANGE_ME",
        endpoint: "https://updates.synara.chat/latest.json",
      }),
    /production Tauri updater public key/,
  );
});

test("release updater config rejects non-production endpoints", () => {
  assert.throws(
    () =>
      buildReleaseUpdaterConfig({
        baseConfig,
        pubkey,
        endpoint: "http://localhost:8080/latest.json",
      }),
    /Invalid production updater endpoint/,
  );
});
