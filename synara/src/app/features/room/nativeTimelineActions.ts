/**
 * SDK-neutral owners for retained timeline actions.
 *
 * Reply/rich send use `matrix_send_text` (optional `formattedBody` + `replyTo`).
 * Reactions remain on the V-SEND.2 owner. These helpers do not select
 * NativeTimelinePresenter or delete RoomTimeline.
 */

import type { DesktopInvokeResult } from '../../utils/desktop';

export const NATIVE_TIMELINE_ACTION_SCHEMA_VERSION = 1;

export type NativeTimelineActionKind =
  | 'edit_text'
  | 'redact'
  | 'forward_text'
  | 'forward_media'
  | 'report'
  | 'pin'
  | 'unpin';

export type NativeTimelineActionStatus =
  | 'sent'
  | 'redacted'
  | 'reported'
  | 'pinned'
  | 'unpinned'
  | 'already_pinned'
  | 'already_unpinned';

export type NativeTimelineActionReadback = {
  schemaVersion: number;
  action: NativeTimelineActionKind;
  roomId: string;
  eventId: string;
  status: NativeTimelineActionStatus;
};

export type NativeTimelineEditTextInput = {
  roomId: string;
  eventId: string;
  body: string;
  formattedBody?: string;
};

export type NativeTimelineRedactInput = {
  roomId: string;
  eventId: string;
  reason?: string;
};

export type NativeTimelineForwardTextInput = {
  sourceRoomId: string;
  eventId: string;
  targetRoomId: string;
  asQuote?: boolean;
};

export type NativeTimelineForwardMediaInput = {
  sourceRoomId: string;
  eventId: string;
  targetRoomId: string;
};

export type NativeTimelineReportInput = {
  roomId: string;
  eventId: string;
  reason?: string;
};

export type NativeTimelinePinInput = {
  roomId: string;
  eventId: string;
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const ACTION_STATUSES = new Set<NativeTimelineActionStatus>([
  'sent',
  'redacted',
  'reported',
  'pinned',
  'unpinned',
  'already_pinned',
  'already_unpinned',
]);

const acceptActionReadback = (
  value: unknown,
  expected: NativeTimelineActionKind
): NativeTimelineActionReadback | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const readback = value as NativeTimelineActionReadback;
  if (
    readback.schemaVersion !== NATIVE_TIMELINE_ACTION_SCHEMA_VERSION ||
    readback.action !== expected ||
    typeof readback.roomId !== 'string' ||
    typeof readback.eventId !== 'string' ||
    !ACTION_STATUSES.has(readback.status)
  ) {
    return undefined;
  }
  return readback;
};

export async function editTextWithNativeTimelineOwner(
  input: NativeTimelineEditTextInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_edit_text', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'edit_text') ?? 'unavailable';
}

export async function redactWithNativeTimelineOwner(
  input: NativeTimelineRedactInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_redact', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'redact') ?? 'unavailable';
}

export async function forwardTextWithNativeTimelineOwner(
  input: NativeTimelineForwardTextInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_forward_text', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'forward_text') ?? 'unavailable';
}

export async function forwardMediaWithNativeTimelineOwner(
  input: NativeTimelineForwardMediaInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_forward_media', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'forward_media') ?? 'unavailable';
}

export async function reportWithNativeTimelineOwner(
  input: NativeTimelineReportInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_report', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'report') ?? 'unavailable';
}

export async function pinWithNativeTimelineOwner(
  input: NativeTimelinePinInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_pin', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'pin') ?? 'unavailable';
}

export async function unpinWithNativeTimelineOwner(
  input: NativeTimelinePinInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeTimelineActionReadback | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_timeline_unpin', { request: input });
  if (!result.available) return 'unavailable';
  return acceptActionReadback(result.value, 'unpin') ?? 'unavailable';
}
