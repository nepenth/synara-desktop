import assert from "node:assert/strict";
import test from "node:test";
import { summarizeCaches } from "../audit-build-cache.mjs";

test("PR and tag snapshots do not count as reusable main cache seeds", () => {
  const report = summarizeCaches({
    caches: [
      {
        key: "synara-kache-v1-validate-rust-desktop-Linux-x64-old",
        ref: "refs/pull/10/merge",
        size_in_bytes: 300,
      },
      {
        key: "synara-kache-v1-release-macos-Darwin-arm64-old",
        ref: "refs/tags/v1.0.0",
        size_in_bytes: 200,
      },
      {
        key: "synara-kache-v1-release-linux-deb-Linux-x64-current",
        ref: "refs/heads/main",
        size_in_bytes: 100,
      },
      {
        key: "synara-kache-v1-release-linux-deb-Linux-x64-fallback",
        ref: "refs/heads/main",
        size_in_bytes: 50,
      },
    ],
    usage: { active_caches_size_in_bytes: 650, active_caches_count: 4 },
    storageLimit: { max_cache_size_gb: 10 },
    retentionLimit: { max_cache_retention_days: 7 },
  });
  assert.deepEqual(report.missingMainFamilies, [
    "validate-rust-desktop",
    "ci-synara-core-apple-simulator-arm64",
    "release-linux-arch",
    "release-macos-host",
    "release-macos",
    "release-synara-core-apple-device",
    "cargo-downloads",
    "xcode-compilation",
  ]);
  assert.deepEqual(report.scopes, [
    { ref: "refs/pull/10/merge", count: 1, bytes: 300 },
    { ref: "refs/tags/v1.0.0", count: 1, bytes: 200 },
    { ref: "refs/heads/main", count: 2, bytes: 150 },
  ]);
  assert.equal(report.limitBytes, 10_000_000_000);
  assert.equal(report.retentionDays, 7);
  assert.equal(report.caches[0].size_in_bytes, 300);
});

test("an empty cache inventory needs all seed families", () => {
  const report = summarizeCaches({
    caches: [],
    usage: { active_caches_size_in_bytes: 0, active_caches_count: 0 },
    storageLimit: { max_cache_size_gb: 20 },
    retentionLimit: { max_cache_retention_days: 14 },
  });
  assert.equal(report.missingMainFamilies.length, 9);
  assert.deepEqual(report.scopes, []);
  assert.equal(report.utilization, 0);
  assert.equal(report.limitBytes, 20_000_000_000);
});

test("Swift compiler cache seeds must belong to main, independently of Rust seeds", () => {
  const cache = {
    key: "xcode-compilation-v1-macOS-ARM64-toolchain-2026-W41",
    ref: "refs/pull/20/merge",
    size_in_bytes: 100,
  };
  const input = {
    caches: [cache],
    usage: { active_caches_size_in_bytes: 100, active_caches_count: 1 },
    storageLimit: { max_cache_size_gb: 10 },
    retentionLimit: { max_cache_retention_days: 7 },
  };
  assert.ok(
    summarizeCaches(input).missingMainFamilies.includes("xcode-compilation"),
  );
  const main = summarizeCaches({
    ...input,
    caches: [{ ...cache, ref: "refs/heads/main" }],
  });
  assert.ok(!main.missingMainFamilies.includes("xcode-compilation"));
  assert.equal(main.missingMainFamilies.length, 8);
});

test("obsolete target snapshots neither satisfy Kache seeds nor disappear from storage accounting", () => {
  const report = summarizeCaches({
    caches: [
      {
        key: "v0-rust-validate-rust-desktop-Linux-x64-old",
        ref: "refs/heads/main",
        size_in_bytes: 123,
      },
      {
        key: "synara-cargo-v1-Linux-X64-lock",
        ref: "refs/heads/main",
        size_in_bytes: 25,
      },
      {
        key: "synara-kache-v1-release-macos-toolchain-version-lock-head",
        ref: "refs/heads/main",
        size_in_bytes: 52,
      },
    ],
    usage: { active_caches_size_in_bytes: 200, active_caches_count: 3 },
    storageLimit: { max_cache_size_gb: 10 },
    retentionLimit: { max_cache_retention_days: 7 },
  });
  assert.equal(report.legacyRustCount, 1);
  assert.equal(report.legacyRustBytes, 123);
  assert.ok(report.missingMainFamilies.includes("validate-rust-desktop"));
  assert.ok(!report.missingMainFamilies.includes("release-macos"));
  assert.ok(!report.missingMainFamilies.includes("cargo-downloads"));
  assert.equal(report.scopes[0].bytes, 200);
});
