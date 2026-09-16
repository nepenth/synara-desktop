import { useCallback, useEffect, useSyncExternalStore } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  clearReplyDraftWithNativeComposerOwner,
  getReplyDraftWithNativeComposerOwner,
  NativeComposerReplyDraftProjection,
  nativeComposerDraftSlotKey,
  setReplyDraftWithNativeComposerOwner,
  type NativeComposerClearReplyDraftInput,
  type NativeComposerReplyDraft,
  type NativeComposerReplyDraftReadback,
  type NativeComposerReplyDraftRoomInput,
  type NativeComposerSetReplyDraftInput,
} from './nativeComposerDraftOwner';

const invoke: Parameters<typeof setReplyDraftWithNativeComposerOwner>[2] = (command, args) =>
  invokeDesktopWithAvailability(command, args);

const projection = new NativeComposerReplyDraftProjection();
const mutationRevisionBySlot = new Map<string, number>();

const mutationRevision = (roomId: string, threadRootEventId?: string): number =>
  mutationRevisionBySlot.get(nativeComposerDraftSlotKey(roomId, threadRootEventId)) ?? 0;

const beginMutation = (roomId: string, threadRootEventId?: string): number => {
  const key = nativeComposerDraftSlotKey(roomId, threadRootEventId);
  const revision = (mutationRevisionBySlot.get(key) ?? 0) + 1;
  mutationRevisionBySlot.set(key, revision);
  return revision;
};

const applyReadback = (
  result: NativeComposerReplyDraftReadback | 'unavailable',
  slotThreadRoot?: string
): NativeComposerReplyDraftReadback | 'unavailable' => {
  if (result !== 'unavailable') projection.apply(result, slotThreadRoot);
  return result;
};

export const setNativeComposerReplyDraft = async (
  input: NativeComposerSetReplyDraftInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const guessedSlot = input.startThread ? input.eventId : undefined;
  const revision = beginMutation(input.roomId, guessedSlot);
  const result = await setReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  const slotThreadRoot = result !== 'unavailable' ? result.draft?.threadRootEventId : guessedSlot;
  if (slotThreadRoot !== guessedSlot) {
    beginMutation(input.roomId, slotThreadRoot);
    return applyReadback(result, slotThreadRoot);
  }
  return mutationRevision(input.roomId, guessedSlot) === revision
    ? applyReadback(result, slotThreadRoot)
    : result;
};

export const clearNativeComposerReplyDraft = async (
  input: NativeComposerClearReplyDraftInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const revision = beginMutation(input.roomId, input.threadRootEventId);
  const result = await clearReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  return mutationRevision(input.roomId, input.threadRootEventId) === revision
    ? applyReadback(result, input.threadRootEventId)
    : result;
};

export const getNativeComposerReplyDraft = async (
  input: NativeComposerReplyDraftRoomInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const revision = mutationRevision(input.roomId, input.threadRootEventId);
  const result = await getReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  // A get that began before a set/clear must never overwrite the mutation's
  // newer authoritative readback when IPC responses complete out of order.
  if (mutationRevision(input.roomId, input.threadRootEventId) !== revision) return result;
  if (result === 'unavailable') {
    projection.clearLocal(input.roomId, input.threadRootEventId);
    return result;
  }
  return applyReadback(result, input.threadRootEventId);
};

/** One UI projection shared by the timeline banner and every send route. */
export const useNativeComposerReplyDraft = (
  roomId: string,
  threadRootEventId?: string
): NativeComposerReplyDraft | undefined => {
  const subscribe = useCallback(
    (listener: () => void) => projection.subscribe(roomId, listener, threadRootEventId),
    [roomId, threadRootEventId]
  );
  const getSnapshot = useCallback(
    () => projection.get(roomId, threadRootEventId),
    [roomId, threadRootEventId]
  );
  const draft = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    void getNativeComposerReplyDraft({ roomId, threadRootEventId });
  }, [roomId, threadRootEventId]);

  return draft;
};

export { nativeComposerSendRelation } from './nativeComposerDraftOwner';
