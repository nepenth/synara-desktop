import chroma from 'chroma-js';
import type { MessageTextTone } from './messageTextTone';
import {
  deriveThemeRichTextRoles,
  MUTED_TEXT_OPACITY,
  TEXT_CONTRAST_AA,
  type ThemeContentRoles,
  type ThemeRampKind,
  type ThemeSurfaceRamp,
  type ThemeSurfaceScale,
} from './themeBase';

/**
 * A palette from outside Synara, such as the desktop's theme (Omarchy). The
 * colors are taken as the chat background, the body text and the accent;
 * everything else is derived the way `deriveThemeSurfaceRamp` derives the
 * built-in themes, and then held to WCAG AA whatever the input.
 */
export type ExternalPalette = {
  /** Where it came from, for the settings copy, e.g. "Omarchy". */
  source: string;
  background: string;
  foreground: string;
  accent?: string;
};

export type ExternalPaletteAccent = {
  /** `Primary.Main`: at least 3:1 against the chat surface. */
  main: string;
  /** Text on `Primary.Main`: the better of black and white, at least AA. */
  onMain: string;
  /** Links and accent text: at least AA on the chat and room list surfaces. */
  link: string;
};

export type ExternalPaletteTheme = {
  kind: ThemeRampKind;
  ramp: ThemeSurfaceRamp;
  /** Undefined when the palette has no usable accent; the accent setting applies. */
  accent?: ExternalPaletteAccent;
  /** `--synara-message-foreground` per Message text tone, each at least 7:1. */
  messageText: Record<MessageTextTone, string>;
};

const WHITE = '#ffffff';
const BLACK = '#000000';
const MAX_SOURCE_LENGTH = 40;
const HEX_COLOR = /^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

/**
 * Black and white text give equal contrast on a background of this relative
 * luminance; darker backgrounds get the dark ramp.
 */
export const EXTERNAL_PALETTE_KIND_SPLIT = 0.179;
/**
 * The chat surface is kept at or below this luminance in the dark ramp (so
 * inline code, a step lighter, still carries 7:1 white text)…
 */
export const EXTERNAL_DARK_MAX_LUMINANCE = 0.04;
/** …and at or above this in the light ramp, so the stacked surfaces keep room for text. */
export const EXTERNAL_LIGHT_MIN_LUMINANCE = 0.6;
/** Body text and message text against the chat surface. */
export const EXTERNAL_PRIMARY_TEXT_CONTRAST = 7;

/** `#rgb`, `#rrggbb` or `#rrggbbaa` (alpha dropped) as lowercase `#rrggbb`. */
export const normalizeExternalColor = (value: unknown): string | undefined => {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  if (!HEX_COLOR.test(trimmed)) return undefined;
  const hex = trimmed.length === 9 ? trimmed.slice(0, 7) : trimmed;
  return chroma(hex).hex('rgb').toLowerCase();
};

const normalizeSource = (value: unknown): string => {
  if (typeof value !== 'string') return '';
  return Array.from(value)
    .filter((char) => char.charCodeAt(0) >= 32 && char.charCodeAt(0) !== 127)
    .join('')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, MAX_SOURCE_LENGTH)
    .trim();
};

/** Providers' `source` ids as people know them. */
const SOURCE_DISPLAY_NAMES: ReadonlyMap<string, string> = new Map([['omarchy', 'Omarchy']]);

/**
 * How copy refers to a palette's origin: "your Omarchy theme" for a provider
 * we know, "your desktop theme" otherwise. An unknown `source` is an id from
 * the shell, not a name to show.
 */
export const externalPaletteSourceLabel = (source: string): string => {
  const name = SOURCE_DISPLAY_NAMES.get(source.trim().toLowerCase());
  return name ? `your ${name} theme` : 'your desktop theme';
};

/**
 * The shell's `desktop_system_palette` payload, checked. `null` when it is
 * not a palette: no object, or no usable background. A bad foreground falls
 * back to black or white; a bad accent is dropped.
 */
