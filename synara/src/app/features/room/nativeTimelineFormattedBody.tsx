import { isTag, isText, type ChildNode } from 'domhandler';
import { reactDomNodes } from '../../utils/reactDomNodes';
import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { attributesToProps, domToReact, HTMLReactParserOptions } from 'html-react-parser';
import Linkify from 'linkify-react';
import type { IntermediateRepresentation, Opts as LinkifyOpts } from 'linkifyjs';
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
import {
  classifyNativeFormattedElement,
  projectNativeFormattedBody,
} from './nativeTimelinePresentationProjection';
import * as htmlCss from './nativeTimelineHtml.css';
import { MatrixColorSpan, MatrixColorSurface } from '../../components/message/MatrixColorSpan';
import { copyToClipboard } from '../../utils/dom';
import { useTimeoutToggle } from '../../hooks/useTimeoutToggle';
import * as customHtmlCss from '../../styles/CustomHtml.css';
import { convertDesktopFileSrc } from '../../utils/desktop';
import { IN_APP_LINK_ATTRIBUTE } from '../../utils/appLinks';
import { getMyUserId } from '../../state/nativeIdentity';
import { highlightText } from '../../utils/highlightText';
import {
  isNativeOpenableHref,
  nativeCustomEmojiContentUri,
  nativeLinkHostMismatch,
  parseNativePermalink,
  shortLinkLabel,
  type NativePermalinkTarget,
} from './nativeTimelineLinks';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';

/** Opens a matrix.to user, room or event link inside Synara. */
export type NativePermalinkHandler = (target: NativePermalinkTarget, anchor: HTMLElement) => void;

const NativePermalinkContext = createContext<NativePermalinkHandler | undefined>(undefined);

/**
 * Supplies in-app navigation for permalinks in formatted messages. Without
 * it (no router, e.g. a test harness) they stay ordinary external links.
 */
export const NativePermalinkProvider = NativePermalinkContext.Provider;

const nodeText = (node: ChildNode): string => {
  if (isText(node)) return node.data;
  if (isTag(node)) return node.children.map(nodeText).join('');
  return '';
};

const isModifiedClick = (event: React.MouseEvent): boolean =>
  event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey;

/** A custom emoji through Core's authenticated media; its shortcode if that fails. */
function NativeCustomEmoji({ mxc, label }: { mxc: string; label: string }) {
  const [failed, setFailed] = useState(false);
  const src = convertDesktopFileSrc(nativeCustomEmojiContentUri(mxc), 'synara-media');
  if (!src || failed) {
    return <span className={htmlCss.InlineImageFallback}>{label}</span>;
  }
  return (
    <img
      className={htmlCss.CustomEmoji}
      src={src}
      alt={label}
      title={label}
      data-mx-emoticon=""
      onError={() => setFailed(true)}
    />
  );
}

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
  const [copied, setCopied] = useTimeoutToggle();

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
      <div className={htmlCss.CodeToolbar}>
        <span className={htmlCss.CodeLanguage}>{languageLabel}</span>
        <button
          type="button"
          className={htmlCss.CodeCopyButton}
          data-native-code-copy="true"
          aria-label="Copy code"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            copyToClipboard(code);
            setCopied();
          }}
        >
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
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

