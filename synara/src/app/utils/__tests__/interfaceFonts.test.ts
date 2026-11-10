import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import {
  applyInterfaceFonts,
  fontFamilyListCss,
  INTERFACE_FONT_SLOT,
  interfaceFontSlot,
  MONOSPACE_FONT_SLOT,
  monospaceFontSlot,
  normalizeInterfaceFont,
  normalizeMonospaceFont,
  sanitizeFontFamilyList,
} from '../interfaceFonts';

test('font choices fall back to the default', () => {
  assert.equal(normalizeInterfaceFont('monospace'), 'monospace');
  assert.equal(normalizeInterfaceFont('custom'), 'custom');
  assert.equal(normalizeInterfaceFont('serif'), 'default');
  assert.equal(normalizeInterfaceFont(undefined), 'default');
  assert.equal(normalizeMonospaceFont('custom'), 'custom');
  assert.equal(normalizeMonospaceFont('monospace'), 'default');
});

test('family lists keep names and drop anything that could leave the CSS value', () => {
  assert.equal(sanitizeFontFamilyList('  Inter ,  Noto   Sans  '), 'Inter, Noto Sans');
  assert.equal(sanitizeFontFamilyList("'Fira Code'"), 'Fira Code');
  assert.equal(
    sanitizeFontFamilyList('Iosevka Term SS04, ui-monospace'),
    'Iosevka Term SS04, ui-monospace'
  );
  assert.equal(sanitizeFontFamilyList('Source_Han.Sans-JP'), 'Source_Han.Sans-JP');
  assert.equal(sanitizeFontFamilyList('ヒラギノ角ゴシック'), 'ヒラギノ角ゴシック');
  for (const hostile of [
    'x"; } body { display: none',
    'x\\22 ; color: red',
    'x) ; background: url(https://example.test/a)',
    'x</style><script>alert(1)</script>',
    'x\n;\t{}',
  ]) {
    const sanitized = sanitizeFontFamilyList(hostile);
    assert.doesNotMatch(sanitized, /["'\\;:{}()<>/\n\t]/, hostile);
  }
  assert.equal(sanitizeFontFamilyList(', , ,'), '');
  assert.equal(sanitizeFontFamilyList(42), '');
  assert.equal(sanitizeFontFamilyList('a, b, c, d, e, f'), 'a, b, c, d');
  assert.equal(sanitizeFontFamilyList('x'.repeat(200)).length, 64);
});

test('families are quoted, generic families are not', () => {
  assert.equal(fontFamilyListCss('Inter, Noto Sans'), '"Inter", "Noto Sans"');
  assert.equal(fontFamilyListCss('JetBrains Mono, Monospace'), '"JetBrains Mono", monospace');
  assert.equal(fontFamilyListCss('  '), undefined);
});

test('slots go in front of the stacks and end with a comma', () => {
  assert.equal(interfaceFontSlot('default', 'Inter'), undefined);
  assert.equal(interfaceFontSlot('monospace', ''), 'var(--font-mono),');
  assert.equal(interfaceFontSlot('custom', 'Inter'), '"Inter",');
  assert.equal(interfaceFontSlot('custom', ''), undefined);
  assert.equal(monospaceFontSlot('default', 'Fira Code'), undefined);
  assert.equal(monospaceFontSlot('custom', 'Fira Code'), '"Fira Code",');
});

test('applying fonts sets and clears the slots on the root', () => {
  const props = new Map<string, string>();
  const root = {
    style: {
      setProperty: (name: string, value: string | null) => {
        props.set(name, value ?? '');
      },
      removeProperty: (name: string) => {
        props.delete(name);
        return '';
      },
    },
  };
  applyInterfaceFonts(root, {
    interfaceFont: 'custom',
    interfaceFontFamily: 'Inter',
    monospaceFont: 'custom',
    monospaceFontFamily: 'Fira Code',
  });
  assert.equal(props.get(INTERFACE_FONT_SLOT), '"Inter",');
  assert.equal(props.get(MONOSPACE_FONT_SLOT), '"Fira Code",');
  applyInterfaceFonts(root, {
    interfaceFont: 'default',
    interfaceFontFamily: 'Inter',
    monospaceFont: 'default',
    monospaceFontFamily: 'Fira Code',
  });
  assert.equal(props.size, 0);
});

test('index.css stacks, the Linux override included, start with the slots', () => {
  const css = readFileSync(join(process.cwd(), 'src/index.css'), 'utf8');
  assert.match(css, /--font-secondary:\s*var\(--synara-font-ui-custom,\) -apple-system/);
  assert.match(css, /--font-mono:\s*var\(--synara-font-mono-custom,\) ui-monospace/);
  assert.match(
    css,
    /html\[data-synara-os='linux'\] \{\s*--font-secondary:\s*var\(--synara-font-ui-custom,\) 'Synara Inter'/
  );
});
