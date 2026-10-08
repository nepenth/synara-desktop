import { useParams } from 'react-router-dom';
import { getCanonicalAliasRoomId, isRoomAlias } from '../../utils/matrix';

export const useSelectedRoom = (): string | undefined => {
  const { roomIdOrAlias } = useParams();
  const roomId =
    roomIdOrAlias && isRoomAlias(roomIdOrAlias)
      ? getCanonicalAliasRoomId(roomIdOrAlias)
      : roomIdOrAlias;

  return roomId;
};
