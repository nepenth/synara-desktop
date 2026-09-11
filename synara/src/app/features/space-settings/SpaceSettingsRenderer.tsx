import React from 'react';
import { ErrorBoundary } from 'react-error-boundary';
import { SpaceSettings } from './SpaceSettings';
import { Modal500 } from '../../components/Modal500';
import { AppErrorFallback } from '../../components/app-error';
import { useCloseSpaceSettings, useSpaceSettingsState } from '../../state/hooks/spaceSettings';
import { useAllJoinedRoomsSet, useGetRoom } from '../../hooks/useGetRoom';
import { SpaceSettingsState } from '../../state/spaceSettings';
import { RoomProvider } from '../../hooks/useRoom';
import { SpaceProvider } from '../../hooks/useSpace';

type RenderSettingsProps = {
  state: SpaceSettingsState;
};
function RenderSettings({ state }: RenderSettingsProps) {
  const { roomId, spaceId, page } = state;
  const closeSettings = useCloseSpaceSettings();
  const allJoinedRooms = useAllJoinedRoomsSet();
  const getRoom = useGetRoom(allJoinedRooms);
  const room = getRoom(roomId);
  const space = spaceId && spaceId !== roomId ? getRoom(spaceId) : undefined;

  if (!room) return null;

  return (
    <Modal500 requestClose={closeSettings}>
      <ErrorBoundary
        fallbackRender={({ error, resetErrorBoundary }) => (
          <AppErrorFallback
            error={error}
            description="Space settings ran into a problem. You can close this panel and keep using the space."
            onRetry={resetErrorBoundary}
            onClose={closeSettings}
          />
        )}
      >
        <SpaceProvider value={space ?? null}>
          <RoomProvider value={room}>
            <SpaceSettings initialPage={page} requestClose={closeSettings} />
          </RoomProvider>
        </SpaceProvider>
      </ErrorBoundary>
    </Modal500>
  );
}

export function SpaceSettingsRenderer() {
  const state = useSpaceSettingsState();

  if (!state) return null;
  return <RenderSettings state={state} />;
}
