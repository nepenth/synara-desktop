import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import { generateReleaseUpdaterMetadata } from "../generate-release-updater-metadata.mjs";

const withTempDir = (fn) => {
  const dir = mkdtempSync(path.join(os.tmpdir(), "synara-updater-metadata-"));
  try {
    return fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
};

const writeArtifact = (root, relativePath, signature) => {
  const artifactPath = path.join(root, relativePath);
  mkdirSync(path.dirname(artifactPath), { recursive: true });
  writeFileSync(artifactPath, "archive");
  writeFileSync(`${artifactPath}.sig`, signature);
};

test("generates static updater metadata for discovered release assets", () =>
  withTempDir((dir) => {
    writeArtifact(
      dir,
      "linux-updater/appimage/Synara_1.2.19_amd64.AppImage.tar.gz",
      "linux-signature"
    );
    writeArtifact(
      dir,
      "macos-updater/macos/Synara_aarch64_x64.app.tar.gz",
      "macos-signature"
    );

    const metadata = generateReleaseUpdaterMetadata({
      artifactsDir: dir,
      repo: "nepenth/synara-desktop",
      tag: "v1.2.19",
      version: "1.2.19",
      pubDate: "2026-06-29T15:00:00.000Z",
    });

    assert.equal(metadata.version, "1.2.19");
    assert.equal(metadata.pub_date, "2026-06-29T15:00:00.000Z");
    assert.deepEqual(Object.keys(metadata.platforms).sort(), [
      "darwin-aarch64",
      "darwin-x86_64",
      "linux-x86_64",
    ]);
    assert.equal(metadata.platforms["linux-x86_64"].signature, "linux-signature");
    assert.equal(metadata.platforms["darwin-aarch64"].signature, "macos-signature");
    assert.equal(metadata.platforms["darwin-x86_64"].signature, "macos-signature");
    assert.equal(
      metadata.platforms["linux-x86_64"].url,
      "https://github.com/nepenth/synara-desktop/releases/download/v1.2.19/Synara_1.2.19_amd64.AppImage.tar.gz"
    );
  }));

test("recognizes macos updater artifact download directories", () =>
  withTempDir((dir) => {
    writeArtifact(
      dir,
      "macos-updater-artifacts/Synara.app.tar.gz",
      "macos-universal-signature"
    );

    const metadata = generateReleaseUpdaterMetadata({
      artifactsDir: dir,
      repo: "nepenth/synara-desktop",
      tag: "v1.2.21",
      version: "1.2.21",
      pubDate: "2026-07-01T00:56:28.000Z",
    });

    assert.deepEqual(Object.keys(metadata.platforms).sort(), [
      "darwin-aarch64",
      "darwin-x86_64",
    ]);
    assert.equal(
      metadata.platforms["darwin-aarch64"].signature,
      "macos-universal-signature"
    );
    assert.equal(
      metadata.platforms["darwin-x86_64"].url,
      "https://github.com/nepenth/synara-desktop/releases/download/v1.2.21/Synara.app.tar.gz"
    );
  }));

test("recognizes flattened macos app updater archives", () =>
  withTempDir((dir) => {
    writeArtifact(dir, "Synara.app.tar.gz", "macos-flattened-signature");

    const metadata = generateReleaseUpdaterMetadata({
      artifactsDir: dir,
      repo: "nepenth/synara-desktop",
      tag: "v1.2.21",
      version: "1.2.21",
    });

    assert.equal(
      metadata.platforms["darwin-aarch64"].signature,
      "macos-flattened-signature"
    );
    assert.equal(
      metadata.platforms["darwin-x86_64"].signature,
      "macos-flattened-signature"
    );
  }));

test("rejects updater archives without signature sidecars", () =>
  withTempDir((dir) => {
    const artifactPath = path.join(
      dir,
      "linux-updater/appimage/Synara_1.2.19_amd64.AppImage.tar.gz"
    );
    mkdirSync(path.dirname(artifactPath), { recursive: true });
    writeFileSync(artifactPath, "archive");

    assert.throws(
      () =>
        generateReleaseUpdaterMetadata({
          artifactsDir: dir,
          repo: "nepenth/synara-desktop",
          tag: "v1.2.19",
          version: "1.2.19",
        }),
      /Missing updater signature sidecar/
    );
  }));

test("requires macos updater platforms by default", () =>
  withTempDir((dir) => {
    writeArtifact(
      dir,
      "linux-updater/appimage/Synara_1.2.19_amd64.AppImage.tar.gz",
      "linux-signature"
    );

    assert.throws(
      () =>
        generateReleaseUpdaterMetadata({
          artifactsDir: dir,
          repo: "nepenth/synara-desktop",
          tag: "v1.2.19",
          version: "1.2.19",
        }),
      /Missing updater metadata for darwin-x86_64/
    );
  }));
