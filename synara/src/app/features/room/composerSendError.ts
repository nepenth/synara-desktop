import {
  BulkActionError,
  bulkFailureMessage,
  type BulkActionFailure,
  type BulkFailureCopy,
} from '../../utils/bulkActions';

/**
 * An error whose message is already user copy: it says what went wrong and
 * what to do next. The composer shows it as is.
 */
export class ComposerUserError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ComposerUserError';
  }
}

export const COMPOSER_NOT_SIGNED_IN =
  'Synara is not signed in to your Matrix account. Sign in again, then send your message.';
const NOT_SIGNED_IN = COMPOSER_NOT_SIGNED_IN;
const ROOM_UNAVAILABLE =
  'This room is not available any more. You may have left it or been removed.';
const QUEUE_FAILED = 'Synara could not queue the message. Check your connection and try again.';
const ENCRYPTION_FAILED =
  'Synara could not encrypt the message. Try again; if it keeps failing, restart Synara.';
/**
 * Core names the cause of an unrecoverable send after this prefix
 * (`d0.4-send-queue-wedged-too-large`, `…-forbidden`, …).
 */
const WEDGED = 'd0.4-send-queue-wedged';
const WEDGED_ENCRYPTION =
  'Synara did not send this message because a verified contact in this room has an unverified session or changed identity. Review it in the room, then retry or discard the failed message in the timeline.';

/** Send diagnostics from Core (`crates/synara-core/src/app/send`) mapped to user copy. */
const SEND_DIAGNOSTIC_MESSAGES: Record<string, string> = {
  'd0.4-send-requires-session': NOT_SIGNED_IN,
  'p2-send-text-no-session': NOT_SIGNED_IN,
  'p2-send-poll-no-session': NOT_SIGNED_IN,
  'p2-send-room-attachment-no-session': NOT_SIGNED_IN,
  'd0.4-send-sdk-auth-required': NOT_SIGNED_IN,
  'd0.4-send-sdk-http-auth-failed':
    'Your Matrix session has expired. Sign in again, then send your message.',
  'd0.4-send-room-not-found': ROOM_UNAVAILABLE,
  'v-send.1-attachment-room-not-found': ROOM_UNAVAILABLE,
  'd0.4-send-sdk-wrong-room-state':
    'You can not send here because you are no longer joined to this room.',
  'd0.4-send-text-payload-too-large':
    'This message is too long to send. Shorten it or split it into several messages.',
  'v-send.1-attachment-too-large': "This file is larger than your homeserver's upload limit.",
  'v-send.1-attachment-empty': 'This file is empty, so there is nothing to send.',
  'v-send.1-attachment-invalid-mime': 'This file type can not be sent.',
  'v-send.1-attachment-invalid-filename': 'This file name can not be sent. Rename the file first.',
  'v-send.1-attachment-sdk-failed': 'Synara could not prepare this file for sending.',
  // An unrecoverable send stays in the timeline as a failed message with
  // Retry and Discard. The composer gets the text back so nothing is lost.
  [WEDGED]:
    'Your homeserver refused this message. Your text is back in the composer; discard the failed copy in the timeline before you send it again.',
  [`${WEDGED}-too-large`]:
    'Your homeserver refused this message because it is too large. Discard the failed copy in the timeline, then send something smaller.',
  [`${WEDGED}-forbidden`]:
    'Your homeserver refused this message: you do not have permission to send it here. Discard the failed copy in the timeline.',
  [`${WEDGED}-not-joined`]:
    'You can not send here because you are no longer joined to this room. Discard the failed copy in the timeline.',
  [`${WEDGED}-insecure-devices`]: WEDGED_ENCRYPTION,
  [`${WEDGED}-identity-violation`]: WEDGED_ENCRYPTION,
  [`${WEDGED}-verification-required`]:
    'Verify this session before sending to this room. Then retry the failed message in the timeline.',
  'd0.4-send-queue-cancelled':
    'The message was discarded before it was sent. Your text is back in the composer.',
  'd0.4-send-queue-timeout': QUEUE_FAILED,
  'd0.4-send-queue-closed': QUEUE_FAILED,
  'd0.4-send-queue-subscribe-failed': QUEUE_FAILED,
  'd0.4-send-sdk-http-network-failed':
    'Synara could not reach your homeserver. Check your connection and try again.',
  'd0.4-send-sdk-http-rate-limited':
    'Your homeserver is limiting how fast you can send. Wait a moment and try again.',
  'd0.4-send-sdk-http-forbidden': 'You do not have permission to send messages in this room.',
  'd0.4-send-sdk-olm-failed': ENCRYPTION_FAILED,
  'd0.4-send-sdk-megolm-failed': ENCRYPTION_FAILED,
  'd0.4-send-sdk-no-olm-machine': ENCRYPTION_FAILED,
  'd0.4-send-sdk-crypto-store-failed': ENCRYPTION_FAILED,
  'd0.4-send-sdk-crypto-store-state': ENCRYPTION_FAILED,
};

