import React from 'react';
import { liveCallChipLabel } from '../../src/app/features/matrix-rtc/liveCallChrome';

type FixtureRoom = {
  name: string;
  isCall: boolean;
  hasActiveCall: boolean;
  activeCallParticipantCount: number;
};

const rooms: FixtureRoom[] = [
  {
    name: 'Standup',
    isCall: false,
    hasActiveCall: true,
    activeCallParticipantCount: 3,
  },
  {
    name: 'Voice room',
    isCall: true,
    hasActiveCall: false,
    activeCallParticipantCount: 0,
  },
];

export function LiveCallFixture() {
  return (
    <main style={{ fontFamily: 'sans-serif', padding: 16 }}>
      <h1>Live-call chrome</h1>
      <p>Voice-room type must not claim a live call. Membership does.</p>
      {rooms.map((room) => (
        <div
          key={room.name}
          data-testid={room.hasActiveCall ? 'live-call-room' : 'voice-room-type'}
          data-is-call={String(room.isCall)}
          data-has-active-call={String(room.hasActiveCall)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            height: 40,
            borderBottom: '1px solid #ddd',
          }}
        >
          <span># {room.name}</span>
          {room.hasActiveCall && (
            <span data-testid="live-call-chip">{liveCallChipLabel(room.activeCallParticipantCount)}</span>
          )}
        </div>
      ))}
    </main>
  );
}
