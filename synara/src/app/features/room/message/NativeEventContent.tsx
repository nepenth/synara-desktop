import React, { ReactNode, useEffect, useState } from 'react';
import { MessageEvent, NativeEventContentEvent } from '../../../../types/matrix/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';
import { nextEventReadbackDelayMs, type EventReadbackState } from './nativeEventReadbackPoll';

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

export type NativeEventSource = {
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
    let timer: number | undefined;
    let delayMs: number | undefined;
    const schedule = (state: EventReadbackState) => {
      delayMs = nextEventReadbackDelayMs(state, delayMs);
      if (!disposed && delayMs !== undefined) {
        timer = window.setTimeout(() => void readback(), delayMs);
      }
    };
    const readback = async () => {
      const result = await invokeDesktopWithAvailability<NativeTimelineEventReadback>(
        'matrix_timeline_event_readback',
        { roomId, eventId }
      ).catch(() => undefined);
      if (disposed) return;
      if (!result?.available || !result.value) {
        schedule('pending');
        return;
      }
      const { item } = result.value;
      if (item.decryptionState === 'pending') {
        schedule('pending');
        return;
      }
      if (item.decryptionState === 'unavailable') {
        setResolvedEvent(toSafeNativeEvent(item, true));
        schedule('unavailable');
        return;
      }
      setResolvedEvent(toSafeNativeEvent(item, false));
      schedule('decrypted');
    };
    void readback();
    return () => {
      disposed = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [mEvent, roomId]);

  return <>{children(resolvedEvent)}</>;
}
