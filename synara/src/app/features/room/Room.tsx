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
import { NativePermalinkProvider } from './nativeTimelineFormattedBody';
import { useNativePermalinkHandler } from './useNativePermalinkHandler';
import { useAutoHideRoomList } from '../room-list-pane/useRoomListPane';
import * as overlayCss from './RoomPanelOverlay.css';
import { portalOpen, useAppShortcutHandler } from '../desktop-keyboard/appShortcutHandlers';

/**
 * Esc closes the overlay first (the registry's Escape, `close-thread`): the
 * most recently mounted handler wins, so an open thread pane closes next.
 */
function OverlayEscape({ onClose }: { onClose: () => void }) {
  useAppShortcutHandler(
    'close-thread',
    useCallback(() => {
      if (portalOpen()) return false;
      onClose();
      return true;
    }, [onClose])
  );
  return null;
}

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
  // Below the Desktop width the members drawer is an overlay, opened for the
  // moment only: the saved drawer setting is the Desktop column's.
  const [membersOverlay, setMembersOverlay] = useState(false);
  const [openedPane, setOpenedPane] = useState<ThreadPaneTarget>();
  const [roomAreaWidth, setRoomAreaWidth] = useState<number>();
  const roomAreaRef = useRef<HTMLDivElement>(null);
  const screenSize = useScreenSizeContext();
  // Desktop: side panels and the members drawer sit beside the room. Below
  // it they open over the room. The thread pane needs more than Mobile.
  const wide = screenSize === ScreenSize.Desktop;
  const threadPaneFits = screenSize !== ScreenSize.Mobile;
  const powerLevels = usePowerLevels(room);
  const permalinkHandler = useNativePermalinkHandler(room.roomId);

  // A pane belongs to one room.
  useEffect(() => setOpenedPane(undefined), [room.roomId]);
  useEffect(() => setMembersOverlay(false), [room.roomId, wide]);

  const pane = resolveThreadPane({
    mode: threadDisplay,
    opened: openedPane,
    routeThreadRootId: threadRootId,
    desktop: threadPaneFits,
  });
  // One right pane at a time: an open thread takes the right side, and the
  // members drawer returns when it closes without changing its setting.
  // Overlays (below Desktop) cover whatever is on the right, the thread too.
  const activeSidePanel = wide
    ? pane
      ? undefined
      : isDrawer
        ? 'members'
        : roomSidePanel
    : membersOverlay
      ? 'members'
      : roomSidePanel;

  // Tablet: a thread pane hides the room list while it is open (L2).
  useAutoHideRoomList(screenSize === ScreenSize.Tablet && !!pane);

  const handleOpenThreadPane = useCallback(
    (rootEventId: string, latestEventId?: string, focusEventId?: string) => {
      setRoomSidePanel(undefined);
      setMembersOverlay(false);
      setOpenedPane({ rootEventId, latestEventId, focusEventId });
    },
    []
  );
  const handleCloseThreadPane = useCallback(() => {
    setOpenedPane(undefined);
    if (threadRootId) navigateRoom(room.roomId);
  }, [navigateRoom, room.roomId, threadRootId]);

  const handleToggleSidePanel = useCallback(
    (panel: RoomSidePanelType) => {
      setRoomSidePanel((currentPanel) => (currentPanel === panel ? undefined : panel));
      if (!wide) {
        setMembersOverlay(false);
        return;
      }
      setOpenedPane(undefined);
      setIsDrawer(false);
    },
    [setIsDrawer, wide]
  );
  const handleToggleMembers = useCallback(() => {
    setRoomSidePanel(undefined);
    if (!wide) {
      setMembersOverlay((open) => !open);
      return;
    }
    setOpenedPane(undefined);
    setIsDrawer(pane ? true : !isDrawer);
  }, [isDrawer, pane, setIsDrawer, wide]);
  const handleCloseSidePanel = useCallback(() => {
    setRoomSidePanel(undefined);
    if (wide) setIsDrawer(false);
    else setMembersOverlay(false);
  }, [setIsDrawer, wide]);

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
      <NativePermalinkProvider value={permalinkHandler}>
        <Box as="main" grow="Yes" ref={roomAreaRef} style={{ position: 'relative', minWidth: 0 }}>
          <Box grow="Yes" direction="Column" style={{ minWidth: 0 }} data-file-drop-region="room">
            <RoomViewHeader
              activeSidePanel={activeSidePanel}
              onToggleSidePanel={handleToggleSidePanel}
              onToggleMembers={handleToggleMembers}
            />
            <Box grow="Yes">
              <RoomView
                eventId={eventId}
                threadRootEventId={pane ? undefined : threadRootId}
                threadDisplay={threadPaneFits ? threadDisplay : 'full'}
                onOpenThreadPane={
                  threadPaneFits && threadDisplay !== 'full' ? handleOpenThreadPane : undefined
                }
              />
            </Box>
          </Box>

          {pane && (
            <ThreadSidePanel
              roomId={room.roomId}
              rootEventId={pane.rootEventId}
              focusEventId={pane.focusEventId}
              width={threadPaneWidth}
              availableWidth={roomAreaWidth}
              onWidthChange={setThreadPaneWidth}
              onClose={handleCloseThreadPane}
              composer={<ThreadPaneComposer rootEventId={pane.rootEventId} />}
            />
          )}

          {!wide && activeSidePanel && (
            // Not modal: the room stays usable; Esc or the panel's Close closes it.
            <div className={overlayCss.Overlay} data-room-panel-overlay={activeSidePanel}>
              <OverlayEscape onClose={handleCloseSidePanel} />
              {activeSidePanel === 'members' ? (
                <MembersDrawer key={room.roomId} room={room} requestClose={handleCloseSidePanel} />
              ) : (
                <RoomSidePanel
                  key={`${room.roomId}-${activeSidePanel}`}
                  room={room}
                  activePanel={activeSidePanel}
                  requestClose={handleCloseSidePanel}
                  overlay
                />
              )}
            </div>
          )}

          {wide && activeSidePanel && (
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
      </NativePermalinkProvider>
    </PowerLevelsContextProvider>
  );
}
