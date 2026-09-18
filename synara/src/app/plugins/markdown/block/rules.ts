import { BlockMDRule } from './type';

const HEADING_REG_1 = /^(#{1,6}) +(.+)\n?/m;
export const HeadingRule: BlockMDRule = {
  match: (text) => text.match(HEADING_REG_1),
  html: (match, parseInline) => {
    const [, g1, g2] = match;
    const level = g1.length;
    return `<h${level} data-md="${g1}">${parseInline ? parseInline(g2) : g2}</h${level}>`;
  },
};

const CODEBLOCK_MD_1 = '```';
const CODEBLOCK_REG_1 = /^`{3}(\S*)\n((?:.*\n)+?)`{3} *(?!.)\n?/m;
export const CodeBlockRule: BlockMDRule = {
  match: (text) => text.match(CODEBLOCK_REG_1),
  html: (match) => {
    const [, g1, g2] = match;
    // use last identifier after dot, e.g. for "example.json" gets us "json" as language code.
    const langCode = g1 ? g1.substring(g1.lastIndexOf('.') + 1) : null;
    const filename = g1 !== langCode ? g1 : null;
    const classNameAtt = langCode ? ` class="language-${langCode}"` : '';
    const filenameAtt = filename ? ` data-label="${filename}"` : '';
    return `<pre data-md="${CODEBLOCK_MD_1}"><code${classNameAtt}${filenameAtt}>${g2}</code></pre>`;
  },
};

const BLOCKQUOTE_MD_1 = '>';
const QUOTE_LINE_PREFIX = /^> */;
const BLOCKQUOTE_TRAILING_NEWLINE = /\n$/;
const BLOCKQUOTE_REG_1 = /(^>.*\n?)+/m;
export const BlockQuoteRule: BlockMDRule = {
  match: (text) => text.match(BLOCKQUOTE_REG_1),
  html: (match, parseInline) => {
    const [blockquoteText] = match;

    const lines = blockquoteText
      .replace(BLOCKQUOTE_TRAILING_NEWLINE, '')
      .split('\n')
      .map((lineText) => {
        const line = lineText.replace(QUOTE_LINE_PREFIX, '');
        if (parseInline) return `${parseInline(line)}<br/>`;
        return `${line}<br/>`;
      })
      .join('');
    return `<blockquote data-md="${BLOCKQUOTE_MD_1}">${lines}</blockquote>`;
  },
};

const listItemHtml = (line: string, parseInline?: (txt: string) => string): string => {
  const task = line.match(/^\[([ xX])\]\s+(.*)$/);
  if (task) {
    const marker = task[1] === ' ' ? '[ ]' : '[x]';
    const txt = parseInline ? parseInline(task[2]) : task[2];
    return `<li><p>${marker} ${txt}</p></li>`;
  }
  const txt = parseInline ? parseInline(line) : line;
  return `<li><p>${txt}</p></li>`;
};

const ORDERED_LIST_MD_1 = '1';
const O_LIST_ITEM_PREFIX = /^(\d+\.|[a-zA-Z]\.) */;
const O_LIST_START = /^(\d+)\./;
const O_LIST_TYPE = /^([aAiI])\./;
const O_LIST_TRAILING_NEWLINE = /\n$/;
const ORDERED_LIST_REG_1 = /(^(?:\d+\.|[a-zA-Z]\.) +.+\n?)+/m;
export const OrderedListRule: BlockMDRule = {
  match: (text) => text.match(ORDERED_LIST_REG_1),
  html: (match, parseInline) => {
    const [listText] = match;
    const [, listStart] = listText.match(O_LIST_START) ?? [];
    const [, listType] = listText.match(O_LIST_TYPE) ?? [];

    const lines = listText
      .replace(O_LIST_TRAILING_NEWLINE, '')
      .split('\n')
      .map((lineText) => listItemHtml(lineText.replace(O_LIST_ITEM_PREFIX, ''), parseInline))
      .join('');

    const dataMdAtt = `data-md="${listType || listStart || ORDERED_LIST_MD_1}"`;
    const startAtt = listStart ? ` start="${listStart}"` : '';
    const typeAtt = listType ? ` type="${listType}"` : '';
    return `<ol ${dataMdAtt}${startAtt}${typeAtt}>${lines}</ol>`;
  },
};

const U_LIST_ITEM_PREFIX = /^(?:\*|-)\s+/;
const U_LIST_TRAILING_NEWLINE = /\n$/;
const UNORDERED_LIST_REG_1 = /(^(?:\*|-)\s+.+\n?)+/m;
export const UnorderedListRule: BlockMDRule = {
  match: (text) => text.match(UNORDERED_LIST_REG_1),
  html: (match, parseInline) => {
    const [listText] = match;
    const bullet = listText.match(/^(?:\*|-)/)?.[0] ?? '*';

    const lines = listText
      .replace(U_LIST_TRAILING_NEWLINE, '')
      .split('\n')
      .map((lineText) => listItemHtml(lineText.replace(U_LIST_ITEM_PREFIX, ''), parseInline))
      .join('');

    return `<ul data-md="${bullet}">${lines}</ul>`;
  },
};

export const UN_ESC_BLOCK_SEQ = /^\\*(#{1,6} +|```|>|(-|[\da-zA-Z]\.) +|\* +)/;
export const ESC_BLOCK_SEQ = /^\\(\\*(#{1,6} +|```|>|(-|[\da-zA-Z]\.) +|\* +))/;

const HR_MD = '---';
const HR_REG = /^(?: {0,3}(?:-{3,}|\*{3,}|_{3,}) *)\n?/m;
export const HrRule: BlockMDRule = {
  match: (text) => text.match(HR_REG),
  html: () => `<hr data-md="${HR_MD}"/>`,
};

const TABLE_SEPARATOR_CELL = /^\s*:?-{3,}:?\s*$/;
const TABLE_REG = /((?:^\|.+\|\s*\n)*(?:^\|.+\|\s*\n?))/m;
const splitTableRow = (line: string): string[] =>
  line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim());

export const TableRule: BlockMDRule = {
  match: (text) => {
    const match = text.match(TABLE_REG);
    if (!match) return null;
    const lines = match[0]
      .replace(/\n$/, '')
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.startsWith('|'));
    return lines.length > 0 ? match : null;
  },
  html: (match, parseInline) => {
    const [tableText] = match;
    const lines = tableText
      .replace(/\n$/, '')
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.startsWith('|'));
    if (lines.length === 0) return tableText;
    const header = splitTableRow(lines[0]);
    const maybeSeparator = lines.length > 1 ? splitTableRow(lines[1]) : [];
    const hasSeparator =
      maybeSeparator.length > 0 && maybeSeparator.every((cell) => TABLE_SEPARATOR_CELL.test(cell));
    const bodyLines = lines.slice(hasSeparator ? 2 : 1);
    const headerHtml = header
      .map((cell) => `<th>${parseInline ? parseInline(cell) : cell}</th>`)
      .join('');
    const bodyHtml = bodyLines
      .map((line) => {
        const cells = splitTableRow(line);
        while (cells.length < header.length) cells.push('');
        const row = cells
          .slice(0, header.length)
          .map((cell) => `<td>${parseInline ? parseInline(cell) : cell}</td>`)
          .join('');
        return `<tr>${row}</tr>`;
      })
      .join('');
    return `<table data-md="table"><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table>`;
  },
};
