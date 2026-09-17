import assert from 'node:assert/strict';
import test from 'node:test';
import { parseBlockMD } from './parser';
import { parseInlineMD } from '../inline/parser';

const parse = (source: string): string => parseBlockMD(source, parseInlineMD);

test('dash and star bullets both produce unordered lists', () => {
  assert.match(parse('- dash'), /<ul[\s>][\s\S]*dash/);
  assert.doesNotMatch(parse('- dash'), /<ol[\s>]/);
  assert.match(parse('* star'), /<ul[\s>][\s\S]*star/);
  assert.doesNotMatch(parse('* star'), /<ol[\s>]/);
});

test('ordered lists still produce ol', () => {
  assert.match(parse('1. ordered'), /<ol[\s>][\s\S]*ordered/);
  assert.doesNotMatch(parse('1. ordered'), /<ul[\s>]/);
});

test('heading and bold still work', () => {
  const html = parse('# Agent notes\n\nUse **bold** for emphasis.\n');
  assert.match(html, /<h1[\s>][\s\S]*Agent notes/);
  assert.match(html, /<strong[\s>][\s\S]*bold/);
});

test('pipe tables produce table markup', () => {
  const html = parse('| Name | Role |\n| --- | --- |\n| Ada | Lead |\n');
  assert.match(html, /<table[\s>][\s\S]*<th[\s>][\s\S]*Name/);
  assert.match(html, /<td[\s>][\s\S]*Ada/);
});

test('thematic breaks produce hr', () => {
  assert.match(parse('before\n\n---\n\nafter'), /<hr/);
  assert.match(parse('***'), /<hr/);
});

test('task list items keep a sanitizer-safe checkbox prefix', () => {
  const html = parse('- [ ] open\n- [x] done\n');
  assert.match(html, /<ul[\s>]/);
  assert.match(html, /\[ \] open/);
  assert.match(html, /\[x\] done/);
  assert.doesNotMatch(html, /<input/);
});
