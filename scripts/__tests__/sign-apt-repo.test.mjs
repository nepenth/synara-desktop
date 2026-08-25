import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repositoryRoot = path.resolve(import.meta.dirname, "../..");
const script = path.join(repositoryRoot, "scripts/sign-apt-repo.sh");

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
};

test("signs and independently verifies APT Release metadata", async () => {
  const fixture = await mkdtemp(path.join(tmpdir(), "synara-apt-signing-"));
  const signingHome = path.join(fixture, "signing-home");
  const repository = path.join(fixture, "repo");
  await mkdir(signingHome, { mode: 0o700 });
  await mkdir(repository);
  await writeFile(path.join(repository, "Release"), "Origin: Synara\nSuite: apt-repo\n");

  run("gpg", [
    "--homedir",
    signingHome,
    "--batch",
    "--passphrase",
    "",
    "--quick-generate-key",
    "Synara APT Test <apt-test@example.invalid>",
    "ed25519",
    "sign",
    "1d",
  ]);
  const privateKey = run("gpg", [
    "--homedir",
    signingHome,
    "--batch",
    "--armor",
    "--export-secret-keys",
  ]);

  const result = spawnSync(script, [repository], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      SYNARA_APT_SIGNING_PRIVATE_KEY: privateKey,
      SYNARA_APT_SIGNING_PRIVATE_KEY_PASSWORD: "",
    },
  });
  assert.equal(result.status, 0, result.stderr);

  const keyring = path.join(repository, "synara-archive-keyring.gpg");
  const release = path.join(repository, "Release");
  const detached = path.join(repository, "Release.gpg");
  const inRelease = path.join(repository, "InRelease");
  assert.equal((await stat(keyring)).size > 0, true);
  assert.match(await readFile(inRelease, "utf8"), /BEGIN PGP SIGNED MESSAGE/);

  run("gpgv", ["--keyring", keyring, detached, release]);
  run("gpgv", ["--keyring", keyring, inRelease]);
});

test("refuses to produce unsigned metadata without a private key", async () => {
  const fixture = await mkdtemp(path.join(tmpdir(), "synara-apt-unsigned-"));
  await writeFile(path.join(fixture, "Release"), "Origin: Synara\n");
  const result = spawnSync(script, [fixture], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      SYNARA_APT_SIGNING_PRIVATE_KEY: "",
    },
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /SYNARA_APT_SIGNING_PRIVATE_KEY is required/);
});
