import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import FocusTrap from 'focus-trap-react';
import { useForceUpdate } from '../../src/app/hooks/useForceUpdate';
import { createPage, usePdfDocumentLoader, usePdfJSLoader } from '../../src/app/plugins/pdfjs-dist';
import { AsyncStatus } from '../../src/app/hooks/useAsyncCallback';

const createPdf = () => {
  const stream = '1 0 0 rg 0 0 100 100 re f\n';
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R >>',
    `<< /Length ${stream.length} >>\nstream\n${stream}endstream`,
  ];
  let body = '%PDF-1.4\n';
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(body.length);
    body += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = body.length;
  body += `xref\n0 5\n0000000000 65535 f \n${offsets
    .slice(1)
    .map((offset) => `${String(offset).padStart(10, '0')} 00000 n \n`)
    .join('')}trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return URL.createObjectURL(new Blob([body], { type: 'application/pdf' }));
};

function MaturityHarness() {
  const [count, update] = useForceUpdate();
  const [trap, setTrap] = useState(false);
  const [pdf, loadPdf] = usePdfJSLoader();
  const [src] = useState(createPdf);
  const [documentState, loadDocument] = usePdfDocumentLoader(
    pdf.status === AsyncStatus.Success ? pdf.data : undefined,
    src
  );
  const [rendered, setRendered] = useState(false);
  useEffect(() => {
    void loadPdf().catch(() => undefined);
  }, [loadPdf]);
  useEffect(() => {
    if (pdf.status === AsyncStatus.Success) void loadDocument().catch(() => undefined);
  }, [pdf, loadDocument]);
  useEffect(() => {
    if (documentState.status !== AsyncStatus.Success) return;
    void createPage(documentState.data, 1, { scale: 1 }).then((canvas) => {
      canvas.dataset.testid = 'pdf-canvas';
      document.getElementById('pdf-output')?.append(canvas);
      setRendered(true);
    });
  }, [documentState]);
  return (
    <>
      <button onClick={update}>Update {count}</button>
      <button onClick={() => setTrap(true)}>Open dialog</button>
      {trap && (
        <FocusTrap>
          <div role="dialog">
            <button onClick={() => setTrap(false)}>Close</button>
            <button>Second</button>
          </div>
        </FocusTrap>
      )}
      <div id="pdf-output" />
      <output>{rendered ? 'PDF rendered' : 'PDF loading'}</output>
      {(pdf.status === AsyncStatus.Error || documentState.status === AsyncStatus.Error) && (
        <p role="alert">PDF failed</p>
      )}
    </>
  );
}
createRoot(document.getElementById('root')!).render(<MaturityHarness />);
