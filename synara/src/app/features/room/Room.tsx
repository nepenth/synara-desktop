import React, { useCallback, useState } from 'react';
import { Box, Line } from 'folds';
import { useParams } from 'react-router-dom';
import { RoomView } from './RoomView';
import { MembersDrawer } from './MembersDrawer';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { PowerLevelsContextProvider, usePowerLevels } from '../../hooks/usePowerLevels';
import { useRoom } from '../../hooks/useRoom';
import { RoomViewHeader } from './RoomViewHeader';
import { RoomSidePanel, RoomSidePanelType } from './RoomSidePanel';

export function Room() {
  const { eventId } = useParams();
  const room = useRoom();

  const [isDrawer, setIsDrawer] = useSetting(settingsAtom, 'isPeopleDrawer');
  const [roomSidePanel, setRoomSidePanel] = useState<RoomSidePanelType>();
  const screenSize = useScreenSizeContext();
  const powerLevels = usePowerLevels(room);

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

        {screenSize === ScreenSize.Desktop && activeSidePanel && (
          <>
            <Line variant="Background" direction="Vertical" size="300" />
            {activeSidePanel === 'members' ? (
              <MembersDrawer key={room.roomId} room={room} />
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
