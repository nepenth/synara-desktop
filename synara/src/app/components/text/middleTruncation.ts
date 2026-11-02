/** Characters kept visible at the end of a middle-truncated name. */
export const MIDDLE_TRUNCATION_TAIL = 7;

/** Names this long or shorter render as one span (plain end truncation). */
export const MIDDLE_TRUNCATION_MIN_LENGTH = 18;

export type MiddleTruncationParts = { start: string; end: string };

/**
 * Split `text` for middle truncation: the start may ellipsize, the last
 * `tail` characters always stay visible ("Project - Ho…agement"). Counts code
 * points, so an emoji or astral character is never cut in half. Returns
 * `undefined` when the text is short enough to render as one span.
 */
export const splitForMiddleTruncation = (
  text: string,
  tail: number = MIDDLE_TRUNCATION_TAIL,
  minLength: number = MIDDLE_TRUNCATION_MIN_LENGTH
): MiddleTruncationParts | undefined => {
  const chars = Array.from(text);
  if (tail <= 0 || chars.length <= minLength || chars.length <= tail) return undefined;
  return {
    start: chars.slice(0, chars.length - tail).join(''),
    end: chars.slice(chars.length - tail).join(''),
  };
};
