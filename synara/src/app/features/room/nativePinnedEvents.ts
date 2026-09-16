import { useCallback, useEffect, useState } from 'react';
import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import type { NativeTimelineReaction } from './nativeTimelineView';
import { useRoomPinnedEvents } from '../../hooks/useRoomPinnedEvents';
import type { EventedRoomReading } from '../../utils/roomEvents';

export const NATIVE_PINNED_EVENTS_SCHEMA_VERSION = 1;

export type NativePinnedEventItem = {
  eventId: string;
  senderId: string;
  originServerTs: number;
  eventType: string;
  body?: string;
  redacted?: boolean;
  reactions?: NativeTimelineReaction[];
  messageType?: string;
};

export type NativePinnedEventsSnapshot = {
  schemaVersion: number;
  roomId: string;
  eventIds: string[];
  items: NativePinnedEventItem[];
};

type NativePinnedInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const PINNED_EVENTS_CHANGED = 'native-pinned-events-changed';
const pinnedEventsBus = new EventTarget();

export function notifyNativePinnedEventsChanged(roomId: string): void {
  pinnedEventsBus.dispatchEvent(new CustomEvent(PINNED_EVENTS_CHANGED, { detail: roomId }));
}

const acceptPinnedSnapshot = (
  value: unknown,
  roomId: string
): NativePinnedEventsSnapshot | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const snapshot = value as NativePinnedEventsSnapshot;
  if (
    snapshot.schemaVersion !== NATIVE_PINNED_EVENTS_SCHEMA_VERSION ||
    snapshot.roomId !== roomId ||
    !Array.isArray(snapshot.eventIds) ||
    !Array.isArray(snapshot.items)
  ) {
    return undefined;
  }
  return snapshot;
};

export async function pinnedEventsWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativePinnedInvoke
): Promise<NativePinnedEventsSnapshot | 'unavailable'> {
  if (!desktopAvailable) return 'unavailable';
  const result = await invoke('matrix_pinned_events', { request: { roomId } });
  if (!result.available) return 'unavailable';
  return acceptPinnedSnapshot(result.value, roomId) ?? 'unavailable';
}

export type NativePinnedEventsState = {
  available: boolean;
  loading: boolean;
  snapshot: NativePinnedEventsSnapshot | null;
};

export function useNativePinnedEvents(room: EventedRoomReading): NativePinnedEventsState {
  const jsPinned = useRoomPinnedEvents(room);
  const jsKey = jsPinned.join('\0');
  const [state, setState] = useState<NativePinnedEventsState>({
    available: isSynaraDesktop(),
    loading: isSynaraDesktop(),
    snapshot: null,
  });

  const load = useCallback(async () => {
    if (!isSynaraDesktop()) {
      setState({ available: false, loading: false, snapshot: null });
      return;
    }
    setState((current) => ({ ...current, available: true, loading: true }));
    try {
      const result = await pinnedEventsWithNativeOwner(
        room.roomId,
        true,
        invokeDesktopWithAvailability
      );
      if (result === 'unavailable') {
        setState({ available: false, loading: false, snapshot: null });
        return;
      }
      setState({ available: true, loading: false, snapshot: result });
    } catch {
      setState({ available: false, loading: false, snapshot: null });
    }
  }, [room.roomId]);

  useEffect(() => {
    void load();
  }, [load, jsKey]);

  useEffect(() => {
    const onChange = (event: Event) => {
      const changedRoomId = (event as CustomEvent<string>).detail;
      if (changedRoomId && changedRoomId !== room.roomId) return;
      void load();
    };
    pinnedEventsBus.addEventListener(PINNED_EVENTS_CHANGED, onChange);
    return () => pinnedEventsBus.removeEventListener(PINNED_EVENTS_CHANGED, onChange);
  }, [load, room.roomId]);

  return state;
}

export function pinnedEventCount(
  native: NativePinnedEventsState,
  jsPinned: readonly string[]
): number {
  if (native.snapshot) return native.snapshot.eventIds.length;
  return jsPinned.length;
}
