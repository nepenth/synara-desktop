import assert from 'node:assert/strict';
import test from 'node:test';
import { findMessageLinks, segmentMessageLinks } from '../messageLinks';

const hrefs = (text: string) => findMessageLinks(text).map((link) => link.href);

test('a file path or file name is not a link (README_SUBMISSION.md became submission.md)', () => {
  assert.deepEqual(
    hrefs(
      'Your one remaining action: open /Users/Shared/ASB_SUBMISSION_20261109/README_SUBMISSION.md and submit through the portal.'
    ),
    []
  );
  assert.deepEqual(hrefs('├── README_SUBMISSION.md        ← start here'), []);
  assert.deepEqual(hrefs('see README.md, setup.py, main.rs, build.sh and notes.zip'), []);
  assert.deepEqual(hrefs('the snake_case_name.md file and docs/guide.md'), []);
});

test('real addresses still link, with or without a scheme', () => {
  assert.deepEqual(hrefs('go to https://example.org/path now'), ['https://example.org/path']);
  assert.deepEqual(hrefs('try example.com or www.example.md'), [
    'http://example.com',
    'http://www.example.md',
  ]);
  assert.deepEqual(hrefs('a site with an md domain: https://example.md'), ['https://example.md']);
  assert.deepEqual(hrefs('(see example.org)'), ['http://example.org']);
  // An address written with its scheme links even when glued to text.
  assert.deepEqual(hrefs('terrible news guys...https://www.example.com/a so sad'), [
    'https://www.example.com/a',
  ]);
  assert.deepEqual(hrefs('mail ada@example.org please'), ['mailto:ada@example.org']);
});

test('segments keep every character and mark only the links', () => {
  const text = 'open notes/README.md or https://example.org.';
  const segments = segmentMessageLinks(text);
  assert.equal(segments.map((segment) => segment.text).join(''), text);
  assert.deepEqual(
    segments.filter((segment) => segment.href).map((segment) => segment.href),
    ['https://example.org']
  );
});
