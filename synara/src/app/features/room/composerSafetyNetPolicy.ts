/**
 * The composer safety net's rules: when an error inside a composer is
 * recovered by remounting it, when it shows the inline fallback instead, and
 * what draft the remounted editor starts with. Pure, with the clock passed
 * in; ComposerSafetyNet.tsx applies it.
 */
import type { Descendant } from 'slate';
import { BlockType } from '../../components/editor/types';

/**
 * A second failure this soon after a remount means the remount (often the
 * restored draft itself) is what fails: show the fallback instead of looping.
 */
export const COMPOSER_RECOVERY_WINDOW_MS = 5_000;

/** What a remounted composer starts with. */
export type ComposerRestore = {
  /** `rich` keeps formatting and chips, `plain` is the draft's text only. */
  mode: 'rich' | 'plain' | 'empty';
  nodes: Descendant[];
};

/** The last draft content known to be readable, taken from a failed editor. */
export type ComposerDraftSnapshot = {
  /** The editor value, when its structure is valid Slate content. */
  rich?: Descendant[];
  /** The draft's text, for a plain restore and for Copy draft. */
  text: string;
};

export const EMPTY_DRAFT_SNAPSHOT: ComposerDraftSnapshot = { text: '' };

export type ComposerSafetyNetState = {
  /** Keys the composer: a new generation is a fresh editor and subtree. */
  generation: number;
  status: 'live' | 'fallback';
  /** What the current generation was mounted with; undefined for the first mount. */
  restore?: ComposerRestore;
  /** When the current generation was mounted by a recovery. */
  remountedAt?: number;
  /** A rich restore already failed: the next restore is plain text. */
  plainOnly: boolean;
  /** Failures in a row, each within the window of the remount before it. */
  failures: number;
  snapshot: ComposerDraftSnapshot;
};

export const initialComposerSafetyNetState = (): ComposerSafetyNetState => ({
  generation: 0,
  status: 'live',
  plainOnly: false,
  failures: 0,
  snapshot: EMPTY_DRAFT_SNAPSHOT,
});

const MAX_DEPTH = 32;
const MAX_NODES = 20_000;

/**
 * True when `value` is a composer value Slate can render: a non-empty list of
 * elements, every element with at least one child, every leaf a text.
 */
export const isRestorableDraft = (value: unknown): value is Descendant[] => {
  let seen = 0;
  const valid = (node: unknown, depth: number, topLevel: boolean): boolean => {
    seen += 1;
    if (seen > MAX_NODES || depth > MAX_DEPTH) return false;
    if (!node || typeof node !== 'object' || Array.isArray(node)) return false;
    const record = node as { text?: unknown; type?: unknown; children?: unknown };
    if ('text' in record) return !topLevel && typeof record.text === 'string';
    if (typeof record.type !== 'string' || !Array.isArray(record.children)) return false;
    if (record.children.length === 0) return false;
    return record.children.every((child) => valid(child, depth + 1, false));
  };
  return Array.isArray(value) && value.length > 0 && value.every((node) => valid(node, 0, true));
};

const INLINE_TYPES = new Set<string>([
  BlockType.Mention,
  BlockType.Emoticon,
  BlockType.Link,
  BlockType.Command,
]);

/**
 * The draft's text as the person wrote it, one line per block. Defensive: it
 * reads whatever is there, since the value may be what broke the editor.
 */
export const composerDraftText = (value: unknown): string => {
  const lines: string[] = [];
  let line = '';
  let seen = 0;
  const walk = (node: unknown, depth: number) => {
    seen += 1;
    if (seen > MAX_NODES || depth > MAX_DEPTH || !node || typeof node !== 'object') return;
    const record = node as Record<string, unknown>;
    if (typeof record.text === 'string') {
      line += record.text;
      return;
    }
    const type = typeof record.type === 'string' ? record.type : undefined;
    if (type === BlockType.Mention && typeof record.name === 'string') {
      line += record.name;
      return;
    }
    if (type === BlockType.Emoticon) {
      const key = typeof record.key === 'string' ? record.key : '';
      const shortcode = typeof record.shortcode === 'string' ? record.shortcode : '';
      line += key.startsWith('mxc://') ? `:${shortcode}:` : key;
      return;
    }
    if (type === BlockType.Command && typeof record.command === 'string') {
      line += `/${record.command}`;
      return;
    }
    const children = Array.isArray(record.children) ? record.children : [];
    const block = !type || !INLINE_TYPES.has(type);
    const hasBlockChildren = children.some(
      (child) =>
        !!child &&
        typeof child === 'object' &&
        typeof (child as { type?: unknown }).type === 'string' &&
        !INLINE_TYPES.has((child as { type: string }).type)
    );
    children.forEach((child) => walk(child, depth + 1));
    if (block && !hasBlockChildren) {
      lines.push(line);
      line = '';
    }
  };
  if (Array.isArray(value)) value.forEach((node) => walk(node, 0));
  else walk(value, 0);
  if (line) lines.push(line);
  return lines.join('\n').trim();
};

