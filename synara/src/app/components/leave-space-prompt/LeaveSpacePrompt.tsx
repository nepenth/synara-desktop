import React, { useCallback, useEffect } from 'react';
import { ConfirmDialog } from '../confirm-dialog';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { leaveRoomWithNativeOwner } from '../nativeRoomLeaveOwner';
import { composerErrorMessage } from '../../features/room/composerSendError';
import { getNativeRoom } from '../../native/nativeSession';

type LeaveSpacePromptProps = {
  roomId: string;
  onDone: () => void;
  onCancel: () => void;
};

/** "Leave Studio?": the shared confirm pattern, named after the space. */
export function LeaveSpacePrompt({ roomId, onDone, onCancel }: LeaveSpacePromptProps) {
  const spaceName = getNativeRoom(roomId)?.name?.trim() || 'this space';
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
      title={`Leave ${spaceName}?`}
      description="Its rooms stay joined, but the space disappears from your sidebar until you are invited again."
      confirmLabel="Leave"
      busyLabel="Leaving…"
      tone="critical"
      busy={leaveState.status === AsyncStatus.Loading || leaveState.status === AsyncStatus.Success}
      error={
        leaveState.status === AsyncStatus.Error
          ? composerErrorMessage(leaveState.error, 'Synara could not leave the space. Try again.')
          : undefined
      }
      onConfirm={() => leaveRoom()}
      onCancel={onCancel}
    />
  );
}
