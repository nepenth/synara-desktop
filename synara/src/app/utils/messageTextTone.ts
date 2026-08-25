export const MESSAGE_TEXT_TONES = ['soft', 'balanced', 'bright'] as const;

export type MessageTextTone = typeof MESSAGE_TEXT_TONES[number];

export const DEFAULT_MESSAGE_TEXT_TONE: MessageTextTone = 'bright';

export const normalizeMessageTextTone = (value: unknown): MessageTextTone =>
  typeof value === 'string' && MESSAGE_TEXT_TONES.includes(value as MessageTextTone)
    ? (value as MessageTextTone)
    : DEFAULT_MESSAGE_TEXT_TONE;

/**
 * Message prose gets its own semantic foreground so readers can tune it
 * without changing navigation, metadata, headings, syntax colors, or surfaces.
 */
export const messageTextForeground = (tone: MessageTextTone): string => {
  switch (tone) {
    case 'soft':
      return 'color-mix(in srgb, var(--synara-content-primary) 78%, var(--synara-content-secondary))';
    case 'balanced':
      return 'var(--synara-content-primary)';
    case 'bright':
      return 'color-mix(in srgb, var(--synara-content-primary) 72%, var(--synara-content-heading))';
  }
};
