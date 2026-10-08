import type { EventedRoomReading } from '../../utils/roomEvents';
import React, { useCallback, useMemo } from 'react';

import { usePowerLevels } from '../../hooks/usePowerLevels';
import { ImagePackContent } from './ImagePackContent';
import { ImagePack, PackContent } from '../../plugins/custom-emoji';
import { StateEvent } from '../../../types/matrix/room';
import { useRoomImagePack } from '../../hooks/useImagePacks';
import { randomStr } from '../../utils/common';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { setRoomImagePackNative } from '../../features/room/nativeImagePack';

import { sendNativeStateEvent } from '../../native/nativeCommands';
type RoomImagePackProps = {
  room: EventedRoomReading;
  stateKey: string;
};

export function RoomImagePack({ room, stateKey }: RoomImagePackProps) {
  const powerLevels = usePowerLevels(room);

  const permissions = useRoomPermissions(powerLevels);
  const canEditImagePack = permissions.stateEvent(StateEvent.PoniesRoomEmotes);

  const fallbackPack = useMemo(() => {
    const fakePackId = randomStr(4);
    return new ImagePack(
      fakePackId,
      {},
      {
        roomId: room.roomId,
        stateKey,
      }
    );
  }, [room.roomId, stateKey]);
  const imagePack = useRoomImagePack(room, stateKey) ?? fallbackPack;

  const handleUpdate = useCallback(
    async (packContent: PackContent) => {
      const { address } = imagePack;
      if (!address) return;

      // V-SEND.R-PACK-WRITE: native room-pack update is fail-closed on desktop.
      // The JS sendNativeStateEvent(PoniesRoomEmotes) path is only for non-native
      // web.
      const result = await setRoomImagePackNative(address.roomId, address.stateKey, packContent);
      if (result === 'legacy') {
        await sendNativeStateEvent(
          address.roomId,
          StateEvent.PoniesRoomEmotes as any,
          packContent,
          address.stateKey
        );
      }
    },
    [imagePack]
  );

  return (
    <ImagePackContent imagePack={imagePack} canEdit={canEditImagePack} onUpdate={handleUpdate} />
  );
}
