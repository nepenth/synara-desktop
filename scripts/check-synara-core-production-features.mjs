#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = process.argv[2] ?? resolve(root, "Cargo.toml");
const graph = (args) => {
  const result = spawnSync(
    "cargo",
    [
      "tree",
      "--locked",
      "--manifest-path",
      manifest,
      "-e",
      "normal,build",
      "--prefix",
      "none",
      "--format",
      "{p} [{f}]",
      ...args,
    ],
    {
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
      env: { ...process.env, CARGO_TERM_COLOR: "never" },
    },
  );
  if (result.error || result.status !== 0) {
    throw new Error(
      `Core production graph query failed: ${result.stderr || result.error}`,
    );
  }
  const packages = new Map();
  for (const line of result.stdout.split("\n")) {
    const match = /^(\S+) v\S+.* \[([^\]]*)\](?: \(\*\))?$/.exec(line);
    if (!match) continue;
    const features = packages.get(match[1]) ?? new Set();
    for (const feature of match[2].split(",").filter(Boolean))
      features.add(feature);
    packages.set(match[1], features);
  }
  return packages;
};

// Query shipping graphs separately; a workspace build intentionally unifies
// the generator/full Apple surface and would conceal a desktop feature leak.
for (const target of [
  "x86_64-unknown-linux-gnu",
  "aarch64-apple-darwin",
  "x86_64-pc-windows-msvc",
  "all",
]) {
  const desktop = graph(["-p", "synara", "--target", target]);
  if (
    desktop.get("synara-core")?.has("full-uniffi") ||
    [...desktop.keys()].some((name) => /^uniffi(?:_|$)/.test(name))
  ) {
    throw new Error(
      `Desktop must not compile Apple UniFFI runtime or scaffolding (${target})`,
    );
  }
  if (
    !desktop.get("synara-core")?.has("full-app") ||
    !desktop.get("matrix-sdk")?.has("automatic-room-key-forwarding")
  ) {
    throw new Error(
      `Desktop must retain full application owners and room-key forwarding (${target})`,
    );
  }
}
for (const target of [
  "aarch64-apple-ios",
  "aarch64-apple-ios-sim",
  "x86_64-apple-ios",
  "all",
]) {
  const apple = graph([
    "-p",
    "synara-core",
    "--no-default-features",
    "--features",
    "full-uniffi",
    "--target",
    target,
  ]);
  const forbidden = new Set([
    "search-index",
    "experimental-search",
    "x509-identity",
    "experimental-x509-identity-verification",
    "rust-x509-verifier-impl",
  ]);
  if (
    apple.has("matrix-sdk-search") ||
    apple.has("tantivy") ||
    [...apple.values()].some((features) =>
      [...features].some((feature) => forbidden.has(feature)),
    )
  ) {
    throw new Error(
      `Apple full Core must not compile desktop search or X.509 dependencies (${target})`,
    );
  }
  if (
    !apple.has("uniffi") ||
    !apple.get("matrix-sdk")?.has("automatic-room-key-forwarding")
  ) {
    throw new Error(
      `Apple full Core must retain UniFFI and room-key forwarding (${target})`,
    );
  }
}
console.log("Core desktop/Apple production feature boundaries passed.");
