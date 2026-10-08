import { Descendant, Editor, Element, Text } from 'slate';
import type { MatrixClientReading } from '../../utils/room';
import { sanitizeText } from '../../utils/sanitize';
import { BlockType } from './types';
import { CustomElement } from './slate';
import {
  unescapeMarkdownBlockSequences,
  unescapeMarkdownInlineSequences,
} from '../../plugins/markdown';
import { sanitizeForRegex } from '../../utils/regex';
import { isUserId } from '../../utils/matrix';

/**
 * Options for the editor's own HTML. Markdown is rendered by Core instead
 * (see composerMarkdown.ts), so this builder only maps editor formatting.
 */
export type OutputOptions = {
  allowTextFormatting?: boolean;
};

const textToCustomHtml = (node: Text, opts: OutputOptions): string => {
  let string = sanitizeText(node.text);
  if (opts.allowTextFormatting) {
    if (node.bold) string = `<strong>${string}</strong>`;
    if (node.italic) string = `<i>${string}</i>`;
    if (node.underline) string = `<u>${string}</u>`;
    if (node.strikeThrough) string = `<s>${string}</s>`;
    if (node.code) string = `<code>${string}</code>`;
    if (node.spoiler) string = `<span data-mx-spoiler>${string}</span>`;
  }

  return string;
};

const elementToCustomHtml = (node: CustomElement, children: string): string => {
  switch (node.type) {
    case BlockType.Paragraph:
      return `${children}<br/>`;
    case BlockType.Heading:
      return `<h${node.level}>${children}</h${node.level}>`;
    case BlockType.CodeLine:
      return `${children}\n`;
    case BlockType.CodeBlock:
      return `<pre><code>${children}</code></pre>`;
    case BlockType.QuoteLine:
      return `${children}<br/>`;
    case BlockType.BlockQuote:
      return `<blockquote>${children}</blockquote>`;
    case BlockType.ListItem:
      return `<li><p>${children}</p></li>`;
    case BlockType.OrderedList: {
      const start =
        Number.isSafeInteger(node.start) && node.start && node.start > 1 ? node.start : undefined;
      const startAttribute = start ? ` start="${start}"` : '';
      return `<ol${startAttribute}>${children}</ol>`;
    }
    case BlockType.UnorderedList:
      return `<ul>${children}</ul>`;

    case BlockType.Mention: {
      let fragment = node.id;

      if (node.eventId) {
        fragment += `/${node.eventId}`;
      }
      if (node.viaServers && node.viaServers.length > 0) {
        fragment += `?${node.viaServers.map((server) => `via=${server}`).join('&')}`;
      }

      const matrixTo = `https://matrix.to/#/${fragment}`;
      return `<a href="${encodeURI(matrixTo)}">${sanitizeText(node.name)}</a>`;
    }
    case BlockType.Emoticon:
      return node.key.startsWith('mxc://')
        ? `<img data-mx-emoticon src="${node.key}" alt="${sanitizeText(
            node.shortcode
          )}" title="${sanitizeText(node.shortcode)}" height="32" />`
        : sanitizeText(node.key);
    case BlockType.Link:
      return `<a href="${sanitizeText(encodeURI(node.href))}">${children}</a>`;
    case BlockType.Command:
      return `/${sanitizeText(node.command)}`;
    default:
      return children;
  }
};

const isListElement = (node: Descendant): boolean =>
  Element.isElement(node) &&
  (node.type === BlockType.OrderedList || node.type === BlockType.UnorderedList);

