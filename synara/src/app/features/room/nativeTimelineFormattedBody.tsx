import React, { useEffect, useMemo, useState } from 'react';
import parse, {
  attributesToProps,
  domToReact,
  Element,
  HTMLReactParserOptions,
} from 'html-react-parser';
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
import { prepareNativeFormattedBody } from './nativeTimelineRichText';
import * as htmlCss from './nativeTimelineHtml.css';

export function NativeCodeBlock({ code, languageClass }: { code: string; languageClass?: string }) {
  // Chromium does not paint an extra gutter row for a final line terminator,
  // but the code node itself must retain the exact source for selection/copy.
  const gutterCode = displayCodeText(code);
  const lineCount = countCodeLines(gutterCode);
  const lineNumbers = formatLineNumbers(lineCount);
  const inferred = inferCodeLanguage(code, languageClass);
  const languageLabel = inferred ?? 'code';
  const className = inferred ? `language-${inferred}` : undefined;
  const largeCode = code.length > NATIVE_PRISM_CHAR_LIMIT;
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
          const html = highlightNativeCode(code, className);
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
  }, [code, className, largeCode]);

  return (
    <pre className={htmlCss.CodePanel} data-native-code-block="true">
      <span className={htmlCss.CodeLanguage}>{languageLabel}</span>
      <div className={htmlCss.CodeRow}>
        <span className={htmlCss.CodeLineNumbers} aria-hidden="true">
          {lineNumbers}
        </span>
        <div className={htmlCss.CodeScroll}>
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
      </div>
    </pre>
  );
}

function NativeSpoiler({ children, reason }: { children: React.ReactNode; reason?: string }) {
  const [revealed, setRevealed] = useState(false);
  const normalizedReason = reason?.trim().slice(0, 160);

  if (revealed) {
    return <span className={htmlCss.SpoilerContent}>{children}</span>;
  }

  return (
    <button
      type="button"
      className={htmlCss.SpoilerButton}
      aria-label={normalizedReason ? `Reveal spoiler: ${normalizedReason}` : 'Reveal spoiler'}
      onClick={() => setRevealed(true)}
    >
      {normalizedReason ? `Spoiler: ${normalizedReason}` : 'Spoiler'} · reveal
    </button>
  );
}

const nativeFormattedHtmlParserOptions: HTMLReactParserOptions = {
  replace: (domNode) => {
    if (!(domNode instanceof Element) || !('name' in domNode)) {
      return undefined;
    }
    if (domNode.name === 'table') {
      return (
        <div
          className={htmlCss.TableScroll}
          role="region"
          aria-label="Scrollable message table"
          // Horizontal overflow needs an explicit keyboard focus target.
          // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex
          tabIndex={0}
        >
          <table {...attributesToProps(domNode.attribs)}>
            {domToReact(domNode.children, nativeFormattedHtmlParserOptions)}
          </table>
        </div>
      );
    }
    if (domNode.name === 'span' && 'data-mx-spoiler' in domNode.attribs) {
      return (
        <NativeSpoiler reason={domNode.attribs['data-mx-spoiler']}>
          {domToReact(domNode.children, nativeFormattedHtmlParserOptions)}
        </NativeSpoiler>
      );
    }
    if (domNode.name === 'img') {
      // Matrix formatted HTML only permits mxc:// image sources. Loading those
      // directly in the webview bypasses shared-core media authentication and
      // simply fails. Preserve the accessible producer fallback without a
      // resource request until inline media has an authenticated core handle.
      const label = domNode.attribs.alt?.trim() || domNode.attribs.title?.trim() || 'Inline image';
      return (
        <span className={htmlCss.InlineImageFallback} role="img" aria-label={label}>
          {label}
        </span>
      );
    }
    if (domNode.name !== 'pre') return undefined;
    const { code, languageClass } = nativeCodeBlockFromPreChildren(domNode.children);
    return <NativeCodeBlock code={code} languageClass={languageClass} />;
  },
};

export function NativeFormattedBody({
  html,
  fallbackBody,
  style,
}: {
  html: string;
  fallbackBody: string;
  style?: React.CSSProperties;
}) {
  const sanitized = useMemo(() => prepareNativeFormattedBody(html), [html]);
  const parsed = useMemo(() => {
    if (!sanitized) return undefined;
    try {
      return parse(sanitized, nativeFormattedHtmlParserOptions);
    } catch {
      return undefined;
    }
  }, [sanitized]);

  return (
    <div
      className={htmlCss.FormattedBody}
      style={{ ...style, whiteSpace: parsed === undefined ? 'pre-wrap' : undefined }}
    >
      {parsed === undefined ? fallbackBody : parsed}
    </div>
  );
}
