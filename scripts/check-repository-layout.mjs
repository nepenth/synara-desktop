import { existsSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const runtimePath = join(root, "synara");

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: options.cwd ?? root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function fail(message) {
  console.error(`[repository-layout] ${message}`);
  process.exitCode = 1;
}

const gitmodulesPath = join(root, ".gitmodules");
if (existsSync(gitmodulesPath)) {
  fail(".gitmodules exists; synara must be tracked directly inside synara-desktop.");
}

try {
  const indexEntries = git(["ls-files", "--stage", "synara"]);
  if (!indexEntries) {
    fail("synara is not tracked by the parent repository.");
  }
  if (indexEntries.split("\n").some((line) => line.startsWith("160000 "))) {
    fail("synara is still tracked as a gitlink submodule; absorb it as normal source files.");
  }
  if (!indexEntries.includes("\tsynara/package.json")) {
    fail("synara/package.json is not tracked by the parent repository.");
  }
} catch (error) {
  fail(`unable to inspect tracked synara files: ${error.message}`);
}

if (!existsSync(runtimePath)) {
  fail("synara runtime directory is missing.");
}

if (existsSync(join(runtimePath, ".git"))) {
  fail("synara contains nested Git metadata; remove synara/.git so it is normal tracked source.");
}

if (existsSync(join(runtimePath, ".github"))) {
  fail("synara contains nested GitHub automation; keep CI/CD only at the synara-desktop repository root.");
}

const projectRoot = dirname(root);
const siblingRuntimePath = join(projectRoot, "synara");
if (basename(root) === "synara-desktop" && existsSync(siblingRuntimePath)) {
  fail(`active sibling checkout found at ${siblingRuntimePath}; use ${runtimePath} for desktop runtime work.`);
}

if (process.exitCode) {
  process.exit(process.exitCode);
}

console.log("Repository layout is canonical: synara runtime is tracked directly inside synara-desktop.");
