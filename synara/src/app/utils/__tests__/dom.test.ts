import test from 'node:test';
import assert from 'node:assert/strict';
import {
  dataTransferHasFiles,
  getDataTransferFiles,
  shouldProbeNativeClipboardImage,
} from '../dom';

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

test('data transfer file detection tolerates desktop payloads without Files type', () => {
  const image = new File([new Uint8Array([1])], 'drop.png', { type: 'image/png' });

  assert.equal(
    dataTransferHasFiles({
      files: { length: 1, 0: image } as unknown as FileList,
      items: { length: 0 } as unknown as DataTransferItemList,
      types: [],
    }),
    true
  );
  assert.equal(
    dataTransferHasFiles({
      files: emptyFileList,
      items: {
        length: 1,
        0: {
          kind: 'file',
          getAsFile: () => image,
        },
      } as unknown as DataTransferItemList,
      types: ['text/plain'],
    }),
    true
  );
  assert.equal(
    dataTransferHasFiles({
      files: emptyFileList,
      items: {
        length: 1,
        0: {
          kind: 'string',
          getAsFile: () => null,
        },
      } as unknown as DataTransferItemList,
      types: ['text/plain'],
    }),
    false
  );
});

test('native clipboard image probing wins for image-like clipboard payloads', () => {
  assert.equal(shouldProbeNativeClipboardImage({ types: [] }), true);
  assert.equal(shouldProbeNativeClipboardImage({ types: ['Files'] }), true);
  assert.equal(
    shouldProbeNativeClipboardImage({
      types: ['text/html', 'image/png'],
    }),
    true
  );
  assert.equal(shouldProbeNativeClipboardImage({ types: ['text/html'] }), false);
  assert.equal(shouldProbeNativeClipboardImage({ types: ['text/plain'] }), false);
});
