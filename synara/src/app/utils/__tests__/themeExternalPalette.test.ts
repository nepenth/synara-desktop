import assert from 'node:assert/strict';
import test from 'node:test';
import chroma from 'chroma-js';
import { MESSAGE_TEXT_TONES } from '../messageTextTone';
import { MUTED_TEXT_OPACITY, TEXT_CONTRAST_AA } from '../themeBase';
import {
  deriveExternalAccent,
  deriveExternalPaletteTheme,
  externalPaletteFollowDescription,
  externalPaletteKind,
  isExternalSurfacePalette,
  normalizeExternalColor,
  normalizeExternalPalette,
  type ExternalSurfacePalette,
  externalPaletteSourceLabel,
} from '../themeExternalPalette';

const contrast = (a: string, b: string) => chroma.contrast(a, b);

/** Every surface text is drawn on, hover and pressed states and the selected row included. */
const textSurfaces = (theme: ReturnType<typeof deriveExternalPaletteTheme>): string[] => {
  const { ramp } = theme;
  return [
    ...[ramp.background, ramp.surface, ramp.surfaceVariant, ramp.secondaryContainer].flatMap(
      (stop) => [stop.Container, stop.ContainerHover, stop.ContainerActive]
    ),
    ramp.content.selectedSurface,
  ];
};

const assertReadable = (palette: ExternalSurfacePalette, label: string) => {
  const theme = deriveExternalPaletteTheme(palette);
  const { ramp, kind } = theme;
  const chat = ramp.surfaceVariant.Container;
  for (const surface of textSurfaces(theme)) {
    const at = `${label}: on ${surface}`;
    assert.ok(contrast(ramp.content.primary, surface) >= TEXT_CONTRAST_AA, `${at} primary`);
    assert.ok(contrast(ramp.content.secondary, surface) >= TEXT_CONTRAST_AA, `${at} secondary`);
    assert.ok(contrast(ramp.content.tertiary, surface) >= TEXT_CONTRAST_AA, `${at} tertiary`);
    // folds `priority="300"` text: primary blended toward the surface.
    const dimmed = chroma
      .mix(ramp.content.primary, surface, 1 - MUTED_TEXT_OPACITY[kind], 'rgb')
      .hex();
    assert.ok(contrast(dimmed, surface) >= TEXT_CONTRAST_AA, `${at} dimmed`);
  }
  for (const stop of [
    ramp.background,
    ramp.surface,
    ramp.surfaceVariant,
    ramp.secondaryContainer,
  ]) {
    assert.equal(stop.OnContainer, ramp.content.primary, `${label}: OnContainer`);
  }
  assert.ok(contrast(ramp.content.primary, chat) >= 7, `${label}: primary on chat`);
  assert.ok(contrast(ramp.content.secondary, chat) >= 5.5, `${label}: secondary on chat`);
  assert.ok(contrast(ramp.content.heading, chat) >= contrast(ramp.content.primary, chat) - 0.01);
  assert.ok(contrast(ramp.content.separator, chat) >= 3, `${label}: separator`);
  for (const tone of MESSAGE_TEXT_TONES) {
    assert.ok(contrast(theme.messageText[tone], chat) >= 7, `${label}: message text ${tone}`);
  }
  if (theme.accent) {
    assert.ok(contrast(theme.accent.main, chat) >= 3, `${label}: accent main`);
    assert.ok(contrast(theme.accent.onMain, theme.accent.main) >= TEXT_CONTRAST_AA, label);
    assert.ok(contrast(theme.accent.link, chat) >= TEXT_CONTRAST_AA, `${label}: link`);
    assert.ok(contrast(theme.accent.link, ramp.surface.Container) >= TEXT_CONTRAST_AA, label);
  }
  // Rich text roles keep their own floors on the palette's chat surface.
  assert.equal(ramp.richText.readingSurface, chat);
  assert.ok(contrast(ramp.richText.inlineCodeForeground, ramp.richText.inlineCodeBackground) >= 7);
  return theme;
};

test('palette colors accept CSS hex only', () => {
  assert.equal(normalizeExternalColor('#1A1B26'), '#1a1b26');
  assert.equal(normalizeExternalColor(' #abc '), '#aabbcc');
  assert.equal(normalizeExternalColor('#1a1b26ff'), '#1a1b26');
  assert.equal(normalizeExternalColor('1a1b26'), undefined);
  assert.equal(normalizeExternalColor('red'), undefined);
  assert.equal(normalizeExternalColor('rgb(0, 0, 0)'), undefined);
  assert.equal(normalizeExternalColor('#12345'), undefined);
  assert.equal(normalizeExternalColor(0x1a1b26), undefined);
});

