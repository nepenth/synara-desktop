import { Descendant, Element, Text } from 'slate';
import { invokeDesktopWithAvailability } from '../../utils/desktop';
import { BlockType } from './types';
import { toMatrixCustomHTML } from './output';
import { htmlToEditorInput, plainToEditorInput } from './input';

/**
 * Markdown composer output, rendered by Core (`render_composer_markdown`), the
 * same markdown renderer iOS uses.
 *
 * The editor tree becomes markdown source. Inline constructs markdown cannot
 * express (mentions, custom emoji, links, underline, spoilers) become HTML
 * fragments behind private-use placeholders, which Core sanitizes and puts
 * back after rendering.
 */
export type ComposerMarkdown = { source: string; fragments: string[] };

const PLACEHOLDER_OPEN = '';
const PLACEHOLDER_CLOSE = '';
// Placeholder and Core's literal stand-ins can never come from typed text.
const RESERVED_CHARS = /[-]/g;

const longestRun = (text: string, char: string): number => {
  let best = 0;
  let current = 0;
  for (const c of text) {
    current = c === char ? current + 1 : 0;
    best = Math.max(best, current);
  }
  return best;
};

const codeSpan = (text: string): string => {
  const fence = '`'.repeat(longestRun(text, '`') + 1);
  const pad = text.startsWith('`') || text.endsWith('`') ? ' ' : '';
  return `${fence}${pad}${text}${pad}${fence}`;
};

const isList = (node: Descendant): boolean =>
  Element.isElement(node) &&
  (node.type === BlockType.OrderedList || node.type === BlockType.UnorderedList);

/**
 * - `draft`: Markdown for a synced draft, in the form the iOS composer writes:
 *   no placeholders; mentions and links are Markdown links, underline and
 *   spoilers the inline HTML tags iOS uses, custom emoji their `:shortcode:`.
 * - `literal`: the text was typed with Markdown off, so its Markdown
 *   characters are escaped and read back as typed.
 */
export type ComposerMarkdownOptions = { draft?: boolean; literal?: boolean };

const INLINE_MARKDOWN = /[\\`*_[\]<>~|]/g;
const BLOCK_MARKER = /^(\s*)(#{1,6}\s|>|[-+*]\s|\d{1,9}[.)]\s)/;
const escapeInline = (text: string) => text.replace(INLINE_MARKDOWN, (char) => `\\${char}`);
const escapeLineStart = (line: string) =>
  line.replace(BLOCK_MARKER, (_match, space: string, marker: string) =>
    /^\d/.test(marker) ? `${space}${marker.replace(/([.)])/, '\\$1')}` : `${space}\\${marker}`
  );
const linkLabel = (text: string) => text.replace(/[\\[\]*_`<>]/g, (char) => `\\${char}`);
const linkTarget = (href: string) => href.replace(/[()\s]/g, encodeURIComponent);

