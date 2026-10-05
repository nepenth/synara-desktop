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
  assert.equal(report.limitBytes, 10 * 2 ** 30);
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
  assert.equal(report.limitBytes, 20 * 2 ** 30);
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

test("GitHub's binary storage limit gives exact utilization and remaining capacity", () => {
  const gib = 2 ** 30;
  const input = {
    caches: [],
    usage: { active_caches_size_in_bytes: 8 * gib, active_caches_count: 0 },
    storageLimit: { max_cache_size_gb: 10 },
    retentionLimit: { max_cache_retention_days: 7 },
  };
  const atEightyPercent = summarizeCaches(input);
  assert.equal(atEightyPercent.limitBytes, 10_737_418_240);
  assert.equal(atEightyPercent.utilization, 0.8);
  assert.equal(
    atEightyPercent.limitBytes - atEightyPercent.activeBytes,
    2 * gib,
  );

  // The observed rollout snapshot is decimal 8.69 GB, or binary 8.09 GiB.
  const rollout = summarizeCaches({
    ...input,
    usage: {
      active_caches_size_in_bytes: 8_687_835_058,
      active_caches_count: 9,
    },
  });
  assert.ok(rollout.utilization > 0.809 && rollout.utilization < 0.81);
  assert.equal(rollout.limitBytes - rollout.activeBytes, 2_049_583_182);

  // A concurrent upload can temporarily exceed the limit. Preserve that
  // signal instead of rounding or clamping its utilization to 100%.
  const overLimit = summarizeCaches({
    ...input,
    usage: { active_caches_size_in_bytes: 11 * gib, active_caches_count: 0 },
  });
  assert.equal(overLimit.utilization, 1.1);
  assert.equal(overLimit.limitBytes - overLimit.activeBytes, -gib);
});

test("native macOS host cache cannot satisfy the universal macOS seed requirement", () => {
  const host = {
    key: "synara-kache-v1-release-macos-host-macOS-ARM64-toolchain-lock-head",
    ref: "refs/heads/main",
    size_in_bytes: 799_037_184,
  };
  const input = {
    caches: [host],
    usage: {
      active_caches_size_in_bytes: host.size_in_bytes,
      active_caches_count: 1,
    },
    storageLimit: { max_cache_size_gb: 10 },
    retentionLimit: { max_cache_retention_days: 7 },
  };
  const hostOnly = summarizeCaches(input);
  assert.ok(!hostOnly.missingMainFamilies.includes("release-macos-host"));
  assert.ok(hostOnly.missingMainFamilies.includes("release-macos"));

  const universal = {
    ...host,
    key: "synara-kache-v1-release-macos-macOS-ARM64-toolchain-lock-head",
  };
  const universalOnly = summarizeCaches({ ...input, caches: [universal] });
  assert.ok(universalOnly.missingMainFamilies.includes("release-macos-host"));
  assert.ok(!universalOnly.missingMainFamilies.includes("release-macos"));
  const both = summarizeCaches({ ...input, caches: [host, universal] });
  assert.ok(!both.missingMainFamilies.includes("release-macos-host"));
  assert.ok(!both.missingMainFamilies.includes("release-macos"));
});
