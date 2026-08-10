import { useCallback, useMemo } from 'react';
import type { MatrixClientReading } from '../utils/room';
import type { EventedRoomReading } from '../utils/roomEvents';
import { getPowerLevelTag, PowerLevelTags, usePowerLevelTags } from './usePowerLevelTags';
import { IPowerLevels, readPowerLevel } from './usePowerLevels';
import { MemberPowerTag, MemberPowerTagIcon } from '../../types/matrix/room';
import { useRoomCreatorsTag } from './useRoomCreatorsTag';
import { ThemeKind } from './useTheme';
import { accessibleColor } from '../plugins/color';
import { resolveMatrixMediaUrl } from '../matrix/media';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';

type JsRoomMemberReading = { userId: string; membership?: string; rawDisplayName?: string };
type RoomMemberListItem = JsRoomMemberReading | NativeRoomMember;

export type GetMemberPowerTag = (userId: string) => MemberPowerTag;

export const useGetMemberPowerTag = (
  room: EventedRoomReading,
  creators: Set<string>,
  powerLevels: IPowerLevels
) => {
  const creatorsTag = useRoomCreatorsTag();
  const powerLevelTags = usePowerLevelTags(room, powerLevels);

  const getMemberPowerTag: GetMemberPowerTag = useCallback(
    (userId) => {
      if (creators.has(userId)) {
        return creatorsTag;
      }

      const power = readPowerLevel.user(powerLevels, userId);
      return getPowerLevelTag(powerLevelTags, power);
    },
    [creators, creatorsTag, powerLevels, powerLevelTags]
  );

  return getMemberPowerTag;
};

export const getPowerTagIconSrc = (
  mx: MatrixClientReading,
  useAuthentication: boolean,
  icon: MemberPowerTagIcon
): string | undefined => {
  if (!icon?.key?.startsWith('mxc://')) return icon?.key;

  try {
    return resolveMatrixMediaUrl(mx, icon.key, {
      useAuthentication,
      width: 96,
      height: 96,
      resizeMethod: 'scale',
    });
  } catch {
    return '🌻';
  }
};

export const useAccessiblePowerTagColors = (
  themeKind: ThemeKind,
  creatorsTag: MemberPowerTag,
  powerLevelTags: PowerLevelTags
): Map<string, string> => {
  const accessibleColors: Map<string, string> = useMemo(() => {
    const colors: Map<string, string> = new Map();
    if (creatorsTag.color) {
      colors.set(creatorsTag.color, accessibleColor(themeKind, creatorsTag.color));
    }

    Object.values(powerLevelTags).forEach((tag) => {
      const { color } = tag;
      if (!color) return;

      colors.set(color, accessibleColor(themeKind, color));
    });

    return colors;
  }, [powerLevelTags, creatorsTag, themeKind]);

  return accessibleColors;
};

export const useFlattenPowerTagMembers = <T extends RoomMemberListItem>(
  members: T[],
  getTag: GetMemberPowerTag
): Array<MemberPowerTag | T> => {
  const PLTagOrRoomMember = useMemo(() => {
    let prevTag: MemberPowerTag | undefined;
    const tagOrMember: Array<MemberPowerTag | T> = [];
    members.forEach((member) => {
      const tag = getTag(member.userId);
      if (tag !== prevTag) {
        prevTag = tag;
        tagOrMember.push(tag);
      }
      tagOrMember.push(member);
    });
    return tagOrMember;
  }, [members, getTag]);

  return PLTagOrRoomMember;
};