export const toComposerMarkdown = (
  nodes: Descendant[],
  options: ComposerMarkdownOptions = {}
): ComposerMarkdown => {
  const { draft = false, literal = false } = options;
  const fragments: string[] = [];
  const fragment = (html: string): string => {
    fragments.push(html);
    return `${PLACEHOLDER_OPEN}${fragments.length - 1}${PLACEHOLDER_CLOSE}`;
  };

  const inline = (node: Descendant): string => {
    if (Text.isText(node)) {
      const text = node.text.replace(RESERVED_CHARS, '');
      if (!text) return '';
      // Marks markdown has no syntax for travel as HTML.
      if (!draft && (node.underline || node.spoiler)) {
        return fragment(toMatrixCustomHTML({ ...node, text }, { allowTextFormatting: true }));
      }
      let out = node.code ? codeSpan(text) : literal ? escapeInline(text) : text;
      if (node.strikeThrough) out = `~~${out}~~`;
      if (node.italic) out = `_${out}_`;
      if (node.bold) out = `**${out}**`;
      if (draft && node.underline) out = `<u>${out}</u>`;
      if (draft && node.spoiler) out = `<span data-mx-spoiler>${out}</span>`;
      return out;
    }
    if (draft) {
      switch (node.type) {
        case BlockType.Mention: {
          const label = node.name.startsWith('@') ? node.name : `@${node.name}`;
          let target = node.id;
          if (node.eventId) target += `/${node.eventId}`;
          return `[${linkLabel(label)}](https://matrix.to/#/${linkTarget(encodeURI(target))})`;
        }
        case BlockType.Emoticon:
          return node.key.startsWith('mxc://') ? `:${node.shortcode}:` : node.key;
        case BlockType.Link:
          return `[${node.children.map(inline).join('')}](${linkTarget(node.href)})`;
        default:
          break;
      }
    }
    switch (node.type) {
      case BlockType.Mention:
      case BlockType.Emoticon:
      case BlockType.Link:
        return fragment(toMatrixCustomHTML(node, { allowTextFormatting: true }));
      case BlockType.Command:
        return `/${node.command}`;
      default:
        return node.children.map(inline).join('');
    }
  };

  const block = (node: Descendant, indent: string): string => {
    if (Text.isText(node) || !Element.isElement(node)) return `${indent}${inline(node)}\n`;
    switch (node.type) {
      case BlockType.Paragraph: {
        const text = node.children.map(inline).join('');
        return `${indent}${literal ? escapeLineStart(text) : text}\n`;
      }
      case BlockType.Heading:
        return `${indent}${'#'.repeat(node.level)} ${node.children.map(inline).join('')}\n`;
      case BlockType.CodeBlock: {
        const body = node.children
          .map((line) => (Element.isElement(line) ? line.children : [line]))
          .map((parts) =>
            parts
              .map((part) => (Text.isText(part) ? part.text.replace(RESERVED_CHARS, '') : ''))
              .join('')
          )
          .join('\n');
        const fence = '`'.repeat(Math.max(3, longestRun(body, '`') + 1));
        return `${indent}${fence}\n${body}\n${indent}${fence}\n`;
      }
      case BlockType.BlockQuote:
        return `${node.children
          .map(
            (line) =>
              `${indent}> ${Element.isElement(line) ? line.children.map(inline).join('') : inline(line)}\n`
          )
          .join('')}\n`;
      case BlockType.OrderedList:
      case BlockType.UnorderedList: {
        const ordered = node.type === BlockType.OrderedList;
        const start =
          ordered && Number.isSafeInteger(node.start) && node.start && node.start > 1
            ? node.start
            : 1;
        const items = node.children.map((item, index) => {
          const marker = ordered ? `${start + index}. ` : '- ';
          const nestedIndent = `${indent}${' '.repeat(marker.length)}`;
          const children = Element.isElement(item) ? item.children : [item];
          const text = children
            .filter((child) => !isList(child))
            .map((child) =>
              Element.isElement(child) && child.type === BlockType.Paragraph
                ? child.children.map(inline).join('')
                : inline(child)
            )
            .join('');
          const nested = children
            .filter(isList)
            .map((child) => block(child, nestedIndent))
            .join('');
          return `${indent}${marker}${text}\n${nested}`;
        });
        return `${items.join('')}\n`;
      }
      default:
        return `${indent}${node.children.map(inline).join('')}\n`;
    }
  };

  const source = nodes
    .map((node) => block(node, ''))
    .join('')
    .replace(/\n+$/, '');
  return { source, fragments };
};

/**
 * Formatted body for a markdown-enabled message, rendered by Core. Plain text
 * (or Core being unavailable) falls back to the editor's non-markdown HTML,
 * which callers compare with the plain body to omit `formatted_body`.
 */
export const renderComposerHtml = async (nodes: Descendant[]): Promise<string> => {
  const fallback = toMatrixCustomHTML(nodes, { allowTextFormatting: true });
  const { source, fragments } = toComposerMarkdown(nodes);
  try {
    const result = await invokeDesktopWithAvailability<string | null>('desktop_render_markdown', {
      source,
      fragments,
    });
    if (result.available && typeof result.value === 'string') return result.value;
  } catch {
    // Core could not format the message; send the editor's own HTML.
  }
  return fallback;
};

/**
 * A draft as Markdown for the account's other devices (iOS keeps drafts as
 * Markdown text). Desktop keeps the rich value alongside it.
 */
export const toDraftMarkdown = (nodes: Descendant[], isMarkdown: boolean): string =>
  toComposerMarkdown(nodes, { draft: true, literal: !isMarkdown }).source;

/**
 * Editor content for Markdown from another device. With Markdown on the
 * composer holds Markdown source, so the text goes in as typed; with it off
 * Core renders it first, so formatting shows as formatting and is not sent
 * as literal asterisks.
 */
export const draftMarkdownToEditorInput = async (
  markdown: string,
  isMarkdown: boolean
): Promise<Descendant[]> => {
  const lines = plainToEditorInput(markdown, false);
  if (isMarkdown || !markdown.trim()) return lines;
  try {
    const result = await invokeDesktopWithAvailability<string | null>('desktop_render_markdown', {
      source: markdown,
      fragments: [],
    });
    if (result.available && typeof result.value === 'string') {
      return htmlToEditorInput(result.value);
    }
  } catch {
    // Core could not render it; the text still arrives.
  }
  return lines;
};
