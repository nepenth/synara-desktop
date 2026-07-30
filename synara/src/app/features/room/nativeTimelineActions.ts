/**
 * SDK-neutral owners for retained timeline actions (edit / redact / forward).
 *
 * Reply continues to use `matrix_send_text` with `replyTo`. Reactions remain on
 * the V-SEND.2 owner. These helpers do not select NativeTimelinePresenter or
 * delete RoomTimeline.
 */

import type { DesktopInvokeResult } from '../../utils/desktop';

export const NATIVE_TIMELINE_ACTION_SCHEMA_VERSION = 1;

export type NativeTimelineActionKind = 'edit_text' | 'redact' | 'forward_text';

export type NativeTimelineActionReadback = {
  schemaVersion: number;
  action: NativeTimelineActionKind;
  roomId: string;
  eventId: string;
  status: 'sent' | 'redacted';
};

export type NativeTimelineEditTextInput = {
  roomId: string;
  eventId: string;
  body: string;
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

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

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
    (readback.status !== 'sent' && readback.status !== 'redacted')
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
