import test from "node:test";
import assert from "node:assert/strict";

import { isIconOnlyChange } from "../ci-icon-only.mjs";

test("accepts reviewed icon assets plus exact icon infrastructure", () => {
  assert.equal(
    isIconOnlyChange([
      "assets/branding/synara-app-icon-master.png",
      "src-tauri/icons/icon.icns",
      "synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png",
      "scripts/check-app-icons.mjs",
      "docs/design/app-icon-refresh/README.md",
    ]),
    true
  );
});

test("accepts release metadata only when a production icon also changed", () => {
  assert.equal(
    isIconOnlyChange([
      "src-tauri/icons/icon.png",
      "CHANGELOG.md",
      "docs/releases/v2.1.16.md",
      "synara-ios/release-notes/v2.1.16-en-US.txt",
    ]),
    true
  );
  assert.equal(isIconOnlyChange(["package.json", "CHANGELOG.md"]), false);
});

test("fails closed for production code, dependencies, and unrelated workflows", () => {
  for (const unrelated of [
    "src-tauri/src/main.rs",
    "synara/src/app/App.tsx",
    "synara-ios/Synara/App/SynaraApp.swift",
    "package-lock.json.backup",
    "package.json",
    "package-lock.json",
    "src-tauri/tauri.conf.json",
    ".github/workflows/ci.yml",
    ".github/workflows/desktop-package-smoke.yml",
    ".github/workflows/release.yml",
    "scripts/build-runtime.mjs",
  ]) {
    assert.equal(
      isIconOnlyChange(["src-tauri/icons/icon.png", unrelated]),
      false,
      unrelated
    );
  }
});

test("does not treat tray-only or empty changes as app-icon releases", () => {
  assert.equal(isIconOnlyChange([]), false);
  assert.equal(isIconOnlyChange(["src-tauri/icons/tray-template.png"]), false);
});
