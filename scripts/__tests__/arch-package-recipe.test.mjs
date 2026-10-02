import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  mkdtemp,
  mkdir,
  readFile,
  writeFile,
  copyFile,
  stat,
  rm,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

const repositoryRoot = path.resolve(import.meta.dirname, "../..");
const sizes = [16, 24, 32, 48, 64, 128, 256, 512];
async function fixture(t, binaryPath) {
  const root = await mkdtemp(path.join(tmpdir(), "synara-arch-recipe-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const startdir = path.join(root, "packaging/arch");
  const pkgdir = path.join(root, "package-prefix");
  await Promise.all(
    [
      "packaging/arch",
      "sources",
      "src-tauri/icons",
      "assets/branding",
      path.dirname(binaryPath),
    ].map((directory) => mkdir(path.join(root, directory), { recursive: true }))
  );
  await Promise.all(
    ["PKGBUILD", "synara.desktop"].map((name) =>
      copyFile(
        path.join(repositoryRoot, "packaging/arch", name),
        path.join(startdir, name)
      )
    )
  );
  await copyFile(
    path.join(repositoryRoot, "packaging/arch/synara"),
    path.join(root, "sources/synara")
  );
  await writeFile(
    path.join(root, "src-tauri/tauri.conf.json"),
    '{\n  "version": "2.1.11-rc.1"\n}\n'
  );
  await writeFile(path.join(root, binaryPath), "workspace-release-binary\n");
  await Promise.all(
    sizes.map((size) =>
      writeFile(
        path.join(root, `src-tauri/icons/${size}x${size}.png`),
        `icon-${size}`
      )
    )
  );
  await writeFile(
    path.join(root, "assets/branding/synara-symbolic.svg"),
    "symbolic"
  );
  await writeFile(path.join(root, "LICENSE"), "license");
  // BSD install lacks GNU -D; translate only that portability flag, then
  // exercise actual file installation and modes within the package prefix.
  const result = spawnSync(
    "bash",
    [
      "-c",
      `
    set -e
    startdir="$1"; srcdir="$2"; pkgdir="$3"
    install() { mkdir -p "$(dirname "$3")"; command install -m "\${1#-Dm}" "$2" "$3"; }
    source "$startdir/PKGBUILD"
    [[ "$pkgver" == '2.1.11_rc.1' ]]
    package
  `,
      "arch-recipe-fixture",
      startdir,
      path.join(root, "sources"),
      pkgdir,
    ],
    {
      encoding: "utf8",
      cwd: root,
    }
  );
  return { root, pkgdir, result };
}

test("Arch package installs canonical workspace binary with no nested target present", async (t) => {
  const { root, pkgdir, result } = await fixture(t, "target/release/synara");
  assert.equal(result.status, 0, result.stderr);
  await assert.rejects(stat(path.join(root, "src-tauri/target")), {
    code: "ENOENT",
  });
  assert.equal(
    await readFile(path.join(pkgdir, "usr/lib/synara/synara"), "utf8"),
    "workspace-release-binary\n"
  );
  assert.equal(
    (await stat(path.join(pkgdir, "usr/lib/synara/synara"))).mode & 0o777,
    0o755
  );
  assert.match(
    await readFile(path.join(pkgdir, "usr/bin/synara"), "utf8"),
    /exec \/usr\/lib\/synara\/synara/
  );
  assert.equal(
    await readFile(
      path.join(pkgdir, "usr/share/applications/Synara.desktop"),
      "utf8"
    ),
    await readFile(
      path.join(repositoryRoot, "packaging/arch/synara.desktop"),
      "utf8"
    )
  );
  for (const size of sizes)
    assert.equal(
      await readFile(
        path.join(
          pkgdir,
          `usr/share/icons/hicolor/${size}x${size}/apps/synara.png`
        ),
        "utf8"
      ),
      `icon-${size}`
    );
  assert.equal(
    await readFile(
      path.join(
        pkgdir,
        "usr/share/icons/hicolor/scalable/apps/synara-symbolic.svg"
      ),
      "utf8"
    ),
    "symbolic"
  );
  assert.equal(
    await readFile(
      path.join(pkgdir, "usr/share/licenses/synara-desktop-bin/LICENSE"),
      "utf8"
    ),
    "license"
  );
});

test("Arch package rejects a stale nested target binary when workspace release is absent", async (t) => {
  const { pkgdir, result } = await fixture(
    t,
    "src-tauri/target/release/synara"
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Synara release binary was not found/);
  await assert.rejects(stat(pkgdir), { code: "ENOENT" });
});
