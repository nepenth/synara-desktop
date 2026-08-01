import { parseRoomMember, type RoomMember } from '../features/matrix-dto/member';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../utils/desktop';

export type NativeRoomMembersSnapshot = {
  sessionGeneration: number;
  roomId: string;
  members: RoomMember[];
};

export type NativeRoomMembersInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const defaultInvoke: NativeRoomMembersInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

const parseNativeRoomMembersSnapshot = (
  value: unknown,
  roomId: string
): NativeRoomMembersSnapshot | null => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (
    typeof record.sessionGeneration !== 'number' ||
    !Number.isFinite(record.sessionGeneration) ||
    record.roomId !== roomId ||
    !Array.isArray(record.members)
  ) {
    return null;
  }

  const members: RoomMember[] = [];
  for (const memberValue of record.members) {
    const member = parseRoomMember(memberValue);
    if (!member || member.roomId !== roomId) return null;
    members.push(member);
  }

  return {
    sessionGeneration: record.sessionGeneration,
    roomId,
    members,
  };
};

/**
 * Read the member-list owner for a native logged-in desktop session.
 *
 * `undefined` is reserved for the non-native route. Once native ownership is
 * selected, unavailable or malformed IPC is terminal and never falls through
 * to the matrix-js-sdk member read.
 */
export async function readRoomMembersWithNativeOwner(
  roomId: string,
  nativeSession: boolean,
  invoke: NativeRoomMembersInvoke = defaultInvoke
): Promise<RoomMember[] | undefined> {
  if (!nativeSession) return undefined;

  const result = await invoke('matrix_room_members_snapshot', { roomId });
  if (!result.available || result.value === undefined) {
    throw new Error('Native Matrix room members are unavailable.');
  }

  const snapshot = parseNativeRoomMembersSnapshot(result.value, roomId);
  if (!snapshot) {
    throw new Error('Native Matrix room members are unavailable.');
  }
  return snapshot.members;
}
