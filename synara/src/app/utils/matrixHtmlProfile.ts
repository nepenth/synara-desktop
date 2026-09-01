export const MATRIX_HTML_PROFILE_VERSION = '2026-05-19';

export const MATRIX_HTML_ALLOWED_SCHEMES = ['https', 'http', 'ftp', 'mailto', 'magnet'];

export const MATRIX_HTML_EMITTED_TAGS = [
  'a',
  'blockquote',
  'br',
  'code',
  'del',
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
  'sup',
  'u',
  'ul',
] as const;

export const MATRIX_HTML_INBOUND_LEGACY_TAGS = [
  'b',
  'caption',
  'details',
  'div',
  'font',
  'strike',
  'summary',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
] as const;

export const MATRIX_HTML_ALLOWED_TAGS = [
  ...MATRIX_HTML_EMITTED_TAGS,
  ...MATRIX_HTML_INBOUND_LEGACY_TAGS,
] as const;

export const MATRIX_HTML_EMITTED_ATTRIBUTES = {
  a: ['href'],
  code: ['class', 'data-label'],
  font: ['color', 'data-mx-bg-color', 'data-mx-color'],
  img: ['alt', 'data-mx-emoticon', 'height', 'src', 'title', 'width'],
  ol: ['start'],
  pre: ['class'],
  span: ['data-mx-bg-color', 'data-mx-color', 'data-mx-maths', 'data-mx-spoiler'],
} as const;

export const SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES = ['data-md'] as const;

export const MATRIX_HTML_INBOUND_ATTRIBUTES = {
  ...MATRIX_HTML_EMITTED_ATTRIBUTES,
  a: ['href', 'name', 'rel', 'target', ...SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES],
  blockquote: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  code: [...MATRIX_HTML_EMITTED_ATTRIBUTES.code, ...SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES],
  del: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  em: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h1: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h2: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h3: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h4: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h5: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  h6: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  i: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  ol: [...MATRIX_HTML_EMITTED_ATTRIBUTES.ol, 'type', ...SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES],
  pre: [...MATRIX_HTML_EMITTED_ATTRIBUTES.pre, ...SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES],
  s: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  span: [
    ...MATRIX_HTML_EMITTED_ATTRIBUTES.span,
    'data-mx-pill',
    'data-mx-ping',
    ...SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  ],
  strong: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  u: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
  ul: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
} as const;

export const MATRIX_HTML_PROFILE = {
  version: MATRIX_HTML_PROFILE_VERSION,
  emittedTags: MATRIX_HTML_EMITTED_TAGS,
  inboundLegacyTags: MATRIX_HTML_INBOUND_LEGACY_TAGS,
  allowedTags: MATRIX_HTML_ALLOWED_TAGS,
  emittedAttributes: MATRIX_HTML_EMITTED_ATTRIBUTES,
  inboundAttributes: MATRIX_HTML_INBOUND_ATTRIBUTES,
  allowedSchemes: MATRIX_HTML_ALLOWED_SCHEMES,
  inboundEditHintAttributes: SYNARA_INBOUND_EDIT_HINT_ATTRIBUTES,
} as const;
