import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { inspectAppIcons } from "../check-app-icons.mjs";

test("reviewed app icons satisfy every platform contract", () => {
  const files = inspectAppIcons();
  const manifest = JSON.parse(
    readFileSync("assets/branding/app-icon-manifest.json", "utf8")
  );
  assert.equal(Object.keys(files).length, 35);
  assert.deepEqual(
    Object.keys(files).sort(),
    Object.keys(manifest.files).sort()
  );
  for (const [path, metadata] of Object.entries(files)) {
    assert.equal(metadata.sha256, manifest.files[path].sha256, path);
  }
});