export const normalizeExternalPalette = (value: unknown): ExternalPalette | null => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  const background = normalizeExternalColor(record.background);
  if (!background) return null;
  const kind = externalPaletteKind(background);
  return {
    source: normalizeSource(record.source),
    background,
    foreground: normalizeExternalColor(record.foreground) ?? (kind === 'dark' ? WHITE : BLACK),
    accent: normalizeExternalColor(record.accent),
  };
};

export const externalPaletteKind = (background: string): ThemeRampKind =>
  chroma(background).luminance() < EXTERNAL_PALETTE_KIND_SPLIT ? 'dark' : 'light';

/** The palette background moved, hue kept, into the luminance band its ramp needs. */
const chatSurface = (background: string, kind: ThemeRampKind): string => {
  const color = chroma(background);
  const luminance = color.luminance();
  if (kind === 'dark' && luminance > EXTERNAL_DARK_MAX_LUMINANCE) {
    return color.luminance(EXTERNAL_DARK_MAX_LUMINANCE).hex();
  }
  if (kind === 'light' && luminance < EXTERNAL_LIGHT_MIN_LUMINANCE) {
    return color.luminance(EXTERNAL_LIGHT_MIN_LUMINANCE).hex();
  }
  return color.hex();
};

const mix = (from: string, to: string, ratio: number): string =>
  chroma.mix(from, to, ratio, 'rgb').hex();

const scale = (container: string, toward: string, onContainer: string): ThemeSurfaceScale => ({
  Container: container,
  ContainerHover: mix(container, toward, 0.04),
  ContainerActive: mix(container, toward, 0.07),
  ContainerLine: mix(container, toward, 0.12),
  OnContainer: onContainer,
});

/** Every surface text is drawn on, hover and pressed states included. */
const textSurfaces = (
  ramp: Pick<ThemeSurfaceRamp, 'background' | 'surface' | 'surfaceVariant' | 'secondaryContainer'>
) =>
  [ramp.background, ramp.surface, ramp.surfaceVariant, ramp.secondaryContainer].flatMap((stop) => [
    stop.Container,
    stop.ContainerHover,
    stop.ContainerActive,
  ]);

/**
 * The first colour on the way from `color` to `toward` that passes `check`
 * on every surface; `toward` itself when none does.
 */
const nearestPassing = (
  color: string,
  toward: string,
  surfaces: readonly string[],
  check: (candidate: string, surface: string) => boolean
): string => {
  for (let step = 0; step <= 100; step += 1) {
    const candidate = mix(color, toward, step / 100);
    if (surfaces.every((surface) => check(candidate, surface))) return candidate;
  }
  return toward;
};

/**
 * The furthest colour on the way from `color` to `background` that still
 * passes `check` on every surface; `color` itself when the first step fails.
 */
const softestPassing = (
  color: string,
  background: string,
  surfaces: readonly string[],
  check: (candidate: string, surface: string) => boolean
): string => {
  let resolved = color;
  for (let step = 1; step <= 95; step += 1) {
    const candidate = mix(color, background, step / 100);
    if (!surfaces.every((surface) => check(candidate, surface))) break;
    resolved = candidate;
  }
  return resolved;
};

const atLeast =
  (minimum: number) =>
  (candidate: string, surface: string): boolean =>
    chroma.contrast(candidate, surface) >= minimum;

/** folds dims `priority="300"` text with an opacity; that blend must hold AA too. */
const dimmedAtLeast =
  (kind: ThemeRampKind, minimum: number) =>
  (candidate: string, surface: string): boolean =>
    chroma.contrast(mix(candidate, surface, 1 - MUTED_TEXT_OPACITY[kind]), surface) >= minimum;

const deriveAccent = (
  accent: string,
  kind: ThemeRampKind,
  chat: string,
  linkSurfaces: readonly string[]
): ExternalPaletteAccent => {
  const extreme = kind === 'dark' ? WHITE : BLACK;
  const main = nearestPassing(accent, extreme, [chat], atLeast(3));
  const onMain = chroma.contrast(WHITE, main) >= chroma.contrast(BLACK, main) ? WHITE : BLACK;
  return {
    main,
    onMain,
    link: nearestPassing(accent, extreme, linkSurfaces, atLeast(TEXT_CONTRAST_AA)),
  };
};

