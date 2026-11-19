import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  generateReleaseUpdaterMetadata,
  readWhatsNewNotes,
  updaterNotesFromWhatsNew,
} from "../generate-release-updater-metadata.mjs";

const scriptsDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

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
      requiredPlatforms: ["darwin-x86_64", "darwin-aarch64"],
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
      requiredPlatforms: ["darwin-x86_64", "darwin-aarch64"],
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
      requiredPlatforms: ["darwin-x86_64", "darwin-aarch64"],
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

const writeReleaseUpdaterArtifacts = (dir) => {
  writeArtifact(dir, "macos-updater-artifacts/Synara.app.tar.gz", "macos-signature");
  writeArtifact(
    dir,
    "windows-updater-artifacts/nsis/Synara_2.6.0_x64-setup.exe",
    "nsis-signature"
  );
  writeArtifact(
    dir,
    "windows-updater-artifacts/msi/Synara_2.6.0_x64_en-US.msi",
    "msi-signature"
  );
};

test("maps each Windows installer to its own updater platforms", () =>
  withTempDir((dir) => {
    writeReleaseUpdaterArtifacts(dir);

    const metadata = generateReleaseUpdaterMetadata({
      artifactsDir: dir,
      repo: "nepenth/synara-desktop",
      tag: "v2.6.0",
      version: "2.6.0",
    });

    assert.deepEqual(Object.keys(metadata.platforms).sort(), [
      "darwin-aarch64",
      "darwin-x86_64",
      "windows-x86_64",
      "windows-x86_64-msi",
      "windows-x86_64-nsis",
    ]);
    const nsisUrl =
      "https://github.com/nepenth/synara-desktop/releases/download/v2.6.0/Synara_2.6.0_x64-setup.exe";
    assert.deepEqual(metadata.platforms["windows-x86_64"], {
      signature: "nsis-signature",
      url: nsisUrl,
    });
    assert.deepEqual(metadata.platforms["windows-x86_64-nsis"], {
      signature: "nsis-signature",
      url: nsisUrl,
    });
    assert.deepEqual(metadata.platforms["windows-x86_64-msi"], {
      signature: "msi-signature",
      url: "https://github.com/nepenth/synara-desktop/releases/download/v2.6.0/Synara_2.6.0_x64_en-US.msi",
    });
    // macOS entries are unchanged by the Windows installers.
    assert.equal(metadata.platforms["darwin-aarch64"].signature, "macos-signature");
  }));

test("requires the Windows updater platforms by default", () =>
  withTempDir((dir) => {
    writeArtifact(dir, "macos-updater-artifacts/Synara.app.tar.gz", "macos-signature");

    assert.throws(
      () =>
        generateReleaseUpdaterMetadata({
          artifactsDir: dir,
          repo: "nepenth/synara-desktop",
          tag: "v2.6.0",
          version: "2.6.0",
        }),
      /Missing updater metadata for windows-x86_64/
    );
  }));

test("rejects a Windows installer without its signature", () =>
  withTempDir((dir) => {
    writeReleaseUpdaterArtifacts(dir);
    rmSync(path.join(dir, "windows-updater-artifacts/msi/Synara_2.6.0_x64_en-US.msi.sig"));

    assert.throws(
      () =>
        generateReleaseUpdaterMetadata({
          artifactsDir: dir,
          repo: "nepenth/synara-desktop",
          tag: "v2.6.0",
          version: "2.6.0",
        }),
      /Missing updater signature sidecar/
    );
  }));

test("the update prompt notes are the What's new headline and desktop items", () => {
  const notes = updaterNotesFromWhatsNew({
    version: "3.2.0",
    headline: "Easier settings, cleaner timestamps",
    items: [
      { text: "Settings are grouped by what you want to do.", platforms: ["desktop"] },
      { text: "Settings rows on iPhone respond to taps again.", platforms: ["ios"] },
      { text: "After an update, Synara now shows what changed.", platforms: ["desktop", "ios"] },
    ],
  });
  assert.equal(
    notes,
    [
      "Easier settings, cleaner timestamps",
      "• Settings are grouped by what you want to do.",
      "• After an update, Synara now shows what changed.",
    ].join("\n")
  );
});

test("the repository's notes for the current version make update prompt notes", () => {
  const root = path.resolve(scriptsDir, "..");
  const version = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")).version;
  const notes = updaterNotesFromWhatsNew(
    readWhatsNewNotes({ notesDir: path.join(root, "release-notes"), version })
  );
  assert.ok(notes.split("\n").length >= 2);
  assert.ok(notes.split("\n").slice(1).every((line) => line.startsWith("• ")));
});

test("metadata carries the notes, and the CLI reads them from the notes file", () =>
  withTempDir((dir) => {
    writeReleaseUpdaterArtifacts(dir);
    const metadata = generateReleaseUpdaterMetadata({
      artifactsDir: dir,
      repo: "nepenth/synara-desktop",
      tag: "v2.6.0",
      version: "2.6.0",
      pubDate: "2026-11-18T00:00:00.000Z",
      notes: "Headline\n• One",
    });
    assert.equal(metadata.notes, "Headline\n• One");
    assert.deepEqual(Object.keys(metadata), ["version", "notes", "pub_date", "platforms"]);

    const notesDir = path.join(dir, "notes");
    mkdirSync(notesDir);
    assert.throws(
      () => readWhatsNewNotes({ notesDir, version: "2.6.0" }),
      /Missing What's new notes for the update prompt/
    );
    writeFileSync(
      path.join(notesDir, "v2.6.0.json"),
      JSON.stringify({
        version: "2.6.0",
        headline: "Faster",
        items: [{ text: "Rooms open faster.", platforms: ["desktop", "ios"] }],
      })
    );
    const output = path.join(dir, "latest.json");
    execFileSync(process.execPath, [
      path.join(scriptsDir, "generate-release-updater-metadata.mjs"),
      "--artifacts",
      dir,
      "--repo",
      "nepenth/synara-desktop",
      "--tag",
      "v2.6.0",
      "--version",
      "2.6.0",
      "--notes-dir",
      notesDir,
      "--output",
      output,
    ]);
    assert.equal(JSON.parse(readFileSync(output, "utf8")).notes, "Faster\n• Rooms open faster.");
  }));
