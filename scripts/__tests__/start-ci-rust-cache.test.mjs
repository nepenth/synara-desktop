import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { startCiRustCache } from "../start-ci-rust-cache.mjs";

function fixture(run) {
  const directory = mkdtempSync(
    path.join(os.tmpdir(), "synara-ci-cache-start-"),
  );
  const binary = path.join(directory, "kache");
  const config = path.join(directory, "kache.toml");
  const log = path.join(directory, "commands.jsonl");
  writeFileSync(config, "[cache]\nlocal_only = true\n");
  writeFileSync(log, "");
  writeFileSync(
    binary,
    `#!${process.execPath}
const fs = require('node:fs');
const path = require('node:path');
const args = process.argv.slice(2);
const scenario = process.env.FIXTURE_SCENARIO;
fs.appendFileSync(process.env.FIXTURE_LOG, JSON.stringify(args) + '\\n');
if (args[0] === '--version') { console.log(scenario === 'wrong-binary' ? 'kache 0.28.1' : 'kache 1.0.0'); process.exit(0); }
if (args.join(' ') === 'daemon start') { process.exit(scenario === 'start-failed' ? 7 : 0); }
if (scenario === 'invalid-json') { console.log('broken response'); process.exit(0); }
const status = {
  schema_version: 1, success: true, daemon_running: true, daemon_version: '1.0.0',
  socket: path.join(process.env.KACHE_RUNTIME_DIR, 'daemon.sock'), daemon_config_path: process.env.KACHE_CONFIG,
};
const stats = {
  schema_version: 1, success: true, daemon_connected: true, daemon_version: '1.0.0',
  disk: { store_limit_bytes: 5368709120 },
  stores: [{ path: process.env.KACHE_CACHE_DIR, max_size: 5368709120, error: null }],
};
if (scenario === 'not-running') status.daemon_running = false;
if (scenario === 'wrong-version') status.daemon_version = '0.28.1';
if (scenario === 'wrong-socket') status.socket += '.wrong';
if (scenario === 'wrong-config') status.daemon_config_path += '.wrong';
if (scenario === 'wrong-budget') stats.disk.store_limit_bytes = stats.stores[0].max_size = 10737418240;
if (scenario === 'wrong-store') stats.stores[0].path += '.wrong';
if (scenario === 'disconnected') stats.daemon_connected = false;
if (scenario === 'store-error') stats.stores[0].error = 'broken index';
if (scenario === 'failed-response') status.success = false;
console.log(JSON.stringify(args[1] === 'daemon' ? status : stats));
`,
    { mode: 0o755 },
  );
  const env = {
    ...process.env,
    RUSTC_WRAPPER: binary,
    KACHE_CONFIG: config,
    KACHE_CACHE_DIR: path.join(directory, "store"),
    KACHE_RUNTIME_DIR: path.join(directory, "runtime"),
    KACHE_MAX_SIZE: "5GiB",
    SYNARA_CACHE_MAX_SIZE: "5GiB",
    FIXTURE_LOG: log,
  };
  const commands = () =>
    readFileSync(log, "utf8")
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
  try {
    run(env, commands);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test("CI starts a real daemon owner before checking machine-readable status and effective budget", () => {
  fixture((env, commands) => {
    const result = startCiRustCache({ env });
    assert.equal(result.version, "1.0.0");
    assert.equal(result.budgetBytes, 5368709120);
    assert.equal(result.cache, env.KACHE_CACHE_DIR);
    assert.deepEqual(commands(), [
      ["--version"],
      ["daemon", "start"],
      ["--json", "daemon", "status"],
      ["--json", "stats"],
    ]);
  });
});

test("CI refuses offline, mismatched or unhealthy daemon state instead of passing setup", () => {
  for (const scenario of [
    "wrong-binary",
    "start-failed",
    "invalid-json",
    "not-running",
    "wrong-version",
    "wrong-socket",
    "wrong-config",
    "wrong-budget",
    "wrong-store",
    "disconnected",
    "store-error",
    "failed-response",
  ]) {
    fixture((env) => {
      assert.throws(
        () => startCiRustCache({ env: { ...env, FIXTURE_SCENARIO: scenario } }),
        undefined,
        scenario,
      );
    });
  }
});

test("CI rejects missing action exports, a changed budget and runtime files mixed into snapshots", () => {
  for (const overrides of [
    { KACHE_RUNTIME_DIR: "" },
    { KACHE_RUNTIME_DIR: "relative/runtime" },
    { KACHE_MAX_SIZE: "10GiB" },
    { SYNARA_CACHE_MAX_SIZE: "unbounded" },
  ]) {
    fixture((env, commands) => {
      assert.throws(() => startCiRustCache({ env: { ...env, ...overrides } }));
      assert.deepEqual(commands(), []);
    });
  }
  fixture((env, commands) => {
    assert.throws(
      () =>
        startCiRustCache({
          env: { ...env, KACHE_RUNTIME_DIR: env.KACHE_CACHE_DIR },
        }),
      /separate/,
    );
    assert.deepEqual(commands(), []);
  });
});
