import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  MIDDLE_TRUNCATION_MIN_LENGTH,
  MIDDLE_TRUNCATION_TAIL,
  middleTruncationDisplay,
  splitForMiddleTruncation,
} from '../text/middleTruncation';

test('a long name keeps its last seven characters in their own span', () => {
  assert.equal(MIDDLE_TRUNCATION_TAIL, 7);
  // Renders as "Project - Ho…agement" when the start has to ellipsize.
  assert.deepEqual(splitForMiddleTruncation('Project - Home Management'), {
    start: 'Project - Home Man',
    end: 'agement',
  });
});

test('names up to the threshold are one span', () => {
  assert.equal(MIDDLE_TRUNCATION_MIN_LENGTH, 18);
  assert.equal(splitForMiddleTruncation('release-train'), undefined);
  assert.equal(splitForMiddleTruncation('x'.repeat(18)), undefined);
  assert.deepEqual(splitForMiddleTruncation('x'.repeat(19)), {
    start: 'x'.repeat(12),
    end: 'x'.repeat(7),
  });
  assert.equal(splitForMiddleTruncation(''), undefined);
});

test('the split counts code points, so an emoji is never cut in half', () => {
  const name = 'Weekend plans and more 🎉🎉';
  const parts = splitForMiddleTruncation(name);
  assert.ok(parts);
  assert.equal(parts.end, 'more 🎉🎉');
  assert.equal(parts.start + parts.end, name);
});

test('the ellipsis leads the tail only while the start overflows', () => {
  const parts = splitForMiddleTruncation('Project - Home Management');
  assert.ok(parts);
  // Overflowing: the start clips under a fade, "…" sits flush before the tail.
  assert.deepEqual(middleTruncationDisplay(parts, true), {
    start: 'Project - Home Man',
    end: '…agement',
  });
  // Fits: no ellipsis anywhere, the two spans read as the plain name.
  const fits = middleTruncationDisplay(parts, false);
  assert.equal(fits.start + fits.end, 'Project - Home Management');
  const css = readFileSync('src/app/components/text/MiddleTruncatedText.css.ts', 'utf8');
  assert.match(css, /MiddleTruncatedStart = style\(\{[\s\S]*?textOverflow: 'clip'/);
  assert.match(css, /black calc\(100% - 12px\), transparent/);
});

test('the tail and threshold are adjustable', () => {
  assert.deepEqual(splitForMiddleTruncation('matrix.whyland.net', 7, 12), {
    start: 'matrix.whyl',
    end: 'and.net',
  });
  assert.equal(splitForMiddleTruncation('matrix.org', 7, 12), undefined);
  assert.equal(splitForMiddleTruncation('abcdefghijklmnopqrstuvwxyz', 0), undefined);
});

test('the component lays the two spans out as a shrinking start and a fixed tail', () => {
  const css = readFileSync('src/app/components/text/MiddleTruncatedText.css.ts', 'utf8');
  assert.match(css, /textOverflow: 'ellipsis'/);
  assert.match(css, /minWidth: 0/);
  assert.match(css, /flexShrink: 0/);
});
