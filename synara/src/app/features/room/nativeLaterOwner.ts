/**
 * SDK-neutral owner for `in.synara.later` account data.
 *
 * Desktop mutations go through native Tauri commands. There is no
 * matrix-js-sdk setAccountData fallback. Does not select NativeTimelinePresenter.
 */

import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';
import type { SynaraLaterContent, SynaraLaterItem } from '../../../types/matrix/accountData';

export type NativeLaterSnapshot = {
  sessionGeneration: number;
  content: SynaraLaterContent;
};

export type NativeLaterInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeLaterSnapshot>>;

const defaultInvoke: NativeLaterInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeLaterSnapshot>(command, args);

async function invokeLater(
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeLaterInvoke
): Promise<NativeLaterSnapshot> {
  const result = await invoke(command, args);
  if (!result.available || !result.value) {
    throw new Error('Native Matrix later account data is unavailable.');
  }
  return result.value;
}

export function getLaterItemId(roomId: string, eventId: string): string {
  return `${roomId}\n${eventId}`;
}

export function createLaterItemFromIds(
  roomId: string,
  eventId: string,
  kind: SynaraLaterItem['kind'],
  dueTs?: number,
  createdAt = Date.now()
): SynaraLaterItem {
  return {
    id: getLaterItemId(roomId, eventId),
    kind,
    roomId,
    eventId,
    createdAt,
    dueTs,
  };
}

export function snapshotLaterWithNativeOwner(invoke: NativeLaterInvoke = defaultInvoke) {
  return invokeLater('matrix_later_snapshot', undefined, invoke);
}

export function upsertLaterWithNativeOwner(
  item: SynaraLaterItem,
  invoke: NativeLaterInvoke = defaultInvoke
) {
  return invokeLater('matrix_later_upsert', { item }, invoke);
}

export function completeLaterWithNativeOwner(
  itemId: string,
  completedAt?: number,
  invoke: NativeLaterInvoke = defaultInvoke
) {
  return invokeLater('matrix_later_complete', { itemId, completedAt }, invoke);
}

export function snoozeLaterWithNativeOwner(
  itemId: string,
  dueTs: number,
  invoke: NativeLaterInvoke = defaultInvoke
) {
  return invokeLater('matrix_later_snooze', { itemId, dueTs }, invoke);
}

export function clearCompletedLaterWithNativeOwner(invoke: NativeLaterInvoke = defaultInvoke) {
  return invokeLater('matrix_later_clear_completed', undefined, invoke);
}

export function markLaterRemindedWithNativeOwner(
  itemId: string,
  remindedAt?: number,
  invoke: NativeLaterInvoke = defaultInvoke
) {
  return invokeLater('matrix_later_mark_reminded', { itemId, remindedAt }, invoke);
}
