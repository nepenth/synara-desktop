import { Room } from 'matrix-js-sdk';
import { useEffect, useMemo, useState } from 'react';
import { IPowerLevels } from './usePowerLevels';
import { useStateEvent } from './useStateEvent';
import { MemberPowerTag, StateEvent } from '../../types/matrix/room';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import { readRoomPowerLevelTagsWithNativeOwner } from './nativeRoomPowerLevelTagsOwner';

export type PowerLevelTags = Record<number, MemberPowerTag>;

/** Native tag metadata is unavailable until its validated snapshot arrives. */
export const NATIVE_UNAVAILABLE_POWER_LEVEL_TAGS: PowerLevelTags = {};

const powerSortFn = (a: number, b: number) => b - a;
const sortPowers = (powers: number[]): number[] => powers.sort(powerSortFn);

export const getPowers = (tags: PowerLevelTags): number[] => {
  const powers: number[] = Object.keys(tags)
    .map((p) => {
      const power = parseInt(p, 10);
      if (Number.isNaN(power)) {
        return undefined;
      }
      return power;
    })
    .filter((power) => typeof power === 'number');

  return sortPowers(powers);
};

export const getUsedPowers = (powerLevels: IPowerLevels): Set<number> => {
  const powers: Set<number> = new Set();

  const findAndAddPower = (data: Record<string, unknown>) => {
    Object.keys(data).forEach((key) => {
      const powerOrAny: unknown = data[key];

      if (typeof powerOrAny === 'number') {
        powers.add(powerOrAny);
        return;
      }
      if (powerOrAny && typeof powerOrAny === 'object') {
        findAndAddPower(powerOrAny as Record<string, unknown>);
      }
    });
  };

  findAndAddPower(powerLevels);

  return powers;
};

const DEFAULT_TAGS: PowerLevelTags = {
  9001: {
    name: 'Goku',
    color: '#ff6a00',
  },
  150: {
    name: 'Manager',
    color: '#ff6a7f',
  },
  101: {
    name: 'Founder',
    color: '#0000ff',
  },
  100: {
    name: 'Admin',
    color: '#0088ff',
  },
  50: {
    name: 'Moderator',
    color: '#1fd81f',
  },
  0: {
    name: 'Member',
    color: '#91cfdf',
  },
  [-1]: {
    name: 'Muted',
    color: '#888888',
  },
};

const generateFallbackTag = (powerLevelTags: PowerLevelTags, power: number): MemberPowerTag => {
  const highToLow = sortPowers(getPowers(powerLevelTags));

  const tagPower = highToLow.find((p) => p < power);
  const tag = typeof tagPower === 'number' ? powerLevelTags[tagPower] : undefined;

  return {
    name: tag ? `${tag.name} ${power}` : `Team ${power}`,
  };
};

export const usePowerLevelTags = (room: Room, powerLevels: IPowerLevels): PowerLevelTags => {
  const nativeSession = isNativeMatrixSession();
  const tagsEvent = useStateEvent(room, StateEvent.PowerLevelTags, '', !nativeSession);
  const [nativeState, setNativeState] = useState<
    | { roomId: string; status: 'idle' | 'loading' }
    | { roomId: string; status: 'ready'; content: PowerLevelTags }
    | { roomId: string; status: 'error'; error: Error }
  >({ roomId: room.roomId, status: 'idle' });

  useEffect(() => {
    if (!nativeSession) return undefined;

    let disposed = false;
    setNativeState({ roomId: room.roomId, status: 'loading' });
    void readRoomPowerLevelTagsWithNativeOwner(room.roomId, true)
      .then((snapshot) => {
        if (!disposed && snapshot) {
          setNativeState({
            roomId: room.roomId,
            status: 'ready',
            content: snapshot.content as PowerLevelTags,
          });
        }
      })
      .catch((error: unknown) => {
        if (!disposed) {
          setNativeState({
            roomId: room.roomId,
            status: 'error',
            error:
              error instanceof Error
                ? error
                : new Error('Native Matrix room power-level tags are unavailable.'),
          });
        }
      });

    return () => {
      disposed = true;
    };
  }, [nativeSession, room.roomId]);

  const powerLevelTags: PowerLevelTags = useMemo(() => {
    if (nativeSession) {
      if (nativeState.status === 'error') throw nativeState.error;
      if (nativeState.roomId !== room.roomId || nativeState.status !== 'ready') {
        return NATIVE_UNAVAILABLE_POWER_LEVEL_TAGS;
      }
    }

    const content = nativeSession
      ? nativeState.status === 'ready'
        ? nativeState.content
        : undefined
      : tagsEvent?.getContent<PowerLevelTags>();
    const powerToTags: PowerLevelTags = { ...content };

    const powers = getUsedPowers(powerLevels);
    Array.from(powers).forEach((power) => {
      if (powerToTags[power]?.name === undefined) {
        powerToTags[power] = DEFAULT_TAGS[power] ?? generateFallbackTag(DEFAULT_TAGS, power);
      }
    });

    return powerToTags;
  }, [nativeSession, nativeState, powerLevels, room.roomId, tagsEvent]);

  return powerLevelTags;
};

export const getPowerLevelTag = (
  powerLevelTags: PowerLevelTags,
  powerLevel: number
): MemberPowerTag => {
  const tag: MemberPowerTag | undefined = powerLevelTags[powerLevel];
  return tag ?? generateFallbackTag(powerLevelTags, powerLevel);
};
