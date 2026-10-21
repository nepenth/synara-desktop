export const REDACT_CONFIRM_COPY = {
  question: "Remove this message? This can't be undone.",
  remove: 'Remove',
  cancel: 'Cancel',
} as const;

export type NativeTimelineRedactDecision = 'remove' | 'cancel';

/** The message a confirmation asks about, as the row menu saw it. */
export type NativeTimelineRedactTarget = {
  sessionGeneration: number;
  roomId: string;
  eventId: string;
};

/**
 * The one redaction a confirmation dialog is asking about. Redaction removes
 * a message for everyone and cannot be undone, so the row menu only asks;
 * `settle` runs the redaction for Remove alone, and at most once: Cancel,
 * Escape, an outside click and the dialog's own unmount all settle the same
 * request, and only the first counts.
 */
export class NativeTimelineRedactConfirmation {
  private pending: NativeTimelineRedactTarget | undefined;

  request(target: NativeTimelineRedactTarget): void {
    this.pending = target;
  }

  /** Returns whether this call settled an open request. */
  settle(
    decision: NativeTimelineRedactDecision,
    redact: (target: NativeTimelineRedactTarget) => void
  ): boolean {
    const target = this.pending;
    if (!target) return false;
    this.pending = undefined;
    if (decision === 'remove') redact(target);
    return true;
  }
}
