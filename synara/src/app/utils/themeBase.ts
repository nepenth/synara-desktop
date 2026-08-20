import chroma from 'chroma-js';

export const DEFAULT_THEME_BASE_COLOR = '#2b2d31';
export const THEME_BASE_COLOR_SETTING_KEY = 'themeBaseColor';

export const THEME_BASE_PRESETS = [
  { id: 'graphite', hex: '#2b2d31', label: 'Graphite' },
  { id: 'blurple', hex: '#5865f2', label: 'Blurple' },
  { id: 'teal', hex: '#0d9488', label: 'Teal' },
  { id: 'slate', hex: '#64748b', label: 'Slate' },
  { id: 'amber', hex: '#b45309', label: 'Amber' },
  { id: 'rose', hex: '#be123c', label: 'Rose' },
] as const;

const HEX_COLOR_REGEX = /^#[0-9a-f]{6}$/i;

export type ThemeRampKind = 'light' | 'dark';

/** Folds tokens each chrome column consumes after ThemeManager applies the ramp. */
export const THEME_CHROME_ROLES = {
  rail: 'background',
  roomList: 'surface',
  chat: 'surfaceVariant',
  composer: 'secondaryContainer',
} as const;

export type ThemeChromeRole = keyof typeof THEME_CHROME_ROLES;

export type ThemeSurfaceScale = {
  Container: string;
  ContainerHover: string;
  ContainerActive: string;
  ContainerLine: string;
  OnContainer: string;
};

export type ThemeSurfaceRamp = {
  background: ThemeSurfaceScale;
  surface: ThemeSurfaceScale;
  surfaceVariant: ThemeSurfaceScale;
  secondaryContainer: ThemeSurfaceScale;
  chrome: string;
  overlay: string;
  shadow: string;
  focusRing: string;
};

export type ThemeChromeColors = {
  rail: string;
  roomList: string;
  chat: string;
  composer: string;
};

export const normalizeThemeBaseColor = (value?: string): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return HEX_COLOR_REGEX.test(trimmed) ? trimmed.toLowerCase() : undefined;
};

export const resolveThemeBaseColor = (value?: string): string =>
  normalizeThemeBaseColor(value) ?? DEFAULT_THEME_BASE_COLOR;

export const shouldApplyDerivedThemeRamp = (themeId: string | undefined): boolean =>
  themeId === 'light-theme' || themeId === 'dark-theme' || themeId === undefined;

const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

const hslHex = (hue: number, saturation: number, lightness: number): string =>
  chroma.hsl(Number.isNaN(hue) ? 220 : hue, clamp(saturation, 0, 1), clamp(lightness, 0, 1)).hex();

const scaleFromStops = (
  hue: number,
  saturation: number,
  stops: { container: number; hover: number; active: number; line: number },
  onContainer: string
): ThemeSurfaceScale => ({
  Container: hslHex(hue, saturation, stops.container),
  ContainerHover: hslHex(hue, saturation, stops.hover),
  ContainerActive: hslHex(hue, saturation, stops.active),
  ContainerLine: hslHex(hue, saturation, stops.line),
  OnContainer: onContainer,
});

const tintScale = (
  scale: ThemeSurfaceScale,
  baseHex: string,
  mixRatio: number
): ThemeSurfaceScale => ({
  Container: chroma.mix(scale.Container, baseHex, mixRatio, 'rgb').hex(),
  ContainerHover: chroma.mix(scale.ContainerHover, baseHex, mixRatio, 'rgb').hex(),
  ContainerActive: chroma.mix(scale.ContainerActive, baseHex, mixRatio, 'rgb').hex(),
  ContainerLine: chroma.mix(scale.ContainerLine, baseHex, mixRatio, 'rgb').hex(),
  OnContainer: scale.OnContainer,
});

