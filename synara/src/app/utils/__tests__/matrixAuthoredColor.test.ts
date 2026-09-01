import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  MATRIX_AUTHORED_INCREASED_CONTRAST,
  MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST,
  matrixAuthoredMinimumContrast,
  resolveMatrixAuthoredColorStyle,
} from '../matrixAuthoredColor';
import {
  deriveThemeRichTextRoles,
  deriveThemeSurfaceRamp,
  THEME_BASE_PRESETS,
  themeContrastRatio,
} from '../themeBase';
import { sanitizeCustomHtml } from '../sanitize';
import {
  prepareNativeFormattedBody,
  sanitizeMatrixV119PresentationHtml,
} from '../../features/room/nativeTimelineRichText';

const RICH_TEXT_PARITY_FIXTURE =
  '<p><strong>strong</strong> and <b>bold</b> with <code>inline()</code></p>' +
  '<pre><code class="language-ts">const ready = true;</code></pre>' +
  '<span data-mx-color="#ffffff" data-mx-bg-color="#000000">authored</span>' +
  '<span data-mx-spoiler="launch plan" data-mx-color="#ffffff" data-mx-bg-color="#000000">hidden</span>' +
  '<table><thead><tr><th>Key</th></tr></thead><tbody><tr><td>Value</td></tr></tbody></table>';

test('safe Matrix-authored foreground and background colors are preserved', () => {
  assert.deepEqual(resolveMatrixAuthoredColorStyle('#ffffff', '#000000', '#1e1f22', '#ffffff'), {
    color: '#ffffff',
    backgroundColor: '#000000',
  });
});

test('unsafe authored foreground is minimally clamped to a readable color', () => {
  const resolved = resolveMatrixAuthoredColorStyle('#333333', undefined, '#1e1f22', '#ffffff');
  assert.ok(resolved.color);
  assert.notEqual(resolved.color, '#333333');
  assert.ok(themeContrastRatio(resolved.color, '#1e1f22') >= MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST);
});

test('authored color minimum follows the runtime Increased Contrast preference', () => {
  assert.equal(matrixAuthoredMinimumContrast(false), 4.5);
  assert.equal(matrixAuthoredMinimumContrast(true), 7);
});

test('authored inline styles cannot override the Increased Contrast floor', () => {
  const resolved = resolveMatrixAuthoredColorStyle(
    '#777777',
    '#eeeeee',
    '#ffffff',
    '#000000',
    MATRIX_AUTHORED_INCREASED_CONTRAST
  );
  const background = resolved.backgroundColor ?? '#ffffff';
  const foreground = resolved.color ?? '#000000';
  assert.ok(themeContrastRatio(foreground, background) >= 7);
});

test('unsafe background-only formatting is dropped instead of harming semantic text', () => {
  assert.deepEqual(resolveMatrixAuthoredColorStyle(undefined, '#000000', '#ffffff', '#000000'), {
    color: undefined,
    backgroundColor: undefined,
  });
});

test('invalid authored values never become presentation styles', () => {
  assert.deepEqual(
    resolveMatrixAuthoredColorStyle('red', 'url(javascript:alert(1))', '#ffffff', '#000000'),
    { color: undefined, backgroundColor: undefined }
  );
});

test('Matrix authored colors require exact six-digit v1.19 hex values', () => {
  assert.deepEqual(resolveMatrixAuthoredColorStyle('#fff', '#000', '#ffffff', '#000000'), {
    color: undefined,
    backgroundColor: undefined,
  });
});

test('nested background-only spans cannot invalidate an inherited authored foreground', () => {
  const parent = resolveMatrixAuthoredColorStyle('#555555', undefined, '#ffffff', '#000000');
  assert.equal(parent.color, '#555555');
  const child = resolveMatrixAuthoredColorStyle(undefined, '#000000', '#ffffff', parent.color);
  assert.equal(child.backgroundColor, undefined);
  assert.equal(child.color, undefined);
});

test('authored foregrounds are safe across contextual table, inline-code, and code-block surfaces', () => {
  const tableSurfaces = ['#ffffff', '#f0f2f4', '#dfe3e8', '#e7eaee'];
  const table = resolveMatrixAuthoredColorStyle('#777777', undefined, tableSurfaces, '#000000');
  assert.ok(table.color);
  tableSurfaces.forEach((surface) => {
    assert.ok(themeContrastRatio(table.color!, surface) >= MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST);
  });

  const inline = resolveMatrixAuthoredColorStyle('#555555', undefined, ['#d9dde3'], '#000000');
  assert.ok(inline.color);
  assert.ok(themeContrastRatio(inline.color, '#d9dde3') >= MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST);

  const codeBlock = resolveMatrixAuthoredColorStyle('#777777', undefined, ['#eceef1'], '#000000');
  assert.ok(codeBlock.color);
  assert.ok(
    themeContrastRatio(codeBlock.color, '#eceef1') >= MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST
  );
});

