import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Box, Line } from 'folds';
import { useParams } from 'react-router-dom';
import { RoomView } from './RoomView';
import { MembersDrawer } from './MembersDrawer';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import {
  PowerLevelsContextProvider,
  usePowerLevels,
  usePowerLevelsContext,
} from '../../hooks/usePowerLevels';
import { useRoom } from '../../hooks/useRoom';
import { RoomViewHeader } from './RoomViewHeader';
import { RoomSidePanel, RoomSidePanelType } from './RoomSidePanel';
import { closeExperimentalWidgets } from '../widgets/experimentalWidgets';
import { ThreadSidePanel } from './ThreadSidePanel';
import { RoomInput } from './RoomInput';
import { useEditor } from '../../components/editor';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { resolveThreadPane, type ThreadPaneTarget } from './threadPane';

function ThreadPaneComposer({ rootEventId }: { rootEventId: string }) {
  const room = useRoom();
  const editor = useEditor();
  const permissions = useRoomPermissions(usePowerLevelsContext());
  if (!permissions.event('m.room.message')) return null;
  return (
    <RoomInput
      key={rootEventId}
      room={room}
      roomId={room.roomId}
      editor={editor}
      threadRootOverride={rootEventId}
    />
  );
}

export function Room() {
  const { eventId, threadRootId } = useParams();
  const room = useRoom();
  const { navigateRoom } = useRoomNavigate();

  const [isDrawer, setIsDrawer] = useSetting(settingsAtom, 'isPeopleDrawer');
  const [experimentalWidgetsEnabled] = useSetting(settingsAtom, 'experimentalWidgetsEnabled');
  const [threadDisplay] = useSetting(settingsAtom, 'threadDisplay');
  const [threadPaneWidth, setThreadPaneWidth] = useSetting(settingsAtom, 'threadPaneWidth');
  const [roomSidePanel, setRoomSidePanel] = useState<RoomSidePanelType>();
  const [openedPane, setOpenedPane] = useState<ThreadPaneTarget>();
  const [roomAreaWidth, setRoomAreaWidth] = useState<number>();
  const roomAreaRef = useRef<HTMLDivElement>(null);
  const screenSize = useScreenSizeContext();
  const powerLevels = usePowerLevels(room);

  // A pane belongs to one room.
  useEffect(() => setOpenedPane(undefined), [room.roomId]);

  const pane = resolveThreadPane({
    mode: threadDisplay,
    opened: openedPane,
    routeThreadRootId: threadRootId,
    desktop: screenSize === ScreenSize.Desktop,
  });
  // One right pane at a time: an open thread takes the right side, and the
  // members drawer returns when it closes without changing its setting.
  const activeSidePanel = pane ? undefined : isDrawer ? 'members' : roomSidePanel;

  const handleOpenThreadPane = useCallback((rootEventId: string, latestEventId?: string) => {
    setRoomSidePanel(undefined);
    setOpenedPane({ rootEventId, latestEventId });
  }, []);
  const handleCloseThreadPane = useCallback(() => {
    setOpenedPane(undefined);
    if (threadRootId) navigateRoom(room.roomId);
  }, [navigateRoom, room.roomId, threadRootId]);

  const handleToggleSidePanel = useCallback(
    (panel: RoomSidePanelType) => {
      setOpenedPane(undefined);
      setIsDrawer(false);
      setRoomSidePanel((currentPanel) => (currentPanel === panel ? undefined : panel));
    },
    [setIsDrawer]
  );
  const handleToggleMembers = useCallback(() => {
    setOpenedPane(undefined);
    setRoomSidePanel(undefined);
    setIsDrawer(pane ? true : !isDrawer);
  }, [isDrawer, pane, setIsDrawer]);
  const handleCloseSidePanel = useCallback(() => {
    setRoomSidePanel(undefined);
    setIsDrawer(false);
  }, [setIsDrawer]);

  useEffect(() => {
    const element = roomAreaRef.current;
    if (!element || typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(([entry]) => setRoomAreaWidth(entry.contentRect.width));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!experimentalWidgetsEnabled) {
      void closeExperimentalWidgets();
    }
  }, [experimentalWidgetsEnabled]);

  useEffect(
    () => () => {
      void closeExperimentalWidgets();
    },
    [room.roomId]
  );

  return (
    <PowerLevelsContextProvider value={powerLevels}>
      <Box grow="Yes" ref={roomAreaRef}>
        <Box grow="Yes" direction="Column" style={{ minWidth: 0 }}>
          <RoomViewHeader
            activeSidePanel={activeSidePanel}
            onToggleSidePanel={handleToggleSidePanel}
            onToggleMembers={handleToggleMembers}
          />
          <Box grow="Yes">
            <RoomView
              eventId={eventId}
              threadRootEventId={pane ? undefined : threadRootId}
              threadDisplay={screenSize === ScreenSize.Desktop ? threadDisplay : 'full'}
              onOpenThreadPane={
                screenSize === ScreenSize.Desktop && threadDisplay !== 'full'
                  ? handleOpenThreadPane
                  : undefined
              }
            />
          </Box>
        </Box>

        {pane && (
          <ThreadSidePanel
            roomId={room.roomId}
            rootEventId={pane.rootEventId}
            width={threadPaneWidth}
            availableWidth={roomAreaWidth}
            onWidthChange={setThreadPaneWidth}
            onClose={handleCloseThreadPane}
            composer={<ThreadPaneComposer rootEventId={pane.rootEventId} />}
          />
        )}

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
