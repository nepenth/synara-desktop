import { useCallback, useEffect, useSyncExternalStore } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  clearReplyDraftWithNativeComposerOwner,
  getReplyDraftWithNativeComposerOwner,
  NativeComposerReplyDraftProjection,
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
const mutationRevisionByRoom = new Map<string, number>();

const mutationRevision = (roomId: string): number => mutationRevisionByRoom.get(roomId) ?? 0;

const beginMutation = (roomId: string): number => {
  const revision = mutationRevision(roomId) + 1;
  mutationRevisionByRoom.set(roomId, revision);
  return revision;
};

const applyReadback = (
  result: NativeComposerReplyDraftReadback | 'unavailable'
): NativeComposerReplyDraftReadback | 'unavailable' => {
  if (result !== 'unavailable') projection.apply(result);
  return result;
};

export const setNativeComposerReplyDraft = async (
  input: NativeComposerSetReplyDraftInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const revision = beginMutation(input.roomId);
  const result = await setReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  return mutationRevision(input.roomId) === revision ? applyReadback(result) : result;
};

export const clearNativeComposerReplyDraft = async (
  input: NativeComposerClearReplyDraftInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const revision = beginMutation(input.roomId);
  const result = await clearReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  return mutationRevision(input.roomId) === revision ? applyReadback(result) : result;
};

export const getNativeComposerReplyDraft = async (
  input: NativeComposerReplyDraftRoomInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> => {
  const revision = mutationRevision(input.roomId);
  const result = await getReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);
  // A get that began before a set/clear must never overwrite the mutation's
  // newer authoritative readback when IPC responses complete out of order.
  if (mutationRevision(input.roomId) !== revision) return result;
  if (result === 'unavailable') {
    projection.clearLocal(input.roomId);
    return result;
  }
  return applyReadback(result);
};

/** One UI projection shared by the timeline banner and every send route. */
export const useNativeComposerReplyDraft = (
  roomId: string
): NativeComposerReplyDraft | undefined => {
  const subscribe = useCallback(
    (listener: () => void) => projection.subscribe(roomId, listener),
    [roomId]
  );
  const getSnapshot = useCallback(() => projection.get(roomId), [roomId]);
  const draft = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  useEffect(() => {
    void getNativeComposerReplyDraft({ roomId });
  }, [roomId]);

  return draft;
};

export { nativeComposerSendRelation } from './nativeComposerDraftOwner';
