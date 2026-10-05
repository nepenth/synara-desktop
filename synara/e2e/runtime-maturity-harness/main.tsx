import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import FocusTrap from 'focus-trap-react';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import parse, { domToReact } from 'html-react-parser';
import { CDATA, Document, Element, Text } from 'domhandler';
import { useForceUpdate } from '../../src/app/hooks/useForceUpdate';
import {
  usePdfPageLoader,
  usePdfDocumentLoader,
  usePdfJSLoader,
} from '../../src/app/plugins/pdfjs-dist';
import { AsyncStatus } from '../../src/app/hooks/useAsyncCallback';
import {
  getReactCustomHtmlParser,
  LINKIFY_OPTS,
} from '../../src/app/plugins/react-custom-html-parser';
import type { MatrixClientReading } from '../../src/app/utils/room';
import { reactDomNodes } from '../../src/app/utils/reactDomNodes';
import { RoomComposer } from '../../src/app/features/room/RoomComposer';
import { PdfViewer } from '../../src/app/components/Pdf-viewer/PdfViewer';
import { useEditor } from '../../src/app/components/editor/Editor';

// These code-block fixtures have no Matrix mentions or media, so no client reads occur.
const parserOptions = getReactCustomHtmlParser({} as MatrixClientReading, undefined, {
  linkifyOpts: LINKIFY_OPTS,
});
const nestedXmlNodes = new Document([
  new Element('strong', {}, [new CDATA([new Text('nested literal <kept>')])]),
]);

const createPdf = () => {
  const stream = '1 0 0 rg 0 0 100 100 re f\n0 0 0 rg BT /F1 8 Tf 10 10 Td (Compatibility) Tj ET\n';
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>',
    `<< /Length ${stream.length} >>\nstream\n${stream}endstream`,
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>',
    `<< /Length ${stream.length} >>\nstream\n${stream}endstream`,
  ];
  let body = '%PDF-1.4\n';
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(body.length);
    body += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xref = body.length;
  body += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n${offsets
    .slice(1)
    .map((offset) => `${String(offset).padStart(10, '0')} 00000 n \n`)
    .join('')}trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return URL.createObjectURL(new Blob([body], { type: 'application/pdf' }));
};

function PdfRenderLifecycle() {
  const completion = useRef<{ resolve?: () => void; reject?: (error: Error) => void }>({});
  const doc = useMemo(
    () =>
      ({
        getPage: async () => ({
          getViewport: () => ({ width: 10, height: 10 }),
          render: () => ({
            promise: new Promise<void>((resolve, reject) => {
              completion.current = { resolve, reject };
            }),
          }),
        }),
      }) as unknown as PDFDocumentProxy,
    []
  );
  const [state, load] = usePdfPageLoader(doc, 1, 1);
  const output = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (state.status !== AsyncStatus.Success) return;
    output.current?.append(state.data);
  }, [state]);
  return (
    <section aria-label="PDF render lifecycle">
      <button
        onClick={() => {
          void load().catch(() => undefined);
        }}
      >
        {state.status === AsyncStatus.Error ? 'Retry PDF render' : 'Start PDF render'}
      </button>
      <button onClick={() => completion.current.resolve?.()}>Finish PDF render</button>
      <button onClick={() => completion.current.reject?.(new Error('Controlled render failure'))}>
        Fail PDF render
      </button>
      <output data-testid="pdf-render-state">{state.status}</output>
      {state.status === AsyncStatus.Success && (
        <div data-testid="completed-pdf-render" ref={output} />
      )}
      {state.status === AsyncStatus.Error && <p role="alert">{String(state.error)}</p>}
    </section>
  );
}

function MaturityHarness() {
  const [count, update] = useForceUpdate();
  const editor = useEditor();
  const [composerState, setComposerState] = useState('[]');
  const [trap, setTrap] = useState(false);
  const [pdf, loadPdf] = usePdfJSLoader();
  const [src] = useState(createPdf);
  const [documentState, loadDocument] = usePdfDocumentLoader(
    pdf.status === AsyncStatus.Success ? pdf.data : undefined,
    src
  );
  const [pageState, loadPage] = usePdfPageLoader(
    documentState.status === AsyncStatus.Success ? documentState.data : undefined,
    1,
    1
  );
  useEffect(() => {
    void loadPdf().catch(() => undefined);
  }, [loadPdf]);
  useEffect(() => {
    if (pdf.status === AsyncStatus.Success) void loadDocument().catch(() => undefined);
  }, [pdf, loadDocument]);
  useEffect(() => {
    if (documentState.status === AsyncStatus.Success) void loadPage().catch(() => undefined);
  }, [documentState, loadPage]);
  useEffect(() => {
    if (pageState.status !== AsyncStatus.Success) return;
    const canvas = pageState.data;
    canvas.dataset.testid = 'pdf-canvas';
    document.getElementById('pdf-output')?.append(canvas);
  }, [pageState]);
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
      <PdfRenderLifecycle />
      <section aria-label="PDF viewer regression">
        <PdfViewer name="Render regression" src={src} requestClose={() => undefined} />
      </section>
      <div id="pdf-output" />
      <output data-testid="pdf-status">
        {pageState.status === AsyncStatus.Success ? 'PDF rendered' : 'PDF loading'}
      </output>
      <section aria-label="Code block regression">
        {parse('<pre></pre><pre><code></code></pre><pre><code>hello</code></pre>', parserOptions)}
      </section>
      <section aria-label="Nested XML regression">
        {domToReact(reactDomNodes([nestedXmlNodes]), parserOptions)}
      </section>
      <RoomComposer
        editor={editor}
        placeholder="Composer compatibility"
        onChange={(value) => setComposerState(JSON.stringify(value))}
      />
      <output data-testid="composer-state">{composerState}</output>
      {(pdf.status === AsyncStatus.Error ||
        documentState.status === AsyncStatus.Error ||
        pageState.status === AsyncStatus.Error) && <p role="alert">PDF failed</p>}
    </>
  );
}
if (new URLSearchParams(location.search).has('sync-recovery')) {
  void import('./sync-recovery').then(({ mountSyncRecovery }) => mountSyncRecovery());
} else {
  createRoot(document.getElementById('root')!).render(<MaturityHarness />);
}
