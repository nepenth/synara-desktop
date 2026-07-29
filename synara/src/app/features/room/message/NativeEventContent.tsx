import { IEvent, MatrixEvent } from 'matrix-js-sdk';
import React, { ReactNode, useEffect, useState } from 'react';
import { MessageEvent } from '../../../../types/matrix/room';
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
  mEvent: MatrixEvent;
  children: (event: MatrixEvent) => ReactNode;
};

const toSafeMatrixEvent = (
  roomId: string,
  item: NativeTimelineItem,
  unavailable: boolean
): MatrixEvent =>
  new MatrixEvent({
    room_id: roomId,
    event_id: item.eventId,
    sender: item.sender,
    origin_server_ts: item.originServerTs,
    type: MessageEvent.RoomMessage,
    content: {
      msgtype: unavailable ? 'm.bad.encrypted' : 'm.text',
      body: unavailable ? 'Unable to decrypt message' : item.body,
    },
  } as IEvent);

/** Polls a Rust-owned focused timeline only while this legacy row is UTD. */
export function NativeEventContent({ roomId, mEvent, children }: NativeEventContentProps) {
  const [resolvedEvent, setResolvedEvent] = useState(mEvent);

  useEffect(() => {
    setResolvedEvent(mEvent);
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
          setResolvedEvent(toSafeMatrixEvent(roomId, item, true));
        }
        return;
      }
      setResolvedEvent(toSafeMatrixEvent(roomId, item, false));
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
