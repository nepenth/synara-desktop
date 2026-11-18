/**
 * When a composer's draft is written: coalesced while typing, at once when it
 * is emptied (sent or erased) and when the composer, room or window is left.
 * Pure, with the writes and the clock injected; RoomInput supplies
 * localStorage and the synced-draft mirror.
 */
import {
  createCoalescingScheduler,
  DRAFT_SAVE_DEBOUNCE_MS,
  DRAFT_SAVE_MAX_WAIT_MS,
  type SchedulerClock,
} from './coalescingScheduler';

/** Which draft a write belongs to, captured with the change. */
export type DraftTarget = {
  /** The local draft key: the room, or `room#thread:root`. */
  draftKey: string;
  roomId: string;
  /** The synced draft slot: `main` or `thread:<root>`. */
  slot: string;
};

export type DraftWriter<V> = {
  save: (target: DraftTarget, value: V, urgent: boolean) => void;
  clear: (target: DraftTarget) => void;
};

export type ComposerDraftPersistence<V> = {
  /**
   * The composer's content changed; `undefined` when it is empty. The value
   * is held by reference, never serialized here.
   */
  change: (target: DraftTarget, value: V | undefined) => void;
  /**
   * The composer is leaving this draft (room or thread switch, unmount):
   * write its latest content now, to this draft's key.
   */
  leave: (target: DraftTarget, value: V | undefined) => void;
  /** Write any pending change now (window blur, hidden, unload). */
  flush: () => void;
};

type Options<V> = {
  writer: DraftWriter<V>;
  clock?: SchedulerClock;
  delayMs?: number;
  maxWaitMs?: number;
};

export function createComposerDraftPersistence<V>({
  writer,
  clock,
  delayMs = DRAFT_SAVE_DEBOUNCE_MS,
  maxWaitMs = DRAFT_SAVE_MAX_WAIT_MS,
}: Options<V>): ComposerDraftPersistence<V> {
  /** The last content written for one key: a reference, or `null` for cleared. */
  let written: { draftKey: string; value: V | null } | undefined;
  const isWritten = (draftKey: string, value: V | null) =>
    written?.draftKey === draftKey && written.value === value;

  const scheduler = createCoalescingScheduler<{ target: DraftTarget; value: V }>({
    delayMs,
    maxWaitMs,
    clock,
    write: (draftKey, { target, value }, urgent) => {
      written = { draftKey, value };
      writer.save(target, value, urgent);
    },
  });

  const clear = (target: DraftTarget) => {
    // A pending save of the old text must never land after the clear.
    scheduler.cancel(target.draftKey);
    if (isWritten(target.draftKey, null)) return;
    written = { draftKey: target.draftKey, value: null };
    writer.clear(target);
  };

  return {
    change: (target, value) => {
      if (value === undefined) clear(target);
      else scheduler.schedule(target.draftKey, { target, value });
    },
    leave: (target, value) => {
      if (value === undefined) {
        // Write another key's pending change first, then this one's clear.
        if (scheduler.pendingKey() !== target.draftKey) scheduler.flush(true);
        clear(target);
      } else {
        if (!isWritten(target.draftKey, value) || scheduler.pendingKey() === target.draftKey) {
          scheduler.schedule(target.draftKey, { target, value });
        }
        scheduler.flush(true);
      }
      // The stored draft may change while it is not shown (a failed send
      // restores into it), so nothing is assumed about it on return.
      written = undefined;
    },
    flush: () => scheduler.flush(true),
  };
}
