import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Descendant, Editor, Transforms } from 'slate';
import { ReactEditor } from 'slate-react';
import { isEmptyEditor, resetEditor, resetEditorHistory } from '../../components/editor';
import {
  draftMarkdownToEditorInput,
  toDraftMarkdown,
} from '../../components/editor/composerMarkdown';
import { composerContentIsEmpty } from './composerSendQueue';
import { isSynaraDesktop } from '../../utils/desktop';
import {
  clearSyncedDraft,
  draftSlotFor,
  draftSyncStatus,
  flushSyncedDrafts,
  isSlateValue,
  resolveSyncedDraft,
  saveSyncedDraft,
  setDraftSyncEnabled,
  subscribeSyncedDrafts,
  type RemoteDraft,
} from './nativeDraftSync';

/** Typing this recently counts as "being typed in" even without focus. */
const ACTIVE_TYPING_MS = 3_000;

/** What a remote draft puts in the composer: desktop's rich value, else its markdown. */
const remoteDraftNodes = async (draft: RemoteDraft, isMarkdown: boolean): Promise<Descendant[]> =>
  isSlateValue(draft.desktopSlate)
    ? (draft.desktopSlate as Descendant[])
    : draftMarkdownToEditorInput(draft.markdown ?? '', isMarkdown);

/**
 * The draft's text as other devices see it, and the text compared to tell a
 * local edit from an applied one: Markdown, as iOS keeps drafts.
 */
export const draftMarkdownOf = (nodes: Descendant[], isMarkdown: boolean): string =>
  toDraftMarkdown(nodes, isMarkdown).trim();

type SyncedRoomDraftOptions = {
  editor: Editor;
  roomId: string;
  threadRootOverride?: string;
  isMarkdown: boolean;
  enabled: boolean;
  /** The reply target this composer shows, synced with the text. */
  replyToEventId?: string;
  /** When the locally stored draft last changed, for the first comparison. */
  storedUpdatedAt?: number;
  /**
   * The text this composer opens with. Restoring it is not an edit: saving
   * it with a fresh stamp would bury a newer draft from another device.
   */
  storedMarkdown: string;
  /** Put a peer's reply target on this composer. */
  applyReplyTo: (eventId: string) => void;
};

/** The synced draft a write belongs to, captured when the change was made. */
export type SyncedDraftTarget = { roomId: string; slot: string };

export type SyncedRoomDraft = {
  /**
   * Call on every content change. Cheap: it only notes the time; the change
   * is classified (a local edit, or a restored or applied draft) when it is
   * next written or compared.
   */
  noteChange: () => void;
  /**
   * Mirror a written draft to the other devices; `undefined` when it is
   * empty. The caller coalesces calls; `urgent` asks Core to write now.
   */
  persist: (target: SyncedDraftTarget, value: Descendant[] | undefined, urgent: boolean) => void;
  /** A newer draft from another device while this one is being typed in. */
  prompt?: RemoteDraft;
  takeRemote: () => void;
  keepLocal: () => void;
};

/**
 * Mirrors one composer's draft to the account's other Synara devices and
 * applies theirs: silently when this composer is idle, behind a prompt when
 * it is being typed in. The local draft stays the composer's source of truth.
 */
