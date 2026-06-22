import { MsgType, RelationType } from 'matrix-js-sdk/lib/@types/event';
import type { MatrixEvent } from 'matrix-js-sdk/lib/models/event';
import type { Room } from 'matrix-js-sdk/lib/models/room';
import { sanitizeCustomHtml } from './sanitize';
import { trimReplyFromBody, trimReplyFromFormattedBody } from './room';
import { getRoomCurrentState } from './timelineLifecycle';

export type ForwardSource = {
  roomId: string;
  eventId: string;
  sender?: string;
  ts?: number;
};

export const escapeHtml = (value: string): string =>
  value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');

export const stripForwardUnsafeFields = (
  content: Record<string, unknown>
): Record<string, unknown> => {
  const rest = { ...content };
  delete rest['m.relates_to'];
  delete rest['m.mentions'];
  delete rest['m.new_content'];
  delete rest['m.forwarded'];
  return rest;
};

export const makeForwardedContent = (
  content: Record<string, unknown>,
  source: ForwardSource
): Record<string, unknown> => {
  const cleanContent = stripForwardUnsafeFields(content);
  const body = typeof cleanContent.body === 'string' ? cleanContent.body : '';
  const formattedBody =
    typeof cleanContent.formatted_body === 'string' ? cleanContent.formatted_body : undefined;
  const senderLabel = source.sender ?? 'unknown sender';
  const trimmedBody = trimReplyFromBody(body);
  const safeFormattedBody = formattedBody
    ? sanitizeCustomHtml(trimReplyFromFormattedBody(formattedBody))
    : undefined;
  const fallbackFormattedBody = `<p>${escapeHtml(trimmedBody)}</p>`;

  if (
    cleanContent.msgtype === MsgType.Text ||
    cleanContent.msgtype === MsgType.Notice ||
    cleanContent.msgtype === MsgType.Emote
  ) {
    return {
      ...cleanContent,
      body: `Forwarded from ${senderLabel}\n\n${trimmedBody}`,
      format: 'org.matrix.custom.html',
      formatted_body: `<p><strong>Forwarded from ${escapeHtml(senderLabel)}</strong></p>${
        safeFormattedBody ?? fallbackFormattedBody
      }`,
      'in.synara.forwarded': source,
    };
  }

  return {
    ...cleanContent,
    body: `Forwarded from ${senderLabel}${trimmedBody ? `\n\n${trimmedBody}` : ''}`,
    'in.synara.forwarded': source,
  };
};

export const makeForwardQuoteContent = (
  content: Record<string, unknown>,
  source: ForwardSource
): Record<string, unknown> => {
  const cleanContent = stripForwardUnsafeFields(content);
  const body = typeof cleanContent.body === 'string' ? cleanContent.body : '';
  const senderLabel = source.sender ?? 'unknown sender';
  const trimmedBody = trimReplyFromBody(body);
  const quoteBody = trimmedBody
    .split('\n')
    .map((line) => `> ${line}`)
    .join('\n');

  return {
    msgtype: MsgType.Text,
    body: `Forwarded quote from ${senderLabel}\n\n${quoteBody}`,
    format: 'org.matrix.custom.html',
    formatted_body: `<p><strong>Forwarded quote from ${escapeHtml(
      senderLabel
    )}</strong></p><blockquote>${escapeHtml(trimmedBody).replace(/\n/g, '<br />')}</blockquote>`,
    'in.synara.forwarded': {
      ...source,
      quote: true,
    },
  };
};

export const getForwardableEventContent = (
  event: MatrixEvent,
  asQuote = false
): Record<string, unknown> | undefined => {
  if (event.isRedacted() || event.isRedaction()) return undefined;
  const relation = event.getRelation();
  if (
    relation?.rel_type === RelationType.Annotation ||
    relation?.rel_type === RelationType.Replace
  ) {
    return undefined;
  }
  const content = event.getContent<Record<string, unknown>>();
  if (!content || typeof content !== 'object') return undefined;
  const source = {
    roomId: event.getRoomId() ?? '',
    eventId: event.getId() ?? '',
    sender: event.getSender() ?? undefined,
    ts: event.getTs(),
  };
  return asQuote ? makeForwardQuoteContent(content, source) : makeForwardedContent(content, source);
};

export const getForwardableEventContents = (
  events: MatrixEvent[],
  asQuote = false
): Record<string, unknown>[] => {
  if (events.length === 0) return [];
  const roomId = events[0].getRoomId();
  if (!events.every((event) => event.getRoomId() === roomId)) return [];
  return events
    .map((event) => getForwardableEventContent(event, asQuote))
    .filter((content): content is Record<string, unknown> => !!content);
};

export const canSendRoomMessage = (room: Room, userId?: string | null): boolean => {
  if (!userId) return true;
  const state = getRoomCurrentState(room);
  const powerLevels = state?.getStateEvents('m.room.power_levels', '')?.getContent() as
    | {
        users?: Record<string, number>;
        users_default?: number;
        events?: Record<string, number>;
        events_default?: number;
      }
    | undefined;
  const userPower = powerLevels?.users?.[userId] ?? powerLevels?.users_default ?? 0;
  const sendPower = powerLevels?.events?.['m.room.message'] ?? powerLevels?.events_default ?? 0;
  return userPower >= sendPower;
};

export const getRoomForwardTargets = (
  rooms: Room[],
  sourceRoomId: string,
  userId?: string | null
): Room[] =>
  rooms
    .filter(
      (room) =>
        room.roomId !== sourceRoomId &&
        room.getMyMembership() === 'join' &&
        !room.isSpaceRoom() &&
        canSendRoomMessage(room, userId)
    )
    .sort((a, b) => a.name.localeCompare(b.name));
