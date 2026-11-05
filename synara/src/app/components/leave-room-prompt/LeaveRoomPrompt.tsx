import React, { useCallback, useEffect } from 'react';
import { ConfirmDialog } from '../confirm-dialog';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { leaveRoomWithNativeOwner } from '../nativeRoomLeaveOwner';
import { composerErrorMessage } from '../../features/room/composerSendError';
import { getNativeRoom } from '../../native/nativeSession';

type LeaveRoomPromptProps = {
  roomId: string;
  onDone: () => void;
  onCancel: () => void;
};

/** "Leave #design?": the shared confirm pattern, named after the room. */
export function LeaveRoomPrompt({ roomId, onDone, onCancel }: LeaveRoomPromptProps) {
  const room = getNativeRoom(roomId);
  const name = room?.name?.trim();
  // Rooms read as #name; direct messages and unknown rooms keep the plain label.
  const title = name ? `Leave ${room?.isDirect ? name : `#${name}`}?` : 'Leave this room?';
  const [leaveState, leaveRoom] = useAsyncCallback<void, Error, []>(
    useCallback(
      () => leaveRoomWithNativeOwner(roomId, isSynaraDesktop(), invokeDesktopWithAvailability),
      [roomId]
    )
  );

  useEffect(() => {
    if (leaveState.status === AsyncStatus.Success) {
      onDone();
    }
  }, [leaveState, onDone]);

  return (
    <ConfirmDialog
      title={title}
      description="You stop receiving its messages and need a new invite to come back if the room is private."
      confirmLabel="Leave"
      busyLabel="Leaving…"
      tone="critical"
      busy={leaveState.status === AsyncStatus.Loading || leaveState.status === AsyncStatus.Success}
      error={
        leaveState.status === AsyncStatus.Error
          ? composerErrorMessage(leaveState.error, 'Synara could not leave the room. Try again.')
          : undefined
      }
      onConfirm={() => leaveRoom()}
      onCancel={onCancel}
    />
  );
}
