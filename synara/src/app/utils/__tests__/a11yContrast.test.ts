import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { DARK_THEME_ACCENT_COLOR, DEFAULT_ACCENT_COLOR } from '../themeAccent';
import {
  deriveThemeSurfaceRamp,
  MUTED_TEXT_OPACITY,
  THEME_BASE_PRESETS,
  type ThemeRampKind,
  type ThemeSurfaceRamp,
} from '../themeBase';

// A WCAG 2.2 contrast helper independent of chroma-js, so the ramp's own
// arithmetic is checked rather than reused.
type Rgb = [number, number, number];

const hexRgb = (hex: string): Rgb => {
  const value = hex.replace('#', '');
  return [0, 2, 4].map((at) => parseInt(value.slice(at, at + 2), 16)) as Rgb;
};

const hslRgb = (hue: number, saturation: number, lightness: number): Rgb => {
  const s = saturation / 100;
  const l = lightness / 100;
  const k = (n: number) => (n + hue / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [f(0), f(8), f(4)].map((channel) => Math.round(channel * 255)) as Rgb;
};

const parseRgba = (value: string): { rgb: Rgb; alpha: number } => {
  const parts = value.match(/[\d.]+/g)?.map(Number) ?? [];
  return { rgb: [parts[0], parts[1], parts[2]], alpha: parts[3] ?? 1 };
};

const luminance = ([r, g, b]: Rgb): number => {
  const linear = (channel: number) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
};

const contrast = (a: Rgb, b: Rgb): number => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

/** `foreground` at `alpha` composited over `background`. */
const over = (foreground: Rgb, background: Rgb, alpha: number): Rgb =>
  foreground.map((channel, i) => channel * alpha + background[i] * (1 - alpha)) as Rgb;

const KINDS: ThemeRampKind[] = ['light', 'dark'];

/** Every derived surface text can sit on, for every base-colour preset. */
const surfaces = (ramp: ThemeSurfaceRamp): Record<string, Rgb> => ({
  rail: hexRgb(ramp.background.Container),
  roomList: hexRgb(ramp.surface.Container),
  roomListHover: hexRgb(ramp.surface.ContainerHover),
  selectedRow: hexRgb(ramp.content.selectedSurface),
  chat: hexRgb(ramp.surfaceVariant.Container),
  chatHover: hexRgb(ramp.surfaceVariant.ContainerHover),
  composer: hexRgb(ramp.secondaryContainer.Container),
});

const eachSurface = (
  check: (kind: ThemeRampKind, ramp: ThemeSurfaceRamp, name: string, surface: Rgb) => void
) => {
  KINDS.forEach((kind) =>
    THEME_BASE_PRESETS.forEach((preset) => {
      const ramp = deriveThemeSurfaceRamp(preset.hex, kind);
      Object.entries(surfaces(ramp)).forEach(([name, surface]) =>
        check(kind, ramp, `${preset.id} ${kind} ${name}`, surface)
      );
    })
  );
};

test('the contrast helper matches known WCAG ratios', () => {
  assert.equal(contrast([0, 0, 0], [255, 255, 255]).toFixed(1), '21.0');
  assert.equal(contrast(hexRgb('#767676'), [255, 255, 255]).toFixed(2), '4.54');
  assert.deepEqual(hslRgb(213, 100, 35), hexRgb('#0050b3'));
});

test('secondary text (read rooms, timestamps, placeholder) is AA on every text surface', () => {
  eachSurface((_kind, ramp, label, surface) => {
    // The rail holds only avatars and icons; secondary text never sits on it.
    if (label.endsWith(' rail')) return;
    const ratio = contrast(hexRgb(ramp.content.secondary), surface);
    assert.ok(ratio >= 4.5, `${label}: secondary ${ratio.toFixed(2)}`);
  });
});

test('the default ramp keeps its secondary colour; only presets that needed it moved', () => {
  // Default light read room name: 3.97 (primary at 0.75) -> 5.04; selected 3.81 -> 4.63.
  const light = deriveThemeSurfaceRamp('#2b2d31', 'light');
  assert.equal(light.content.secondary, '#606060');
  const dark = deriveThemeSurfaceRamp('#2b2d31', 'dark');
  assert.equal(dark.content.secondary, '#949495');
});

test('priority="300" text keeps AA at the muted opacity on every surface', () => {
  eachSurface((kind, ramp, label, surface) => {
    const muted = over(hexRgb(ramp.content.primary), surface, MUTED_TEXT_OPACITY[kind]);
    const ratio = contrast(muted, surface);
    assert.ok(ratio >= 4.5, `${label}: muted primary ${ratio.toFixed(2)}`);
  });
  // folds' 0.75 failed in light (header topic 4.26, room list 3.97).
  assert.ok(MUTED_TEXT_OPACITY.light > 0.75);
});

test('the spoiler label is AA on the spoiler fill', () => {
  KINDS.forEach((kind) =>
    THEME_BASE_PRESETS.forEach((preset) => {
      const ramp = deriveThemeSurfaceRamp(preset.hex, kind);
      const ratio = contrast(hexRgb(ramp.content.primary), hexRgb(ramp.richText.spoilerBackground));
      assert.ok(ratio >= 4.5, `${preset.id} ${kind}: spoiler label ${ratio.toFixed(2)}`);
    })
  );
});

test('the folds focus ring reaches 3:1 on every surface', () => {
  eachSurface((_kind, ramp, label, surface) => {
    const ring = parseRgba(ramp.focusRing);
    const ratio = contrast(over(ring.rgb, surface, ring.alpha), surface);
    assert.ok(ratio >= 3, `${label}: focus ring ${ratio.toFixed(2)}`);
  });
  const colors = readFileSync('src/colors.css.ts', 'utf8');
  assert.match(colors, /FocusRing: 'rgba\(15, 17, 21, 0\.55\)'/);
});

test('Primary.Main focus outlines (room list, fields, Following bar) reach 3:1', () => {
  eachSurface((kind, _ramp, label, surface) => {
    const primary = hexRgb(kind === 'dark' ? DARK_THEME_ACCENT_COLOR : DEFAULT_ACCENT_COLOR);
    const ratio = contrast(primary, surface);
    assert.ok(ratio >= 3, `${label}: Primary.Main ${ratio.toFixed(2)}`);
  });
});

const cssBlock = (css: string, selector: string): string => {
  const start = css.indexOf(`${selector} {`);
  assert.ok(start >= 0, `missing ${selector}`);
  return css.slice(start, css.indexOf('}', start));
};

const hslVar = (block: string, name: string): Rgb => {
  const match = block.match(new RegExp(`${name}: hsl\\((\\d+)(?:deg)?,? (\\d+)%,? (\\d+)%\\)`));
  assert.ok(match, `missing ${name}`);
  return hslRgb(Number(match[1]), Number(match[2]), Number(match[3]));
};

test('links, user colours and avatar initials are AA on every surface', () => {
  const css = readFileSync('src/index.css', 'utf8');
  const blocks: Record<ThemeRampKind, string> = {
    light: cssBlock(css, ':root'),
    dark: cssBlock(css, '.dark-theme,\n.butter-theme'),
  };
  eachSurface((kind, ramp, label, surface) => {
    const block = blocks[kind];
    const link = contrast(hslVar(block, '--tc-link'), surface);
    assert.ok(link >= 4.5, `${label}: link ${link.toFixed(2)}`);
    for (let n = 1; n <= 8; n += 1) {
      const userColor = hslVar(block, `--mx-uc-${n}`);
      const name = contrast(userColor, surface);
      assert.ok(name >= 4.5, `${label}: --mx-uc-${n} ${name.toFixed(2)}`);
      // Fallback avatars draw Surface.Container initials on the user colour.
      const initials = contrast(hexRgb(ramp.surface.Container), userColor);
      assert.ok(initials >= 4.5, `${label}: initials on --mx-uc-${n} ${initials.toFixed(2)}`);
    }
  });
});
