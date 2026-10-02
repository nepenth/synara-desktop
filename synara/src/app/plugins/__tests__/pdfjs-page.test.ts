import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { createPage } from '../pdfjs-page';

const fixture = () => {
  let complete!: () => void;
  let fail!: (reason: Error) => void;
  const renderPromise = new Promise<void>((resolve, reject) => {
    complete = resolve;
    fail = reject;
  });
  const context = {} as CanvasRenderingContext2D;
  const canvas = { width: 0, height: 0, getContext: () => context } as unknown as HTMLCanvasElement;
  const previousDocument = globalThis.document;
  globalThis.document = { createElement: () => canvas } as unknown as Document;
  const doc = {
    getPage: async () => ({
      getViewport: () => ({ width: 100, height: 200 }),
      render: () => ({ promise: renderPromise }),
    }),
  } as unknown as PDFDocumentProxy;
  return {
    doc,
    canvas,
    complete,
    fail,
    restore: () => {
      if (previousDocument === undefined) Reflect.deleteProperty(globalThis, 'document');
      else globalThis.document = previousDocument;
    },
  };
};

test('PDF page publication waits for the actual deferred RenderTask completion', async () => {
  const owned = fixture();
  try {
    let published = false;
    const result = createPage(owned.doc, 1, { scale: 1 }).then((canvas) => {
      published = true;
      return canvas;
    });
    await new Promise<void>((resolve) => setImmediate(resolve));
    assert.equal(published, false, 'An unfinished RenderTask cannot publish a completed canvas');
    owned.complete();
    assert.equal(await result, owned.canvas);
    assert.equal(published, true);
    assert.equal(owned.canvas.width, 100);
    assert.equal(owned.canvas.height, 200);
  } finally {
    owned.restore();
  }
});

test('PDF RenderTask rejection reaches the caller instead of publishing a partial canvas', async () => {
  const owned = fixture();
  try {
    const failure = new Error('render task failed');
    let published = false;
    const result = createPage(owned.doc, 1, { scale: 1 }).then((canvas) => {
      published = true;
      return canvas;
    });
    const rejected = assert.rejects(result, (error) => error === failure);
    await new Promise<void>((resolve) => setImmediate(resolve));
    owned.fail(failure);
    await rejected;
    assert.equal(published, false);
  } finally {
    owned.restore();
  }
});
