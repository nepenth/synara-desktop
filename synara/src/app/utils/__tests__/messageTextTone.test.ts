import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { messageTextForeground, normalizeMessageTextTone } from '../messageTextTone';

test('message text tone rejects unknown persisted values', () => {
  assert.equal(normalizeMessageTextTone('soft'), 'soft');
  assert.equal(normalizeMessageTextTone('balanced'), 'balanced');
  assert.equal(normalizeMessageTextTone('bright'), 'bright');
  assert.equal(normalizeMessageTextTone('white'), 'bright');
  assert.equal(normalizeMessageTextTone(undefined), 'bright');
});

test('message text tones preserve semantic content-role variables', () => {
  assert.match(messageTextForeground('soft'), /--synara-content-secondary/);
  assert.equal(messageTextForeground('balanced'), 'var(--synara-content-primary)');
  assert.match(messageTextForeground('bright'), /--synara-content-heading/);
});

test('increased contrast overrides every inline message text tone', () => {
  const globalCss = readFileSync('src/index.css', 'utf8');
  assert.match(
    globalCss,
    /@media \(prefers-contrast: more\)[\s\S]*body \{[\s\S]*--synara-message-foreground: var\(--synara-content-primary\) !important;/
  );
});