/**
 * Surfaces, text roles, accent and message text for an external palette.
 * Light or dark follows the background's luminance. The chat surface is the
 * palette background (moved only as far as readable text needs); the rail,
 * room list and composer stack around it as in the built-in ramp.
 */
export const deriveExternalPaletteTheme = (palette: ExternalPalette): ExternalPaletteTheme => {
  const kind = externalPaletteKind(palette.background);
  const dark = kind === 'dark';
  const extreme = dark ? WHITE : BLACK;
  // Hover and pressed states move toward the text colour in both kinds.
  const chat = chatSurface(palette.background, kind);
  const stops = {
    background: scale(mix(chat, BLACK, dark ? 0.28 : 0.09), extreme, extreme),
    surface: scale(mix(chat, BLACK, dark ? 0.08 : 0.035), extreme, extreme),
    surfaceVariant: scale(chat, extreme, extreme),
    secondaryContainer: scale(
      dark ? mix(chat, WHITE, 0.06) : mix(chat, BLACK, 0.06),
      extreme,
      extreme
    ),
  };
  const selectedSurface = mix(stops.surface.Container, extreme, dark ? 0.1 : 0.06);
  const surfaces = [...textSurfaces(stops), selectedSurface];

  // Body text: 7:1 on the chat surface, and AA on every other surface also
  // when folds dims it.
  const primary = nearestPassing(
    palette.foreground,
    extreme,
    surfaces,
    (candidate, surface) =>
      (surface !== chat || atLeast(EXTERNAL_PRIMARY_TEXT_CONTRAST)(candidate, surface)) &&
      atLeast(TEXT_CONTRAST_AA)(candidate, surface) &&
      dimmedAtLeast(kind, TEXT_CONTRAST_AA)(candidate, surface)
  );
  const heading = nearestPassing(primary, extreme, [chat], atLeast(12));
  const secondary = softestPassing(primary, chat, surfaces, (candidate, surface) =>
    atLeast(surface === chat ? 5.5 : TEXT_CONTRAST_AA)(candidate, surface)
  );
  const tertiary = softestPassing(secondary, chat, surfaces, atLeast(TEXT_CONTRAST_AA));
  const separator = softestPassing(tertiary, chat, [chat], atLeast(3));

  const content: ThemeContentRoles = {
    heading,
    primary,
    secondary,
    tertiary,
    separator,
    selectedSurface,
    tableCanvas: stops.secondaryContainer.Container,
    tableHeader: stops.secondaryContainer.ContainerActive,
    tableOdd: stops.surfaceVariant.Container,
    tableEven: stops.surface.ContainerHover,
    tableHover: stops.surface.ContainerActive,
  };
  const withText = (stop: ThemeSurfaceScale): ThemeSurfaceScale => ({
    ...stop,
    OnContainer: primary,
  });

  const ramp: ThemeSurfaceRamp = {
    background: withText(stops.background),
    surface: withText(stops.surface),
    surfaceVariant: withText(stops.surfaceVariant),
    secondaryContainer: withText(stops.secondaryContainer),
    content,
    richText: deriveThemeRichTextRoles(kind, chat, stops.surfaceVariant.ContainerHover),
    chrome: stops.background.Container,
    overlay: dark ? 'rgba(0, 0, 0, 0.72)' : 'rgba(15, 17, 21, 0.45)',
    shadow: dark ? 'rgba(0, 0, 0, 0.55)' : 'rgba(15, 17, 21, 0.16)',
    focusRing: dark ? 'rgba(255, 255, 255, 0.45)' : 'rgba(15, 17, 21, 0.55)',
  };

  return {
    kind,
    ramp,
    accent: palette.accent
      ? deriveAccent(palette.accent, kind, chat, [
          chat,
          stops.surface.Container,
          stops.surfaceVariant.ContainerHover,
        ])
      : undefined,
    messageText: {
      bright: heading,
      balanced: primary,
      soft: softestPassing(primary, chat, [chat], atLeast(EXTERNAL_PRIMARY_TEXT_CONTRAST)),
    },
  };
};
