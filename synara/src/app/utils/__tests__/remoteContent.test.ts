import assert from 'node:assert/strict';
import test from 'node:test';
import { isSafeHttpsUrl, safeRemoteContentUrl } from '../remoteContent';

test('safeRemoteContentUrl accepts public HTTPS URLs', () => {
  assert.equal(
    safeRemoteContentUrl('https://cdn.example.org/image.gif'),
    'https://cdn.example.org/image.gif'
  );
});

test('safeRemoteContentUrl rejects non-HTTPS and credentialed URLs', () => {
  assert.equal(safeRemoteContentUrl('http://cdn.example.org/image.gif'), undefined);
  assert.equal(safeRemoteContentUrl('https://user:pass@cdn.example.org/image.gif'), undefined);
});

test('safeRemoteContentUrl rejects localhost and private address targets', () => {
  assert.equal(isSafeHttpsUrl('https://localhost/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://LOCALHOST/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://127.0.0.1/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://10.2.3.4/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://192.168.1.10/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://100.64.0.1/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://[fd00::1]/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://[::ffff:127.0.0.1]/image.gif'), false);
});

test('safeRemoteContentUrl rejects local host suffixes', () => {
  assert.equal(isSafeHttpsUrl('https://media.internal/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://media.lan/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://media.localdomain/image.gif'), false);
  assert.equal(isSafeHttpsUrl('https://media.home.arpa/image.gif'), false);
});

test('safeRemoteContentUrl rejects unsafe remote-content targets', () => {
  assert.equal(safeRemoteContentUrl('http://example.org/public'), undefined);
  assert.equal(safeRemoteContentUrl('https://127.0.0.1/_matrix/client/versions'), undefined);
  assert.equal(safeRemoteContentUrl('https://[::ffff:169.254.169.254]/latest'), undefined);
  assert.equal(safeRemoteContentUrl('https://preview.internal/status'), undefined);
  assert.equal(safeRemoteContentUrl('https://example.org/public'), 'https://example.org/public');
});
