import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import {
  MAX_NATIVE_FORMATTED_BODY_BYTES,
  prepareNativeFormattedBody,
} from '../nativeTimelineRichText';
import { projectNativeFormattedBody } from '../nativeTimelinePresentationProjection';

type Generator =
  | { kind: 'nestedTag'; tag: string; count: number; text: string }
  | { kind: 'repeatedText'; prefix: string; unit: string; count: number; suffix: string };

type CorpusCase = {
  id: string;
  body: string;
  formattedBody: string;
  generator?: Generator;
  expect: {
    accepted: boolean;
    textContains: string[];
    textExcludes: string[];
    linkSchemes: string[];
    containsSpoiler: boolean;
    forbiddenFragments: string[];
    semanticKinds?: SemanticKind[];
    mentionTargets?: string[];
    spoilerReasons?: string[];
    inlineCode?: string[];
    codeBlocks?: string[];
    orderedListStarts?: number[];
  };
};

type SemanticKind =
  | 'bold'
  | 'heading'
  | 'inlineCode'
  | 'orderedList'
  | 'preformattedCode'
  | 'spoiler'
  | 'strikethrough'
  | 'table'
  | 'unorderedList';

type Corpus = {
  schemaVersion: number;
  presentationFormattedBodyMaxBytes: number;
  coverage: Record<string, string[]>;
  cases: CorpusCase[];
};

const REQUIRED_COVERAGE_AREAS = [
  'executable-content',
  'formatted-reply-fallback',
  'inline-code',
  'links',
  'lists',
  'malformed-html',
  'mentions',
  'plaintext-fallback',
  'preformatted-code',
  'presentation-size-boundary',
  'remote-resource-blocking',
  'spoilers',
  'tables',
] as const;

const corpus = JSON.parse(
  readFileSync(
    resolve(
      process.cwd(),
      '../docs/future-projects/rust-ownership-expansion/fixtures/message-format/corpus.json'
    ),
    'utf8'
  )
) as Corpus;

const expand = (fixture: CorpusCase): string => {
  const generator = fixture.generator;
  if (!generator) return fixture.formattedBody;
  if (generator.kind === 'nestedTag') {
    return `${`<${generator.tag}>`.repeat(generator.count)}${
      generator.text
    }${`</${generator.tag}>`.repeat(generator.count)}`;
  }
  return `${generator.prefix}${generator.unit.repeat(generator.count)}${generator.suffix}`;
};

const decodeBasicEntities = (value: string): string =>
  value
    .replace(/&#(\d+);/g, (_match, decimalValue: string) =>
      String.fromCodePoint(Number(decimalValue))
    )
    .replace(/&#x([0-9a-f]+);/gi, (_match, hexadecimalValue: string) =>
      String.fromCodePoint(Number.parseInt(hexadecimalValue, 16))
    )
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, '&');

const semanticText = (html: string): string =>
  decodeBasicEntities(
    html
      .replace(/<img\b[^>]*\balt="([^"]*)"[^>]*>/gi, ' $1 ')
      .replace(/<br\s*\/?>/gi, '\n')
      .replace(/<[^>]*>/g, ' ')
  )
    .replace(/\s+/g, ' ')
    .trim();

const linkSchemes = (html: string): string[] =>
  Array.from(html.matchAll(/\bhref="([A-Za-z][A-Za-z0-9+.-]*):[^"]*"/g), (match) =>
    match[1].toLowerCase()
  ).sort();

test('desktop presentation satisfies the shared Matrix and Hermes format corpus', () => {
  assert.equal(corpus.schemaVersion, 1);
  assert.equal(corpus.presentationFormattedBodyMaxBytes, MAX_NATIVE_FORMATTED_BODY_BYTES);
  const fixtureIDs = new Set(corpus.cases.map(({ id }) => id));
  assert.equal(fixtureIDs.size, corpus.cases.length);
  assert.deepEqual(Object.keys(corpus.coverage).sort(), [...REQUIRED_COVERAGE_AREAS].sort());
  for (const [area, ids] of Object.entries(corpus.coverage)) {
    assert.ok(ids.length > 0, `coverage area has no fixtures: ${area}`);
    for (const id of ids) assert.ok(fixtureIDs.has(id), `unknown ${area} fixture: ${id}`);
  }

  for (const fixture of corpus.cases) {
    const html = expand(fixture);
    const sanitized = prepareNativeFormattedBody(html);
    const projected = projectNativeFormattedBody(html);
    assert.equal(Boolean(sanitized), fixture.expect.accepted, fixture.id);
    assert.equal(projected !== undefined, fixture.expect.accepted, `${fixture.id}: presenter`);
    const text = sanitized ? semanticText(sanitized) : fixture.body;

    for (const expected of fixture.expect.textContains) {
      assert.ok(text.includes(expected), `${fixture.id}: missing text ${expected}`);
    }
    for (const excluded of fixture.expect.textExcludes) {
      assert.ok(!text.includes(excluded), `${fixture.id}: exposed text ${excluded}`);
    }
    assert.deepEqual(
      linkSchemes(sanitized ?? ''),
      [...fixture.expect.linkSchemes].sort(),
      fixture.id
    );
    assert.equal(
      (projected?.semanticKinds ?? []).includes('spoiler'),
      fixture.expect.containsSpoiler,
      fixture.id
    );
    if (fixture.expect.containsSpoiler) {
      assert.ok(
        sanitized?.includes('data-mx-spoiler') ?? false,
        `${fixture.id}: spoiler attr stripped`
      );
    }
    for (const forbidden of fixture.expect.forbiddenFragments) {
      assert.ok(!(sanitized ?? '').includes(forbidden), `${fixture.id}: retained ${forbidden}`);
    }
    const actualKinds = new Set(projected?.semanticKinds ?? []);
    for (const kind of fixture.expect.semanticKinds ?? []) {
      assert.ok(actualKinds.has(kind), `${fixture.id}: missing semantic kind ${kind}`);
    }
    assert.deepEqual(
      (projected?.links ?? []).filter((href) => href.startsWith('https://matrix.to/#/@')),
      fixture.expect.mentionTargets ?? [],
      `${fixture.id}: mention targets`
    );
    assert.deepEqual(
      projected?.spoilerReasons ?? [],
      fixture.expect.spoilerReasons ?? [],
      `${fixture.id}: spoiler reasons`
    );
    assert.deepEqual(projected?.inlineCode ?? [], fixture.expect.inlineCode ?? [], fixture.id);
    assert.deepEqual(projected?.codeBlocks ?? [], fixture.expect.codeBlocks ?? [], fixture.id);
    assert.deepEqual(
      projected?.orderedListStarts ?? [],
      fixture.expect.orderedListStarts ?? [],
      `${fixture.id}: ordered-list starts`
    );
    if (fixture.id === 'adversarial-mxc-inline-image') {
      assert.deepEqual(projected?.inlineImageFallbacks, []);
      assert.equal(projected?.resourceOwningElements, 0);
    }
  }
});
