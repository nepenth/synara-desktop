import sanitizeHtml, { Transformer } from 'sanitize-html';
import {
  MATRIX_HTML_ALLOWED_SCHEMES,
  MATRIX_HTML_ALLOWED_TAGS,
  MATRIX_HTML_INBOUND_ATTRIBUTES,
} from './matrixHtmlProfile';

const MAX_TAG_NESTING = 100;
const MATRIX_COLOR_RE = /^#[0-9a-f]{6}$/i;
const matrixHtmlInboundAttributes = Object.fromEntries(
  Object.entries(MATRIX_HTML_INBOUND_ATTRIBUTES).map(([tag, attributes]) => [tag, [...attributes]])
);

const transformColorTag: Transformer = (_tagName, attribs) => {
  const next = { ...attribs };
  delete next.style;
  const legacyForeground = next.color;
  delete next.color;
  if (!next['data-mx-color'] && legacyForeground) next['data-mx-color'] = legacyForeground;
  if (next['data-mx-color'] && !MATRIX_COLOR_RE.test(next['data-mx-color'])) {
    delete next['data-mx-color'];
  }
  if (next['data-mx-bg-color'] && !MATRIX_COLOR_RE.test(next['data-mx-bg-color'])) {
    delete next['data-mx-bg-color'];
  }
  return { tagName: 'span', attribs: next };
};

const transformATag: Transformer = (tagName, attribs) => ({
  tagName,
  attribs: {
    ...attribs,
    rel: 'noreferrer noopener',
    target: '_blank',
  },
});

const transformImgTag: Transformer = (tagName, attribs) => {
  const { src } = attribs;
  if (typeof src === 'string' && src.startsWith('mxc://') === false) {
    return {
      tagName: 'a',
      attribs: {
        href: src,
        rel: 'noreferrer noopener',
        target: '_blank',
      },
      text: attribs.alt || src,
    };
  }
  return {
    tagName,
    attribs: {
      ...attribs,
    },
  };
};

export const sanitizeCustomHtml = (customHtml: string): string =>
  sanitizeHtml(customHtml, {
    allowedTags: [...MATRIX_HTML_ALLOWED_TAGS],
    allowedAttributes: matrixHtmlInboundAttributes,
    disallowedTagsMode: 'discard',
    allowedSchemes: [...MATRIX_HTML_ALLOWED_SCHEMES],
    allowedSchemesByTag: {
      a: [...MATRIX_HTML_ALLOWED_SCHEMES],
    },
    allowedSchemesAppliedToAttributes: ['href'],
    allowProtocolRelative: false,
    allowedClasses: {
      code: ['language-*'],
    },
    transformTags: {
      font: transformColorTag,
      span: transformColorTag,
      a: transformATag,
      img: transformImgTag,
    },
    nonTextTags: ['style', 'script', 'textarea', 'option', 'noscript', 'mx-reply'],
    nestingLimit: MAX_TAG_NESTING,
  });

export const sanitizeText = (body: string) => {
  const tagsToReplace: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  };
  return body.replace(/[&<>'"]/g, (tag) => tagsToReplace[tag] || tag);
};
