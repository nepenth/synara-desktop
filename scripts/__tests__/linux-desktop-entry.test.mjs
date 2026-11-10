import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

const repositoryRoot = path.resolve(import.meta.dirname, "../..");
const read = (relative) =>
  readFile(path.join(repositoryRoot, relative), "utf8");

const tauriConfig = async () =>
  JSON.parse(await read("src-tauri/tauri.conf.json"));

test("the deb and rpm desktop entry is the Arch one", async () => {
  const config = await tauriConfig();
  const { deb, rpm } = config.bundle.linux;
  assert.equal(deb.desktopTemplate, "../packaging/deb/synara.desktop");
  assert.equal(rpm.desktopTemplate, deb.desktopTemplate);

  // The bundler fills these three; everything else is literal.
  const template = await read("packaging/deb/synara.desktop");
  const rendered = template
    .replaceAll("{{name}}", config.productName)
    .replaceAll("{{exec}}", config.mainBinaryName)
    .replaceAll("{{icon}}", config.mainBinaryName);
  assert.doesNotMatch(rendered, /\{\{/);
  assert.equal(rendered, await read("packaging/arch/synara.desktop"));
});

test("every desktop registers the synara scheme", async () => {
  const config = await tauriConfig();
  // Read by the bundler alone (NSIS and WiX registry entries on Windows);
  // the app does not load tauri-plugin-deep-link.
  assert.deepEqual(config.plugins["deep-link"], {
    desktop: { schemes: ["synara"] },
  });

  for (const entry of [
    "packaging/arch/synara.desktop",
    "packaging/deb/synara.desktop",
  ]) {
    const text = await read(entry);
    assert.match(text, /^MimeType=x-scheme-handler\/synara;$/m, entry);
    assert.match(text, /^Exec=\S+ %U$/m, entry);
    assert.match(text, /^StartupWMClass=(synara|\{\{exec\}\})$/m, entry);
  }

  const plist = await read("src-tauri/Info.plist");
  assert.match(
    plist,
    /<key>CFBundleURLSchemes<\/key>\s*<array>\s*<string>synara<\/string>/
  );
});
