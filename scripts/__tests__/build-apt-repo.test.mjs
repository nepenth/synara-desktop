import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = path.resolve(import.meta.dirname, "../..");
const script = path.join(repositoryRoot, "scripts/build-apt-repo.sh");

const writeExecutable = async (file, contents) => {
  await writeFile(file, contents);
  await chmod(file, 0o755);
};

test("builds a flat APT repository with a GitHub Release asset path", async () => {
  const fixture = await mkdtemp(path.join(tmpdir(), "synara-apt-repo-"));
  const packageDirectory = path.join(fixture, "packages");
  const outputDirectory = path.join(fixture, "output");
  const binDirectory = path.join(fixture, "bin");
  await mkdir(packageDirectory);
  await mkdir(binDirectory);
  await writeFile(path.join(packageDirectory, "Synara_2.1.10_amd64.deb"), "deb");

  await writeExecutable(
    path.join(binDirectory, "dpkg-scanpackages"),
    "#!/usr/bin/env bash\n" +
      "printf '%s\\n' 'Package: synara' 'Version: 2.1.10' 'Architecture: amd64' 'Filename: ./Synara_2.1.10_amd64.deb' 'SHA256: fixture'\n"
  );
  await writeExecutable(
    path.join(binDirectory, "apt-ftparchive"),
    "#!/usr/bin/env bash\n" +
      "printf '%s\\n' 'Origin: Synara' 'SHA256:' ' fixture Packages'\n"
  );

  const result = spawnSync(script, [packageDirectory, outputDirectory], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${binDirectory}:${process.env.PATH}`,
      SYNARA_APT_REPO_TAG: "apt-repo",
    },
  });

  assert.equal(result.status, 0, result.stderr);
  const packages = await readFile(path.join(outputDirectory, "Packages"), "utf8");
  assert.match(packages, /Filename: apt-repo\/Synara_2\.1\.10_amd64\.deb/);
  assert.match(await readFile(path.join(outputDirectory, "Release"), "utf8"), /Origin: Synara/);
  assert.equal((await stat(path.join(outputDirectory, "Packages.gz"))).size > 0, true);
  assert.equal(
    await readFile(path.join(outputDirectory, "Synara_2.1.10_amd64.deb"), "utf8"),
    "deb"
  );
});

test("rejects a directory containing more than one release package", async () => {
  const fixture = await mkdtemp(path.join(tmpdir(), "synara-apt-repo-many-"));
  const packageDirectory = path.join(fixture, "packages");
  await mkdir(packageDirectory);
  await writeFile(path.join(packageDirectory, "Synara_1.0.0_amd64.deb"), "old");
  await writeFile(path.join(packageDirectory, "Synara_2.0.0_amd64.deb"), "new");

  const result = spawnSync(script, [packageDirectory, path.join(fixture, "output")], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Expected exactly one Synara_\*\.deb.*found 2/);
});
