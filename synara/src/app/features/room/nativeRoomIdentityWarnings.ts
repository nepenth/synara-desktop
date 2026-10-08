import { invokeDesktopWithAvailability } from '../../utils/desktop';
import type {
  NativeIdentityWarningAction,
  NativeIdentityWarningKind,
  NativeRoomIdentityWarning,
  NativeRoomIdentityWarnings,
} from '../matrix-dto/generated';

/** Identity changes arrive with key queries, not timeline events; a slow poll is enough. */
export const ROOM_IDENTITY_WARNINGS_POLL_MS = 30_000;

export type RoomIdentityBanner = {
  userId: string;
  /** Fixed copy with the member's display name, or their user id. */
  message: string;
  kind: NativeIdentityWarningKind;
  /** `dismiss` pins a changed identity; `withdraw_verification` drops an old verification. */
  action: NativeIdentityWarningAction;
  actionLabel: 'Dismiss' | 'Withdraw verification';
};

const isWarningKind = (value: unknown): value is NativeIdentityWarningKind =>
  value === 'verification_violation' || value === 'pin_violation';

const parseWarning = (value: unknown): NativeRoomIdentityWarning | undefined => {
  if (typeof value !== 'object' || value === null) return undefined;
  const { userId, displayName, kind } = value as Record<string, unknown>;
  if (typeof userId !== 'string' || !userId.startsWith('@') || !isWarningKind(kind)) {
    return undefined;
  }
  return {
    userId,
    displayName: typeof displayName === 'string' && displayName.trim() ? displayName : undefined,
    kind,
  };
};

/** Parse a Core readback, keeping only closed kinds and well-formed user ids. */
export const parseRoomIdentityWarnings = (
  roomId: string,
  value: unknown
): NativeRoomIdentityWarning[] => {
  if (typeof value !== 'object' || value === null) return [];
  const readback = value as Partial<NativeRoomIdentityWarnings>;
  if (readback.roomId !== roomId || !Array.isArray(readback.warnings)) return [];
  return readback.warnings
    .map(parseWarning)
    .filter((warning): warning is NativeRoomIdentityWarning => Boolean(warning));
};

/** The banner for the most serious warning, or undefined when nothing needs attention. */
export const roomIdentityBanner = (
  warnings: NativeRoomIdentityWarning[]
): RoomIdentityBanner | undefined => {
  const warning =
    warnings.find((candidate) => candidate.kind === 'verification_violation') ?? warnings[0];
  if (!warning) return undefined;
  const name = warning.displayName ?? warning.userId;
  if (warning.kind === 'verification_violation') {
    return {
      userId: warning.userId,
      message: `${name}'s verified identity changed.`,
      kind: warning.kind,
      action: 'withdraw_verification',
      actionLabel: 'Withdraw verification',
    };
  }
  return {
    userId: warning.userId,
    message: `${name}'s identity changed.`,
    kind: warning.kind,
    action: 'dismiss',
    actionLabel: 'Dismiss',
  };
};

export const loadRoomIdentityWarnings = async (
  roomId: string
): Promise<NativeRoomIdentityWarning[] | undefined> => {
  const result = await invokeDesktopWithAvailability<unknown>('matrix_room_identity_warnings', {
    roomId,
  });
  if (!result.available) return undefined;
  return parseRoomIdentityWarnings(roomId, result.value);
};

export const resolveRoomIdentityWarning = async (
  roomId: string,
  userId: string,
  action: NativeIdentityWarningAction
): Promise<NativeRoomIdentityWarning[]> => {
  const result = await invokeDesktopWithAvailability<unknown>(
    'matrix_room_identity_warning_resolve',
    { roomId, userId, action }
  );
  if (!result.available) throw new Error('Native identity warnings are unavailable.');
  return parseRoomIdentityWarnings(roomId, result.value);
};
