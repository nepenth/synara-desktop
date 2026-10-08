import { MatchResult, MatchRule, replaceMatch } from './internal';

/**
 * Markdown escape sequences the editor writes and strips.
 *
 * Markdown itself is rendered by Core (`render_composer_markdown`); the editor
 * only needs to know which characters it escaped while the user typed, so the
 * plain-text body and the editor's own input handling stay consistent.
 */

const URL_NEG_LB = '(?<!(https?|ftp|mailto|magnet):\\/\\/\\S*)';

export const UN_ESC_BLOCK_SEQ = /^\\*(#{1,6} +|```|>|(-|[\da-zA-Z]\.) +|\* +)/;
export const ESC_BLOCK_SEQ = /^\\(\\*(#{1,6} +|```|>|(-|[\da-zA-Z]\.) +|\* +))/;

export const INLINE_SEQUENCE_SET = '[*_~`|]';
export const CAP_INLINE_SEQ = `${URL_NEG_LB}${INLINE_SEQUENCE_SET}`;
const ESC_REG = new RegExp(`${URL_NEG_LB}\\\\(${INLINE_SEQUENCE_SET})`);

type EscapeRule = {
  match: MatchRule;
  replacement: (match: MatchResult) => string;
};

const ESCAPE_RULE: EscapeRule = {
  match: (text) => text.match(ESC_REG),
  replacement: (match) => match[2] ?? '',
};

/** Apply the inline escape rule once, recursing on the text around it. */
export const runEscapeRule = (
  text: string,
  recurse: (part: string) => string
): string | undefined => {
  const match = ESCAPE_RULE.match(text);
  if (!match) return undefined;
  return replaceMatch(text, match, ESCAPE_RULE.replacement(match), (part) => [recurse(part)]).join(
    ''
  );
};
