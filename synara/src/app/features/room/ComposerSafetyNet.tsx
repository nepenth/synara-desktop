import React, {
  MutableRefObject,
  ReactNode,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { ErrorBoundary } from 'react-error-boundary';
import { Button, Icon, Icons, Text } from 'folds';
import { Editor, Transforms } from 'slate';
import { createComposerEditor, focusEditorAfterRender } from '../../components/editor';
import { copyToClipboard } from '../../utils/dom';
import { recordDesktopDiagnostic } from '../../utils/desktopDiagnostics';
import {
  COMPOSER_RECOVERY_WINDOW_MS,
  type ComposerDraftSnapshot,
  type ComposerRestore,
  type ComposerSafetyNetState,
  composerErrorDetail,
  composerFailed,
  composerRestoreFor,
  composerStartFresh,
  composerTryAgain,
  initialComposerSafetyNetState,
  snapshotComposerDraft,
  snapshotHasDraft,
} from './composerSafetyNetPolicy';
import * as css from './ComposerSafetyNet.css';

export type ComposerSurface = 'room' | 'thread' | 'edit';

/** What one generation of the composer is mounted with. */
export type ComposerSession = {
  /** A fresh Slate editor for this generation. */
  editor: Editor;
  /** The draft to start with after a recovery; undefined on the first mount. */
  restore?: ComposerRestore;
  generation: number;
};

type ComposerSafetyNetProps = {
  surface: ComposerSurface;
  children: (session: ComposerSession) => ReactNode;
  /** The editor the composer shows now; undefined while the fallback shows. */
  editorRef?: MutableRefObject<Editor | undefined>;
  /**
   * Write the draft the fallback holds to where the composer reads its draft,
   * so it is saved even if the person leaves before choosing.
   */
  keepDraft?: (restore: ComposerRestore) => void;
  recoveryWindowMs?: number;
  now?: () => number;
};

const FALLBACK_MAX_HEIGHT_PX = 240;

const COPY: Record<ComposerSurface, { problem: string; saved: string }> = {
  room: {
    problem: 'The message box ran into a problem.',
    saved: 'Your draft is saved.',
  },
  thread: {
    problem: 'The message box ran into a problem.',
    saved: 'Your draft is saved.',
  },
  edit: {
    problem: 'The message editor ran into a problem.',
    saved: 'Your edit is saved.',
  },
};

const logContained = (surface: ComposerSurface, outcome: string, error: unknown) => {
  try {
    // eslint-disable-next-line no-console
    console.error(
      `[composer-safety-net] contained a ${surface} composer error (${outcome})`,
      error
    );
    recordDesktopDiagnostic(
      `composer-safety-net contained surface=${surface} outcome=${outcome} error=${composerErrorDetail(error)}`
    );
  } catch {
    // Logging must never be what breaks the composer.
  }
};

type FallbackProps = {
  surface: ComposerSurface;
  snapshot: ComposerDraftSnapshot;
  minHeight?: number;
  autoFocus: boolean;
  onTryAgain: () => void;
  onStartFresh: () => void;
};

function ComposerFallback({
  surface,
  snapshot,
  minHeight,
  autoFocus,
  onTryAgain,
  onStartFresh,
}: FallbackProps) {
  const tryAgainRef = useRef<HTMLButtonElement>(null);
  const [copied, setCopied] = useState(false);
  const hasDraft = snapshotHasDraft(snapshot);
  const copy = COPY[surface];

  useEffect(() => {
    if (autoFocus) tryAgainRef.current?.focus({ preventScroll: true });
  }, [autoFocus]);

  return (
    <section
      className={css.ComposerFallback}
      style={minHeight ? { minHeight } : undefined}
      aria-label="Message composer"
      data-composer-fallback={surface}
    >
      <div className={css.ComposerFallbackMessage}>
        <Icon className={css.ComposerFallbackIcon} src={Icons.Warning} size="100" />
        <Text size="T300" role="alert">
          {hasDraft ? `${copy.problem} ${copy.saved}` : copy.problem}
        </Text>
      </div>
      <div className={css.ComposerFallbackActions}>
        <Button
          ref={tryAgainRef}
          size="300"
          radii="300"
          variant="Primary"
          fill="Soft"
          onClick={onTryAgain}
        >
          <Text size="B300">Try again</Text>
        </Button>
        <Button
          size="300"
          radii="300"
          variant="Secondary"
          fill="Soft"
          outlined
          onClick={onStartFresh}
        >
          <Text size="B300">Start fresh</Text>
        </Button>
        {hasDraft && (
          <Button
            size="300"
            radii="300"
            variant="Secondary"
            fill="None"
            onClick={() => {
              copyToClipboard(snapshot.text);
              setCopied(true);
            }}
          >
            <Text size="B300">{copied ? 'Copied' : 'Copy draft'}</Text>
          </Button>
        )}
      </div>
    </section>
  );
}

/**
 * Contains an error thrown while the composer renders or commits (the Slate
 * editor, its toolbar, menus, pickers, attachments and reply banner) so it
 * never reaches the route's full-screen error page. The first failure
 * remounts the composer with a fresh editor and the draft; a second soon
 * after shows a one-line fallback in the composer's place.
 */
export function ComposerSafetyNet({
  surface,
  children,
  editorRef,
  keepDraft,
  recoveryWindowMs = COMPOSER_RECOVERY_WINDOW_MS,
  now = Date.now,
}: ComposerSafetyNetProps) {
  const [state, setState] = useState<ComposerSafetyNetState>(initialComposerSafetyNetState);
  // A new generation gets a new editor: nothing of the failed one is reused.
  const editor = useMemo(
    () => createComposerEditor(),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [state.generation]
  );
  const session: ComposerSession = {
    editor,
    restore: state.restore,
    generation: state.generation,
  };

  const stateRef = useRef(state);
  stateRef.current = state;
  const nowRef = useRef(now);
  nowRef.current = now;
  const keepDraftRef = useRef(keepDraft);
  keepDraftRef.current = keepDraft;

  // Focus and height of the live composer, read when it fails.
  const rootRef = useRef<HTMLDivElement>(null);
  const focusedRef = useRef(false);
  const heightRef = useRef<number | undefined>(undefined);
  const refocusRef = useRef(false);
  const [fallbackFocus, setFallbackFocus] = useState(false);

  useLayoutEffect(() => {
    const element = rootRef.current;
    if (!element || state.status !== 'live') return undefined;
    heightRef.current = element.getBoundingClientRect().height || heightRef.current;
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(([entry]) => {
      const height = entry?.borderBoxSize?.[0]?.blockSize ?? entry?.contentRect.height;
      if (height) heightRef.current = height;
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [state.status, state.generation]);

  const handleError = useCallback(
    (generation: number, failedEditor: Editor, error: unknown) => {
      const current = stateRef.current;
      // One failure per generation: errors from its cleanup are the same failure.
      if (generation !== current.generation || current.status !== 'live') return;
      const next = composerFailed(
        current,
        nowRef.current(),
        () => snapshotComposerDraft(failedEditor.children),
        recoveryWindowMs
      );
      // A remount that fails before its refocus ran still had the focus.
      const hadFocus = focusedRef.current || refocusRef.current;
      focusedRef.current = false;
      refocusRef.current = false;
      if (next.status === 'fallback') {
        logContained(surface, 'fallback', error);
        setFallbackFocus(hadFocus);
        // Whatever the failed editor's cleanup wrote, the stored draft is the
        // last good one while the fallback shows.
        try {
          keepDraftRef.current?.(composerRestoreFor(next.snapshot, next.plainOnly));
        } catch {
          // The snapshot is still held here for Try again and Copy draft.
        }
      } else {
        logContained(surface, 'recovered', error);
        refocusRef.current = hadFocus;
      }
      stateRef.current = next;
      setState(next);
    },
    [recoveryWindowMs, surface]
  );

  useEffect(() => {
    if (editorRef) editorRef.current = state.status === 'live' ? session.editor : undefined;
  });
  useEffect(
    () => () => {
      if (editorRef) editorRef.current = undefined;
    },
    [editorRef]
  );

  // After a recovery the composer gets focus back if it had it. Runs after
  // the child's own effects, so the draft is in place.
  useEffect(() => {
    if (!refocusRef.current || state.status !== 'live') return;
    refocusRef.current = false;
    const target = session.editor;
    try {
      if (!target.selection && target.children.length > 0) {
        Transforms.select(target, Editor.end(target, []));
      }
    } catch {
      // Focus without moving the caret.
    }
    focusEditorAfterRender(target);
    // Once per generation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.generation, state.status]);

  const tryAgain = useCallback(() => {
    refocusRef.current = true;
    setState((current) => composerTryAgain(current, nowRef.current()));
  }, []);
  const startFresh = useCallback(() => {
    refocusRef.current = true;
    setState((current) => composerStartFresh(current, nowRef.current()));
  }, []);

  if (state.status === 'fallback') {
    const height = heightRef.current;
    return (
      <ComposerFallback
        surface={surface}
        snapshot={state.snapshot}
        minHeight={height ? Math.min(height, FALLBACK_MAX_HEIGHT_PX) : undefined}
        autoFocus={fallbackFocus}
        onTryAgain={tryAgain}
        onStartFresh={startFresh}
      />
    );
  }

  const { generation } = session;
  const generationEditor = session.editor;
  return (
    <div
      ref={rootRef}
      data-composer-generation={generation}
      // React's focus events bubble through portals, so a picker or menu
      // opened from the composer counts as the composer having focus.
      onFocus={() => {
        focusedRef.current = true;
      }}
      onBlur={() => {
        focusedRef.current = false;
      }}
    >
      <ErrorBoundary
        key={generation}
        fallback={null}
        onError={(error) => handleError(generation, generationEditor, error)}
      >
        {children(session)}
      </ErrorBoundary>
    </div>
  );
}
