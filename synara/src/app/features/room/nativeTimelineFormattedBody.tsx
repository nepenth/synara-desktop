import React, { useEffect, useMemo, useState } from 'react';
import parse, { Element, HTMLReactParserOptions } from 'html-react-parser';
import { sanitizeCustomHtml } from '../../utils/sanitize';
import '../../plugins/react-prism/ReactPrism.css';
import {
  NATIVE_PRISM_CHAR_LIMIT,
  countCodeLines,
  displayCodeText,
  formatLineNumbers,
  highlightNativeCode,
  inferCodeLanguage,
  nativeCodeBlockFromPreChildren,
} from './nativeTimelineCodeHighlight';
import * as htmlCss from './nativeTimelineHtml.css';

export function NativeCodeBlock({ code, languageClass }: { code: string; languageClass?: string }) {
  const displayCode = displayCodeText(code);
  const lineCount = countCodeLines(displayCode);
  const lineNumbers = formatLineNumbers(lineCount);
  const inferred = inferCodeLanguage(displayCode, languageClass);
  const languageLabel = inferred ?? 'code';
  const className = inferred ? `language-${inferred}` : undefined;
  const largeCode = displayCode.length > NATIVE_PRISM_CHAR_LIMIT;
  const [highlightedHtml, setHighlightedHtml] = useState<string | undefined>(undefined);

  useEffect(() => {
    setHighlightedHtml(undefined);
    if (largeCode) {
      return undefined;
    }

    let cancelled = false;
    // Load ReactPrism so Prism language grammars register on the shared Prism instance.
    void import('../../plugins/react-prism/ReactPrism')
      .then(() => {
        if (cancelled) return;
        try {
          const html = highlightNativeCode(displayCode, className);
          if (cancelled) return;
          setHighlightedHtml(html);
        } catch {
          if (!cancelled) setHighlightedHtml(undefined);
        }
      })
      .catch(() => {
        if (!cancelled) setHighlightedHtml(undefined);
      });

    return () => {
      cancelled = true;
    };
  }, [displayCode, className, largeCode]);

  return (
    <pre className={htmlCss.CodePanel} data-native-code-block="true">
      <span className={htmlCss.CodeLanguage}>{languageLabel}</span>
      <div className={htmlCss.CodeRow}>
        <span className={htmlCss.CodeLineNumbers} aria-hidden="true">
          {lineNumbers}
        </span>
        <div className={htmlCss.CodeScroll}>
          {highlightedHtml === undefined ? (
            <code className={className}>{displayCode}</code>
          ) : (
            <code
              className={className}
              // Highlighted markup is produced from already-sanitized text via Prism.
              // eslint-disable-next-line react/no-danger
              dangerouslySetInnerHTML={{ __html: highlightedHtml }}
            />
          )}
        </div>
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
