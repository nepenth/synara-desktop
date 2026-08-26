export const MESSAGE_TEXT_TONES = ['soft', 'balanced', 'bright'] as const;

export type MessageTextTone = typeof MESSAGE_TEXT_TONES[number];
export type MessageTextAppearance = 'light' | 'dark';

export const DEFAULT_MESSAGE_TEXT_TONE: MessageTextTone = 'bright';

/**
 * Explicit sRGB values keep the control effective in WebKit and make the
 * three choices perceptually distinct. Every value maintains at least 7:1
 * contrast against its supported message surfaces; Bright is deliberately
 * true black/white rather than another blend of adjacent semantic grays.
 */
export const MESSAGE_TEXT_FOREGROUNDS = {
  light: {
    soft: '#484c53',
    balanced: '#24272c',
    bright: '#000000',
  },
  dark: {
    soft: '#c6c9ce',
    balanced: '#e4e6ea',
    bright: '#ffffff',
  },
} as const satisfies Record<MessageTextAppearance, Record<MessageTextTone, string>>;

export const normalizeMessageTextTone = (value: unknown): MessageTextTone =>
  typeof value === 'string' && MESSAGE_TEXT_TONES.includes(value as MessageTextTone)
    ? (value as MessageTextTone)
    : DEFAULT_MESSAGE_TEXT_TONE;

/**
 * Message prose gets its own semantic foreground so readers can tune it
 * without changing navigation, metadata, headings, syntax colors, or surfaces.
 */
export const messageTextForeground = (
  tone: MessageTextTone,
  appearance: MessageTextAppearance
): string => MESSAGE_TEXT_FOREGROUNDS[appearance][tone];
