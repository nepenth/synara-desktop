import assert from "node:assert/strict";
import test from "node:test";

import {
  cacheFamily,
  selectSupersededCaches,
} from "../prune-actions-caches.mjs";

const lock = "3ec4589d709d36b84761719fe2d860b4f7739cc79c5c83e73dde68eaacb1aec3";
const digest = "8c848423615927ca9bb49b5e4d37118b48a10fb10e7525b9c1e62010115c4166";
const sha = "86361b3d3eb752c6919de041de5e2f5737c6728c";

test("cache families drop lockfile, commit, compiler digest and week suffixes", () => {
  assert.equal(
    cacheFamily(`synara-kache-v1-validate-rust-desktop-Linux-X64-${digest}-${lock}-${sha}`),
    "synara-kache-v1-validate-rust-desktop-Linux-X64",
  );
  assert.equal(
    cacheFamily(`synara-kache-v1-release-macos-macOS-ARM64-${digest}-${lock}-2026-W41`),
    "synara-kache-v1-release-macos-macOS-ARM64",
  );
  assert.equal(cacheFamily(`synara-cargo-v1-Linux-X64-${lock}`), "synara-cargo-v1-Linux-X64");
  assert.equal(
    cacheFamily(`xcode-compilation-v1-macOS-ARM64-${lock}-2026-W41`),
    "xcode-compilation-v1-macOS-ARM64",
  );
});

test("keeps the newest entry per family and ref", () => {
  const caches = [
    { id: 1, ref: "refs/heads/main", key: `synara-kache-v1-validate-rust-desktop-Linux-X64-${digest}-${lock}-${sha}`, created_at: "2026-10-06T10:00:00Z" },
    { id: 2, ref: "refs/heads/main", key: `synara-kache-v1-validate-rust-desktop-Linux-X64-${digest}-${lock}-2026-W41`, created_at: "2026-10-07T10:00:00Z" },
    { id: 3, ref: "refs/heads/main", key: `synara-kache-v1-release-macos-macOS-ARM64-${digest}-${lock}-2026-W41`, created_at: "2026-10-05T10:00:00Z" },
    { id: 4, ref: "refs/pull/9/merge", key: `npm-downloads-Linux-X64-${lock}`, created_at: "2026-10-01T10:00:00Z" },
    { id: 5, ref: "refs/heads/main", key: `npm-downloads-Linux-X64-${lock}`, created_at: "2026-10-02T10:00:00Z" },
  ];
  assert.deepEqual(
    selectSupersededCaches(caches).map((cache) => cache.id),
    [1],
  );
});
