/**
 * Interface and monospace font settings.
 *
 * `index.css` starts `--font-secondary` and `--font-mono` with an optional
 * slot, `var(--synara-font-ui-custom,)` and `var(--synara-font-mono-custom,)`,
 * so a chosen family goes in front of the platform stack (the Linux override
 * included) and every fallback stays in place. The slots are set inline on
 * the root element; their value ends with a comma.
 */
export type InterfaceFontChoice = 'default' | 'monospace' | 'custom';
export type MonospaceFontChoice = 'default' | 'custom';

export const INTERFACE_FONT_CHOICES: readonly InterfaceFontChoice[] = [
  'default',
  'monospace',
  'custom',
];
export const MONOSPACE_FONT_CHOICES: readonly MonospaceFontChoice[] = ['default', 'custom'];

export const INTERFACE_FONT_SLOT = '--synara-font-ui-custom';
export const MONOSPACE_FONT_SLOT = '--synara-font-mono-custom';

const MAX_FONT_FAMILIES = 4;
const MAX_FONT_FAMILY_LENGTH = 64;

/** Generic families stay unquoted; quoted they would name a font called "serif". */
const GENERIC_FONT_FAMILIES = new Set([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  'emoji',
  'math',
]);

export const normalizeInterfaceFont = (value: unknown): InterfaceFontChoice =>
  INTERFACE_FONT_CHOICES.includes(value as InterfaceFontChoice)
    ? (value as InterfaceFontChoice)
    : 'default';

export const normalizeMonospaceFont = (value: unknown): MonospaceFontChoice =>
  MONOSPACE_FONT_CHOICES.includes(value as MonospaceFontChoice)
    ? (value as MonospaceFontChoice)
    : 'default';

/**
 * A comma-separated family list reduced to letters, digits, spaces, `.`, `_`
 * and `-` per family. The value is written into CSS, so quotes, backslashes,
 * semicolons, braces and parentheses never survive. Up to four families.
 */
export const sanitizeFontFamilyList = (value: unknown): string => {
  if (typeof value !== 'string') return '';
  return value
    .slice(0, MAX_FONT_FAMILIES * (MAX_FONT_FAMILY_LENGTH + 2))
    .split(',')
    .map((family) =>
      family
        .replace(/[^\p{L}\p{N} ._-]/gu, '')
        .replace(/\s+/g, ' ')
        .trim()
        .slice(0, MAX_FONT_FAMILY_LENGTH)
        .trim()
    )
    .filter((family) => family.length > 0)
    .slice(0, MAX_FONT_FAMILIES)
    .join(', ');
};

/** CSS `font-family` list for a sanitized family list, or `undefined` when empty. */
export const fontFamilyListCss = (value: string): string | undefined => {
  const families = sanitizeFontFamilyList(value)
    .split(', ')
    .filter((family) => family.length > 0);
  if (families.length === 0) return undefined;
  return families
    .map((family) =>
      GENERIC_FONT_FAMILIES.has(family.toLowerCase()) ? family.toLowerCase() : `"${family}"`
    )
    .join(', ');
};

/** Slot value in front of the interface stack, or `undefined` for the default. */
export const interfaceFontSlot = (
  choice: InterfaceFontChoice,
  family: string
): string | undefined => {
  if (choice === 'monospace') return 'var(--font-mono),';
  if (choice !== 'custom') return undefined;
  const css = fontFamilyListCss(family);
  return css ? `${css},` : undefined;
};

/** Slot value in front of the monospace stack, or `undefined` for the default. */
export const monospaceFontSlot = (
  choice: MonospaceFontChoice,
  family: string
): string | undefined => {
  if (choice !== 'custom') return undefined;
  const css = fontFamilyListCss(family);
  return css ? `${css},` : undefined;
};

export type InterfaceFontSettings = {
  interfaceFont: InterfaceFontChoice;
  interfaceFontFamily: string;
  monospaceFont: MonospaceFontChoice;
  monospaceFontFamily: string;
};

type StyleTarget = {
  style: Pick<CSSStyleDeclaration, 'setProperty' | 'removeProperty'>;
};

const setSlot = (target: StyleTarget, name: string, value: string | undefined) => {
  if (value) target.style.setProperty(name, value);
  else target.style.removeProperty(name);
};

/** Writes both slots on `target` (the root element), clearing the defaults. */
export const applyInterfaceFonts = (target: StyleTarget, settings: InterfaceFontSettings) => {
  setSlot(
    target,
    INTERFACE_FONT_SLOT,
    interfaceFontSlot(settings.interfaceFont, settings.interfaceFontFamily)
  );
  setSlot(
    target,
    MONOSPACE_FONT_SLOT,
    monospaceFontSlot(settings.monospaceFont, settings.monospaceFontFamily)
  );
};
