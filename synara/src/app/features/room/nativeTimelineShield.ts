/**
 * Authenticity shields for timeline rows in encrypted rooms.
 *
 * Core copies the SDK's closed shield tone and code onto each message-like row.
 * Anything outside those closed sets is ignored rather than rendered.
 */

export type NativeTimelineShieldTone = 'red' | 'grey';

export type NativeTimelineShieldCode =
  | 'authenticity_not_guaranteed'
  | 'unknown_device'
  | 'unsigned_device'
  | 'unverified_identity'
  | 'verification_violation'
  | 'mismatched_sender'
  | 'sent_in_clear';

export type NativeTimelineEncryptionShield = {
  tone: NativeTimelineShieldTone;
  code: NativeTimelineShieldCode;
};

export type NativeTimelineShieldPresentation = {
  tone: NativeTimelineShieldTone;
  /** `unencrypted` only for plaintext in an encrypted room. */
  icon: 'shield' | 'unencrypted';
  /** Fixed copy for the tooltip and accessible name. */
  label: string;
};

const SHIELD_LABELS: Record<NativeTimelineShieldCode, string> = {
  authenticity_not_guaranteed:
    "The authenticity of this encrypted message can't be guaranteed on this device.",
  unknown_device: 'Encrypted by an unknown or deleted device.',
  unsigned_device: 'Encrypted by a device not verified by its owner.',
  unverified_identity: 'Encrypted by an unverified user.',
  verification_violation: "The sender's verified identity has changed.",
  mismatched_sender: 'The sender of this message does not match the device that encrypted it.',
  sent_in_clear: 'Not encrypted.',
};

const isShieldTone = (value: unknown): value is NativeTimelineShieldTone =>
  value === 'red' || value === 'grey';

const isShieldCode = (value: unknown): value is NativeTimelineShieldCode =>
  typeof value === 'string' && Object.prototype.hasOwnProperty.call(SHIELD_LABELS, value);

/** Presentation for a row's shield, or undefined when no shield applies. */
export const nativeTimelineShieldPresentation = (
  shield: unknown
): NativeTimelineShieldPresentation | undefined => {
  if (typeof shield !== 'object' || shield === null) return undefined;
  const { tone, code } = shield as { tone?: unknown; code?: unknown };
  if (!isShieldTone(tone) || !isShieldCode(code)) return undefined;
  return {
    tone,
    icon: code === 'sent_in_clear' ? 'unencrypted' : 'shield',
    label: SHIELD_LABELS[code],
  };
};