test('authored colors are clamped against actual Silver and Butter message canvases', () => {
  const legacySurfaces = [
    { surfaces: ['#DEDEDE', '#D3D3D3'], fallback: '#000000' },
    { surfaces: ['#33322C', '#403F38'], fallback: '#ffffff' },
  ];
  for (const { surfaces, fallback } of legacySurfaces) {
    const resolved = resolveMatrixAuthoredColorStyle('#777777', undefined, surfaces, fallback);
    const effective = resolved.color ?? fallback;
    surfaces.forEach((surface) => {
      assert.ok(themeContrastRatio(effective, surface) >= MATRIX_AUTHORED_TEXT_MINIMUM_CONTRAST);
    });
  }
});

test('Increased Contrast authored colors meet 7:1 on derived and legacy rich surfaces', () => {
  const roles = [
    ...THEME_BASE_PRESETS.flatMap(({ hex }) =>
      (['light', 'dark'] as const).map((kind) => deriveThemeSurfaceRamp(hex, kind).richText)
    ),
    deriveThemeRichTextRoles('light', '#DEDEDE', '#D3D3D3'),
    deriveThemeRichTextRoles('dark', '#33322C', '#403F38'),
  ];

  for (const richText of roles) {
    for (const surfaces of [
      [richText.readingSurface, richText.readingSurfaceHover],
      [richText.inlineCodeBackground],
      [richText.codeBlockBackground],
      [richText.spoilerBackground, richText.spoilerHover],
      [
        richText.tableCanvas,
        richText.tableHeader,
        richText.tableOdd,
        richText.tableEven,
        richText.tableHover,
      ],
    ]) {
      const resolved = resolveMatrixAuthoredColorStyle(
        '#777777',
        undefined,
        surfaces,
        richText.contrastForeground,
        MATRIX_AUTHORED_INCREASED_CONTRAST
      );
      const effective = resolved.color ?? richText.contrastForeground;
      surfaces.forEach((surface) => assert.ok(themeContrastRatio(effective, surface) >= 7));
    }
  }
});

test('native legacy font normalization matches compatibility strict span semantics', () => {
  const fixture =
    '<font color="#ABCDEF">valid</font>' +
    '<font color="#abc">invalid</font>' +
    '<font data-mx-color="#12" data-mx-bg-color="#fff">invalid data</font>' +
    '<font data-mx-color="#123456" data-mx-bg-color="#ffffff">data</font>' +
    '<font color="#111111"><font color="#EEEEEE">nested override</font></font>';
  const compatibility = sanitizeCustomHtml(fixture);
  const native = sanitizeMatrixV119PresentationHtml(fixture);

  assert.equal(native, compatibility);
  assert.match(native, /<span data-mx-color="#ABCDEF">valid<\/span>/);
  assert.match(native, /<span>invalid<\/span>/);
  assert.match(native, /<span>invalid data<\/span>/);
  assert.match(
    native,
    /<span data-mx-color="#111111"><span data-mx-color="#EEEEEE">nested override<\/span><\/span>/
  );
  assert.doesNotMatch(native, /<font|style=|data-mx-color="#abc"/);
});

