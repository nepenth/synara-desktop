import { Descendant, Editor, Transforms } from 'slate';
import { isEmptyEditor } from '../../components/editor/utils';
import { BlockType } from '../../components/editor/types';

/** Deep copy of composer content, detached from the live Slate document. */
export const cloneComposerContent = (nodes: Descendant[]): Descendant[] =>
  JSON.parse(JSON.stringify(nodes));

/** True when stored composer content has no characters, mentions or emoticons. */
export const composerContentIsEmpty = (nodes: Descendant[] | undefined): boolean => {
  if (!nodes || nodes.length === 0) return true;
  const hasContent = (node: unknown): boolean => {
    if (!node || typeof node !== 'object') return false;
    const record = node as { text?: unknown; type?: unknown; children?: unknown };
    if (typeof record.text === 'string') return record.text.length > 0;
    if (
      record.type === BlockType.Mention ||
      record.type === BlockType.Emoticon ||
      record.type === BlockType.Command
    ) {
      return true;
    }
    return Array.isArray(record.children) && record.children.some(hasContent);
  };
  return !nodes.some(hasContent);
};

/**
 * Content to keep after a send failed: the unsent text first, then anything
 * typed since. Nothing the user wrote is dropped.
 */
export const mergeRestoredComposerContent = (
  restored: Descendant[],
  current: Descendant[] | undefined
): Descendant[] => (composerContentIsEmpty(current) ? restored : [...restored, ...(current ?? [])]);

/**
 * Put unsent content back at the top of a live editor. Text typed while the
 * send was in flight stays below it; an empty composer is replaced.
 */
export const restoreComposerContent = (editor: Editor, restored: Descendant[]): void => {
  if (restored.length === 0) return;
  const wasEmpty = isEmptyEditor(editor);
  const nodes = cloneComposerContent(restored);
  Editor.withoutNormalizing(editor, () => {
    Transforms.insertNodes(editor, nodes, { at: [0], select: false });
    if (wasEmpty) Transforms.removeNodes(editor, { at: [nodes.length] });
  });
  if (wasEmpty || !editor.selection) Transforms.select(editor, Editor.end(editor, []));
};

/**
 * Send errors for composers that closed before their send failed (the room
 * or thread pane was left). The text goes back into that composer's draft;
 * the error is shown when the composer opens again.
 */
const stashedSendErrors = new Map<string, string>();

export const stashComposerSendError = (draftKey: string, message: string): void => {
  stashedSendErrors.set(draftKey, message);
};

export const takeComposerSendError = (draftKey: string): string | undefined => {
  const message = stashedSendErrors.get(draftKey);
  stashedSendErrors.delete(draftKey);
  return message;
};

type ComposerSendJob<T> = {
  item: T;
  run: () => Promise<void>;
};

/**
 * Runs composer sends one at a time, in the order they were submitted.
 *
 * The composer clears as soon as a send is queued, so the user can type and
 * submit the next message while Core waits for the server. When a send
 * fails, the sends queued after it are not attempted (that would reorder the
 * conversation); `onFailure` gets the failed item and every skipped item so
 * their content can go back to the composer.
 */
export class ComposerSendQueue<T> {
  private jobs: ComposerSendJob<T>[] = [];

  private running = false;

  /**
   * @param onWaitingChange Called with the number of sends waiting behind
   *   the one in flight. Their messages have no timeline row yet (Core has
   *   not seen them), so the composer says they are waiting.
   */
  constructor(
    private readonly onFailure: (error: unknown, failed: T, skipped: T[]) => void,
    private readonly onWaitingChange?: (waiting: number) => void
  ) {}

  /** True from the first enqueue until the last queued send settles. */
  get busy(): boolean {
    return this.running;
  }

  /** Items waiting behind the send in flight. */
  get pending(): readonly T[] {
    return this.jobs.map((job) => job.item);
  }

  enqueue(item: T, run: () => Promise<void>): void {
    this.jobs.push({ item, run });
    if (!this.running) void this.drain();
    else this.onWaitingChange?.(this.jobs.length);
  }

  private async drain(): Promise<void> {
    this.running = true;
    try {
      while (this.jobs.length > 0) {
        const job = this.jobs.shift() as ComposerSendJob<T>;
        this.onWaitingChange?.(this.jobs.length);
        try {
          // Sequential on purpose: Core's per-room send queue orders sends.
          // eslint-disable-next-line no-await-in-loop
          await job.run();
        } catch (error) {
          const skipped = this.jobs.map((pending) => pending.item);
          this.jobs = [];
          this.onWaitingChange?.(0);
          this.onFailure(error, job.item, skipped);
        }
      }
    } finally {
      this.running = false;
    }
  }
}
