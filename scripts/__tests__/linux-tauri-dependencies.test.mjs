import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const installerPath = path.join(root, "scripts/install-linux-tauri-dependencies.sh");
const workflowPaths = [
  ".github/workflows/ci.yml",
  ".github/workflows/release.yml",
];

test("Linux Tauri dependency installation is bounded and centralized", async () => {
  const installer = await readFile(installerPath, "utf8");

  assert.match(installer, /https:\/\/archive\.ubuntu\.com\/ubuntu/);
  assert.match(installer, /Acquire::Retries=3/);
  assert.match(installer, /Acquire::https::Timeout=20/);
  assert.match(installer, /--no-install-recommends/);
  // A failed index fetch must fail the update instead of passing silently.
  assert.match(installer, /APT::Update::Error-Mode=any update/);

  for (const relativePath of workflowPaths) {
    const workflow = await readFile(path.join(root, relativePath), "utf8");
    assert.match(workflow, /run: scripts\/install-linux-tauri-dependencies\.sh/);
    assert.doesNotMatch(workflow, /azure\.archive\.ubuntu\.com|sudo apt-get update/);
  }

  // Playwright's own apt run must come after a reachable source is ensured.
  const ci = await readFile(path.join(root, ".github/workflows/ci.yml"), "utf8");
  assert.ok(
    ci.indexOf("install-linux-tauri-dependencies.sh --sources-only") <
      ci.indexOf("npx playwright install --with-deps"),
  );
});

// Runs the installer against stub `curl`, `sudo` and `apt-get`. `reachable`
// lists the hosts whose probe succeeds.
function runInstaller(reachable, args = []) {
  const dir = mkdtempSync(path.join(os.tmpdir(), "synara-apt-"));
  const bin = path.join(dir, "bin");
  spawnSync("mkdir", ["-p", bin]);
  const log = path.join(dir, "calls.log");
  const stub = (name, body) => {
    writeFileSync(path.join(bin, name), `#!/usr/bin/env bash\n${body}\n`);
    chmodSync(path.join(bin, name), 0o755);
  };
  stub("curl", `url="\${@: -1}"; for host in $REACHABLE; do [[ "$url" == *"//$host/"* ]] && exit 0; done; exit 6`);
  // sudo runs the stubbed tool; apt-get and cp only record what they were asked.
  stub("sudo", `[[ "$1" == env ]] && shift 2; exec "$@"`);
  stub("apt-get", `echo "apt-get $*" >> "${log}"`);
  stub("cp", `echo "cp $(basename "$1") -> $2" >> "${log}"`);
  writeFileSync(path.join(dir, "os-release"), "VERSION_CODENAME=jammy\n");
  writeFileSync(path.join(dir, "apt-mirrors.txt"), "http://azure.archive.ubuntu.com/ubuntu/\tpriority:1\n");
  const result = spawnSync("bash", [installerPath, ...args], {
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      REACHABLE: reachable.join(" "),
      SYNARA_OS_RELEASE_FILE: path.join(dir, "os-release"),
      SYNARA_APT_MIRRORS_FILE: path.join(dir, "apt-mirrors.txt"),
      SYNARA_APT_SOURCE_LIST: path.join(dir, "sources.list"),
    },
  });
  let calls = "";
  try {
    calls = readFileSync(log, "utf8");
  } catch {
    // no apt call was made
  }
  return { ...result, calls };
}

const UBUNTU = ["archive.ubuntu.com", "azure.archive.ubuntu.com"];
const MIRROR = "mirrors.edge.kernel.org";

test("the installer uses the official archive when it is reachable", () => {
  const { status, stdout, calls } = runInstaller([...UBUNTU, MIRROR]);
  assert.equal(status, 0);
  assert.match(stdout, /Using package source: official/);
  assert.match(calls, /official\.list.* install -y --no-install-recommends/);
});

test("the installer falls back to the runner's sources, then to the mirror", () => {
  const runner = runInstaller(["azure.archive.ubuntu.com", MIRROR]);
  assert.equal(runner.status, 0);
  assert.match(runner.stdout, /Using package source: runner/);
  assert.doesNotMatch(runner.calls, /sourcelist=.* install/);

  // The 2026-11-11 outage: no ubuntu.com name resolves on the runner.
  const mirror = runInstaller([MIRROR]);
  assert.equal(mirror.status, 0);
  assert.match(mirror.stdout, /Using package source: mirror/);
  assert.match(mirror.calls, /mirror\.list.* install -y --no-install-recommends/);
  // Nothing is fetched from a source whose probe failed.
  assert.doesNotMatch(mirror.calls, /official\.list/);
});

test("the installer fails clearly when no source is reachable", () => {
  const { status, stderr, calls } = runInstaller([]);
  assert.equal(status, 1);
  assert.match(stderr, /No Ubuntu package source is reachable/);
  assert.equal(calls, "");
});

test("--sources-only installs nothing and replaces the system list only when the runner's is unreachable", () => {
  const healthy = runInstaller([...UBUNTU, MIRROR], ["--sources-only"]);
  assert.equal(healthy.status, 0);
  assert.doesNotMatch(healthy.calls, / install |cp /);

  const outage = runInstaller([MIRROR], ["--sources-only"]);
  assert.equal(outage.status, 0);
  assert.doesNotMatch(outage.calls, / install /);
  assert.match(outage.calls, /cp mirror\.list -> .*sources\.list/);
});
