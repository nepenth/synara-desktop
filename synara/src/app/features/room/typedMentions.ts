import { Descendant, Element, Text } from 'slate';
import { BlockType } from '../../components/editor/types';
import { createMentionElement } from '../../components/editor/utils';
import type { InlineElement } from '../../components/editor/slate';
import type { TypedMention } from '../matrix-dto/generated';

/**
 * Mentions typed out as "@Name" become mention nodes at send time, so the
 * message carries a pill and `m.mentions` as if the name had been picked
 * from the list. Core decides which names resolve
 * (`resolveTypedMentions`); this maps its ranges onto the composer content.
 *
 * Only plain text leaves are searched: code, links and existing pills are
 * left alone. Leaves are joined with a line break so a name never spans two.
 */

const SEPARATOR = '\n';

/** Whether `node`'s text may hold a typed mention. */
const searchable = (node: Descendant): boolean =>
  !Element.isElement(node) ||
  (node.type !== BlockType.CodeBlock &&
    node.type !== BlockType.CodeLine &&
    node.type !== BlockType.Link &&
    node.type !== BlockType.Mention &&
    node.type !== BlockType.Command &&
    node.type !== BlockType.Emoticon);

/** The searchable text leaves of `content`, in order. */
const textLeaves = (content: Descendant[]): Text[] => {
  const leaves: Text[] = [];
  const walk = (node: Descendant): void => {
    if (Text.isText(node)) {
      if (!(node as { code?: boolean }).code) leaves.push(node);
      return;
    }
    if (!searchable(node)) return;
    node.children.forEach((child) => walk(child as Descendant));
  };
  content.forEach(walk);
  return leaves;
};

/** The text Core searches for typed mentions. */
export const typedMentionSource = (content: Descendant[]): string =>
  textLeaves(content)
    .map((leaf) => leaf.text)
    .join(SEPARATOR);

/** `text` split at `mentions` (offsets into it) into text and mention nodes. */
const splitLeaf = (leaf: Text, mentions: TypedMention[], offset: number): InlineElement[] => {
  const parts: InlineElement[] = [];
  let at = 0;
  mentions.forEach((mention) => {
    const start = mention.start - offset;
    const end = mention.end - offset;
    parts.push({ ...leaf, text: leaf.text.slice(at, start) });
    parts.push(createMentionElement(mention.userId, mention.text, false));
    at = end;
  });
  parts.push({ ...leaf, text: leaf.text.slice(at) });
  return parts;
};

/**
 * `content` with each of `mentions` (UTF-16 ranges into
 * {@link typedMentionSource}) turned into a mention node. A range that is
 * not inside one leaf is ignored.
 */
export const applyTypedMentions = (
  content: Descendant[],
  mentions: readonly TypedMention[]
): Descendant[] => {
  if (mentions.length === 0) return content;
  const byLeaf = new Map<Text, { offset: number; mentions: TypedMention[] }>();
  let offset = 0;
  textLeaves(content).forEach((leaf) => {
    const end = offset + leaf.text.length;
    const inside = mentions
      .filter((m) => m.start >= offset && m.end <= end && m.start < m.end)
      .sort((a, b) => a.start - b.start);
    if (inside.length > 0) byLeaf.set(leaf, { offset, mentions: inside });
    offset = end + SEPARATOR.length;
  });
  if (byLeaf.size === 0) return content;
  const rebuild = (node: Descendant): Descendant[] => {
    if (Text.isText(node)) {
      const found = byLeaf.get(node);
      return found ? (splitLeaf(node, found.mentions, found.offset) as Descendant[]) : [node];
    }
    if (!searchable(node)) return [node];
    return [
      {
        ...node,
        children: node.children.flatMap((child) => rebuild(child as Descendant)),
      } as Descendant,
    ];
  };
  return content.flatMap(rebuild);
};
