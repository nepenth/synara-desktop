import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import htmlToDom from 'html-dom-parser';
import { isTag } from 'domhandler';
import Prism from 'prismjs';
import { sanitizeCustomHtml } from '../../../utils/sanitize';
import {
  deriveThemeSurfaceRamp,
  THEME_BASE_PRESETS,
  themeContrastRatio,
} from '../../../utils/themeBase';
import {
  countCodeLines,
  displayCodeText,
  formatLineNumbers,
  highlightNativeCode,
  inferCodeLanguage,
  languageClassFromClassName,
  nativeCodeBlockFromPreChildren,
  normalizePrismLanguage,
} from '../nativeTimelineCodeHighlight';
import {
  NATIVE_SYNTAX_PALETTES,
  NATIVE_SYNTAX_ROLES,
  type NativeSyntaxPalette,
} from '../nativeTimelineSyntaxPalette';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
const formattedBody = readFileSync('src/app/features/room/nativeTimelineFormattedBody.tsx', 'utf8');
const highlight = readFileSync('src/app/features/room/nativeTimelineCodeHighlight.ts', 'utf8');
const htmlCss = readFileSync('src/app/features/room/nativeTimelineHtml.css.ts', 'utf8');

const THEME_BASE_FIXTURES = [
  ...THEME_BASE_PRESETS.map(({ hex }) => hex),
  '#000000',
  '#ffffff',
  '#ff0000',
  '#00ff00',
  '#0000ff',
];

const codePanelBackgrounds = (kind: 'light' | 'dark'): string[] => [
  ...(kind === 'light' ? ['#F2F3F5', '#EAEAEA'] : ['#2B2D31', '#262621']),
  ...THEME_BASE_FIXTURES.map((base) => deriveThemeSurfaceRamp(base, kind).surface.Container),
];

const assertPaletteContrast = (
  paletteName: string,
  palette: NativeSyntaxPalette,
  backgrounds: string[],
  minimum: number
) => {
  NATIVE_SYNTAX_ROLES.forEach((role) => {
    backgrounds.forEach((background) => {
      const ratio = themeContrastRatio(palette[role], background);
      assert.ok(
        ratio >= minimum,
        `${paletteName}.${role} ${palette[role]} is ${ratio.toFixed(
          2
        )}:1 on ${background}; expected ${minimum}:1`
      );
    });
  });
};

const PYTHON_FIXTURE = `<p>run this:</p><pre><code class="language-python">def greet(name):
    return f"hi {name}"
</code></pre><p>inline <code>print</code> stays inline</p>`;

const firstPreChildren = (html: string) => {
  const pre = htmlToDom(html).find((node) => isTag(node) && node.name === 'pre');
  assert.ok(pre && isTag(pre), 'expected a <pre> node');
  return pre.children;
};

const ensurePythonGrammar = () => {
  if (Prism.languages.python) return;
  Prism.languages.python = {
    comment: /#.*/,
    keyword: /\b(?:def|return|class|import|from|as)\b/,
    function: {
      pattern: /((?:^|\s)def[ \t]+)[A-Za-z_]\w*(?=\s*\()/,
      lookbehind: true,
    },
    string: {
      pattern: /f?(?:"""[\s\S]*?"""|"(?:\\.|[^"\\])*")/,
      greedy: true,
    },
  };
};

