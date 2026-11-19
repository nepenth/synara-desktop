import React, { useCallback, useRef } from 'react';
import { Box, Text, config } from 'folds';
import type { Editor } from 'slate';
import { ReactEditor } from 'slate-react';
import { isKeyHotkey } from 'is-hotkey';
import { useStateEvent } from '../../hooks/useStateEvent';
import { StateEvent } from '../../../types/matrix/room';
import { usePowerLevelsContext } from '../../hooks/usePowerLevels';
import { RoomInputPlaceholder } from './RoomInputPlaceholder';
import { NativeTimelinePresenter } from './NativeTimelinePresenter';
import { RoomViewTyping } from './RoomViewTyping';
import { RoomTombstone } from './RoomTombstone';
import { RoomInput, useKeepRoomInputDraft } from './RoomInput';
import { ComposerSafetyNet } from './ComposerSafetyNet';
import { RoomIdentityWarningBanner } from './RoomIdentityWarningBanner';
import { RoomViewFollowing, RoomViewFollowingPlaceholder } from './RoomViewFollowing';
import { Page } from '../../components/page';
import { useKeyDown } from '../../hooks/useKeyDown';
import { editableActiveElement } from '../../utils/dom';
import { settingsAtom } from '../../state/settings';
import { useSetting } from '../../state/hooks/settings';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { VoiceRoom } from './VoiceRoom';
import { useRoom } from '../../hooks/useRoom';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { useNativeRoomSummary } from '../../state/room-list/roomList';
import type { ThreadDisplayMode } from '../../utils/threadDisplay';

const FN_KEYS_REGEX = /^F\d+$/;
const shouldFocusMessageField = (evt: KeyboardEvent): boolean => {
  const { code } = evt;
  if (evt.metaKey || evt.altKey || evt.ctrlKey) {
    return false;
  }

  if (FN_KEYS_REGEX.test(code)) return false;

  if (
    code.startsWith('OS') ||
    code.startsWith('Meta') ||
    code.startsWith('Shift') ||
    code.startsWith('Alt') ||
    code.startsWith('Control') ||
    code.startsWith('Arrow') ||
    code.startsWith('Page') ||
    code.startsWith('End') ||
    code.startsWith('Home') ||
    code === 'Tab' ||
    code === 'Space' ||
    code === 'Enter' ||
    code === 'NumLock' ||
    code === 'ScrollLock'
  ) {
    return false;
  }

  return true;
};

export function RoomView({
  eventId,
  threadRootEventId,
  threadDisplay = 'full',
  onOpenThreadPane,
  threadPaneRootEventId,
}: {
  eventId?: string;
  threadRootEventId?: string;
  threadDisplay?: ThreadDisplayMode;
  onOpenThreadPane?: (rootEventId: string, latestEventId?: string, focusEventId?: string) => void;
  /** The thread shown in the side pane, if any. */
  threadPaneRootEventId?: string;
}) {
  const roomInputRef = useRef<HTMLDivElement>(null);
  const roomViewRef = useRef<HTMLDivElement>(null);

  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');

  const room = useRoom();
  const { roomId } = room;
  // The composer's editor; the safety net replaces it if the composer fails.
  const editorRef = useRef<Editor | undefined>(undefined);
  const keepComposerDraft = useKeepRoomInputDraft(roomId);
  const { navigateRoom, navigateThread } = useRoomNavigate();

  // The native room facade has no state events, so the upgrade comes from
  // Core's room summary (D15). Core does not fill it yet; see the batch notes.
  const tombstoneEvent = useStateEvent(room, StateEvent.RoomTombstone);
  const tombstoneSuccessorRoomId =
    useNativeRoomSummary(roomId)?.tombstoneSuccessorRoomId ??
    tombstoneEvent?.getContent().replacement_room;
  const powerLevels = usePowerLevelsContext();

  const permissions = useRoomPermissions(powerLevels);
  const canMessage = permissions.event('m.room.message');

  useKeyDown(
    window,
    useCallback((evt) => {
      if (editableActiveElement()) return;
      const portalContainer = document.getElementById('portalContainer');
      if (portalContainer && portalContainer.children.length > 0) {
        return;
      }
      const editor = editorRef.current;
      if (!editor) return;
      if (shouldFocusMessageField(evt) || isKeyHotkey('mod+v', evt)) {
        ReactEditor.focus(editor);
      }
    }, [])
  );

  return (
    <Page ref={roomViewRef}>
      <Box grow="Yes" direction="Column" style={{ minHeight: 0 }}>
        {room.isCallRoom() && <VoiceRoom />}
        <NativeTimelinePresenter
          key={roomId}
          roomId={roomId}
          eventId={eventId}
          threadRootEventId={threadRootEventId}
          onOpenThreadRoute={(rootEventId) => navigateThread(roomId, rootEventId)}
          onCloseThreadRoute={() => navigateRoom(roomId)}
          threadDisplay={threadDisplay}
          onOpenThreadPane={onOpenThreadPane}
          threadPaneRootEventId={threadPaneRootEventId}
        />
        <RoomViewTyping room={room} />
      </Box>
      <Box shrink="No" direction="Column">
        <div style={{ padding: `0 ${config.space.S400}` }}>
          {tombstoneSuccessorRoomId ? (
            <RoomTombstone
              roomId={roomId}
              body={tombstoneEvent?.getContent().body}
              replacementRoomId={tombstoneSuccessorRoomId}
            />
          ) : (
            <>
              <RoomIdentityWarningBanner roomId={roomId} />
              {canMessage && (
                <ComposerSafetyNet
                  surface="room"
                  editorRef={editorRef}
                  keepDraft={keepComposerDraft}
                >
                  {({ editor, restore }) => (
                    <RoomInput
                      room={room}
                      editor={editor}
                      restore={restore}
                      roomId={roomId}
                      ref={roomInputRef}
                    />
                  )}
                </ComposerSafetyNet>
              )}
              {!canMessage && (
                <RoomInputPlaceholder
                  style={{ padding: config.space.S200 }}
                  alignItems="Center"
                  justifyContent="Center"
                >
                  <Text align="Center">You do not have permission to post in this room</Text>
                </RoomInputPlaceholder>
              )}
            </>
          )}
        </div>
        {hideActivity ? <RoomViewFollowingPlaceholder /> : <RoomViewFollowing />}
      </Box>
    </Page>
  );
}
