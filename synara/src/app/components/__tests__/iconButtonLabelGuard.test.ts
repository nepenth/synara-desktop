import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

// folds tooltips only mount their role="tooltip" portal while open and do not
// wire it to the trigger, so a tooltip never names an icon-only control. Every
// such control has to carry its own accessible name (D52).

const APP_ROOT = 'src/app';

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : walk(path);
    return path.endsWith('.tsx') ? [path] : [];
  });

type OpeningTag = { line: number; text: string };

// Returns each `<Name ...>` opening tag, skipping `>` inside JSX expressions
// and string literals so arrow functions and comparisons do not end the tag.
const openingTags = (source: string, name: string): OpeningTag[] => {
  const tags: OpeningTag[] = [];
  const pattern = new RegExp(`<${name}(?=[\\s>/])`, 'g');
  let match = pattern.exec(source);
  while (match) {
    let depth = 0;
    let quote: string | undefined;
    let i = match.index + match[0].length;
    for (; i < source.length; i += 1) {
      const ch = source[i];
      if (quote) {
        if (ch === quote) quote = undefined;
      } else if (ch === '"' || ch === "'" || ch === '`') {
        if (depth > 0 || ch !== '`') quote = ch;
      } else if (ch === '{') depth += 1;
      else if (ch === '}') depth -= 1;
      else if (ch === '>' && depth === 0) break;
    }
    tags.push({
      line: source.slice(0, match.index).split('\n').length,
      text: source.slice(match.index, i + 1),
    });
    match = pattern.exec(source);
  }
  return tags;
};

const NAMED = /aria-label(ledby)?=|aria-hidden|\{\.\.\./;

const unnamed = (name: string, filter: (tag: OpeningTag) => boolean = () => true) =>
  walk(APP_ROOT).flatMap((file) =>
    openingTags(readFileSync(file, 'utf8'), name)
      .filter((tag) => filter(tag) && !NAMED.test(tag.text))
      .map((tag) => `${file}:${tag.line}`)
  );

test('every IconButton has an accessible name', () => {
  const offenders = unnamed('IconButton');
  assert.deepEqual(
    offenders,
    [],
    `IconButton without aria-label/aria-labelledby/aria-hidden:\n${offenders.join('\n')}`
  );
});

test('sidebar rail buttons have an accessible name', () => {
  const asButton = (tag: OpeningTag) => /\sas="button"/.test(tag.text);
  const offenders = [...unnamed('SidebarAvatar', asButton), ...unnamed('SidebarFolder', asButton)];
  assert.deepEqual(
    offenders,
    [],
    `Sidebar button without aria-label/aria-labelledby:\n${offenders.join('\n')}`
  );
});

test('emoji board sidebar does not point aria-labelledby at a closed tooltip', () => {
  const sidebar = readFileSync(`${APP_ROOT}/components/emoji-board/components/Sidebar.tsx`, 'utf8');
  assert.doesNotMatch(sidebar, /aria-labelledby=\{`SidebarStackItem-/);
  assert.match(sidebar, /aria-label=\{label\}/);
});