test('native presenter routes formatted HTML through the sanitized Prism code-block path', () => {
  assert.match(presenter, /NativeFormattedBody/);
  assert.match(formattedBody, /sanitizeCustomHtml/);
  assert.match(formattedBody, /react-prism\/ReactPrism/);
  assert.match(formattedBody, /highlightNativeCode/);
  assert.match(formattedBody, /displayCodeText/);
  assert.match(formattedBody, /data-native-code-block/);
  assert.match(formattedBody, /CodeLineNumbers/);
  assert.match(formattedBody, /CodeRow/);
  assert.match(formattedBody, /setHighlightedHtml\(undefined\)/);
  assert.match(formattedBody, /\.catch\(/);
  assert.match(highlight, /language-/);
  assert.match(htmlCss, /CodeLineNumbers/);
  assert.match(htmlCss, /CodeRow/);
  assert.match(htmlCss, /whiteSpace: 'pre'/);
  assert.match(htmlCss, /ui-monospace/);
  assert.match(htmlCss, /fontSize: '0\.92em'/);
  assert.match(htmlCss, /lineHeight: 1\.5/);
  assert.match(htmlCss, /color\.Surface\.Container/);
  assert.match(htmlCss, /NATIVE_SYNTAX_PALETTES\.light/);
  assert.match(htmlCss, /NATIVE_SYNTAX_PALETTES\.dark/);
  assert.match(htmlCss, /NATIVE_SYNTAX_PALETTES\.moreLight/);
  assert.match(htmlCss, /NATIVE_SYNTAX_PALETTES\.moreDark/);
  assert.match(htmlCss, /prefers-contrast: more/);
  assert.match(htmlCss, /color: syntaxMeta/);
  assert.match(htmlCss, /token\.namespace[\s\S]*?opacity: 1/);
  assert.doesNotMatch(
    htmlCss,
    /#7a8478|#9aa0a6|#e06c75|#d19a66|#98c379|#56b6c2|#61afef|#c678dd|#e5c07b/i
  );
  assert.doesNotMatch(htmlCss, /color\.Background\.Container/);
  assert.match(htmlCss, /export const CodeScroll[\s\S]*?overflowX: 'auto'/);
  assert.match(formattedBody, /htmlCss\.CodeLineNumbers[\s\S]*?htmlCss\.CodeScroll/);
  assert.doesNotMatch(formattedBody, /matrix-js-sdk/);
  assert.doesNotMatch(highlight, /matrix-js-sdk/);
});

test('native syntax palettes meet ordinary and increased text contrast on every code surface', () => {
  const lightBackgrounds = codePanelBackgrounds('light');
  const darkBackgrounds = codePanelBackgrounds('dark');

  assertPaletteContrast('light', NATIVE_SYNTAX_PALETTES.light, lightBackgrounds, 4.5);
  assertPaletteContrast('dark', NATIVE_SYNTAX_PALETTES.dark, darkBackgrounds, 4.5);
  assertPaletteContrast('moreLight', NATIVE_SYNTAX_PALETTES.moreLight, lightBackgrounds, 7);
  assertPaletteContrast('moreDark', NATIVE_SYNTAX_PALETTES.moreDark, darkBackgrounds, 7);
});

test('code metadata and line numbers use an opaque semantic syntax role', () => {
  const languageStyle = htmlCss.slice(
    htmlCss.indexOf('export const CodeLanguage'),
    htmlCss.indexOf('export const CodeRow')
  );
  const lineNumberStyle = htmlCss.slice(
    htmlCss.indexOf('export const CodeLineNumbers'),
    htmlCss.indexOf('globalStyle(`${FormattedBody} p`')
  );

  assert.match(languageStyle, /color: syntaxMeta/);
  assert.doesNotMatch(languageStyle, /opacity:/);
  assert.match(lineNumberStyle, /color: syntaxMeta/);
  assert.doesNotMatch(lineNumberStyle, /opacity:/);
});

test('sanitized language-python fixtures become highlighted tokens with line numbers', () => {
  ensurePythonGrammar();
  const sanitized = sanitizeCustomHtml(PYTHON_FIXTURE);
  assert.match(sanitized, /class="language-python"/);
  assert.match(sanitized, /<code>print<\/code>/);

  const block = nativeCodeBlockFromPreChildren(firstPreChildren(sanitized));
  assert.equal(block.languageClass, 'language-python');
  assert.equal(languageClassFromClassName(block.languageClass), 'language-python');
  assert.equal(normalizePrismLanguage(block.languageClass), 'python');
  assert.equal(countCodeLines(block.code), 2);
  assert.equal(formatLineNumbers(2), '1\n2');
  assert.match(block.code, /def greet\(name\):/);

  const highlighted = highlightNativeCode(block.code, block.languageClass);
  assert.match(highlighted, /class="token/);
  assert.match(highlighted, /token keyword/);
  assert.match(highlighted, /greet/);
});

test('language-js aliases to javascript and uses Prism core grammar', () => {
  const highlighted = highlightNativeCode('const ready = true;', 'language-js');
  assert.match(highlighted, /class="token keyword"/);
  assert.match(highlighted, /const/);
});

test('unlabeled python and shell fences infer a language and emit tokens', () => {
  const python = 'from pathlib import Path\nprint(Path("."))\n';
  assert.equal(inferCodeLanguage(python), 'python');
  const pythonHtml = highlightNativeCode(python);
  assert.match(pythonHtml, /token keyword/);
  assert.match(pythonHtml, /import/);

  const shell = `python3 - <<'PY'\nfrom pathlib import Path\nPY\ncurl -sS http://example.test\n`;
  assert.equal(inferCodeLanguage(shell), 'python');
  assert.match(highlightNativeCode(shell), /token/);
});

test('unknown languages still produce a numbered code panel model', () => {
  const sanitized = sanitizeCustomHtml(
    '<pre><code class="language-not-a-real-lang">alpha\nbeta\n</code></pre>'
  );
  const block = nativeCodeBlockFromPreChildren(firstPreChildren(sanitized));
  assert.equal(block.languageClass, 'language-not-a-real-lang');
  assert.equal(countCodeLines(block.code), 2);
  assert.equal(formatLineNumbers(countCodeLines(block.code)), '1\n2');
  assert.equal(highlightNativeCode(block.code, block.languageClass), 'alpha\nbeta\n');
});

const visualLinesForPre = (text: string): number => text.split('\n').length;

test('Synara language-python fences strip one trailing newline so gutter count matches visual lines', () => {
  const sanitized = sanitizeCustomHtml(PYTHON_FIXTURE);
  const block = nativeCodeBlockFromPreChildren(firstPreChildren(sanitized));
  assert.equal(block.languageClass, 'language-python');
  assert.equal(block.code.endsWith('\n'), true);
  assert.equal(block.code.includes('\n\n'), false);

  const display = displayCodeText(block.code);
  assert.equal(display.endsWith('\n'), false);
  assert.match(display, /def greet\(name\):/);
  assert.match(display, /return f"hi \{name\}"/);

  const gutterCount = countCodeLines(block.code);
  assert.equal(gutterCount, countCodeLines(display));
  assert.equal(gutterCount, visualLinesForPre(display));
  assert.equal(gutterCount, 2);
  assert.notEqual(gutterCount, visualLinesForPre(block.code));
  assert.equal(formatLineNumbers(gutterCount), '1\n2');
  assert.equal(formattedBody.includes('{displayCode}'), true);

  const withInternalBlank = 'one\n\nthree\n';
  assert.equal(displayCodeText(withInternalBlank), 'one\n\nthree');
  assert.equal(countCodeLines(withInternalBlank), 3);
  assert.equal(visualLinesForPre(displayCodeText(withInternalBlank)), 3);
});