/** The draft's text as paragraphs: what a plain restore puts in the editor. */
export const plainDraftNodes = (text: string): Descendant[] =>
  text.split('\n').map((lineText) => ({
    type: BlockType.Paragraph,
    children: [{ text: lineText }],
  }));

const cloneDraft = (value: Descendant[]): Descendant[] | undefined => {
  try {
    return JSON.parse(JSON.stringify(value)) as Descendant[];
  } catch {
    return undefined;
  }
};

/**
 * Snapshot a failed editor's value. Slate keeps its value as plain data, so
 * it is readable even when the editor's DOM is what broke; a value that is
 * not valid Slate content keeps only its text.
 */
export const snapshotComposerDraft = (value: unknown): ComposerDraftSnapshot => {
  let text: string;
  try {
    text = composerDraftText(value);
  } catch {
    text = '';
  }
  if (!text) return EMPTY_DRAFT_SNAPSHOT;
  const rich = isRestorableDraft(value) ? cloneDraft(value) : undefined;
  return rich ? { rich, text } : { text };
};

export const snapshotHasDraft = (snapshot: ComposerDraftSnapshot): boolean =>
  snapshot.text.length > 0;

/** What a remount restores: rich content when it is valid and has not failed, else text. */
export const composerRestoreFor = (
  snapshot: ComposerDraftSnapshot,
  plainOnly: boolean
): ComposerRestore => {
  if (!snapshotHasDraft(snapshot)) return { mode: 'empty', nodes: [] };
  if (snapshot.rich && !plainOnly) return { mode: 'rich', nodes: snapshot.rich };
  return { mode: 'plain', nodes: plainDraftNodes(snapshot.text) };
};

/**
 * A composer failed at `now`. `takeSnapshot` reads the failed editor; it is
 * only trusted when that editor had a life of its own (not a failure right
 * after a remount, when it may hold a partly restored draft).
 */
export const composerFailed = (
  state: ComposerSafetyNetState,
  now: number,
  takeSnapshot: () => ComposerDraftSnapshot,
  windowMs: number = COMPOSER_RECOVERY_WINDOW_MS
): ComposerSafetyNetState => {
  if (state.status === 'fallback') return state;
  const quick = state.remountedAt !== undefined && now - state.remountedAt < windowMs;
  if (!quick) {
    let snapshot = state.snapshot;
    try {
      const taken = takeSnapshot();
      // An empty read of a failed editor keeps the last good draft.
      if (snapshotHasDraft(taken) || !snapshotHasDraft(snapshot)) snapshot = taken;
    } catch {
      // Keep the last good snapshot.
    }
    const restore = composerRestoreFor(snapshot, false);
    return {
      generation: state.generation + 1,
      status: 'live',
      restore,
      remountedAt: now,
      plainOnly: false,
      failures: 1,
      snapshot,
    };
  }
  return {
    ...state,
    status: 'fallback',
    // The rich draft was restored and failed at once: it is the suspect.
    plainOnly: state.plainOnly || state.restore?.mode === 'rich',
    failures: state.failures + 1,
  };
};

/** Try again: remount, restoring the draft (as text once rich content has failed). */
export const composerTryAgain = (
  state: ComposerSafetyNetState,
  now: number
): ComposerSafetyNetState => ({
  ...state,
  generation: state.generation + 1,
  status: 'live',
  restore: composerRestoreFor(state.snapshot, state.plainOnly),
  remountedAt: now,
});

/** Start fresh: remount with an empty editor; the stored draft is erased. */
export const composerStartFresh = (
  state: ComposerSafetyNetState,
  now: number
): ComposerSafetyNetState => ({
  ...state,
  generation: state.generation + 1,
  status: 'live',
  restore: { mode: 'empty', nodes: [] },
  remountedAt: now,
  plainOnly: false,
  snapshot: EMPTY_DRAFT_SNAPSHOT,
});

/**
 * The error as Diagnostics may keep it: its name and message up to the first
 * JSON-looking part, since Slate's messages quote the node, and so the draft.
 */
export const composerErrorDetail = (error: unknown): string => {
  const name = error instanceof Error && error.name ? error.name : 'Error';
  const raw =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : 'non-error value thrown';
  const message = raw
    .replace(/[{[][\s\S]*$/, '[node]')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 120);
  return `${name}: ${message || 'no message'}`;
};