test('native and compatibility renderers share the guarded authored-color component', () => {
  const nativeRenderer = readFileSync(
    'src/app/features/room/nativeTimelineFormattedBody.tsx',
    'utf8'
  );
  const compatibilityRenderer = readFileSync(
    'src/app/plugins/react-custom-html-parser.tsx',
    'utf8'
  );
  const sanitizer = readFileSync('src/app/utils/sanitize.ts', 'utf8');

  assert.match(nativeRenderer, /MatrixColorSpan/);
  for (const surface of ['table', 'inlineCode', 'spoiler', 'codeBlock']) {
    assert.match(nativeRenderer, new RegExp(`MatrixColorSurface surface="${surface}"`));
  }
  assert.match(compatibilityRenderer, /MatrixColorSpan/);
  for (const surface of ['table', 'inlineCode', 'spoiler', 'codeBlock']) {
    assert.match(compatibilityRenderer, new RegExp(`MatrixColorSurface surface="${surface}"`));
  }
  assert.doesNotMatch(sanitizer, /style: `background-color:/);
  assert.match(sanitizer, /delete next\.style/);
});

test('native and compatibility sanitizers preserve the same rich-text fixture semantics', () => {
  const compatibility = sanitizeCustomHtml(RICH_TEXT_PARITY_FIXTURE);
  const native = prepareNativeFormattedBody(RICH_TEXT_PARITY_FIXTURE);
  assert.ok(native);

  for (const semantic of [
    '<strong>',
    '<b>',
    '<code>',
    '<pre>',
    '<table>',
    'data-mx-color="#ffffff"',
    'data-mx-bg-color="#000000"',
    'data-mx-spoiler="launch plan"',
  ]) {
    assert.ok(compatibility.includes(semantic), `compatibility path lost ${semantic}`);
    assert.ok(native.includes(semantic), `native path lost ${semantic}`);
  }
  assert.doesNotMatch(compatibility, /style=/);
  assert.doesNotMatch(native, /style=/);
  const combinedSpoilerColors =
    /<span(?=[^>]*data-mx-spoiler="launch plan")(?=[^>]*data-mx-color="#ffffff")(?=[^>]*data-mx-bg-color="#000000")[^>]*>hidden<\/span>/;
  assert.match(compatibility, combinedSpoilerColors);
  assert.match(native, combinedSpoilerColors);
});

test('spoilers retain authored colors only through their guarded reveal presentation', () => {
  const nativeRenderer = readFileSync(
    'src/app/features/room/nativeTimelineFormattedBody.tsx',
    'utf8'
  );
  const compatibilityRenderer = readFileSync(
    'src/app/plugins/react-custom-html-parser.tsx',
    'utf8'
  );
  const compatibilityCss = readFileSync('src/app/styles/CustomHtml.css.ts', 'utf8');

  assert.match(
    nativeRenderer,
    /<NativeSpoiler[\s\S]*foreground=\{domNode\.attribs\['data-mx-color'\]\}[\s\S]*background=\{domNode\.attribs\['data-mx-bg-color'\]\}/
  );
  assert.match(
    nativeRenderer,
    /if \(revealed\)[\s\S]*MatrixColorSurface surface="spoiler"[\s\S]*<span className=\{htmlCss\.SpoilerContent\}>[\s\S]*<MatrixColorSpan/
  );
  assert.doesNotMatch(
    nativeRenderer,
    /<MatrixColorSpan[^>]*>[\s\S]*<span className=\{htmlCss\.SpoilerContent\}>/
  );
  assert.match(compatibilityRenderer, /role=\{revealed \? undefined : 'button'\}/);
  assert.match(compatibilityRenderer, /tabIndex=\{revealed \? undefined : 0\}/);
  assert.match(compatibilityRenderer, /onClick=\{revealed \? undefined : reveal\}/);
  assert.match(compatibilityRenderer, /aria-pressed=\{revealed \? undefined : true\}/);
  assert.match(compatibilityRenderer, /revealed \?[\s\S]*MatrixColorSpan[\s\S]*children/);
  assert.match(compatibilityRenderer, /<span aria-hidden>spoiler<\/span>/);
  assert.doesNotMatch(
    compatibilityRenderer,
    /<span aria-hidden=\{revealed \? undefined : true\}>[\s\S]*children/
  );
  assert.match(
    compatibilityCss,
    /globalStyle\(`\$\{Spoiler\(\)\}\[aria-pressed=true\] \[aria-hidden=true\]`/
  );
});

test('authored-color context is read once per shared theme revision', () => {
  const source = readFileSync('src/app/utils/matrixAuthoredColor.ts', 'utf8');
  assert.match(source, /cachedColorContextRevision !== colorContextRevision/);
  assert.match(
    source,
    /refreshMatrixColorContext[\s\S]*cachedColorContext = readMatrixColorContext\(\)[\s\S]*colorContextSubscribers\.forEach/
  );
  assert.match(source, /useMatrixColorContext[\s\S]*return getMatrixColorContextSnapshot\(\)/);
  assert.match(source, /matchMedia\('\(prefers-contrast: more\)'\)/);
  assert.match(source, /addEventListener\('change', refreshMatrixColorContext\)/);
  assert.match(source, /removeEventListener\('change', refreshMatrixColorContext\)/);
  assert.match(source, /minimumContrast: matrixAuthoredMinimumContrast/);
});
