import { RoomType } from '../../../types/matrix/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { setSpaceChild } from '../../features/lobby/nativeSpaceChild';
import { createRoomWithNativeOwner, type NativeRoomCreateRequest } from '../nativeRoomCreateOwner';
import { CreateRoomAccess } from './types';

export const createRoomCreationContent = (
  type: RoomType | undefined,
  allowFederation: boolean,
  additionalCreators: string[] | undefined
): NativeRoomCreateRequest['creationContent'] => {
  const content: NonNullable<NativeRoomCreateRequest['creationContent']> = {};
  if (typeof type === 'string') {
    content.type = type;
  }
  if (allowFederation === false) {
    content.federate = false;
  }
  if (Array.isArray(additionalCreators)) {
    content.additionalCreators = additionalCreators;
  }

  return content;
};

export const createRoomJoinRule = (
  access: CreateRoomAccess,
  parentRoomId: string | undefined,
  knock: boolean
): NativeRoomCreateRequest['joinRule'] => {
  if (access === CreateRoomAccess.Public) return 'public';
  if (access === CreateRoomAccess.Restricted && parentRoomId) return 'restricted';
  return 'invite';
};

export type CreateRoomData = {
  version: string;
  type?: RoomType;
  parentRoomId?: string;
  access: CreateRoomAccess;
  name: string;
  topic?: string;
  aliasLocalPart?: string;
  encryption?: boolean;
  knock: boolean;
  allowFederation: boolean;
  additionalCreators?: string[];
};
const getRoomIdServer = (roomId: string): string | undefined => {
  const separator = roomId.indexOf(':');
  return separator === -1 ? undefined : roomId.slice(separator + 1);
};

export const createRoom = async (data: CreateRoomData): Promise<string> => {
  const request: NativeRoomCreateRequest = {
    roomVersion: data.version,
    name: data.name,
    topic: data.topic,
    roomAliasName: data.aliasLocalPart,
    creationContent: createRoomCreationContent(
      data.type,
      data.allowFederation,
      data.additionalCreators
    ),
    encryption: data.encryption,
    joinRule: createRoomJoinRule(data.access, data.parentRoomId, data.knock),
    knock: data.access !== CreateRoomAccess.Public && data.knock,
    parentRoomId: data.parentRoomId,
    powerLevelContentOverride:
      data.type === RoomType.Space
        ? { eventsDefault: 50 }
        : data.type === RoomType.Call
        ? { events: { 'org.matrix.msc3401.call.member': 0 } }
        : undefined,
  };

  const roomId = await createRoomWithNativeOwner(request, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );

  if (data.parentRoomId) {
    const via = getRoomIdServer(roomId);
    await setSpaceChild(data.parentRoomId, roomId, {
      suggested: false,
      via: via ? [via] : [],
    });
  }

  return roomId;
};
