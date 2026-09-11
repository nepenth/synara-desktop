import React from 'react';
import { ErrorBoundary } from 'react-error-boundary';
import { RoomSettings } from './RoomSettings';
import { Modal500 } from '../../components/Modal500';
import { AppErrorFallback } from '../../components/app-error';
import { useCloseRoomSettings, useRoomSettingsState } from '../../state/hooks/roomSettings';
import { useAllJoinedRoomsSet, useGetRoom } from '../../hooks/useGetRoom';
import { RoomSettingsState } from '../../state/roomSettings';
import { RoomProvider } from '../../hooks/useRoom';
import { SpaceProvider } from '../../hooks/useSpace';

type RenderSettingsProps = {
  state: RoomSettingsState;
};
function RenderSettings({ state }: RenderSettingsProps) {
  const { roomId, spaceId, page } = state;
  const closeSettings = useCloseRoomSettings();
  const allJoinedRooms = useAllJoinedRoomsSet();
  const getRoom = useGetRoom(allJoinedRooms);
  const room = getRoom(roomId);
  const space = spaceId ? getRoom(spaceId) : undefined;

  if (!room) return null;

  return (
    <Modal500 requestClose={closeSettings}>
      <ErrorBoundary
        fallbackRender={({ error, resetErrorBoundary }) => (
          <AppErrorFallback
            error={error}
            description="Room settings ran into a problem. You can close this panel and keep using the room."
            onRetry={resetErrorBoundary}
            onClose={closeSettings}
          />
        )}
      >
        <SpaceProvider value={space ?? null}>
          <RoomProvider value={room}>
            <RoomSettings initialPage={page} requestClose={closeSettings} />
          </RoomProvider>
        </SpaceProvider>
      </ErrorBoundary>
    </Modal500>
  );
}

export function RoomSettingsRenderer() {
  const state = useRoomSettingsState();

  if (!state) return null;
  return <RenderSettings state={state} />;
}
