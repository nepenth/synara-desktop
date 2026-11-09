import assert from 'node:assert/strict';
import test from 'node:test';
import type { Descendant } from 'slate';
import { BlockType } from '../../../components/editor/types';
import { getContentMentions, toMatrixCustomHTML } from '../../../components/editor/output';
import { createMentionElement } from '../../../components/editor/utils';
import { applyTypedMentions, typedMentionSource } from '../typedMentions';
import type { TypedMention } from '../../matrix-dto/generated';

const paragraph = (...children: unknown[]): Descendant =>
  ({ type: BlockType.Paragraph, children }) as unknown as Descendant;

const mention = (start: number, end: number, userId: string, text: string): TypedMention => ({
  start,
  end,
  userId,
  text,
});

test('typed @Name becomes a pill in the HTML and m.mentions', () => {
  const content = [paragraph({ text: 'hi @Puck, lunch?' })];
  const source = typedMentionSource(content);
  assert.equal(source, 'hi @Puck, lunch?');
  const resolved = applyTypedMentions(content, [mention(3, 8, '@puck:example.org', '@Puck')]);
  assert.equal(
    toMatrixCustomHTML(resolved, { allowTextFormatting: true }),
    'hi <a href="https://matrix.to/#/@puck:example.org">@Puck</a>, lunch?<br/>'
  );
  assert.deepEqual([...getContentMentions(resolved).users], ['@puck:example.org']);
  // The composer content itself is not changed.
  assert.equal(getContentMentions(content).users.size, 0);
});

test('offsets span paragraphs and skip code and existing pills', () => {
  const content = [
    paragraph({ text: 'first' }),
    paragraph({ text: 'ask ' }, createMentionElement('@sam:example.org', '@Sam', false), {
      text: ' and @Al',
    }),
    {
      type: BlockType.CodeBlock,
      children: [{ type: BlockType.CodeLine, children: [{ text: '@Al' }] }],
    } as unknown as Descendant,
    paragraph({ text: '@Al', code: true }),
  ];
  // Leaves: "first", "ask ", " and @Al" (code and pills are left out).
  const source = typedMentionSource(content);
  assert.equal(source, 'first\nask \n and @Al');
  const resolved = applyTypedMentions(content, [mention(16, 19, '@al:example.org', '@Al')]);
  const mentions = getContentMentions(resolved);
  assert.deepEqual([...mentions.users].sort(), ['@al:example.org', '@sam:example.org']);
});

test('a range that leaves its leaf is ignored', () => {
  const content = [paragraph({ text: 'one' }), paragraph({ text: 'two' })];
  assert.equal(applyTypedMentions(content, [mention(2, 5, '@x:example.org', 'e\nt')]), content);
  assert.equal(applyTypedMentions(content, []), content);
});
