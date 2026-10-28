/**
 * Whether the message starts with `/gif` or `/gif <search>` typed as text,
 * rather than the command picked from the composer's command list.
 */
export const isTypedGifCommand = (plainText: string): boolean => /^\/gif(?=\s|$)/.test(plainText);
