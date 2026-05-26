import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const runtimePath = join(root, "synara");
const expectedRuntimeRemote = "https://github.com/nepenth/synara.git";

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
if (!existsSync(gitmodulesPath)) {
  fail(".gitmodules is missing; synara must remain an explicit submodule until the repo is intentionally absorbed.");
} else {
  const gitmodules = readFileSync(gitmodulesPath, "utf8");
  if (!/path\s*=\s*synara/.test(gitmodules)) {
    fail(".gitmodules does not declare the synara submodule path.");
  }
  if (!gitmodules.includes(`url = ${expectedRuntimeRemote}`)) {
    fail(`.gitmodules must point synara at ${expectedRuntimeRemote}.`);
  }
}

let gitlinkCommit = "";
try {
  const lsTree = git(["ls-tree", "HEAD", "synara"]);
  const match = lsTree.match(/^160000 commit ([0-9a-f]{40})\tsynara$/);
  if (!match) {
    fail("parent repository does not record synara as a gitlink submodule.");
  } else {
    gitlinkCommit = match[1];
  }
} catch (error) {
  fail(`unable to inspect parent synara gitlink: ${error.message}`);
}

if (!existsSync(join(runtimePath, ".git"))) {
  fail("synara submodule is not initialized; run `git submodule update --init --recursive`.");
} else {
  try {
    const runtimeHead = git(["rev-parse", "HEAD"], { cwd: runtimePath });
    if (gitlinkCommit && runtimeHead !== gitlinkCommit) {
      fail(`synara submodule HEAD ${runtimeHead} does not match parent pointer ${gitlinkCommit}.`);
    }

    const runtimeRemote = git(["remote", "get-url", "origin"], { cwd: runtimePath });
    if (runtimeRemote !== expectedRuntimeRemote) {
      fail(`synara origin is ${runtimeRemote}; expected ${expectedRuntimeRemote}.`);
    }

    const dirty = git(["status", "--porcelain"], { cwd: runtimePath });
    if (dirty) {
      fail("synara submodule has uncommitted changes; commit/push runtime work before updating the parent pointer.");
    }
  } catch (error) {
    fail(`unable to validate synara submodule checkout: ${error.message}`);
  }
}

const projectRoot = dirname(root);
const siblingRuntimePath = join(projectRoot, "synara");
if (basename(root) === "synara-desktop" && existsSync(siblingRuntimePath)) {
  fail(`active sibling checkout found at ${siblingRuntimePath}; use ${runtimePath} for desktop runtime work.`);
}

if (process.exitCode) {
  process.exit(process.exitCode);
}

console.log("Repository layout is canonical: synara-desktop with initialized synara submodule.");
