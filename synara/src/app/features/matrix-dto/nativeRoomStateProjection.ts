export type NativeRoomPowerLevelsProjection = Record<string, unknown>;

export type NativeRoomStateProjection = {
  sessionGeneration: number;
  creators?: readonly string[];
  powerLevels?: NativeRoomPowerLevelsProjection;
};

const roomProjections = new Map<string, NativeRoomStateProjection>();
let activeSessionGeneration: number | undefined;

const acceptSessionGeneration = (sessionGeneration: number): boolean => {
  if (activeSessionGeneration === undefined) {
    activeSessionGeneration = sessionGeneration;
  } else if (sessionGeneration > activeSessionGeneration) {
    roomProjections.clear();
    activeSessionGeneration = sessionGeneration;
  } else if (sessionGeneration < activeSessionGeneration) {
    return false;
  }

  return true;
};

/**
 * Establish the session generation before issuing a room read. A newer native
 * session invalidates every projection, even when its first room read fails.
 */
export const beginNativeRoomStateSession = (sessionGeneration: number): boolean =>
  acceptSessionGeneration(sessionGeneration);

export const publishNativeRoomPowerLevelsProjection = (
  roomId: string,
  sessionGeneration: number,
  content: NativeRoomPowerLevelsProjection
): void => {
  if (!acceptSessionGeneration(sessionGeneration)) return;
  const current = roomProjections.get(roomId);
  roomProjections.set(roomId, {
    sessionGeneration,
    ...(current?.creators ? { creators: current.creators } : {}),
    powerLevels: content,
  });
};

export const publishNativeRoomCreatorsProjection = (
  roomId: string,
  sessionGeneration: number,
  creators: readonly string[]
): void => {
  if (!acceptSessionGeneration(sessionGeneration)) return;
  const current = roomProjections.get(roomId);
  roomProjections.set(roomId, {
    sessionGeneration,
    creators: [...creators],
    ...(current?.powerLevels ? { powerLevels: current.powerLevels } : {}),
  });
};

/**
 * Read the latest validated native state outside React. An absent projection
 * is terminal for native direct readers; callers must not reopen JS state reads.
 */
export const getNativeRoomStateProjection = (
  roomId: string
): NativeRoomStateProjection | undefined => {
  const projection = roomProjections.get(roomId);
  if (projection?.sessionGeneration !== activeSessionGeneration) return undefined;
  return projection;
};

export const invalidateNativeRoomStateProjection = (roomId: string): void => {
  roomProjections.delete(roomId);
};

export const clearNativeRoomStateProjections = (): void => {
  roomProjections.clear();
  activeSessionGeneration = undefined;
};

export const getNativeHighestPowerUserId = (
  projection: NativeRoomStateProjection | undefined
): string | undefined => {
  const creator = projection?.creators?.[0];
  if (creator) return creator;

  const powerLevels = projection?.powerLevels;
  if (!powerLevels) return undefined;

  const usersDefault =
    typeof powerLevels.users_default === 'number' ? powerLevels.users_default : 0;
  const users = powerLevels.users;
  if (!users || typeof users !== 'object' || Array.isArray(users)) return undefined;
  const userMap = users as Record<string, unknown>;

  let powerUserId: string | undefined;
  Object.keys(userMap).forEach((userId) => {
    const power = userMap[userId];
    if (typeof power !== 'number' || power <= usersDefault) return;
    const previousPower = powerUserId ? userMap[powerUserId] : undefined;
    if (!powerUserId || (typeof previousPower === 'number' && power > previousPower)) {
      powerUserId = userId;
    }
  });
  return powerUserId;
};

export const getNativeSpecialUsers = (
  projection: NativeRoomStateProjection | undefined
): string[] => {
  if (!projection) return [];

  const specialUsers = new Set(projection.creators ?? []);
  const powerLevels = projection.powerLevels;
  if (!powerLevels) return Array.from(specialUsers);

  const usersDefault =
    typeof powerLevels.users_default === 'number' ? powerLevels.users_default : 0;
  const users = powerLevels.users;
  if (!users || typeof users !== 'object' || Array.isArray(users)) {
    return Array.from(specialUsers);
  }
  const userMap = users as Record<string, unknown>;

  Object.keys(userMap).forEach((userId) => {
    const power = userMap[userId];
    if (typeof power === 'number' && power > usersDefault) specialUsers.add(userId);
  });
  return Array.from(specialUsers);
};
