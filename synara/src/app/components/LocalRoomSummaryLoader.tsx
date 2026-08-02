import { type ReactNode } from 'react';
import { type LocalRoomSummary, useLocalRoomSummary } from '../hooks/useLocalRoomSummary';

type LocalRoom = Parameters<typeof useLocalRoomSummary>[0];

export function LocalRoomSummaryLoader({
  room,
  children,
}: {
  room: LocalRoom;
  children: (roomSummary: LocalRoomSummary) => ReactNode;
}) {
  const summary = useLocalRoomSummary(room);

  return children(summary);
}
