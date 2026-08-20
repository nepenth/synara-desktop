import React, { useEffect, useMemo, useState } from 'react';
import parse, { Element, HTMLReactParserOptions } from 'html-react-parser';
import { sanitizeCustomHtml } from '../../utils/sanitize';
import {
  NATIVE_PRISM_CHAR_LIMIT,
  countCodeLines,
  displayCodeLanguage,
  formatLineNumbers,
  highlightNativeCode,
  nativeCodeBlockFromPreChildren,
  prismLanguageClass,
} from './nativeTimelineCodeHighlight';
import * as htmlCss from './nativeTimelineHtml.css';

function NativeCodeBlock({ code, languageClass }: { code: string; languageClass?: string }) {
  const lineCount = countCodeLines(code);
  const lineNumbers = formatLineNumbers(lineCount);
  const languageLabel = displayCodeLanguage(languageClass);
  const className = prismLanguageClass(languageClass);
  const largeCode = code.length > NATIVE_PRISM_CHAR_LIMIT;
  const [highlightedHtml, setHighlightedHtml] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (largeCode) {
      setHighlightedHtml(undefined);
      return undefined;
    }

    let cancelled = false;
    // Load ReactPrism so Prism language grammars register on the shared Prism instance.
    void import('../../plugins/react-prism/ReactPrism').then(() => {
      if (cancelled) return;
      setHighlightedHtml(highlightNativeCode(code, languageClass));
    });

    return () => {
      cancelled = true;
    };
  }, [code, languageClass, largeCode]);

  return (
    <pre className={htmlCss.CodePanel} data-native-code-block="true">
      <span className={htmlCss.CodeLanguage}>{languageLabel}</span>
      <div className={htmlCss.CodeScroll}>
        <span className={htmlCss.CodeLineNumbers} aria-hidden="true">
          {lineNumbers}
        </span>
        {highlightedHtml === undefined ? (
          <code className={className}>{code}</code>
        ) : (
          <code
            className={className}
            // Highlighted markup is produced from already-sanitized text via Prism.
            // eslint-disable-next-line react/no-danger
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        )}
      </div>
    </pre>
  );
}

const nativeFormattedHtmlParserOptions: HTMLReactParserOptions = {
  replace: (domNode) => {
    if (!(domNode instanceof Element) || !('name' in domNode)) {
      return undefined;
    }
    if (domNode.name !== 'pre') {
      return undefined;
    }
    const { code, languageClass } = nativeCodeBlockFromPreChildren(domNode.children);
    return <NativeCodeBlock code={code} languageClass={languageClass} />;
  },
};

export function NativeFormattedBody({
  html,
  style,
}: {
  html: string;
  style?: React.CSSProperties;
}) {
  const sanitized = useMemo(() => sanitizeCustomHtml(html), [html]);
  const parsed = useMemo(() => parse(sanitized, nativeFormattedHtmlParserOptions), [sanitized]);

  return (
    <div className={htmlCss.FormattedBody} style={style}>
      {parsed}
    </div>
  );
}
