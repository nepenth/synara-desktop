export const NATIVE_SYNTAX_ROLES = [
  'meta',
  'comment',
  'punctuation',
  'property',
  'number',
  'string',
  'operator',
  'function',
  'keyword',
  'regex',
] as const;

export type NativeSyntaxRole = typeof NATIVE_SYNTAX_ROLES[number];
export type NativeSyntaxPalette = Record<NativeSyntaxRole, string>;

/**
 * Opaque syntax colors measured against every built-in and derived code-panel surface.
 * Ordinary palettes meet WCAG AA (4.5:1); increased-contrast palettes target 7:1.
 */
export const NATIVE_SYNTAX_PALETTES = {
  light: {
    meta: '#40566B',
    comment: '#40566B',
    punctuation: '#30343B',
    property: '#A31545',
    number: '#5A32A3',
    string: '#326300',
    operator: '#30343B',
    function: '#685300',
    keyword: '#005F73',
    regex: '#7A3E00',
  },
  dark: {
    meta: '#B5C0CC',
    comment: '#B5C0CC',
    punctuation: '#F3F4F6',
    property: '#FF9AB5',
    number: '#D0B0FF',
    string: '#B4E36D',
    operator: '#F3F4F6',
    function: '#F3D86B',
    keyword: '#7DDDF2',
    regex: '#FFB45B',
  },
  moreLight: {
    meta: '#1F3B53',
    comment: '#1F3B53',
    punctuation: '#101317',
    property: '#7A002C',
    number: '#341078',
    string: '#183E00',
    operator: '#101317',
    function: '#483900',
    keyword: '#004252',
    regex: '#5A2A00',
  },
  moreDark: {
    meta: '#D9E2EC',
    comment: '#D9E2EC',
    punctuation: '#FFFFFF',
    property: '#FFD4DE',
    number: '#EAD9FF',
    string: '#D4F5A8',
    operator: '#FFFFFF',
    function: '#FFE98A',
    keyword: '#B8F1FF',
    regex: '#FFD6A6',
  },
} as const satisfies Record<'light' | 'dark' | 'moreLight' | 'moreDark', NativeSyntaxPalette>;
