import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const installerPath = path.join(root, "scripts/install-linux-tauri-dependencies.sh");
const workflowPaths = [
  ".github/workflows/ci.yml",
  ".github/workflows/desktop-package-smoke.yml",
  ".github/workflows/release.yml",
];

test("Linux Tauri dependency installation is bounded and centralized", async () => {
  const installer = await readFile(installerPath, "utf8");

  assert.match(installer, /https:\/\/archive\.ubuntu\.com\/ubuntu/);
  assert.match(installer, /Acquire::Retries=3/);
  assert.match(installer, /Acquire::https::Timeout=20/);
  assert.match(installer, /--no-install-recommends/);

  for (const relativePath of workflowPaths) {
    const workflow = await readFile(path.join(root, relativePath), "utf8");
    assert.match(workflow, /run: scripts\/install-linux-tauri-dependencies\.sh/);
    assert.doesNotMatch(workflow, /azure\.archive\.ubuntu\.com|sudo apt-get update/);
  }
});
