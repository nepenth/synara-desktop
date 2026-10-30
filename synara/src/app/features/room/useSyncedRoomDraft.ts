import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Descendant, Editor, Transforms } from 'slate';
import { ReactEditor } from 'slate-react';
import {
  plainToEditorInput,
  resetEditor,
  resetEditorHistory,
  toPlainText,
} from '../../components/editor';
import { isSynaraDesktop } from '../../utils/desktop';
import {
  clearSyncedDraft,
  draftSlotFor,
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
const remoteDraftNodes = (draft: RemoteDraft): Descendant[] =>
  isSlateValue(draft.desktopSlate)
    ? (draft.desktopSlate as Descendant[])
    : plainToEditorInput(draft.markdown ?? '', false);

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

export type SyncedRoomDraft = {
  /** Call from the editor's change handler. */
  noteChange: (value: Descendant[], empty: boolean) => void;
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
  /** Markdown just applied from a peer; its change event is not a local edit. */
  const appliedRef = useRef<string | undefined>(undefined);
  const replyRef = useRef(replyToEventId);
  replyRef.current = replyToEventId;
  const applyReplyRef = useRef(applyReplyTo);
  applyReplyRef.current = applyReplyTo;

  useEffect(() => {
    // The status read also asks this account's other devices for the key
    // when this one lacks it (Core rate-limits the request).
    if (isSynaraDesktop()) void setDraftSyncEnabled(enabled);
  }, [enabled, roomId]);

  /** Until the first real edit: the text the composer opened with. */
  const baselineRef = useRef<string | undefined>(storedMarkdown);
  // Another room or thread: start from its stored draft.
  const storedRef = useRef({ updatedAt: storedUpdatedAt, markdown: storedMarkdown });
  storedRef.current = { updatedAt: storedUpdatedAt, markdown: storedMarkdown };
  useEffect(() => {
    stampRef.current = storedRef.current.updatedAt;
    baselineRef.current = storedRef.current.markdown;
    dirtyRef.current = false;
    unsyncedRef.current = false;
    appliedRef.current = undefined;
    setPrompt(undefined);
  }, [roomId, slot]);

  const currentMarkdown = useCallback(
    () => toPlainText(editor.children, isMarkdown).trim(),
    [editor, isMarkdown]
  );

  const apply = useCallback(
    (draft: RemoteDraft) => {
      const nodes = draft.cleared ? [] : remoteDraftNodes(draft);
      appliedRef.current = draft.cleared ? '' : toPlainText(nodes, isMarkdown).trim();
      baselineRef.current = undefined;
      resetEditor(editor);
      if (nodes.length > 0) Transforms.insertFragment(editor, nodes);
      resetEditorHistory(editor);
      stampRef.current = draft.updatedAt;
      dirtyRef.current = false;
      unsyncedRef.current = false;
      if (draft.replyTo && draft.replyTo !== replyRef.current) {
        applyReplyRef.current(draft.replyTo);
      }
    },
    [editor, isMarkdown]
  );

  const resolve = useCallback(
    async (fetchFresh: boolean) => {
      const typing =
        dirtyRef.current &&
        (ReactEditor.isFocused(editor) || Date.now() - lastTypedAtRef.current < ACTIVE_TYPING_MS);
      const resolution = await resolveSyncedDraft(
        roomId,
        slot,
        { updatedAt: stampRef.current, dirty: typing, markdown: currentMarkdown() },
        fetchFresh
      );
      if (!resolution?.draft) return;
      if (resolution.decision === 'apply') {
        setPrompt(undefined);
        apply(resolution.draft);
      } else if (resolution.decision === 'prompt') {
        setPrompt(resolution.draft);
      }
    },
    [apply, currentMarkdown, editor, roomId, slot]
  );

  const save = useCallback(
    async (value: Descendant[] | undefined, flush: boolean) => {
      const markdown = value ? currentMarkdown() : '';
      const updatedAt = await saveSyncedDraft({
        roomId,
        slot,
        markdown,
        desktopSlate: value,
        replyTo: value ? replyRef.current : undefined,
        flush,
      });
      if (updatedAt !== undefined) {
        stampRef.current = updatedAt;
        if (flush) unsyncedRef.current = false;
      }
    },
    [currentMarkdown, roomId, slot]
  );

  // Opening the composer: take a newer draft written elsewhere.
  useEffect(() => {
    if (!active) return undefined;
    void resolve(true);
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void subscribeSyncedDrafts((updatedRoomId) => {
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
      void flushSyncedDrafts();
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
  }, [active, roomId, slot]);

  const noteChange = useCallback(
    (value: Descendant[], empty: boolean) => {
      if (!active) return;
      const markdown = empty ? '' : currentMarkdown();
      if (appliedRef.current !== undefined) {
        const applied = appliedRef.current;
        appliedRef.current = undefined;
        if (applied === markdown) return;
      }
      if (baselineRef.current !== undefined) {
        if (baselineRef.current === markdown) return;
        baselineRef.current = undefined;
      }
      lastTypedAtRef.current = Date.now();
      if (empty) {
        dirtyRef.current = false;
        unsyncedRef.current = false;
        // Sent or erased: clear it everywhere now, not after the pause.
        void clearSyncedDraft(roomId, slot).then((updatedAt) => {
          if (updatedAt !== undefined) stampRef.current = updatedAt;
        });
        return;
      }
      dirtyRef.current = true;
      unsyncedRef.current = true;
      void save(value, false);
    },
    [active, currentMarkdown, roomId, save, slot]
  );

  const takeRemote = useCallback(() => {
    if (prompt) apply(prompt);
    setPrompt(undefined);
  }, [apply, prompt]);

  const keepLocal = useCallback(() => {
    setPrompt(undefined);
    // Writing ours now makes it the newest everywhere.
    void save(JSON.parse(JSON.stringify(editor.children)) as Descendant[], true);
  }, [editor, save]);

  return useMemo(
    () => ({ noteChange, prompt, takeRemote, keepLocal }),
    [keepLocal, noteChange, prompt, takeRemote]
  );
}
