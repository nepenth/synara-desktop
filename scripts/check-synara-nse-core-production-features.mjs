#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = process.argv[2] ?? resolve(root, "Cargo.toml");
// Cover every slice supported by generate-synara-nse-core-swift.sh. A host-only
// graph can hide target_os="ios" features. Cargo tree needs no installed stdlib.
const appleTargets = ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"];
for (const target of appleTargets) {
  // Resolver 2 excludes dev-only features from cargo build. Include both normal
  // and build edges so a production feature leak still fails this boundary.
  const result = spawnSync(
    "cargo",
    ["tree", "--locked", "--manifest-path", manifest, "-p", "synara-nse-core",
      "-e", "normal,build,features", "-i", "synara-core", "--target", target],
    { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  );
  if (result.error || result.status !== 0) {
    console.error(`SynaraNseCore production feature query failed for ${target}.`);
    if (result.stderr) console.error(result.stderr.trim());
    process.exit(1);
  }
  // Inspect completed output instead of a cargo | grep -q pipeline: a producer
  // failure (including SIGPIPE) must never be interpreted as a clean graph.
  if (result.stdout.includes('synara-core feature "full-uniffi"')) {
    console.error(`SynaraNseCore must not enable the full Core UniFFI feature in its production graph (${target})`);
    process.exit(1);
  }
  if (result.stdout.includes('synara-core feature "search-index"')) {
    console.error(`SynaraNseCore must not enable the desktop search-index feature (${target})`);
    process.exit(1);
  }
  // Inverse synara-core readback does not list matrix-sdk-crypto features.
  // Query the production graph and fail if gossip is compiled into NSE.
  const forwarding = spawnSync(
    "cargo",
    ["tree", "--locked", "--manifest-path", manifest, "-p", "synara-nse-core",
      "-e", "normal,build,features", "--target", target],
    { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  );
  if (forwarding.error || forwarding.status !== 0) {
    console.error(`SynaraNseCore forwarding-feature query failed for ${target}.`);
    if (forwarding.stderr) console.error(forwarding.stderr.trim());
    process.exit(1);
  }
  if (forwarding.stdout.includes("automatic-room-key-forwarding")) {
    console.error(`SynaraNseCore must not compile automatic-room-key-forwarding (${target})`);
    process.exit(1);
  }
  const tree = spawnSync(
    "cargo",
    ["tree", "--locked", "--manifest-path", manifest, "-p", "synara-nse-core",
      "-e", "normal,build", "--target", target],
    { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  );
  if (tree.error || tree.status !== 0) {
    console.error(`SynaraNseCore production crate tree query failed for ${target}.`);
    if (tree.stderr) console.error(tree.stderr.trim());
    process.exit(1);
  }
  for (const leaked of ["matrix-sdk-search", "tantivy"]) {
    if (tree.stdout.includes(leaked)) {
      console.error(`SynaraNseCore must not pull ${leaked} on ${target}`);
      process.exit(1);
    }
  }
}
console.log("Synara NSE Core production feature isolation passed for all Apple slices.");
