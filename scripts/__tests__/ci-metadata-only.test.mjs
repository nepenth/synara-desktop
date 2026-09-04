import assert from "node:assert/strict";
import test from "node:test";
import { isMetadataOnlyChange } from "../ci-metadata-only.mjs";

test("only release prose is metadata-only", () => {
  assert.equal(
    isMetadataOnlyChange([
      "CHANGELOG.md",
      "docs/releases/v2.1.2.md",
      "synara-ios/release-notes/v2.1.2-en-US.txt",
    ]),
    true
  );
  assert.equal(isMetadataOnlyChange([]), true);
});

test("build inputs, dependencies, and executable files must run validation", () => {
  for (const file of [
    "package.json",
    "package-lock.json",
    "synara/package.json",
    "synara/package-lock.json",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    "src-tauri/tauri.conf.json",
    "synara-ios/project.yml",
    "synara-ios/Synara.xcodeproj/project.pbxproj",
    "packaging/arch/PKGBUILD",
    "devAssets/index.html",
    "synara/src/app/features/settings/about/About.tsx",
    "synara/src/app/pages/auth/AuthFooter.tsx",
    "synara/src/app/pages/client/WelcomePage.tsx",
    ".github/workflows/ci.yml",
    "docs/releases/script.js",
  ]) {
    assert.equal(isMetadataOnlyChange([file]), false, file);
    assert.equal(isMetadataOnlyChange(["CHANGELOG.md", file]), false, file);
  }
});
