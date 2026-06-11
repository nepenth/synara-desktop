#!/usr/bin/env node
/**
 * Ensures tracked devAssets/index.html matches synara/dist after a runtime build.
 * Run after `npm run build:runtime` to avoid accidental devAssets hash churn in git.
 */

import { readFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const devAssetsIndex = join(root, "devAssets/index.html");
const distIndex = join(root, "synara/dist/index.html");

function read(path) {
  if (!existsSync(path)) {
    throw new Error(`Missing runtime asset: ${path}`);
  }
  return readFileSync(path, "utf8");
}

const devAssets = read(devAssetsIndex);
const dist = read(distIndex);

if (devAssets !== dist) {
  console.error(
    "devAssets/index.html is out of sync with synara/dist/index.html.\n" +
      "Run: npm run build:runtime\n" +
      "Then commit both outputs together, or discard devAssets changes if you did not intend to rebuild."
  );
  process.exit(1);
}

console.log("Runtime assets are in sync (devAssets/index.html matches synara/dist).");