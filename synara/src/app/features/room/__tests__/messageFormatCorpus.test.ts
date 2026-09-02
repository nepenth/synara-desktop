import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import {
  MAX_NATIVE_FORMATTED_BODY_BYTES,
  prepareNativeFormattedBody,
} from '../nativeTimelineRichText';

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
  };
};

type Corpus = {
  schemaVersion: number;
  presentationFormattedBodyMaxBytes: number;
  cases: CorpusCase[];
};

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
    .replace(/&#(\d+);/g, (_match, value: string) => String.fromCodePoint(Number(value)))
    .replace(/&#x([0-9a-f]+);/gi, (_match, value: string) =>
      String.fromCodePoint(Number.parseInt(value, 16))
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
  assert.equal(new Set(corpus.cases.map(({ id }) => id)).size, corpus.cases.length);

  for (const fixture of corpus.cases) {
    const html = expand(fixture);
    const sanitized = prepareNativeFormattedBody(html);
    assert.equal(Boolean(sanitized), fixture.expect.accepted, fixture.id);
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
      sanitized?.includes('data-mx-spoiler') ?? false,
      fixture.expect.containsSpoiler,
      fixture.id
    );
    for (const forbidden of fixture.expect.forbiddenFragments) {
      assert.ok(!(sanitized ?? '').includes(forbidden), `${fixture.id}: retained ${forbidden}`);
    }
  }
});
