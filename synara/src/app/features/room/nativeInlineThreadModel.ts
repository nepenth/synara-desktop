import type { NativeTimelineViewRow } from './nativeTimelineView';

export type InlineThreadReply = {
  key: string;
  senderName: string;
  body: string;
  originServerTs?: number;
};

const replyText = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'message') return row.body;
  if (row.kind === 'poll') return 'Poll';
  if (row.kind === 'sticker') return 'Sticker';
  return 'Event';
};

/** Thread rows without the root, in timeline order. Only message-like rows. */
export const inlineThreadReplies = (
  rows: readonly NativeTimelineViewRow[],
  rootEventId: string
): InlineThreadReply[] =>
  rows.flatMap((row) => {
    if (row.kind !== 'message' && row.kind !== 'poll' && row.kind !== 'sticker') return [];
    const event = row.kind === 'sticker' ? row.event : row;
    if (event.eventId === rootEventId) return [];
    return [
      {
        key: event.itemId,
        senderName: event.senderName,
        body: replyText(row),
        originServerTs: event.originServerTs,
      },
    ];
  });
