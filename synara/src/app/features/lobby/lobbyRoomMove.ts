import type { MSpaceChildContent } from '../../../types/matrix/room';
import type { BulkActionFailure } from '../../utils/bulkActions';

export type LobbyChildWrite = {
  roomId: string;
  content: MSpaceChildContent;
};

export type LobbyRoomMovePlan = {
  roomId: string;
  roomName: string;
  fromParentId: string;
  toParentId: string;
  toParentName: string;
  /** The room's join rule is `restricted`, so its allow list names its parent space. */
  restricted: boolean;
  /** `m.space.child` content for the moved room when it gets no new order key. */
  movedContent: MSpaceChildContent;
  /** New order keys in the target space; may include the moved room. */
  reorders: LobbyChildWrite[];
};

export type LobbyRoomMoveDeps = {
  setSpaceChild: (parentId: string, childId: string, content: MSpaceChildContent) => Promise<void>;
  removeSpaceChild: (parentId: string, childId: string) => Promise<void>;
  reparentRestrictedJoin: (
    roomId: string,
    removeParentId: string,
    addParentId: string
  ) => Promise<void>;
  /** Runs every write and returns the ones that failed (`rateLimitedActions`). */
  runWrites: (
    writes: LobbyChildWrite[],
    write: (item: LobbyChildWrite) => Promise<void>
  ) => Promise<BulkActionFailure<LobbyChildWrite>[]>;
};

/**
 * Move or reorder a room in the lobby. The room is added to the target space
 * before it is removed from its old one, so a failure leaves it listed in its
 * original space (or in both), never in none. Every failure throws an Error
 * whose message is user copy for the lobby.
 */
export const moveLobbyRoom = async (
  plan: LobbyRoomMovePlan,
  deps: LobbyRoomMoveDeps
): Promise<void> => {
  const crossParent = plan.fromParentId !== plan.toParentId;
  const movedWrite = plan.reorders.find((write) => write.roomId === plan.roomId);
  const siblingWrites = plan.reorders.filter((write) => write.roomId !== plan.roomId);

  if (crossParent || movedWrite) {
    try {
      await deps.setSpaceChild(
        plan.toParentId,
        plan.roomId,
        movedWrite?.content ?? plan.movedContent
      );
    } catch {
      throw new Error(
        crossParent
          ? `Couldn't move ${plan.roomName} to ${plan.toParentName}. It is still in its original space. Try again.`
          : `Couldn't reorder ${plan.roomName}. Try again.`
      );
    }
  }

  if (crossParent) {
    if (plan.restricted) {
      try {
        await deps.reparentRestrictedJoin(plan.roomId, plan.fromParentId, plan.toParentId);
      } catch {
        // Keep the old parent: its members may still rely on it to join.
        throw new Error(
          `${plan.roomName} was added to ${plan.toParentName}, but who can join it couldn't be updated, so it is also still in its original space. Try again.`
        );
      }
    }
    try {
      await deps.removeSpaceChild(plan.fromParentId, plan.roomId);
    } catch {
      throw new Error(
        `${plan.roomName} was added to ${plan.toParentName} but couldn't be removed from its original space, so it now appears in both. Try again.`
      );
    }
  }

  const failures = await deps.runWrites(siblingWrites, (write) =>
    deps.setSpaceChild(plan.toParentId, write.roomId, write.content)
  );
  if (failures.length > 0) {
    throw new Error(
      `${plan.roomName} was moved, but the order of ${failures.length} other ${
        failures.length === 1 ? 'room' : 'rooms'
      } in ${plan.toParentName} couldn't be saved. Try again.`
    );
  }
};
