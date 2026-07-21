import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { encryptFile } from '../matrix';

const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'window');
Object.defineProperty(globalThis, 'window', {
  configurable: true,
  value: globalThis,
});

after(() => {
  if (originalWindowDescriptor) {
    Object.defineProperty(globalThis, 'window', originalWindowDescriptor);
  } else {
    Reflect.deleteProperty(globalThis, 'window');
  }
});

test('encryptFile assigns a stable name to encrypted thumbnail blobs', async () => {
  const thumbnail = new Blob(['thumbnail'], { type: 'image/png' });

  const encrypted = await encryptFile(thumbnail);

  assert.equal(encrypted.file.name, 'encrypted-thumbnail');
  assert.equal(encrypted.file.type, thumbnail.type);
  assert.equal(encrypted.originalFile, thumbnail);
});

test('encryptFile preserves the name of user-selected files', async () => {
  const upload = new File(['attachment'], 'attachment.txt', { type: 'text/plain' });

  const encrypted = await encryptFile(upload);

  assert.equal(encrypted.file.name, upload.name);
  assert.equal(encrypted.file.type, upload.type);
  assert.equal(encrypted.originalFile, upload);
});
