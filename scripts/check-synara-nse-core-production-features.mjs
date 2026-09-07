#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = process.argv[2] ?? resolve(root, "Cargo.toml");
// Resolver 2 excludes dev-only features from cargo build. Include both normal
// and build edges so a production feature leak still fails this boundary.
const result = spawnSync(
  "cargo",
  ["tree", "--locked", "--manifest-path", manifest, "-p", "synara-nse-core",
    "-e", "normal,build,features", "-i", "synara-core"],
  { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
);
if (result.error || result.status !== 0) {
  console.error("SynaraNseCore production feature query failed.");
  if (result.stderr) console.error(result.stderr.trim());
  process.exit(1);
}
// Inspect completed output instead of a cargo | grep -q pipeline: a producer
// failure (including SIGPIPE) must never be interpreted as a clean graph.
if (result.stdout.includes('synara-core feature "full-uniffi"')) {
  console.error("SynaraNseCore must not enable the full Core UniFFI feature in its production graph");
  process.exit(1);
}
console.log("Synara NSE Core production feature isolation passed.");