export function useSyncedRoomDraft({
  editor,
  roomId,
  threadRootOverride,
  isMarkdown,
  enabled,
  replyToEventId,
  storedUpdatedAt,
  storedMarkdown,
  applyReplyTo,
}: SyncedRoomDraftOptions): SyncedRoomDraft {
  const active = enabled && isSynaraDesktop();
  const slot = draftSlotFor(threadRootOverride);
  const [prompt, setPrompt] = useState<RemoteDraft>();
  /** The stamp of the draft this composer last saved or applied. */
  const stampRef = useRef<number | undefined>(storedUpdatedAt);
  const dirtyRef = useRef(false);
  const unsyncedRef = useRef(false);
  const lastTypedAtRef = useRef(0);
  /** When the content last changed, and whether that change is unclassified. */
  const lastChangeAtRef = useRef(0);
  const uncheckedRef = useRef(false);
  /** A local edit not yet mirrored. */
  const editedRef = useRef(false);
  /** The draft this composer shows; a late write of another must not touch its stamp. */
  const targetKeyRef = useRef(`${roomId}\u0000${slot}`);
  /**
   * Synced writes go to Core one after another, so a clear issued after a
   * save can never be queued before it.
   */
  const writeChainRef = useRef<Promise<unknown>>(Promise.resolve());
  const enqueueWrite = useCallback((write: () => Promise<unknown>) => {
    const next = writeChainRef.current.then(write, write);
    writeChainRef.current = next.catch(() => undefined);
    return next;
  }, []);
  /** Markdown just applied from a peer; its change event is not a local edit. */
  const appliedRef = useRef<string | undefined>(undefined);
  const replyRef = useRef(replyToEventId);
  // Updated on commit, not in render: the write made when the composer
  // leaves a room (an effect cleanup) must carry that room's reply target.
  useEffect(() => {
    replyRef.current = replyToEventId;
  }, [replyToEventId]);
  const applyReplyRef = useRef(applyReplyTo);
  applyReplyRef.current = applyReplyTo;

  /**
   * Whether this device holds the drafts key. Without it Core rejects every
   * write, so typing must not call it (it did once per keystroke).
   */
  const keyReadyRef = useRef(false);
  useEffect(() => {
    // The status read also asks this account's other devices for the key
    // when this one lacks it (Core rate-limits the request).
    if (!isSynaraDesktop()) return undefined;
    let live = true;
    void setDraftSyncEnabled(enabled).then((status) => {
      if (live) keyReadyRef.current = status?.keyState === 'ready';
    });
    return () => {
      live = false;
    };
  }, [enabled, roomId]);

  /** Until the first real edit: the text the composer opened with. */
  const baselineRef = useRef<string | undefined>(storedMarkdown);
  // Another room or thread: start from its stored draft.
  const storedRef = useRef({ updatedAt: storedUpdatedAt, markdown: storedMarkdown });
  storedRef.current = { updatedAt: storedUpdatedAt, markdown: storedMarkdown };
  useEffect(() => {
    targetKeyRef.current = `${roomId}\u0000${slot}`;
    stampRef.current = storedRef.current.updatedAt;
    baselineRef.current = storedRef.current.markdown;
    dirtyRef.current = false;
    unsyncedRef.current = false;
    uncheckedRef.current = false;
    editedRef.current = false;
    appliedRef.current = undefined;
    setPrompt(undefined);
  }, [roomId, slot]);

  const currentMarkdown = useCallback(
    () => draftMarkdownOf(editor.children, isMarkdown),
    [editor, isMarkdown]
  );

  /**
   * Classify the latest change once, from its Markdown: the text a peer's
   * draft just put in, or the text the composer opened with, is not a local
   * edit. Returns whether a local edit is waiting to be mirrored.
   */
  const settle = useCallback((markdown: string, empty: boolean): boolean => {
    if (!uncheckedRef.current) return editedRef.current;
    uncheckedRef.current = false;
    if (appliedRef.current !== undefined) {
      const applied = appliedRef.current;
      appliedRef.current = undefined;
      if (applied === markdown) return editedRef.current;
    }
    if (baselineRef.current !== undefined) {
      if (baselineRef.current === markdown) return editedRef.current;
      baselineRef.current = undefined;
    }
    lastTypedAtRef.current = lastChangeAtRef.current;
    dirtyRef.current = !empty;
    editedRef.current = true;
    return true;
  }, []);

  const apply = useCallback(
    async (draft: RemoteDraft) => {
      const nodes = draft.cleared ? [] : await remoteDraftNodes(draft, isMarkdown);
      // A blank draft (an empty list) is no text: never a stray bullet.
      const blank = composerContentIsEmpty(nodes);
      appliedRef.current = blank ? '' : draftMarkdownOf(nodes, isMarkdown);
      baselineRef.current = undefined;
      resetEditor(editor);
      if (!blank) Transforms.insertFragment(editor, nodes);
      resetEditorHistory(editor);
      stampRef.current = draft.updatedAt;
      dirtyRef.current = false;
      unsyncedRef.current = false;
      editedRef.current = false;
      if (draft.replyTo && draft.replyTo !== replyRef.current) {
        applyReplyRef.current(draft.replyTo);
      }
    },
    [editor, isMarkdown]
  );

  const resolve = useCallback(
    async (fetchFresh: boolean) => {
      const markdown = currentMarkdown();
      settle(markdown, isEmptyEditor(editor));
      const typing =
        dirtyRef.current &&
        (ReactEditor.isFocused(editor) || Date.now() - lastTypedAtRef.current < ACTIVE_TYPING_MS);
      const resolution = await resolveSyncedDraft(
        roomId,
        slot,
        { updatedAt: stampRef.current, dirty: typing, markdown },
        fetchFresh
      );
      if (!resolution?.draft) return;
      if (resolution.decision === 'apply') {
        setPrompt(undefined);
        await apply(resolution.draft);
      } else if (resolution.decision === 'prompt') {
        setPrompt(resolution.draft);
      }
    },
    [apply, currentMarkdown, editor, roomId, settle, slot]
  );

  /** Queue one synced write; its stamp counts only if the draft is still shown. */
  const write = useCallback(
    (
      target: SyncedDraftTarget,
      value: Descendant[] | undefined,
      markdown: string,
      flush: boolean
    ) => {
      const key = `${target.roomId}\u0000${target.slot}`;
      // A clear is written at once; queued text needs a flush later.
      unsyncedRef.current = !!value && !flush;
      void enqueueWrite(async () => {
        const updatedAt = value
          ? await saveSyncedDraft({
              roomId: target.roomId,
              slot: target.slot,
              markdown,
              desktopSlate: value,
              replyTo: replyRef.current,
              flush,
            })
          : await clearSyncedDraft(target.roomId, target.slot);
        if (updatedAt !== undefined && targetKeyRef.current === key) stampRef.current = updatedAt;
      });
    },
    [enqueueWrite]
  );

  // Opening the composer: take a newer draft written elsewhere.
  useEffect(() => {
    if (!active) return undefined;
    void resolve(true);
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void subscribeSyncedDrafts((updatedRoomId) => {
      // "Every room" is also how Core announces that the key arrived.
      if (updatedRoomId === undefined) {
        void draftSyncStatus().then((status) => {
          if (!disposed) keyReadyRef.current = status?.keyState === 'ready';
        });
      }
      if (updatedRoomId === undefined || updatedRoomId === roomId) void resolve(false);
    }).then((stop) => {
      if (disposed) stop();
      else unsubscribe = stop;
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [active, resolve, roomId]);

  // Leaving the room or the window: write what is queued now.
  useEffect(() => {
    if (!active) return undefined;
    const flush = () => {
      if (!unsyncedRef.current) return;
      unsyncedRef.current = false;
      void enqueueWrite(() => flushSyncedDrafts());
    };
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') flush();
    };
    window.addEventListener('blur', flush);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      window.removeEventListener('blur', flush);
      document.removeEventListener('visibilitychange', onVisibility);
      flush();
    };
  }, [active, enqueueWrite, roomId, slot]);

  const noteChange = useCallback(() => {
    if (!active) return;
    lastChangeAtRef.current = Date.now();
    uncheckedRef.current = true;
  }, [active]);

  const persist = useCallback(
    (target: SyncedDraftTarget, value: Descendant[] | undefined, urgent: boolean) => {
      // Without the key Core rejects every write. The change stays
      // unclassified; a later write or comparison settles it.
      if (!active || !keyReadyRef.current) return;
      const markdown = value ? draftMarkdownOf(value, isMarkdown) : '';
      if (!settle(markdown, !value)) return;
      editedRef.current = false;
      // Empty (sent or erased) clears it everywhere now; text is queued, or
      // written now when the composer or window is being left.
      write(target, value, markdown, urgent);
    },
    [active, isMarkdown, settle, write]
  );

  const takeRemote = useCallback(() => {
    if (prompt) void apply(prompt);
    setPrompt(undefined);
  }, [apply, prompt]);

  const keepLocal = useCallback(() => {
    setPrompt(undefined);
    // Writing ours now makes it the newest everywhere.
    const value = JSON.parse(JSON.stringify(editor.children)) as Descendant[];
    editedRef.current = false;
    write({ roomId, slot }, value, currentMarkdown(), true);
  }, [currentMarkdown, editor, roomId, slot, write]);

  return useMemo(
    () => ({ noteChange, persist, prompt, takeRemote, keepLocal }),
    [keepLocal, noteChange, persist, prompt, takeRemote]
  );
}