/**
 * Generic diagnostic endings shared by many Core commands (`…-forbidden`,
 * `…-rate-limited`, …). Used when no exact send mapping exists, e.g. for
 * slash commands that invite, kick or join.
 */
const DIAGNOSTIC_SUFFIX_MESSAGES: [string, string][] = [
  ['-no-session', NOT_SIGNED_IN],
  ['-requires-session', NOT_SIGNED_IN],
  ['-rate-limited', 'Your homeserver is limiting requests. Wait a moment and try again.'],
  [
    '-network-failed',
    'Synara could not reach your homeserver. Check your connection and try again.',
  ],
  ['-forbidden', 'You do not have permission to do that in this room.'],
  ['-too-large', 'That is larger than your homeserver allows.'],
  ['-not-found', 'Matrix could not find that room or user.'],
];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/** The Core diagnostic id of a rejected desktop command, if it has one. */
export const coreDiagnosticId = (error: unknown): string | undefined => {
  if (!isRecord(error)) return undefined;
  const id = error.diagnosticId ?? error.diagnostic_id;
  return typeof id === 'string' && id.trim() ? id : undefined;
};

/** User copy for a Core diagnostic id, or undefined when there is no specific copy. */
export const diagnosticMessage = (diagnosticId: string): string | undefined => {
  const exact = SEND_DIAGNOSTIC_MESSAGES[diagnosticId];
  if (exact) return exact;
  // A wedge cause without its own copy reads as the generic refusal.
  if (diagnosticId.startsWith(WEDGED)) return SEND_DIAGNOSTIC_MESSAGES[WEDGED];
  return DIAGNOSTIC_SUFFIX_MESSAGES.find(([suffix]) => diagnosticId.endsWith(suffix))?.[1];
};

// Internal renderer copy ("Native Matrix … is unavailable.") names an
// implementation detail; it is replaced instead of shown.
const isInternalCopy = (message: string): boolean => /\bnative\b/i.test(message);

/**
 * A specific, actionable sentence for a composer failure. Accepts an Error,
 * a Core rejection (`{ code, message, diagnosticId }`) or a string. Unknown
 * Core failures keep their diagnostic id so it can be reported.
 */
export const composerErrorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof ComposerUserError || error instanceof BulkActionError) {
    return error.message;
  }
  const diagnosticId = coreDiagnosticId(error);
  if (diagnosticId) {
    return diagnosticMessage(diagnosticId) ?? `${fallback} (Error code: ${diagnosticId})`;
  }
  const message =
    typeof error === 'string'
      ? error
      : error instanceof Error || isRecord(error)
        ? (error as { message?: unknown }).message
        : undefined;
  if (typeof message === 'string' && message.trim() && !isInternalCopy(message)) {
    return message.trim();
  }
  return fallback;
};

export const SEND_FAILED_FALLBACK = 'Synara could not send the message. Try again.';

/** Copy for a failed text or attachment send. */
export const composerSendErrorMessage = (error: unknown): string =>
  composerErrorMessage(error, SEND_FAILED_FALLBACK);

/** Copy for a slash command that did not complete, e.g. "/invite did not finish: …". */
export const composerCommandErrorMessage = (commandName: string, error: unknown): string => {
  if (error instanceof ComposerUserError || error instanceof BulkActionError) {
    return error.message;
  }
  return `/${commandName} did not finish: ${composerErrorMessage(
    error,
    'Synara could not run the command. Try again.'
  )}`;
};

/**
 * The error for a per-user command (/invite, /kick, …) that left some users
 * unchanged: who failed and, when known, why the first one did. Undefined
 * when every user succeeded.
 */
export const commandFailureError = <T extends string>(
  users: readonly T[],
  failures: BulkActionFailure<T>[],
  copy: BulkFailureCopy
): BulkActionError<T> | undefined => {
  if (failures.length === 0) return undefined;
  const summary = bulkFailureMessage(
    users.length,
    failures.map((failure) => failure.item),
    copy
  );
  const reason = composerErrorMessage(failures[0].error, '').trim();
  return new BulkActionError(reason ? `${summary} ${reason}` : summary, failures);
};
