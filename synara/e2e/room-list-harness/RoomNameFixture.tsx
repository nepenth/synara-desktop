import React, { useMemo, useState } from 'react';
import { useRoomName } from '../../src/app/hooks/useRoomMeta';
import type { EventedRoomReading } from '../../src/app/utils/roomEvents';

// Mirrors the native room facade: one stable object reading the latest
// summary, with no per-room events. The row re-renders from the summary
// subscription, as RoomNavItem does through useNativeRoomSummary.
function RoomNameRow({ room }: { room: EventedRoomReading }) {
  const name = useRoomName(room);
  return <span data-testid="room-name-row"># {name}</span>;
}

export function RoomNameFixture() {
  const [summaryName, setSummaryName] = useState('Empty Room');
  const summaryRef = useMemo(() => ({ current: summaryName }), []);
  summaryRef.current = summaryName;
  const room = useMemo(
    () =>
      ({
        roomId: '!new:example.org',
        get name() {
          return summaryRef.current;
        },
        on: () => undefined,
        removeListener: () => undefined,
      }) as unknown as EventedRoomReading,
    [summaryRef]
  );

  return (
    <main style={{ fontFamily: 'sans-serif', padding: 16 }}>
      <h1>Room name refresh</h1>
      <RoomNameRow room={room} />
      <button type="button" onClick={() => setSummaryName('Project Room')}>
        Name arrives
      </button>
      <button type="button" onClick={() => setSummaryName('Renamed Room')}>
        Rename
      </button>
    </main>
  );
}
