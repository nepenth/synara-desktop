export const DEFAULT_ACCENT_COLOR = '#6bdbb8';

const HEX_COLOR_REGEX = /^#[0-9a-f]{6}$/i;

export const normalizeAccentColor = (value?: string): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return HEX_COLOR_REGEX.test(trimmed) ? trimmed.toLowerCase() : undefined;
};
