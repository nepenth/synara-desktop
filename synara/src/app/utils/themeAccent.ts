/** Light-theme Primary.Main from `colors.css.ts` — the accent actually in use. */
export const DEFAULT_ACCENT_COLOR = '#1245a8';

/** Dark-theme Primary.Main from `colors.css.ts`. */
export const DARK_THEME_ACCENT_COLOR = '#bdb6ec';

const HEX_COLOR_REGEX = /^#[0-9a-f]{6}$/i;

export const normalizeAccentColor = (value?: string): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return HEX_COLOR_REGEX.test(trimmed) ? trimmed.toLowerCase() : undefined;
};

export const themeDefaultAccentColor = (kind: 'light' | 'dark'): string =>
  kind === 'dark' ? DARK_THEME_ACCENT_COLOR : DEFAULT_ACCENT_COLOR;
