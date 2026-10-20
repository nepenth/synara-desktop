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

/** Shown when Synara cannot tell whether anyone's identity changed. */
export const ROOM_IDENTITY_CHECK_FAILED_COPY =
  "Couldn't check this room for identity changes. Synara will keep trying.";

const isRoomIdentityReadback = (roomId: string, value: unknown): boolean =>
  typeof value === 'object' &&
  value !== null &&
  (value as Partial<NativeRoomIdentityWarnings>).roomId === roomId &&
  Array.isArray((value as Partial<NativeRoomIdentityWarnings>).warnings);

export const loadRoomIdentityWarnings = async (
  roomId: string
): Promise<NativeRoomIdentityWarning[] | undefined> => {
  const result = await invokeDesktopWithAvailability<unknown>('matrix_room_identity_warnings', {
    roomId,
  });
  if (!result.available) return undefined;
  // A readback for another room or without a warnings list is a failed check,
  // not "no warnings": treating it as empty would hide a real warning.
  if (!isRoomIdentityReadback(roomId, result.value)) {
    throw new Error('Room identity warnings readback was malformed.');
  }
  return parseRoomIdentityWarnings(roomId, result.value);
};

export type RoomIdentityWarningsState = {
  warnings: NativeRoomIdentityWarning[];
  /** The latest check failed; `warnings` is the last known result. */
  checkFailed: boolean;
};

export const INITIAL_ROOM_IDENTITY_WARNINGS_STATE: RoomIdentityWarningsState = {
  warnings: [],
  checkFailed: false,
};

/**
 * Fold one load outcome into the banner state. Security UI fails closed: a
 * failed load keeps the last known warnings and flags the check as failed
 * instead of clearing what the user was being warned about.
 */
export const applyRoomIdentityWarningsLoad = (
  state: RoomIdentityWarningsState,
  outcome: { ok: true; warnings: NativeRoomIdentityWarning[] } | { ok: false }
): RoomIdentityWarningsState =>
  outcome.ok
    ? { warnings: outcome.warnings, checkFailed: false }
    : { warnings: state.warnings, checkFailed: true };

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
