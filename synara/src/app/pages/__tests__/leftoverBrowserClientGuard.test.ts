import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

test('desktop app no longer gates on leftover IndexedDB FeatureCheck', () => {
  const app = readFileSync(join(process.cwd(), 'src/app/pages/App.tsx'), 'utf8');
  const vite = readFileSync(join(process.cwd(), 'vite.config.js'), 'utf8');
  const pkg = readFileSync(join(process.cwd(), 'package.json'), 'utf8');
  assert.doesNotMatch(app, /FeatureCheck|checkIndexedDBSupport|IndexedDB/);
  assert.doesNotMatch(
    vite,
    /matrix-sdk-crypto-wasm|serverMatrixSdkCryptoWasm|@rollup\/plugin-wasm/
  );
  assert.doesNotMatch(pkg, /@rollup\/plugin-wasm|matrix-sdk-crypto-wasm/);
});
