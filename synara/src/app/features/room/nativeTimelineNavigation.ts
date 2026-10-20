/** Native viewport intent after a successful composer send; contains no Matrix write. */
const eventName = 'synara:room-message-sent';

/** Where a composer message went: the room's main timeline, or one thread in it. */
export type RoomMessageSentTarget = {
  roomId: string;
  threadRootId?: string;
};

/**
 * True when a timeline view shows the timeline a message was sent into. A
 * room view ignores thread sends and a thread view ignores sends to the room
 * or to another thread, so a send never moves or closes an unrelated view.
 */
export const roomMessageSentTargetsView = (
  target: RoomMessageSentTarget,
  roomId: string,
  threadRootId: string | undefined
): boolean => target.roomId === roomId && (target.threadRootId || undefined) === threadRootId;

export const requestRoomLatestAfterSend = (roomId: string, threadRootId?: string): void => {
  window.dispatchEvent(
    new CustomEvent<RoomMessageSentTarget>(eventName, { detail: { roomId, threadRootId } })
  );
};

/**
 * Call `navigate` after a send into the timeline this view shows. A thread
 * view passes its root; it should scroll to its own tail, never leave the
 * thread.
 */
export const observeRoomLatestAfterSend = (
  roomId: string,
  threadRootId: string | undefined,
  navigate: () => void
): (() => void) => {
  const listener = (event: Event) => {
    if (!(event instanceof CustomEvent)) return;
    const target = event.detail as RoomMessageSentTarget | undefined;
    if (target && roomMessageSentTargetsView(target, roomId, threadRootId || undefined)) navigate();
  };
  window.addEventListener(eventName, listener);
  return () => window.removeEventListener(eventName, listener);
};