/** Stacked Discord-like greys: rail < room list < chat < composer. Hue tints; mix keeps white/black distinct. */
export const deriveThemeSurfaceRamp = (
  baseColor: string,
  kind: ThemeRampKind
): ThemeSurfaceRamp => {
  const hex = resolveThemeBaseColor(baseColor);
  const [rawHue, rawSaturation] = chroma(hex).hsl();
  const hue = Number.isNaN(rawHue) ? 220 : rawHue;
  const sourceSaturation = Number.isNaN(rawSaturation) ? 0 : rawSaturation;

  if (kind === 'dark') {
    const saturation = clamp(sourceSaturation * 0.45, 0.045, 0.145);
    const onContainer = hslHex(hue, saturation * 0.22, 0.95);
    const mixRatio = clamp(0.1 + sourceSaturation * 0.12, 0.1, 0.22);
    const stacked = {
      background: scaleFromStops(
        hue,
        saturation,
        { container: 0.075, hover: 0.11, active: 0.135, line: 0.16 },
        onContainer
      ),
      surface: scaleFromStops(
        hue,
        saturation * 0.92,
        { container: 0.1, hover: 0.13, active: 0.155, line: 0.18 },
        onContainer
      ),
      surfaceVariant: scaleFromStops(
        hue,
        saturation * 0.88,
        { container: 0.104, hover: 0.132, active: 0.158, line: 0.185 },
        onContainer
      ),
      secondaryContainer: scaleFromStops(
        hue,
        saturation * 0.8,
        { container: 0.135, hover: 0.165, active: 0.195, line: 0.225 },
        onContainer
      ),
    };
    return {
      background: tintScale(stacked.background, hex, mixRatio),
      surface: tintScale(stacked.surface, hex, mixRatio),
      surfaceVariant: tintScale(stacked.surfaceVariant, hex, mixRatio),
      secondaryContainer: tintScale(stacked.secondaryContainer, hex, mixRatio),
      chrome: chroma.mix(hslHex(hue, saturation, 0.07), hex, mixRatio, 'rgb').hex(),
      overlay: 'rgba(0, 0, 0, 0.72)',
      shadow: 'rgba(0, 0, 0, 0.55)',
      focusRing: 'rgba(255, 255, 255, 0.45)',
    };
  }

  const saturation = clamp(sourceSaturation * 0.28, 0.02, 0.09);
  const onContainer = hslHex(hue, Math.min(saturation * 1.4, 0.12), 0.09);
  const mixRatio = clamp(0.06 + sourceSaturation * 0.1, 0.06, 0.16);
  const stacked = {
    background: scaleFromStops(
      hue,
      saturation,
      { container: 0.895, hover: 0.87, active: 0.845, line: 0.82 },
      onContainer
    ),
    surface: scaleFromStops(
      hue,
      saturation * 0.75,
      { container: 0.952, hover: 0.93, active: 0.91, line: 0.88 },
      onContainer
    ),
    surfaceVariant: scaleFromStops(
      hue,
      saturation * 0.45,
      { container: 1, hover: 0.975, active: 0.955, line: 0.92 },
      onContainer
    ),
    secondaryContainer: scaleFromStops(
      hue,
      saturation * 0.7,
      { container: 0.91, hover: 0.885, active: 0.86, line: 0.83 },
      onContainer
    ),
  };
  return {
    background: tintScale(stacked.background, hex, mixRatio),
    surface: tintScale(stacked.surface, hex, mixRatio),
    surfaceVariant: tintScale(stacked.surfaceVariant, hex, mixRatio),
    secondaryContainer: tintScale(stacked.secondaryContainer, hex, mixRatio),
    chrome: chroma.mix(hslHex(hue, saturation * 0.45, 1), hex, mixRatio * 0.4, 'rgb').hex(),
    overlay: 'rgba(15, 17, 21, 0.45)',
    shadow: 'rgba(15, 17, 21, 0.16)',
    focusRing: 'rgba(15, 17, 21, 0.45)',
  };
};

export const chromeColorsForRamp = (ramp: ThemeSurfaceRamp): ThemeChromeColors => ({
  rail: ramp.background.Container,
  roomList: ramp.surface.Container,
  chat: ramp.surfaceVariant.Container,
  composer: ramp.secondaryContainer.Container,
});

export const themeChromeAssignments = (
  ramp: ThemeSurfaceRamp
): Record<typeof THEME_CHROME_ROLES[ThemeChromeRole], ThemeSurfaceScale> => ({
  background: ramp.background,
  surface: ramp.surface,
  surfaceVariant: ramp.surfaceVariant,
  secondaryContainer: ramp.secondaryContainer,
});

export const themeSurfaceLuminance = (hex: string): number => chroma(hex).luminance();

export const themeContrastRatio = (foreground: string, background: string): number =>
  chroma.contrast(foreground, background);
