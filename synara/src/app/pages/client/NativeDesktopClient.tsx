import React, { FormEvent, useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { RoomSummary } from '../../features/matrix-dto';
import { NativeRoomTimeline } from '../../features/room/NativeRoomTimeline';
import {
  clearActiveNativeMatrixSession,
  type NativeMatrixSessionIdentity,
} from '../../state/nativeMatrixSession';
import { clearPersistedSessions } from '../../state/sessionPersistence';
import { platformSessionStore } from '../../platform';
import { invokeDesktopWithAvailability } from '../../utils/desktop';
import { getLoginPath } from '../pathUtils';

type NativeRoomSummary = Pick<
  RoomSummary,
  'roomId' | 'name' | 'canonicalAlias' | 'isEncrypted' | 'unreadCount' | 'highlightCount'
>;

type NativeRoomListSnapshot = {
  sessionGeneration: number;
  orderedRoomIds: string[];
  rooms: NativeRoomSummary[];
};

const shellStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'minmax(220px, 320px) minmax(0, 1fr)',
  width: '100%',
  height: '100vh',
  minHeight: 0,
  background: 'var(--bg-surface-low, #111318)',
  color: 'var(--tc-surface-normal, #f4f5f7)',
};

export function NativeDesktopClient({ identity }: { identity: NativeMatrixSessionIdentity }) {
  const navigate = useNavigate();
  const [rooms, setRooms] = useState<NativeRoomSummary[]>([]);
  const [selectedRoomId, setSelectedRoomId] = useState<string>();
  const [roomListError, setRoomListError] = useState(false);
  const [message, setMessage] = useState('');
  const [sendState, setSendState] = useState<'idle' | 'sending' | 'sent' | 'error'>('idle');
  const [logoutError, setLogoutError] = useState(false);

  const loadRooms = useCallback(async () => {
    const result = await invokeDesktopWithAvailability<NativeRoomListSnapshot>(
      'matrix_room_list_snapshot'
    );
    if (!result.available || !result.value) throw new Error('Native room list unavailable');
    const byId = new Map(result.value.rooms.map((room) => [room.roomId, room]));
    const ordered = result.value.orderedRoomIds.map(
      (roomId) =>
        byId.get(roomId) ?? {
          roomId,
          isEncrypted: false,
          unreadCount: 0,
          highlightCount: 0,
        }
    );
    setRooms(ordered);
    setSelectedRoomId((current) =>
      current && ordered.some((room) => room.roomId === current) ? current : ordered[0]?.roomId
    );
    setRoomListError(false);
  }, []);

  useEffect(() => {
    let disposed = false;
    const refresh = () =>
      loadRooms().catch(() => {
        if (!disposed) setRoomListError(true);
      });
    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [loadRooms]);

  const handleSend = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const body = message.trim();
    if (!selectedRoomId || !body || sendState === 'sending') return;

    setSendState('sending');
    try {
      const result = await invokeDesktopWithAvailability('matrix_send_text', {
        roomId: selectedRoomId,
        body,
      });
      if (!result.available || !result.value) throw new Error('Native send unavailable');
      setMessage('');
      setSendState('sent');
    } catch {
      setSendState('error');
    }
  };

  const handleLogout = async () => {
    setLogoutError(false);
    try {
      const result = await invokeDesktopWithAvailability('matrix_logout');
      if (!result.available) throw new Error('Native logout unavailable');
      await clearPersistedSessions({ nativeSessionStore: platformSessionStore });
      clearActiveNativeMatrixSession();
      navigate(getLoginPath(), { replace: true });
    } catch {
      setLogoutError(true);
    }
  };

  const selectedRoom = rooms.find((room) => room.roomId === selectedRoomId);

  return (
    <main data-client-owner="matrix-rust-sdk" style={shellStyle}>
      <aside
        style={{
          minHeight: 0,
          overflowY: 'auto',
          borderRight: '1px solid rgba(127, 127, 127, 0.25)',
          padding: 16,
        }}
      >
        <div style={{ marginBottom: 16 }}>
          <strong>Synara Native</strong>
          <div style={{ marginTop: 4, fontSize: 12, opacity: 0.7 }}>{identity.userId}</div>
        </div>
        <button type="button" onClick={() => void handleLogout()}>
          Sign out
        </button>
        {logoutError && <p role="alert">Native sign out failed. Your session is still active.</p>}
        <h2 style={{ fontSize: 15, margin: '20px 0 8px' }}>Rooms</h2>
        {roomListError && (
          <p role="status" style={{ fontSize: 13 }}>
            Room refresh paused; showing the last native snapshot.
          </p>
        )}
        {!roomListError && rooms.length === 0 && (
          <p role="status" style={{ fontSize: 13 }}>
            Waiting for rooms from native sync…
          </p>
        )}
        <nav aria-label="Native Matrix rooms">
          {rooms.map((room) => (
            <button
              key={room.roomId}
              type="button"
              aria-current={room.roomId === selectedRoomId ? 'page' : undefined}
              onClick={() => setSelectedRoomId(room.roomId)}
              style={{
                display: 'block',
                width: '100%',
                margin: '4px 0',
                padding: '10px 12px',
                border: 0,
                borderRadius: 6,
                textAlign: 'left',
                color: 'inherit',
                background:
                  room.roomId === selectedRoomId ? 'rgba(127, 127, 127, 0.25)' : 'transparent',
              }}
            >
              <span>{room.name ?? room.canonicalAlias ?? room.roomId}</span>
              {(room.unreadCount > 0 || room.highlightCount > 0) && (
                <span style={{ float: 'right', fontSize: 12 }}>
                  {room.highlightCount > 0 ? `${room.highlightCount}!` : room.unreadCount}
                </span>
              )}
            </button>
          ))}
        </nav>
      </aside>
      <section style={{ display: 'flex', minWidth: 0, minHeight: 0, flexDirection: 'column' }}>
        {selectedRoomId ? (
          <>
            <header
              style={{
                padding: '12px 24px',
                borderBottom: '1px solid rgba(127, 127, 127, 0.25)',
              }}
            >
              <strong>
                {selectedRoom?.name ?? selectedRoom?.canonicalAlias ?? selectedRoomId}
              </strong>
              {selectedRoom?.isEncrypted && (
                <span style={{ marginLeft: 8, fontSize: 12, opacity: 0.7 }}>Encrypted</span>
              )}
            </header>
            <NativeRoomTimeline key={selectedRoomId} roomId={selectedRoomId} />
            <form
              onSubmit={(event) => void handleSend(event)}
              style={{
                display: 'flex',
                gap: 8,
                padding: 16,
                borderTop: '1px solid rgba(127, 127, 127, 0.25)',
              }}
            >
              <input
                aria-label="Message"
                value={message}
                onChange={(event) => {
                  setMessage(event.target.value);
                  setSendState('idle');
                }}
                style={{ minWidth: 0, flex: 1, padding: '10px 12px' }}
              />
              <button type="submit" disabled={!message.trim() || sendState === 'sending'}>
                {sendState === 'sending' ? 'Sending…' : 'Send'}
              </button>
              {sendState === 'error' && <span role="alert">Send failed.</span>}
            </form>
          </>
        ) : (
          <div style={{ margin: 'auto', padding: 24 }}>Select a room after native sync loads.</div>
        )}
      </section>
    </main>
  );
}
