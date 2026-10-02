import type * as PdfJsDist from 'pdfjs-dist';
import type { GetViewportParameters } from 'pdfjs-dist/types/src/display/api';

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

  await page.render({
    canvas,
    canvasContext: context,
    viewport: pageViewport,
  }).promise;

  return canvas;
};
