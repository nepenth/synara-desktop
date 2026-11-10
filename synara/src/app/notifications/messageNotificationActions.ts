/**
 * Actions on a message notification. The desktop shell handles both in Rust
 * (ids and kind are mirrored in `desktop_notification_responses.rs`): Mark as
 * read never reaches the renderer; Reply reaches it only when the banner
 * could not collect text, to open the room with the composer focused.
 */
export const MESSAGE_NOTIFICATION_KIND = 'message';
export const MESSAGE_NOTIFICATION_ACTION_MARK_READ = 'message.mark-read';
export const MESSAGE_NOTIFICATION_ACTION_REPLY = 'message.reply';

export const MESSAGE_NOTIFICATION_ACTIONS: { id: string; label: string }[] = [
  { id: MESSAGE_NOTIFICATION_ACTION_MARK_READ, label: 'Mark as read' },
  { id: MESSAGE_NOTIFICATION_ACTION_REPLY, label: 'Reply' },
];

/** The room a Reply action asks to open, or `undefined` if it names none. */
export function messageReplyActionRoom(payload: {
  actionId: string;
  context?: { kind: string; roomId?: string };
}): string | undefined {
  if (payload.actionId !== MESSAGE_NOTIFICATION_ACTION_REPLY) return undefined;
  if (payload.context?.kind !== MESSAGE_NOTIFICATION_KIND) return undefined;
  const roomId = payload.context.roomId?.trim();
  return roomId && roomId.startsWith('!') ? roomId : undefined;
}