test('the shell payload is checked before use', () => {
  assert.equal(normalizeExternalPalette(null), null);
  assert.equal(normalizeExternalPalette('#000000'), null);
  assert.equal(normalizeExternalPalette([]), null);
  assert.equal(normalizeExternalPalette({ foreground: '#ffffff' }), null);
  assert.equal(normalizeExternalPalette({ background: 'black' }), null);

  assert.deepEqual(
    normalizeExternalPalette({
      source: '  Omarchy\u0000\n tokyo-night ',
      background: '#1A1B26',
      foreground: '#a9b1d6',
      accent: '#7aa2f7',
      colors: ['#f7768e'],
    }),
    {
      source: 'Omarchy tokyo-night',
      background: '#1a1b26',
      foreground: '#a9b1d6',
      accent: '#7aa2f7',
    }
  );
  // A bad foreground reads as plain text colour; a bad accent is dropped.
  assert.deepEqual(
    normalizeExternalPalette({ background: '#000000', foreground: 'x', accent: 3 }),
    {
      source: '',
      background: '#000000',
      foreground: '#ffffff',
      accent: undefined,
    }
  );
  assert.equal(
    normalizeExternalPalette({ background: '#ffffff', source: 'x'.repeat(100) })?.source.length,
    40
  );
});

test('a payload with an accent and no background is an accent-only palette', () => {
  const macos = normalizeExternalPalette({ source: 'macos', accent: '#007AFF' });
  assert.deepEqual(macos, { source: 'macos', accent: '#007aff' });
  assert.ok(macos && !isExternalSurfacePalette(macos));
  // A foreground alone does not make it a theme.
  assert.deepEqual(
    normalizeExternalPalette({ source: 'macos', accent: '#007aff', foreground: '#ffffff' }),
    { source: 'macos', accent: '#007aff' }
  );
  assert.equal(normalizeExternalPalette({ source: 'macos' }), null);
  assert.equal(normalizeExternalPalette({ source: 'macos', accent: 'blue' }), null);

  const omarchy = normalizeExternalPalette({ source: 'omarchy', background: '#1a1b26' });
  assert.ok(omarchy && isExternalSurfacePalette(omarchy));
});

test('an accent-only palette is used as chosen, with readable text on it', () => {
  // The macOS accents in light appearance, and the extremes.
  for (const accent of [
    '#007aff',
    '#a550a7',
    '#f74f9e',
    '#ff5257',
    '#f7821b',
    '#ffc600',
    '#62ba46',
    '#8c8c8c',
    '#000000',
    '#ffffff',
  ]) {
    const derived = deriveExternalAccent({ source: 'macos', accent });
    assert.equal(derived.main, accent);
    assert.equal(derived.link, accent);
    assert.ok(['#ffffff', '#000000'].includes(derived.onMain), accent);
    assert.ok(contrast(derived.onMain, derived.main) >= TEXT_CONTRAST_AA, accent);
  }
  assert.equal(deriveExternalAccent({ source: 'macos', accent: '#ffc600' }).onMain, '#000000');
  assert.equal(deriveExternalAccent({ source: 'macos', accent: '#1245a8' }).onMain, '#ffffff');
});

test('light or dark follows the background luminance', () => {
  assert.equal(externalPaletteKind('#000000'), 'dark');
  assert.equal(externalPaletteKind('#1a1b26'), 'dark');
  assert.equal(externalPaletteKind('#ffffff'), 'light');
  assert.equal(externalPaletteKind('#eff1f5'), 'light');
  assert.equal(externalPaletteKind('#ffff00'), 'light');
  assert.equal(externalPaletteKind('#0000ff'), 'dark');
});

