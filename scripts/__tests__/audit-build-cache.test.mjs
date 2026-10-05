import assert from "node:assert/strict";
import test from "node:test";
import { summarizeCaches } from "../audit-build-cache.mjs";

test("PR and tag snapshots do not count as reusable main cache seeds", () => {
  const report = summarizeCaches({
    caches: [
      {
        key: "v0-rust-validate-rust-desktop-Linux-x64-old",
        ref: "refs/pull/10/merge",
        size_in_bytes: 300,
      },
      {
        key: "v0-rust-release-macos-Darwin-arm64-old",
        ref: "refs/tags/v1.0.0",
        size_in_bytes: 200,
      },
      {
        key: "v0-rust-release-linux-deb-Linux-x64-current",
        ref: "refs/heads/main",
        size_in_bytes: 100,
      },
      {
        key: "v0-rust-release-linux-deb-Linux-x64-fallback",
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
    "release-macos",
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
  assert.equal(report.missingMainFamilies.length, 4);
  assert.deepEqual(report.scopes, []);
  assert.equal(report.utilization, 0);
  assert.equal(report.limitBytes, 20_000_000_000);
});
