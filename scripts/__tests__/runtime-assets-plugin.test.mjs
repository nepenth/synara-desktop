import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  build,
  createServer,
} from "../../synara/node_modules/vite/dist/node/index.js";
import { runtimeAssetsPlugin } from "../../synara/scripts/runtime-assets-plugin.mjs";

test("Vite dev and production preserve stable asset URLs and upstream bytes", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "synara-assets-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const sources = {
    "node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs":
      "// worker\n\0upstream bytes",
    "config.json": '{"homeserver":"test"}\n',
    "public/locales/en.json": '{"hello":"Hello"}\n',
    "public/locales/de.json": '{"hello":"Hallo"}\n',
  };
  for (const [name, contents] of Object.entries(sources)) {
    await mkdir(path.dirname(path.join(root, name)), { recursive: true });
    await writeFile(path.join(root, name), contents);
  }
  await writeFile(
    path.join(root, "index.html"),
    "<!doctype html><title>assets</title>",
  );
  const urls = {
    "pdf.worker.min.js":
      sources["node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs"],
    "config.json": sources["config.json"],
    "public/locales/en.json": sources["public/locales/en.json"],
    "public/locales/de.json": sources["public/locales/de.json"],
  };
  for (const base of ["/", "/nested/"]) {
    const server = await createServer({
      configFile: false,
      root,
      base,
      publicDir: false,
      plugins: [runtimeAssetsPlugin()],
      server: { host: "127.0.0.1", port: 0 },
    });
    try {
      await server.listen();
      const address = server.httpServer.address();
      const origin = `http://127.0.0.1:${address.port}`;
      for (const [url, contents] of Object.entries(urls)) {
        const response = await fetch(`${origin}${base}${url}?test=1`);
        assert.equal(response.status, 200, `${base}${url}`);
        assert.equal(await response.text(), contents, `${base}${url}`);
      }
      const head = await fetch(`${origin}${base}pdf.worker.min.js`, {
        method: "HEAD",
      });
      assert.equal(head.status, 200);
      assert.equal(await head.text(), "");
      assert.match(head.headers.get("content-type"), /javascript/);
      await writeFile(
        path.join(
          root,
          "node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs",
        ),
        "updated",
      );
      assert.equal(
        await (await fetch(`${origin}${base}pdf.worker.min.js`)).text(),
        "updated",
      );
      await writeFile(
        path.join(
          root,
          "node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs",
        ),
        urls["pdf.worker.min.js"],
      );
    } finally {
      await server.close();
    }
    await build({
      configFile: false,
      root,
      base,
      publicDir: false,
      plugins: [runtimeAssetsPlugin()],
      logLevel: "silent",
      build: { outDir: "output", copyPublicDir: false },
    });
    for (const [url, contents] of Object.entries(urls)) {
      assert.equal(
        await readFile(path.join(root, "output", url), "utf8"),
        contents,
      );
    }
  }
});
