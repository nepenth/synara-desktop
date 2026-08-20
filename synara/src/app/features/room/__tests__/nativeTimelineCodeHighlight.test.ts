import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import htmlToDom from 'html-dom-parser';
import { isTag } from 'domhandler';
import Prism from 'prismjs';
import { sanitizeCustomHtml } from '../../../utils/sanitize';
import {
  countCodeLines,
  formatLineNumbers,
  highlightNativeCode,
  languageClassFromClassName,
  nativeCodeBlockFromPreChildren,
  normalizePrismLanguage,
} from '../nativeTimelineCodeHighlight';

const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
const formattedBody = readFileSync('src/app/features/room/nativeTimelineFormattedBody.tsx', 'utf8');
const highlight = readFileSync('src/app/features/room/nativeTimelineCodeHighlight.ts', 'utf8');
const htmlCss = readFileSync('src/app/features/room/nativeTimelineHtml.css.ts', 'utf8');

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
  assert.match(formattedBody, /data-native-code-block/);
  assert.match(formattedBody, /CodeLineNumbers/);
  assert.match(highlight, /language-/);
  assert.match(htmlCss, /CodeLineNumbers/);
  assert.match(htmlCss, /whiteSpace: 'pre'/);
  assert.match(htmlCss, /color\.Background\.Container/);
  assert.doesNotMatch(formattedBody, /matrix-js-sdk/);
  assert.doesNotMatch(highlight, /matrix-js-sdk/);
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