test('ordinary desktop themes keep their own background and text', () => {
  const tokyo = assertReadable(
    { source: 'Omarchy', background: '#1a1b26', foreground: '#a9b1d6', accent: '#7aa2f7' },
    'tokyo night'
  );
  assert.equal(tokyo.kind, 'dark');
  assert.equal(tokyo.ramp.surfaceVariant.Container, '#1a1b26');
  assert.equal(tokyo.accent?.main, '#7aa2f7');

  const latte = assertReadable(
    { source: 'Omarchy', background: '#eff1f5', foreground: '#4c4f69', accent: '#1e66f5' },
    'catppuccin latte'
  );
  assert.equal(latte.kind, 'light');
  assert.equal(latte.ramp.surfaceVariant.Container, '#eff1f5');
  // Their text passes on their own background but is a little faint, once
  // folds dims it, on the stacked surfaces; it moves only that far.
  assert.ok(chroma.deltaE(latte.ramp.content.primary, '#4c4f69') < 8);
  assert.ok(chroma.deltaE(tokyo.ramp.content.primary, '#a9b1d6') < 8);

  // The surfaces stack as in the built-in ramp: rail darker than the list,
  // the list darker than the chat.
  const lum = (hex: string) => chroma(hex).luminance();
  for (const theme of [tokyo, latte]) {
    assert.ok(lum(theme.ramp.background.Container) < lum(theme.ramp.surface.Container));
    assert.ok(lum(theme.ramp.surface.Container) < lum(theme.ramp.surfaceVariant.Container));
  }
});

test('hostile palettes stay readable', () => {
  const hostile: Array<[string, ExternalSurfacePalette]> = [
    [
      'low contrast dark',
      { source: '', background: '#333333', foreground: '#3a3a3a', accent: '#404040' },
    ],
    [
      'low contrast light',
      { source: '', background: '#dddddd', foreground: '#cccccc', accent: '#e0e0e0' },
    ],
    ['pure black', { source: '', background: '#000000', foreground: '#000000', accent: '#000000' }],
    ['pure white', { source: '', background: '#ffffff', foreground: '#ffffff', accent: '#ffffff' }],
    [
      'saturated red',
      { source: '', background: '#ff0000', foreground: '#00ff00', accent: '#0000ff' },
    ],
    [
      'saturated blue',
      { source: '', background: '#0000ff', foreground: '#ff00ff', accent: '#ffff00' },
    ],
    ['mid grey', { source: '', background: '#777777', foreground: '#888888', accent: '#7f7f7f' }],
    [
      'inverted text',
      { source: '', background: '#101010', foreground: '#111111', accent: undefined },
    ],
  ];
  for (const [label, palette] of hostile) assertReadable(palette, label);
});

test('every background on a coarse sRGB grid stays readable', () => {
  const steps = ['00', '33', '66', '99', 'cc', 'ff'];
  for (const r of steps) {
    for (const g of steps) {
      for (const b of steps) {
        const background = `#${r}${g}${b}`;
        assertReadable(
          { source: '', background, foreground: background, accent: `#${b}${r}${g}` },
          background
        );
      }
    }
  }
});

test('copy names a known provider properly and falls back to "your desktop theme"', () => {
  assert.equal(externalPaletteSourceLabel('omarchy'), 'your Omarchy theme');
  assert.equal(externalPaletteSourceLabel('Omarchy'), 'your Omarchy theme');
  // An unknown id is not shown, capitalised or otherwise.
  assert.equal(externalPaletteSourceLabel('some-shell'), 'your desktop theme');
  assert.equal(externalPaletteSourceLabel('constructor'), 'your desktop theme');
  assert.equal(externalPaletteSourceLabel(''), 'your desktop theme');
});

test('the setting says what each provider shares', () => {
  assert.equal(
    externalPaletteFollowDescription({
      source: 'omarchy',
      background: '#1a1b26',
      foreground: '#a9b1d6',
      accent: '#7aa2f7',
    }),
    'Use the background, text and accent colors of your Omarchy theme. Synara adjusts them where text would be hard to read.'
  );
  assert.equal(
    externalPaletteFollowDescription({
      source: 'some-shell',
      background: '#000000',
      foreground: '#ffffff',
    }),
    'Use the background, text and accent colors of your desktop theme. Synara adjusts them where text would be hard to read.'
  );
  // An accent alone never promises a background or text color.
  assert.equal(
    externalPaletteFollowDescription({ source: 'macos', accent: '#007aff' }),
    'Use your macOS accent color in place of the Accent color below. The theme keeps its own background and text.'
  );
  assert.equal(
    externalPaletteFollowDescription({ source: 'some-shell', accent: '#007aff' }),
    'Use your desktop accent color in place of the Accent color below. The theme keeps its own background and text.'
  );
});
