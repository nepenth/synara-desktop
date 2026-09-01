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
  content: ThemeContentRoles;
  richText: ThemeRichTextRoles;
  chrome: string;
  overlay: string;
  shadow: string;
  focusRing: string;
};

/**
 * Semantic presentation colors for Matrix rich text. These are deliberately
 * derived from the chat reading surface rather than a generic Folds surface:
 * a generic token can resolve to the canvas itself and make formatting vanish.
 */
export type ThemeRichTextRoles = {
  readingSurface: string;
  readingSurfaceHover: string;
  inlineCodeBackground: string;
  inlineCodeBorder: string;
  inlineCodeForeground: string;
  codeBlockBackground: string;
  codeBlockBorder: string;
  spoilerBackground: string;
  spoilerHover: string;
  spoilerBorder: string;
  tableCanvas: string;
  tableHeader: string;
  tableOdd: string;
  tableEven: string;
  tableHover: string;
  contrastBorder: string;
  contrastForeground: string;
};

export type ThemeContentRoles = {
  heading: string;
  primary: string;
  secondary: string;
  tertiary: string;
  separator: string;
  selectedSurface: string;
  tableCanvas: string;
  tableHeader: string;
  tableOdd: string;
  tableEven: string;
  tableHover: string;
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

const softenToContrast = (
  foreground: string,
  background: string,
  minimumContrast: number
): string => {
  let resolved = foreground;
  for (let step = 1; step <= 95; step += 1) {
    const candidate = chroma.mix(foreground, background, step / 100, 'rgb').hex();
    if (chroma.contrast(candidate, background) < minimumContrast) break;
    resolved = candidate;
  }
  return resolved;
};

const surfaceAtContrast = (surface: string, toward: string, targetContrast: number): string => {
  for (let step = 1; step <= 100; step += 1) {
    const candidate = chroma.mix(surface, toward, step / 100, 'rgb').hex();
    if (chroma.contrast(candidate, surface) >= targetContrast) return candidate;
  }
  return toward;
};

const neutralSurfaceAtContrast = (
  surface: string,
  kind: ThemeRampKind,
  targetContrast: number
): string => {
  const [rawHue, rawSaturation, rawLightness] = chroma(surface).hsl();
  const hue = Number.isNaN(rawHue) ? 220 : rawHue;
  const saturation = Math.min(Number.isNaN(rawSaturation) ? 0 : rawSaturation, 0.025);
  const lightness = Number.isNaN(rawLightness) ? (kind === 'dark' ? 0.1 : 0.95) : rawLightness;
  const direction = kind === 'dark' ? 1 : -1;
  for (let step = 0; step <= 100; step += 1) {
    const candidateLightness = lightness + direction * ((step / 100) * 0.75);
    const candidate = hslHex(hue, saturation, candidateLightness);
    if (chroma.contrast(candidate, surface) >= targetContrast) return candidate;
  }
  return kind === 'dark' ? '#ffffff' : '#000000';
};

export const deriveThemeRichTextRoles = (
  kind: ThemeRampKind,
  readingSurface: string,
  readingSurfaceHover: string
): ThemeRichTextRoles => {
  const toward = kind === 'dark' ? '#ffffff' : '#000000';
  const inlineCodeBackground = surfaceAtContrast(readingSurface, toward, 1.5);
  // Syntax palettes are verified against a calm neutral well. Retain a small
  // amount of the selected theme hue without letting saturated presets move
  // token contrast below its accessibility floor.
  const codeBlockBackground = chroma
    .mix(readingSurface, kind === 'dark' ? '#2b2d31' : '#f2f3f5', 0.85, 'rgb')
    .hex();
  const spoilerBackground = surfaceAtContrast(readingSurface, toward, 1.42);
  const contrastForeground = softenToContrast(toward, inlineCodeBackground, 7);

  return {
    readingSurface,
    readingSurfaceHover,
    inlineCodeBackground,
    inlineCodeBorder: surfaceAtContrast(readingSurface, toward, 2),
    inlineCodeForeground: contrastForeground,
    codeBlockBackground,
    codeBlockBorder: neutralSurfaceAtContrast(readingSurface, kind, 1.7),
    spoilerBackground,
    spoilerHover: surfaceAtContrast(readingSurface, toward, 1.62),
    spoilerBorder: surfaceAtContrast(readingSurface, toward, 2),
    tableCanvas: surfaceAtContrast(readingSurface, toward, 1.1),
    tableHeader: surfaceAtContrast(readingSurface, toward, 1.38),
    tableOdd: readingSurface,
    tableEven: surfaceAtContrast(readingSurface, toward, 1.16),
    tableHover: surfaceAtContrast(readingSurface, toward, 1.28),
    contrastBorder: surfaceAtContrast(readingSurface, toward, 3.1),
    contrastForeground,
  };
};

const contentRoles = (
  kind: ThemeRampKind,
  background: string,
  surface: ThemeSurfaceScale,
  surfaceVariant: ThemeSurfaceScale,
  secondaryContainer: ThemeSurfaceScale
): ThemeContentRoles => {
  const neutral = kind === 'dark' ? '#ffffff' : '#000000';
  const heading = softenToContrast(neutral, background, 12);
  const primary = softenToContrast(neutral, background, 8);
  const secondary = softenToContrast(neutral, background, 5.5);
  const tertiary = softenToContrast(neutral, background, 4.5);
  const selectedSurface = chroma
    .mix(surface.Container, primary, kind === 'dark' ? 0.1 : 0.06, 'rgb')
    .hex();

  return {
    heading,
    primary,
    secondary,
    tertiary,
    separator: softenToContrast(neutral, background, 3),
    selectedSurface,
    tableCanvas: secondaryContainer.Container,
    tableHeader: secondaryContainer.ContainerActive,
    tableOdd: surfaceVariant.Container,
    tableEven: surface.ContainerHover,
    tableHover: surface.ContainerActive,
  };
};

const withContentRoles = (
  ramp: Omit<ThemeSurfaceRamp, 'content' | 'richText'>,
  kind: ThemeRampKind
): ThemeSurfaceRamp => {
  const content = contentRoles(
    kind,
    ramp.surfaceVariant.Container,
    ramp.surface,
    ramp.surfaceVariant,
    ramp.secondaryContainer
  );
  return {
    ...ramp,
    background: { ...ramp.background, OnContainer: content.primary },
    surface: { ...ramp.surface, OnContainer: content.primary },
    surfaceVariant: { ...ramp.surfaceVariant, OnContainer: content.primary },
    secondaryContainer: { ...ramp.secondaryContainer, OnContainer: content.primary },
    content,
    richText: deriveThemeRichTextRoles(
      kind,
      ramp.surfaceVariant.Container,
      ramp.surfaceVariant.ContainerHover
    ),
  };
};

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
    return withContentRoles(
      {
        background: tintScale(stacked.background, hex, mixRatio),
        surface: tintScale(stacked.surface, hex, mixRatio),
        surfaceVariant: tintScale(stacked.surfaceVariant, hex, mixRatio),
        secondaryContainer: tintScale(stacked.secondaryContainer, hex, mixRatio),
        chrome: chroma.mix(hslHex(hue, saturation, 0.07), hex, mixRatio, 'rgb').hex(),
        overlay: 'rgba(0, 0, 0, 0.72)',
        shadow: 'rgba(0, 0, 0, 0.55)',
        focusRing: 'rgba(255, 255, 255, 0.45)',
      },
      kind
    );
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
  return withContentRoles(
    {
      background: tintScale(stacked.background, hex, mixRatio),
      surface: tintScale(stacked.surface, hex, mixRatio),
      surfaceVariant: tintScale(stacked.surfaceVariant, hex, mixRatio),
      secondaryContainer: tintScale(stacked.secondaryContainer, hex, mixRatio),
      chrome: chroma.mix(hslHex(hue, saturation * 0.45, 1), hex, mixRatio * 0.4, 'rgb').hex(),
      overlay: 'rgba(15, 17, 21, 0.45)',
      shadow: 'rgba(15, 17, 21, 0.16)',
      focusRing: 'rgba(15, 17, 21, 0.45)',
    },
    kind
  );
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
