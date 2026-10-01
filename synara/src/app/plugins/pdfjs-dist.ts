import { useCallback } from 'react';
import 'core-js/modules/es.promise.with-resolvers.js';
import pdfWorkerUrl from './pdfjs-worker?worker&url';
import type * as PdfJsDist from 'pdfjs-dist';
import type { GetViewportParameters } from 'pdfjs-dist/types/src/display/api';
import { useAsyncCallback } from '../hooks/useAsyncCallback';

export const usePdfJSLoader = () =>
  useAsyncCallback(
    useCallback(async () => {
      // Native minimum webviews need the upstream translated/polyfilled distribution.
      const pdf = await import('pdfjs-dist/legacy/build/pdf.mjs');
      pdf.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;
      return pdf;
    }, [])
  );

export const usePdfDocumentLoader = (pdfJS: typeof PdfJsDist | undefined, src: string) =>
  useAsyncCallback(
    useCallback(async () => {
      if (!pdfJS) {
        throw new Error('PdfJS is not loaded');
      }
      const doc = await pdfJS.getDocument({ url: src }).promise;
      return doc;
    }, [pdfJS, src])
  );

export const createPage = async (
  doc: PdfJsDist.PDFDocumentProxy,
  pNo: number,
  opts: GetViewportParameters
): Promise<HTMLCanvasElement> => {
  const page = await doc.getPage(pNo);
  const pageViewport = page.getViewport(opts);
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d');

  if (!context) throw new Error('failed to render page.');

  canvas.width = pageViewport.width;
  canvas.height = pageViewport.height;

  page.render({
    canvas,
    canvasContext: context,
    viewport: pageViewport,
  });

  return canvas;
};
