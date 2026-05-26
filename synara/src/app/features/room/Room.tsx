import React, { useCallback, useState } from 'react';
import { Box, Line } from 'folds';
import { useParams } from 'react-router-dom';
import { isKeyHotkey } from 'is-hotkey';
import { useAtomValue } from 'jotai';
import { RoomView } from './RoomView';
import { MembersDrawer } from './MembersDrawer';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { PowerLevelsContextProvider, usePowerLevels } from '../../hooks/usePowerLevels';
import { useRoom } from '../../hooks/useRoom';
import { useKeyDown } from '../../hooks/useKeyDown';
import { markAsRead } from '../../utils/notifications';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoomMembers } from '../../hooks/useRoomMembers';
import { CallView } from '../call/CallView';
import { RoomViewHeader } from './RoomViewHeader';
import { callChatAtom } from '../../state/callEmbed';
import { CallChatView } from './CallChatView';
import { RoomSidePanel, RoomSidePanelType } from './RoomSidePanel';

export function Room() {
  const { eventId } = useParams();
  const room = useRoom();
  const mx = useMatrixClient();

  const [isDrawer, setIsDrawer] = useSetting(settingsAtom, 'isPeopleDrawer');
  const [roomSidePanel, setRoomSidePanel] = useState<RoomSidePanelType>();
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
  const screenSize = useScreenSizeContext();
  const powerLevels = usePowerLevels(room);
  const members = useRoomMembers(mx, room.roomId);
  const chat = useAtomValue(callChatAtom);

  useKeyDown(
    window,
    useCallback(
      (evt) => {
        if (isKeyHotkey('escape', evt)) {
          markAsRead(mx, room.roomId, hideActivity);
        }
      },
      [mx, room.roomId, hideActivity]
    )
  );

  const callView = room.isCallRoom();
  const activeSidePanel = isDrawer ? 'members' : roomSidePanel;
  const handleToggleSidePanel = useCallback(
    (panel: RoomSidePanelType) => {
      setIsDrawer(false);
      setRoomSidePanel((currentPanel) => (currentPanel === panel ? undefined : panel));
    },
    [setIsDrawer]
  );
  const handleToggleMembers = useCallback(() => {
    setRoomSidePanel(undefined);
    setIsDrawer(!isDrawer);
  }, [isDrawer, setIsDrawer]);
  const handleCloseSidePanel = useCallback(() => {
    setRoomSidePanel(undefined);
    setIsDrawer(false);
  }, [setIsDrawer]);

  return (
    <PowerLevelsContextProvider value={powerLevels}>
      <Box grow="Yes">
        {callView && (screenSize === ScreenSize.Desktop || !chat) && (
          <Box grow="Yes" direction="Column">
            <RoomViewHeader callView />
            <Box grow="Yes">
              <CallView />
            </Box>
          </Box>
        )}
        {!callView && (
          <Box grow="Yes" direction="Column">
            <RoomViewHeader
              activeSidePanel={activeSidePanel}
              onToggleSidePanel={handleToggleSidePanel}
              onToggleMembers={handleToggleMembers}
            />
            <Box grow="Yes">
              <RoomView eventId={eventId} />
            </Box>
          </Box>
        )}

        {callView && chat && (
          <>
            {screenSize === ScreenSize.Desktop && (
              <Line variant="Background" direction="Vertical" size="300" />
            )}
            <CallChatView />
          </>
        )}
        {!callView && screenSize === ScreenSize.Desktop && activeSidePanel && (
          <>
            <Line variant="Background" direction="Vertical" size="300" />
            {activeSidePanel === 'members' ? (
              <MembersDrawer key={room.roomId} room={room} members={members} />
            ) : (
              <RoomSidePanel
                key={`${room.roomId}-${activeSidePanel}`}
                room={room}
                activePanel={activeSidePanel}
                requestClose={handleCloseSidePanel}
              />
            )}
          </>
        )}
      </Box>
    </PowerLevelsContextProvider>
  );
}
