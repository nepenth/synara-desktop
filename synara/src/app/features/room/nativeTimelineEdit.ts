/**
 * Pure rules for editing a timeline message in place with the Slate composer.
 * The save goes through `matrix_edit_message`, which keeps the msgtype and
 * `m.mentions` (`synara_core::app::send::edit_message_content`).
 */

export type NativeEditMsgType = 'm.text' | 'm.emote' | 'm.notice';

/** Core's row `messageType` (`text`, `emote`, `notice`) as a Matrix msgtype. */
export const nativeEditMsgType = (messageType: string | undefined): NativeEditMsgType => {
  if (messageType === 'emote') return 'm.emote';
  if (messageType === 'notice') return 'm.notice';
  return 'm.text';
};

/** Only text messages are edited with the composer; media captions are not. */
export const isNativeTextEditable = (messageType: string | undefined): boolean =>
  messageType === undefined ||
  messageType === 'text' ||
  messageType === 'emote' ||
  messageType === 'notice';

const trimReplyFallback = (html: string): string => {
  const suffix = '</mx-reply>';
  const end = html.lastIndexOf(suffix);
  return (end < 0 ? html : html.slice(end + suffix.length)).trim();
};

/** The HTML the editor starts from: the message's own HTML without a reply fallback. */
export const nativeEditInitialHtml = (formattedBody: string | undefined): string | undefined => {
  const html = formattedBody ? trimReplyFallback(formattedBody) : '';
  return html || undefined;
};

/**
 * Nothing to send: the text and its formatting are what the message already
 * has. Saving then just closes the editor instead of posting an empty edit.
 */
export const isNativeEditUnchanged = (
  original: { body: string; formattedBody?: string },
  next: { body: string; html: string; htmlDiffers: boolean }
): boolean => {
  if (next.body !== original.body.trim()) return false;
  const originalHtml = nativeEditInitialHtml(original.formattedBody);
  if (originalHtml) return originalHtml === next.html;
  return !next.htmlDiffers;
};

/** A draft kept while its row scrolls out of the virtual window. */
const drafts = new Map<string, unknown>();
const DRAFT_LIMIT = 20;

const draftKey = (roomId: string, eventId: string) => `${roomId}\u0000${eventId}`;

export const takeNativeEditDraft = <T>(roomId: string, eventId: string): T | undefined => {
  const key = draftKey(roomId, eventId);
  const draft = drafts.get(key) as T | undefined;
  drafts.delete(key);
  return draft;
};

export const keepNativeEditDraft = (roomId: string, eventId: string, draft: unknown): void => {
  const key = draftKey(roomId, eventId);
  drafts.delete(key);
  drafts.set(key, draft);
  if (drafts.size > DRAFT_LIMIT) {
    const oldest = drafts.keys().next().value;
    if (oldest !== undefined) drafts.delete(oldest);
  }
};

export const dropNativeEditDraft = (roomId: string, eventId: string): void => {
  drafts.delete(draftKey(roomId, eventId));
};
