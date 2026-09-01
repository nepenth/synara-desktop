import sanitizeHtml, { type Transformer } from 'sanitize-html';
import { sanitizeCustomHtml } from '../../utils/sanitize';

/**
 * Keep untrusted event HTML below the same bounded presentation budget used by
 * the native clients. The required Matrix `body` remains the lossless fallback.
 */
export const MAX_NATIVE_FORMATTED_BODY_BYTES = 256 * 1024;

const utf8ByteLength = (value: string): number => new TextEncoder().encode(value).byteLength;

const MATRIX_V1_19_TAGS = [
  'a',
  'b',
  'blockquote',
  'br',
  'caption',
  'code',
  'del',
  'details',
  'div',
  'em',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'hr',
  'i',
  'img',
  'li',
  'ol',
  'p',
  'pre',
  's',
  'span',
  'strong',
  'sub',
  'summary',
  'sup',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
  'u',
  'ul',
] as const;

const MATRIX_V1_19_SCHEMES = new Set(['https', 'http', 'ftp', 'mailto', 'magnet']);
const MATRIX_COLOR_RE = /^#[0-9a-f]{6}$/i;
const MATRIX_MXC_RE = /^mxc:\/\/[^/\s]+\/[^/?#\s]+$/;
const MATRIX_LANGUAGE_RE = /^language-[\w+.-]+$/;
const MATRIX_OL_START_RE = /^-?\d+$/;

const absoluteMatrixHref = (value?: string): string | undefined => {
  if (
    !value ||
    Array.from(value).some((character) => {
      const codePoint = character.codePointAt(0) ?? 0;
      return codePoint <= 0x1f || codePoint === 0x7f;
    })
  ) {
    return undefined;
  }
  const match = value.match(/^([A-Za-z][A-Za-z0-9+.-]*):/);
  if (!match || !MATRIX_V1_19_SCHEMES.has(match[1].toLowerCase())) return undefined;
  return value;
};

const transformCurrentAnchor: Transformer = (_tagName, attribs) => {
  const href = absoluteMatrixHref(attribs.href);
  const next: Record<string, string> = {};
  if (href) {
    next.href = href;
    next.target = '_blank';
    next.rel = 'noreferrer noopener';
  }
  return {
    tagName: 'a',
    attribs: next,
  };
};

const transformCurrentSpan: Transformer = (_tagName, attribs) => {
  const next: Record<string, string> = {};
  const foreground = attribs['data-mx-color'] ?? attribs.color;
  const background = attribs['data-mx-bg-color'];
  if (foreground && MATRIX_COLOR_RE.test(foreground)) next['data-mx-color'] = foreground;
  if (background && MATRIX_COLOR_RE.test(background)) next['data-mx-bg-color'] = background;
  if ('data-mx-spoiler' in attribs) next['data-mx-spoiler'] = attribs['data-mx-spoiler'];
  if ('data-mx-maths' in attribs) next['data-mx-maths'] = attribs['data-mx-maths'];

  return { tagName: 'span', attribs: next };
};

const transformCurrentImage: Transformer = (_tagName, attribs) => {
  const next: Record<string, string> = {};
  if (attribs.src && MATRIX_MXC_RE.test(attribs.src)) next.src = attribs.src;
  if (attribs.alt) next.alt = attribs.alt;
  if (attribs.title) next.title = attribs.title;
  if (attribs.width && /^\d+$/.test(attribs.width)) next.width = attribs.width;
  if (attribs.height && /^\d+$/.test(attribs.height)) next.height = attribs.height;
  return { tagName: 'img', attribs: next };
};

export const sanitizeMatrixV119PresentationHtml = (html: string): string =>
  sanitizeHtml(html, {
    // `font` and `strike` are accepted only long enough to normalize legacy
    // events to their current v1.19 equivalents.
    allowedTags: [...MATRIX_V1_19_TAGS, 'font', 'strike'],
    allowedAttributes: {
      a: ['href', 'target', 'rel'],
      code: ['class'],
      div: ['data-mx-maths'],
      img: ['width', 'height', 'alt', 'title', 'src'],
      ol: ['start'],
      font: ['color', 'data-mx-bg-color', 'data-mx-color'],
      span: ['data-mx-bg-color', 'data-mx-color', 'data-mx-spoiler', 'data-mx-maths'],
    },
    allowedClasses: { code: ['language-*'] },
    transformTags: {
      a: transformCurrentAnchor,
      font: transformCurrentSpan,
      img: transformCurrentImage,
      strike: 's',
      span: transformCurrentSpan,
      code: (_tagName, attribs) => {
        const next: Record<string, string> = {};
        if (attribs.class && MATRIX_LANGUAGE_RE.test(attribs.class)) next.class = attribs.class;
        return { tagName: 'code', attribs: next };
      },
      ol: (_tagName, attribs) => {
        const next: Record<string, string> = {};
        if (attribs.start && MATRIX_OL_START_RE.test(attribs.start)) next.start = attribs.start;
        return { tagName: 'ol', attribs: next };
      },
      div: (_tagName, attribs) => {
        const next: Record<string, string> = {};
        if ('data-mx-maths' in attribs) next['data-mx-maths'] = attribs['data-mx-maths'];
        return { tagName: 'div', attribs: next };
      },
    },
    disallowedTagsMode: 'discard',
    nonTextTags: ['style', 'script', 'textarea', 'option', 'noscript', 'mx-reply'],
    allowedSchemes: [...MATRIX_V1_19_SCHEMES, 'mxc'],
    allowedSchemesByTag: {
      a: [...MATRIX_V1_19_SCHEMES],
      img: ['mxc'],
    },
    allowedSchemesAppliedToAttributes: ['href', 'src'],
    allowProtocolRelative: false,
    nestingLimit: 100,
  });

/** Return presentation-safe HTML, or `undefined` when the caller must show `body`. */
export const prepareNativeFormattedBody = (html: string): string | undefined => {
  // UTF-8 is never shorter than the number of UTF-16 code units. Avoid an
  // unnecessary allocation for obviously oversized payloads.
  if (html.length > MAX_NATIVE_FORMATTED_BODY_BYTES) return undefined;
  if (utf8ByteLength(html) > MAX_NATIVE_FORMATTED_BODY_BYTES) return undefined;

  try {
    // The editor sanitizer intentionally tolerates legacy Synara metadata.
    // Apply the exact current Matrix presentation profile afterwards so those
    // editing hints and non-spec attributes never reach the timeline DOM.
    const sanitized = sanitizeMatrixV119PresentationHtml(sanitizeCustomHtml(html));
    return sanitized.trim() ? sanitized : undefined;
  } catch {
    return undefined;
  }
};
