import assert from "node:assert/strict";
import test from "node:test";

import { isMetadataOnlyChange } from "../ci-metadata-only.mjs";

test("version bumps and release notes are metadata-only", () => {
  assert.equal(
    isMetadataOnlyChange([
      "package.json",
      "package-lock.json",
      "synara/package.json",
      "synara/package-lock.json",
      "src-tauri/tauri.conf.json",
      "synara-ios/project.yml",
      "synara-ios/Synara.xcodeproj/project.pbxproj",
      "CHANGELOG.md",
      "docs/releases/v2.1.2.md",
      "synara-ios/release-notes/v2.1.2-en-US.txt",
    ]),
    true
  );
});

test("empty diffs are metadata-only", () => {
  assert.equal(isMetadataOnlyChange([]), true);
});

test("product or workflow changes are not metadata-only", () => {
  assert.equal(
    isMetadataOnlyChange(["package.json", "synara/src/app/utils/common.ts"]),
    false
  );
  assert.equal(isMetadataOnlyChange([".github/workflows/ci.yml"]), false);
  assert.equal(
    isMetadataOnlyChange(["src-tauri/src/matrix/auth/product.rs"]),
    false
  );
  assert.equal(
    isMetadataOnlyChange(["src-tauri/Cargo.toml", "src-tauri/Cargo.lock"]),
    false,
    "Dependabot rust lockfile bumps must run cargo validation"
  );
});
