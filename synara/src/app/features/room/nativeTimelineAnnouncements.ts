import type { NativeTimelineViewRow } from './nativeTimelineView';

const MAX_ANNOUNCED_MESSAGES = 3;
const MAX_ANNOUNCED_CHARS = 160;

type AnnouncedRow = {
  eventId: string;
  senderId?: string;
  senderName: string;
  /** The message text, or an action such as "sent an image" when `action` is set. */
  text: string;
  action?: boolean;
};

const announcedRow = (row: NativeTimelineViewRow): AnnouncedRow | undefined => {
  if (row.kind === 'message' && row.eventId) {
    const media =
      row.messageType === 'image' || row.messageType === 'video' || row.messageType === 'file'
        ? `sent ${row.messageType === 'image' ? 'an image' : `a ${row.messageType}`}`
        : undefined;
    return {
      eventId: row.eventId,
      senderId: row.senderId,
      senderName: row.senderName || row.senderId,
      text: media ?? row.body,
      action: media !== undefined,
    };
  }
  if (row.kind === 'sticker' && row.event.eventId) {
    return {
      eventId: row.event.eventId,
      senderId: row.event.senderId,
      senderName: row.event.senderName || row.event.senderId,
      text: 'sent a sticker',
      action: true,
    };
  }
  if (row.kind === 'poll' && row.eventId) {
    return {
      eventId: row.eventId,
      senderId: row.senderId,
      senderName: row.senderName || row.senderId,
      text: `started a poll: ${row.question}`,
      action: true,
    };
  }
  return undefined;
};

const clip = (text: string): string => {
  const flat = text.replace(/\s+/g, ' ').trim();
  return flat.length > MAX_ANNOUNCED_CHARS ? `${flat.slice(0, MAX_ANNOUNCED_CHARS - 1)}…` : flat;
};

export type NativeTimelineArrivals = {
  /** The newest announceable event; the next call compares against it. */
  tailEventId?: string;
  /** What a screen reader should hear, or undefined when nothing new arrived. */
  announcement?: string;
};

/**
 * Messages from other people that arrived after `previousTailEventId`, as
 * one polite announcement. The first snapshot, history loading, and a window
 * that no longer contains the previous tail (a jump) announce nothing.
 */
export const nativeTimelineArrivals = (
  previousTailEventId: string | undefined,
  rows: readonly NativeTimelineViewRow[],
  ownUserId: string | undefined
): NativeTimelineArrivals => {
  const announced = rows.map(announcedRow).filter((row): row is AnnouncedRow => !!row);
  const tailEventId = announced.at(-1)?.eventId;
  if (!previousTailEventId || tailEventId === previousTailEventId) return { tailEventId };
  const previousIndex = announced.findIndex((row) => row.eventId === previousTailEventId);
  if (previousIndex < 0) return { tailEventId };
  const fresh = announced
    .slice(previousIndex + 1)
    .filter((row) => row.senderId !== ownUserId)
    .slice(-MAX_ANNOUNCED_MESSAGES);
  if (fresh.length === 0) return { tailEventId };
  return {
    tailEventId,
    announcement: fresh
      .map((row) =>
        row.action ? `${row.senderName} ${clip(row.text)}` : `${row.senderName}: ${clip(row.text)}`
      )
      .join('. '),
  };
};
