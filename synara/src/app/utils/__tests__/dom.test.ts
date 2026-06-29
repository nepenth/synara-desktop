import test from 'node:test';
import assert from 'node:assert/strict';
import { getDataTransferFiles, shouldProbeNativeClipboardImage } from '../dom';

const emptyFileList = { length: 0 } as unknown as FileList;

test('data transfer files fall back to file items', () => {
  const image = new File([new Uint8Array([1, 2, 3])], 'screenshot.png', {
    type: 'image/png',
  });
  const dataTransfer = {
    files: emptyFileList,
    items: {
      length: 2,
      0: {
        kind: 'string',
        getAsFile: () => null,
      },
      1: {
        kind: 'file',
        getAsFile: () => image,
      },
    },
  } as unknown as DataTransfer;

  const files = getDataTransferFiles(dataTransfer);

  assert.equal(files?.length, 1);
  assert.equal(files?.[0], image);
});

test('anonymous clipboard files get a usable filename', () => {
  const image = new File([new Uint8Array([1])], '', { type: 'image/png' });
  const dataTransfer = {
    files: emptyFileList,
    items: {
      length: 1,
      0: {
        kind: 'file',
        getAsFile: () => image,
      },
    },
  } as unknown as DataTransfer;

  const files = getDataTransferFiles(dataTransfer);

  assert.equal(files?.[0]?.name, 'clipboard-file.png');
  assert.equal(files?.[0]?.type, 'image/png');
});

test('native clipboard image probing wins for image-like clipboard payloads', () => {
  assert.equal(shouldProbeNativeClipboardImage({ types: [] as unknown as DOMStringList }), true);
  assert.equal(
    shouldProbeNativeClipboardImage({ types: ['Files'] as unknown as DOMStringList }),
    true
  );
  assert.equal(
    shouldProbeNativeClipboardImage({
      types: ['text/html', 'image/png'] as unknown as DOMStringList,
    }),
    true
  );
  assert.equal(
    shouldProbeNativeClipboardImage({ types: ['text/html'] as unknown as DOMStringList }),
    false
  );
  assert.equal(
    shouldProbeNativeClipboardImage({ types: ['text/plain'] as unknown as DOMStringList }),
    false
  );
});
