import { IPowerLevels } from '../hooks/usePowerLevels';
import { creatorsSupported, getMxIdServer } from '../utils/matrix';
import { IRoomCreateContent, StateEvent } from '../../types/matrix/room';
import { getStateEvent } from '../utils/room';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import { readRoomMembersWithNativeOwner } from '../hooks/nativeRoomMembersOwner';
import {
  getNativeHighestPowerUserId,
  getNativeRoomStateProjection,
} from '../features/matrix-dto/nativeRoomStateProjection';

export type ViaServerRoom = Parameters<typeof getStateEvent>[0];
export type ViaServerMember = { userId: string };
export type NativeViaServerMemberReader = (
  roomId: string,
  nativeSession: boolean
) => Promise<readonly ViaServerMember[] | undefined>;

const getLegacyHighestPowerUserId = (room: ViaServerRoom): string | undefined => {
  const creatorEvent = getStateEvent(room, StateEvent.RoomCreate);
  if (
    creatorEvent &&
    creatorsSupported(creatorEvent.getContent<IRoomCreateContent>().room_version)
  ) {
    return creatorEvent.getSender();
  }

  const powerLevels = getStateEvent(room, StateEvent.RoomPowerLevels)?.getContent<IPowerLevels>();

  if (!powerLevels) return undefined;
  const userIdToPower = powerLevels.users;
  if (!userIdToPower) return undefined;
  let powerUserId: string | undefined;

  Object.keys(userIdToPower).forEach((userId) => {
    if (userIdToPower[userId] <= (powerLevels.users_default ?? 0)) return;

    if (!powerUserId) {
      powerUserId = userId;
      return;
    }
    if (userIdToPower[userId] > userIdToPower[powerUserId]) {
      powerUserId = userId;
    }
  });
  return powerUserId;
};

export const getViaServersForMembers = (
  highestPowerUserId: string | undefined,
  members: readonly ViaServerMember[]
): string[] => {
  const serverToPopulation = new Map<string, number>();

  members.forEach(({ userId }) => {
    const server = getMxIdServer(userId);
    if (!server) return;
    serverToPopulation.set(server, (serverToPopulation.get(server) ?? 0) + 1);
  });

  const sortedServers = Array.from(serverToPopulation.entries())
    .sort(([, populationA], [, populationB]) => populationB - populationA)
    .map(([server]) => server);
  const mostPop3 = sortedServers.slice(0, 3);

  const via: string[] = [];
  if (highestPowerUserId) {
    const server = getMxIdServer(highestPowerUserId);
    if (server) via.push(server);
  }
  if (via.length === 0) return mostPop3;
  if (mostPop3.includes(via[0])) {
    mostPop3.splice(mostPop3.indexOf(via[0]), 1);
  }
  return via.concat(mostPop3.slice(0, 2));
};

export const getViaServers = async (
  room: ViaServerRoom,
  readNativeMembers: NativeViaServerMemberReader = readRoomMembersWithNativeOwner
): Promise<string[]> => {
  const nativeSession = isNativeMatrixSession();
  const highestPowerUserId = nativeSession
    ? getNativeHighestPowerUserId(getNativeRoomStateProjection(room.roomId))
    : getLegacyHighestPowerUserId(room);

  let members: readonly ViaServerMember[];
  if (nativeSession) {
    const nativeMembers = await readNativeMembers(room.roomId, true);
    if (!nativeMembers) {
      throw new Error('Native Matrix room members are unavailable.');
    }
    members = nativeMembers.map(({ userId }) => ({ userId }));
  } else {
    members = room.getMembers()?.map(({ userId }) => ({ userId })) ?? [];
  }

  return getViaServersForMembers(highestPowerUserId, members);
};
