import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const dist = join(root, 'dist');
const files = [];
let compatibilityApiFound = false;
const walk = async (directory, prefix = '') => {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const relative = `${prefix}${entry.name}`;
    if (entry.isDirectory()) await walk(join(directory, entry.name), `${relative}/`);
    else files.push(relative);
  }
};
await walk(dist);
for (const file of files) {
  assert.doesNotMatch(
    file,
    /(^|\/)(?:element-call|node_modules)(\/|$)|(^|\/)(?:sw|dev-sw)\.js(?:\.map)?$|\.webmanifest$/
  );
  if (file.endsWith('.js.map')) {
    const map = JSON.parse(await readFile(join(dist, file), 'utf8'));
    for (const source of map.sources ?? []) {
      assert.doesNotMatch(source, /node_modules\/pdfjs-dist\/build\/pdf(?:\.min)?\.mjs$/);
      if (/node_modules\/pdfjs-dist\/legacy\/build\/pdf\.mjs$/.test(source)) {
        compatibilityApiFound = true;
      }
      assert.doesNotMatch(
        source,
        /node_modules\/(?:@element-hq\/element-call-embedded|buffer|@esbuild-plugins\/node-globals-polyfill|vite-plugin-top-level-await)\//
      );
    }
  }
}
assert.ok(
  compatibilityApiFound,
  'The packaged PDF API must use the upstream compatibility distribution.'
);
const compatibilityWorker = JSON.parse(
  await readFile(join(dist, 'pdf.compat.worker.js.map'), 'utf8')
);
for (const module of [
  'es.promise.with-resolvers.js',
  'es.array-buffer.transfer-to-fixed-length.js',
]) {
  assert.ok(
    compatibilityWorker.sources.some((source) => source.endsWith(`/core-js/modules/${module}`)),
    `The independent PDF worker must initialize the standard ${module} shim.`
  );
}
assert.deepEqual(
  await readFile(join(dist, 'pdf.worker.min.js')),
  await readFile(join(root, 'node_modules/pdfjs-dist/legacy/build/pdf.worker.min.mjs')),
  'The packaged PDF worker must match the installed PDF.js runtime.'
);
for (const locale of await readdir(join(root, 'public/locales'))) {
  assert.deepEqual(
    await readFile(join(dist, 'public/locales', locale)),
    await readFile(join(root, 'public/locales', locale)),
    `Locale ${locale} must remain at its application URL.`
  );
}
assert.deepEqual(
  await readFile(join(dist, 'config.json')),
  await readFile(join(root, 'config.json'))
);
console.log(
  `Runtime output verified: ${files.length} files; matching PDF worker, config and locales; retired assets absent.`
);
