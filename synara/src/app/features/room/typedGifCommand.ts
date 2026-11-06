/**
 * The command a message starts with when it was typed out as text (`/kick
 * @a:b`), rather than picked from the composer's command list, or undefined when the text isn't `/name` followed by a space or the end, so a
 * path like `/usr/bin` or `//escaped` is ordinary text.
 */
export const typedCommandName = (plainText: string): string | undefined =>
  /^\/([a-z][a-z0-9_-]*)(?=\s|$)/.exec(plainText)?.[1];

/**
 * Whether the message starts with `/gif` or `/gif <search>` typed as text,
 * rather than the command picked from the composer's command list.
 */
export const isTypedGifCommand = (plainText: string): boolean =>
  typedCommandName(plainText) === 'gif';
