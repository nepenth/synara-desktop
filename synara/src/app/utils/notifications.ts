import { invokeDesktopWithAvailability, isSynaraDesktop } from './desktop';
import { setRoomReadStateWithNativeOwner } from './nativeRoomReadStateOwner';
import { bulkFailureMessage, runBulkAction } from './bulkActions';

/** Mark a room unread through the native Core owner (desktop shell only). */
export async function markAsUnread(roomId: string): Promise<void> {
  if (typeof window === 'undefined' || !isSynaraDesktop()) return;
  await setRoomReadStateWithNativeOwner(roomId, 'mark_unread', true, invokeDesktopWithAvailability);
}

/**
 * Mark a room read through the native Core owner. Core writes the fully-read
 * marker and the private read receipt for the newest receipt-capable event; the
 * renderer no longer resolves read targets itself. Only the desktop shell ships
 * this renderer, so a missing native owner fails closed instead of guessing.
 */
export async function markAsRead(roomId: string): Promise<void> {
  if (typeof window === 'undefined' || !isSynaraDesktop()) {
    throw new Error('Mark as read requires the Synara desktop app.');
  }
  await setRoomReadStateWithNativeOwner(roomId, 'mark_read', true, invokeDesktopWithAvailability);
}

/**
 * Starts a read-marker update from UI code that cannot await it. The core
 * markAsRead API remains awaitable so workflows and tests can observe failures.
 */
export function markAsReadInBackground(roomId: string): void {
  void markAsRead(roomId).catch(() => undefined);
}

/**
 * Executes an explicit user-facing Mark Read command. It always uses the
 * private receipt channel plus Matrix's fully-read marker in native Core.
 */
export async function markAsReadFromExplicitUserAction(roomId: string): Promise<void> {
  await markAsRead(roomId);
}

export function markAsReadFromExplicitUserActionInBackground(roomId: string): void {
  void markAsReadFromExplicitUserAction(roomId).catch(() => undefined);
}

/**
 * Mark every room read (a space, Home or Direct "Mark as Read"). One failure
 * does not stop the rest. Resolves to user copy naming the rooms that are
 * still unread, or `undefined` when every room was marked.
 */
export async function markRoomsAsReadFromExplicitUserAction(
  roomIds: readonly string[],
  roomLabel: (roomId: string) => string,
  markRead: (roomId: string) => Promise<void> = markAsReadFromExplicitUserAction
): Promise<string | undefined> {
  const failures = await runBulkAction(roomIds, markRead);
  if (failures.length === 0) return undefined;
  return bulkFailureMessage(
    roomIds.length,
    failures.map(({ item }) => roomLabel(item)),
    { done: 'Marked', verb: 'mark', noun: 'rooms as read' }
  );
}