function NativeSpoiler({
  children,
  reason,
  foreground,
  background,
}: {
  children: React.ReactNode;
  reason?: string;
  foreground?: string;
  background?: string;
}) {
  const [revealed, setRevealed] = useState(false);
  const normalizedReason = reason?.trim().slice(0, 160);

  if (revealed) {
    return (
      <MatrixColorSurface surface="spoiler">
        <span className={htmlCss.SpoilerContent}>
          <MatrixColorSpan foreground={foreground} background={background}>
            {children}
          </MatrixColorSpan>
        </span>
      </MatrixColorSurface>
    );
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

type NativeFormattedRenderContext = {
  onPermalink?: NativePermalinkHandler;
  ownUserId?: string;
  /** Search terms to mark in the text (message search results). */
  highlightRegex?: RegExp;
  /** Show a long bare web address as a short label; the link is unchanged. */
  shortenLinks?: boolean;
};

const renderNativeAnchor = (
  href: string,
  text: string,
  children: React.ReactNode,
  context: NativeFormattedRenderContext
): React.JSX.Element => {
  const permalink = parseNativePermalink(href);
  if (permalink) {
    const { onPermalink } = context;
    return (
      <a
        href={href}
        data-mention-pill=""
        className={customHtmlCss.Mention({
          highlight: permalink.kind === 'user' && permalink.userId === context.ownUserId,
        })}
        {...(onPermalink
          ? {
              [IN_APP_LINK_ATTRIBUTE]: '',
              onClick: (event: React.MouseEvent<HTMLAnchorElement>) => {
                if (isModifiedClick(event)) return;
                event.preventDefault();
                onPermalink(permalink, event.currentTarget);
              },
            }
          : { target: '_blank', rel: 'noreferrer noopener' })}
      >
        {children}
      </a>
    );
  }
  // The desktop opener only opens web and mail links (`desktop_url.rs`); an
  // ftp or magnet link would do nothing, so show its address to copy instead.
  if (!isNativeOpenableHref(href)) {
    return (
      <span title={href} data-native-unopenable-link="true">
        {children}
        <span className={htmlCss.LinkAddress}>{` (${href})`}</span>
      </span>
    );
  }
  // The webview has no status bar: name the destination on hover, and say
  // where a link really goes when its text is a different web address.
  const mismatchHost = nativeLinkHostMismatch(text, href);
  // Search results keep the whole address, so a match in it stays visible.
  const shortLabel =
    context.shortenLinks && !context.highlightRegex ? shortLinkLabel(text, href) : undefined;
  return (
    <>
      <a
        href={href}
        target="_blank"
        rel="noreferrer noopener"
        title={href}
        {...(shortLabel ? { 'data-native-short-link': 'true' } : {})}
      >
        {shortLabel ?? children}
      </a>
      {mismatchHost ? (
        <span className={htmlCss.LinkHostWarning} data-native-link-host-warning="true">
          {` (link to ${mismatchHost})`}
        </span>
      ) : null}
    </>
  );
};

const hasAncestorTag = (node: ChildNode, name: string): boolean => {
  for (let parent = node.parent; parent !== null; parent = parent.parent) {
    if (isTag(parent) && parent.name === name) return true;
  }
  return false;
};

export const createNativeFormattedHtmlParserOptions = (
  context: NativeFormattedRenderContext
): HTMLReactParserOptions => {
  const linkifyOptions = createNativeLinkifyOptions(context);
  const nativeFormattedHtmlParserOptions: HTMLReactParserOptions = {
    replace: (domNode) => {
      if (isText(domNode)) {
        const { highlightRegex } = context;
        // Inline code keeps its exact text (code blocks never reach here).
        const inCode =
          domNode.parent !== null && isTag(domNode.parent) && domNode.parent.name === 'code';
        if (inCode) return undefined;
        const content = highlightRegex
          ? highlightText(highlightRegex, [domNode.data])
          : domNode.data;
        // Bare addresses in formatted text become links, as in plain bodies.
        // Text already inside a link stays as it is.
        if (hasAncestorTag(domNode, 'a')) {
          return highlightRegex ? <>{content}</> : undefined;
        }
        return <Linkify options={linkifyOptions}>{content}</Linkify>;
      }
      if (!isTag(domNode) || !('name' in domNode)) {
        return undefined;
      }
      if (domNode.name === 'a' && domNode.attribs.href) {
        return renderNativeAnchor(
          domNode.attribs.href,
          nodeText(domNode),
          domToReact(reactDomNodes(domNode.children), nativeFormattedHtmlParserOptions),
          context
        );
      }
      const parentName =
        domNode.parent !== null && isTag(domNode.parent) ? domNode.parent.name : undefined;
      const presentation = classifyNativeFormattedElement(
        domNode.name,
        domNode.attribs,
        parentName
      );
      if (presentation === 'table') {
        return (
          <MatrixColorSurface surface="table">
            <div
              className={htmlCss.TableScroll}
              role="region"
              aria-label="Scrollable message table"
              // Horizontal overflow needs an explicit keyboard focus target.
              // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex
              tabIndex={0}
            >
              <table {...attributesToProps(domNode.attribs)}>
                {domToReact(reactDomNodes(domNode.children), nativeFormattedHtmlParserOptions)}
              </table>
            </div>
          </MatrixColorSurface>
        );
      }
      if (presentation === 'spoiler') {
        return (
          <NativeSpoiler
            reason={domNode.attribs['data-mx-spoiler']}
            foreground={domNode.attribs['data-mx-color']}
            background={domNode.attribs['data-mx-bg-color']}
          >
            {domToReact(reactDomNodes(domNode.children), nativeFormattedHtmlParserOptions)}
          </NativeSpoiler>
        );
      }
      if (presentation === 'matrixColor') {
        return (
          <MatrixColorSpan
            foreground={domNode.attribs['data-mx-color']}
            background={domNode.attribs['data-mx-bg-color']}
          >
            {domToReact(reactDomNodes(domNode.children), nativeFormattedHtmlParserOptions)}
          </MatrixColorSpan>
        );
      }
      if (presentation === 'customEmoji') {
        return (
          <NativeCustomEmoji
            mxc={domNode.attribs['data-mx-emoticon']}
            label={nodeText(domNode).trim() || 'Custom emoji'}
          />
        );
      }
      if (presentation === 'inlineImageFallback') {
        // Matrix formatted HTML only permits mxc:// image sources. Loading those
        // directly in the webview bypasses shared-core media authentication and
        // simply fails. Preserve the accessible producer fallback without a
        // resource request until inline media has an authenticated core handle.
        const label =
          domNode.attribs.alt?.trim() || domNode.attribs.title?.trim() || 'Inline image';
        return (
          <span className={htmlCss.InlineImageFallback} role="img" aria-label={label}>
            {label}
          </span>
        );
      }
      if (presentation === 'inlineCode') {
        return (
          <MatrixColorSurface surface="inlineCode">
            <code {...attributesToProps(domNode.attribs)}>
              {domToReact(reactDomNodes(domNode.children), nativeFormattedHtmlParserOptions)}
            </code>
          </MatrixColorSurface>
        );
      }
      if (presentation !== 'codeBlock') return undefined;
      const { code, languageClass } = nativeCodeBlockFromPreChildren(domNode.children);
      return (
        <MatrixColorSurface surface="codeBlock">
          <NativeCodeBlock code={code} languageClass={languageClass} />
        </MatrixColorSurface>
      );
    },
  };
  return nativeFormattedHtmlParserOptions;
};

export function NativeFormattedBody({
  html,
  fallbackBody,
  style,
  highlightRegex,
}: {
  html: string;
  fallbackBody: string;
  style?: React.CSSProperties;
  highlightRegex?: RegExp;
}) {
  const onPermalink = useContext(NativePermalinkContext);
  const ownUserId = getMyUserId() ?? undefined;
  const [shortenLinks] = useSetting(settingsAtom, 'shortenLinks');
  const parserOptions = useMemo(
    () =>
      createNativeFormattedHtmlParserOptions({
        onPermalink,
        ownUserId,
        highlightRegex,
        shortenLinks,
      }),
    [onPermalink, ownUserId, highlightRegex, shortenLinks]
  );
  const projection = useMemo(() => projectNativeFormattedBody(html), [html]);
  const parsed = useMemo(() => {
    if (!projection) return undefined;
    try {
      return domToReact(projection.domNodes, parserOptions);
    } catch {
      return undefined;
    }
  }, [parserOptions, projection]);

  return (
    <div
      className={htmlCss.FormattedBody}
      style={{ ...style, whiteSpace: parsed === undefined ? 'pre-wrap' : undefined }}
    >
      {parsed === undefined ? fallbackBody : parsed}
    </div>
  );
}

/**
 * Plain-text links: only addresses the desktop opener handles (an ftp or
 * magnet address is already readable as text), each through the formatted
 * renderer's anchor, so matrix.to links become mentions and permalinks too.
 */
const createNativeLinkifyOptions = (context: NativeFormattedRenderContext): LinkifyOpts => ({
  validate: {
    url: (value) => isNativeOpenableHref(value),
  },
  ignoreTags: ['span'],
  render: ({ tagName, attributes, content }: IntermediateRepresentation) => {
    const href = typeof attributes.href === 'string' ? attributes.href : undefined;
    if (tagName !== 'a' || !href) return <>{content}</>;
    return renderNativeAnchor(href, content, content, context);
  },
});

/**
 * Plain text with its links: message bodies without HTML, room topics and
 * pack attributions. Renders inline; the caller supplies the text box.
 */
export function NativeLinkifiedText({
  text,
  highlightRegex,
}: {
  text: string;
  highlightRegex?: RegExp;
}) {
  const onPermalink = useContext(NativePermalinkContext);
  const ownUserId = getMyUserId() ?? undefined;
  const [shortenLinks] = useSetting(settingsAtom, 'shortenLinks');
  const options = useMemo(
    () => createNativeLinkifyOptions({ onPermalink, ownUserId, highlightRegex, shortenLinks }),
    [onPermalink, ownUserId, highlightRegex, shortenLinks]
  );
  return (
    <Linkify options={options}>
      {highlightRegex ? highlightText(highlightRegex, [text]) : text}
    </Linkify>
  );
}

/**
 * A message's text outside the timeline (inbox, pins, search): the timeline's
 * sanitizer and renderer with no row state. Mentions, permalinks and custom
 * emoji behave as in the timeline; inside `NativeRoomPermalinks` (or a room)
 * permalinks open in Synara.
 */
export function NativeMessageText({
  body,
  formattedBody,
  highlightRegex,
}: {
  body: string;
  formattedBody?: string;
  highlightRegex?: RegExp;
}) {
  if (formattedBody) {
    return (
      <NativeFormattedBody
        html={formattedBody}
        fallbackBody={body}
        highlightRegex={highlightRegex}
      />
    );
  }
  return <NativeLinkifiedText text={body} highlightRegex={highlightRegex} />;
}