const EDITOR_METADATA_ATTR_REG_G = /\sdata-md(?:=(?:"[^"]*"|'[^']*'|[^\s>]+))?/g;

export const stripEditorMetadataFromCustomHtml = (customHtml: string): string =>
  customHtml.replace(EDITOR_METADATA_ATTR_REG_G, '');

const toMatrixCustomHTMLInternal = (
  node: Descendant | Descendant[],
  opts: OutputOptions
): string => {
  const parseNode = (n: Descendant) => toMatrixCustomHTMLInternal(n, opts);
  if (Array.isArray(node)) return node.map(parseNode).join('');
  if (Text.isText(node)) return textToCustomHtml(node, opts);

  if (node.type === BlockType.ListItem) {
    let html = '';
    let inlineChildren: Descendant[] = [];
    const flushInlineChildren = () => {
      if (inlineChildren.length === 0) return;
      html += `<p>${inlineChildren.map(parseNode).join('')}</p>`;
      inlineChildren = [];
    };

    node.children.forEach((child) => {
      if (isListElement(child)) {
        flushInlineChildren();
        html += parseNode(child);
        return;
      }
      if (Element.isElement(child) && child.type === BlockType.Paragraph) {
        flushInlineChildren();
        html += `<p>${child.children.map(parseNode).join('')}</p>`;
        return;
      }
      inlineChildren.push(child);
    });
    flushInlineChildren();
    return `<li>${html || '<p></p>'}</li>`;
  }

  const children = node.children.map(parseNode).join('');
  return elementToCustomHtml(node, children);
};

export const toMatrixCustomHTML = (node: Descendant | Descendant[], opts: OutputOptions): string =>
  stripEditorMetadataFromCustomHtml(toMatrixCustomHTMLInternal(node, opts));

const elementToPlainText = (node: CustomElement, children: string): string => {
  switch (node.type) {
    case BlockType.Paragraph:
      return `${children}\n`;
    case BlockType.Heading:
      return `${children}\n`;
    case BlockType.CodeLine:
      return `${children}\n`;
    case BlockType.CodeBlock:
      return `${children}\n`;
    case BlockType.QuoteLine:
      return `| ${children}\n`;
    case BlockType.BlockQuote:
      return `${children}\n`;
    case BlockType.ListItem:
      return `- ${children}\n`;
    case BlockType.OrderedList:
      return `${children}\n`;
    case BlockType.UnorderedList:
      return `${children}\n`;
    case BlockType.Mention:
      return node.id;
    case BlockType.Emoticon:
      return node.key.startsWith('mxc://') ? `:${node.shortcode}:` : node.key;
    case BlockType.Link:
      return `[${children}](${node.href})`;
    case BlockType.Command:
      return `/${node.command}`;
    default:
      return children;
  }
};

export const toPlainText = (node: Descendant | Descendant[], isMarkdown: boolean): string => {
  if (Array.isArray(node)) return node.map((n) => toPlainText(n, isMarkdown)).join('');
  if (Text.isText(node)) {
    if (node.spoiler) return '[spoiler]';
    return isMarkdown
      ? unescapeMarkdownBlockSequences(node.text, unescapeMarkdownInlineSequences)
      : node.text;
  }

  if (node.type === BlockType.OrderedList) {
    const start = Number.isSafeInteger(node.start) && node.start && node.start > 1 ? node.start : 1;
    return `${node.children
      .map((listItem, index) => {
        const inlineChildren = listItem.children
          .filter((n) => !isListElement(n))
          .map((n) =>
            Element.isElement(n) && n.type === BlockType.Paragraph
              ? n.children.map((child) => toPlainText(child, isMarkdown)).join('')
              : toPlainText(n, isMarkdown)
          )
          .join('');
        const nestedChildren = listItem.children
          .filter(isListElement)
          .map((n) => toPlainText(n, isMarkdown))
          .join('')
          .trimEnd()
          .split('\n')
          .filter(Boolean)
          .map((line) => `  ${line}`)
          .join('\n');
        return `${start + index}. ${inlineChildren}${
          nestedChildren ? `\n${nestedChildren}` : ''
        }\n`;
      })
      .join('')}\n`;
  }

  if (node.type === BlockType.UnorderedList) {
    return `${node.children
      .map((listItem) => {
        const inlineChildren = listItem.children
          .filter((n) => !isListElement(n))
          .map((n) =>
            Element.isElement(n) && n.type === BlockType.Paragraph
              ? n.children.map((child) => toPlainText(child, isMarkdown)).join('')
              : toPlainText(n, isMarkdown)
          )
          .join('');
        const nestedChildren = listItem.children
          .filter(isListElement)
          .map((n) => toPlainText(n, isMarkdown))
          .join('')
          .trimEnd()
          .split('\n')
          .filter(Boolean)
          .map((line) => `  ${line}`)
          .join('\n');
        return `- ${inlineChildren}${nestedChildren ? `\n${nestedChildren}` : ''}\n`;
      })
      .join('')}\n`;
  }

  const children = node.children.map((n) => toPlainText(n, isMarkdown)).join('');
  return elementToPlainText(node, children);
};

/**
 * Check if customHtml is equals to plainText
 * by replacing `<br/>` with `/n` in customHtml
 * and sanitizing plainText before comparison
 * because text are sanitized in customHtml
 * @param customHtml string
 * @param plain string
 * @returns boolean
 */
export const customHtmlEqualsPlainText = (customHtml: string, plain: string): boolean =>
  customHtml.replace(/<br\/>/g, '\n') === sanitizeText(plain);

export const trimCustomHtml = (customHtml: string) => customHtml.replace(/<br\/>$/g, '').trim();

export const trimCommand = (cmdName: string, str: string) => {
  const cmdRegX = new RegExp(`^(\\s+)?(\\/${sanitizeForRegex(cmdName)})([^\\S\n]+)?`);

  const match = str.match(cmdRegX);
  if (!match) return str;
  return str.slice(match[0].length);
};

export type MentionsData = {
  room: boolean;
  users: Set<string>;
};
export const getMentions = (
  mx: MatrixClientReading,
  roomId: string,
  editor: Editor
): MentionsData => {
  const mentionData: MentionsData = {
    room: false,
    users: new Set(),
  };

  const parseMentions = (node: Descendant): void => {
    if (Text.isText(node)) return;
    if (node.type === BlockType.CodeBlock) return;

    if (node.type === BlockType.Mention) {
      if (node.name === '@room') {
        mentionData.room = true;
      }

      if (isUserId(node.id) && node.id !== mx.getUserId()) {
        mentionData.users.add(node.id);
      }

      return;
    }

    node.children.forEach(parseMentions);
  };

  editor.children.forEach(parseMentions);

  return mentionData;
};
