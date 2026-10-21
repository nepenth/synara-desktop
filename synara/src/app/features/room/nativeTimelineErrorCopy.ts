import { composerErrorMessage, coreDiagnosticId } from './composerSendError';

const ROOM_UNAVAILABLE =
  'This room is not available any more. You may have left it or been removed.';

/**
 * Timeline open and stream diagnostics (`src-tauri/src/bridge/timeline_open.rs`)
 * mapped to user copy. Unknown ids keep their code via `composerErrorMessage`.
 */
const TIMELINE_DIAGNOSTIC_MESSAGES: Record<string, string> = {
  'd0.3-timeline-requires-session':
    'Synara is not signed in to your Matrix account. Sign in again to see messages.',
  'd0.3-timeline-room-not-found': ROOM_UNAVAILABLE,
  'v-timeline-normal-room-not-found': ROOM_UNAVAILABLE,
  'v-timeline-thread-room-not-found': ROOM_UNAVAILABLE,
  'v-timeline-thread-root-invalid': 'Synara could not find the start of this thread.',
  'v-timeline-thread-open-failed': 'Synara could not open this thread. Try again.',
  'v-timeline-view-not-open': 'This view was closed. Try again.',
};

export const TIMELINE_LOAD_FAILED = 'Synara could not load these messages. Try again.';

/** Copy for a timeline that could not be opened or kept in sync. */
export const nativeTimelineLoadErrorMessage = (error: unknown): string => {
  const diagnosticId = coreDiagnosticId(error);
  const exact = diagnosticId ? TIMELINE_DIAGNOSTIC_MESSAGES[diagnosticId] : undefined;
  return exact ?? composerErrorMessage(error, TIMELINE_LOAD_FAILED);
};

/** User copy for each row action, used when Core gives no specific reason. */
export const TIMELINE_ACTION_FAILED = {
  reply: 'Synara could not start a reply. Try again.',
  threadReply: 'Synara could not start a thread reply. Try again.',
  redact: 'Synara could not remove this message. Try again.',
  pin: 'Synara could not pin this message. Try again.',
  unpin: 'Synara could not unpin this message. Try again.',
  later: 'Synara could not save this message for later. Try again.',
  edit: 'Synara could not save your edit. Your changes are still here; try again.',
  report: 'Synara could not send the report. Try again.',
  forward: 'Synara could not forward this message. Try again.',
  reaction: 'Synara could not update the reaction. Try again.',
  callDecline: 'Synara could not decline the call. Try again.',
  pollVote: 'Synara could not send your vote. Try again.',
  download: 'Synara could not download this file. Try again.',
  latest: 'Synara could not open the latest messages. Try again.',
  lastRead: 'Synara could not open your last-read position. Try again.',
  history: 'Synara could not load more messages. Try again.',
} as const;

/**
 * A specific sentence for a failed timeline action: Core's reason when it has
 * one, otherwise `fallback`. Internal "native …" renderer copy is never shown.
 */
export const nativeTimelineActionErrorMessage = (error: unknown, fallback: string): string =>
  composerErrorMessage(error, fallback);
