import test from "node:test";
import assert from "node:assert/strict";

import { buildReleaseUpdaterConfig } from "../configure-release-updater.mjs";

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

test("release updater config rejects missing or placeholder public keys", () => {
  assert.throws(
    () =>
      buildReleaseUpdaterConfig({
        baseConfig,
        pubkey: "CHANGE_ME",
        endpoint: "https://updates.synara.chat/latest.json",
      }),
    /production Tauri updater public key/
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
    /Invalid production updater endpoint/
  );
});
