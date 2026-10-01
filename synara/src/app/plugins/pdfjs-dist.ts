import { useCallback } from 'react';
import 'core-js/modules/es.promise.with-resolvers.js';
import pdfWorkerUrl from './pdfjs-worker?worker&url';
import type * as PdfJsDist from 'pdfjs-dist';
import { createPage } from './pdfjs-page';
import { useAsyncCallback } from '../hooks/useAsyncCallback';

export { createPage } from './pdfjs-page';

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

export const usePdfPageLoader = (
  doc: PdfJsDist.PDFDocumentProxy | undefined,
  pageNo: number,
  scale: number
) =>
  useAsyncCallback(
    useCallback(async () => {
      if (!doc) throw new Error('PDF document is not loaded');
      return createPage(doc, pageNo, { scale });
    }, [doc, pageNo, scale])
  );
