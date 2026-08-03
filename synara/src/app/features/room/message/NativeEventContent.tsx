import React, { ReactNode, useEffect, useState } from 'react';
import { MessageEvent, NativeEventContentEvent } from '../../../../types/matrix/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';

type NativeTimelineItem = {
  itemId: string;
  eventId: string;
  sender: string;
  type: string;
  body: string;
  originServerTs: number;
  decryptionState?: 'pending' | 'unavailable';
};

type NativeTimelineEventReadback = {
  sessionGeneration: number;
  roomId: string;
  eventId: string;
  item: NativeTimelineItem;
};

type NativeEventContentProps = {
  roomId: string;
  mEvent: NativeEventSource;
  children: (event: NativeEventContentEvent) => ReactNode;
};

type NativeEventSource = {
  getId(): string | undefined;
  getSender(): string | undefined;
  getType(): string;
  getTs(): number;
  getContent<T = Record<string, unknown>>(): T;
  isRedacted(): boolean;
};

const toNativeEvent = (event: NativeEventSource): NativeEventContentEvent => ({
  eventId: event.getId() ?? '',
  sender: event.getSender() ?? '',
  type: event.getType(),
  originServerTs: event.getTs(),
  content: event.getContent<Record<string, unknown>>(),
  redacted: event.isRedacted(),
});

const toSafeNativeEvent = (
  item: NativeTimelineItem,
  unavailable: boolean
): NativeEventContentEvent => ({
  eventId: item.eventId,
  sender: item.sender,
  originServerTs: item.originServerTs,
  type: MessageEvent.RoomMessage,
  redacted: false,
  content: {
    msgtype: unavailable ? 'm.bad.encrypted' : 'm.text',
    body: unavailable ? 'Unable to decrypt message' : item.body,
  },
});

/** Polls a Rust-owned focused timeline only while this legacy row is UTD. */
export function NativeEventContent({ roomId, mEvent, children }: NativeEventContentProps) {
  const [resolvedEvent, setResolvedEvent] = useState(() => toNativeEvent(mEvent));

  useEffect(() => {
    setResolvedEvent(toNativeEvent(mEvent));
    if (!isSynaraDesktop() || mEvent.getType() !== MessageEvent.RoomMessageEncrypted) return;
    const eventId = mEvent.getId();
    if (!eventId) return;

    let disposed = false;
    let unavailableShown = false;
    const readback = async () => {
      const result = await invokeDesktopWithAvailability<NativeTimelineEventReadback>(
        'matrix_timeline_event_readback',
        { roomId, eventId }
      ).catch(() => undefined);
      if (disposed || !result?.available || !result.value) return;
      const { item } = result.value;
      if (item.decryptionState === 'pending') return;
      if (item.decryptionState === 'unavailable') {
        if (!unavailableShown) {
          unavailableShown = true;
          setResolvedEvent(toSafeNativeEvent(item, true));
        }
        return;
      }
      setResolvedEvent(toSafeNativeEvent(item, false));
      window.clearInterval(pollId);
    };
    const pollId = window.setInterval(() => void readback(), 1000);
    void readback();
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [mEvent, roomId]);

  return <>{children(resolvedEvent)}</>;
}
