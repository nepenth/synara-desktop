import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../utils/desktop';

/**
 * A9 desktop decision stream — Core owns the suppress/show policy.
 *
 * The renderer only observes that an event arrived and submits its identity
 * (`roomId`, `eventId`) with privacy-filtered product strings. Core loads the
 * exact event through the pinned Matrix SDK, compares its sender with the
 * bound session, and reads the SDK-evaluated push actions. Room mode, account
 * defaults, intentional mentions, keywords, display-name and room-wide mention
 * matching (with power levels), the suppress-edits override, and rule ordering
 * therefore have one owner. There is no JS push-rule reading, no mode
 * resolution, no mention matcher, and no message body on this path.
 */

export type NativeNotificationDecisionKind =
  | 'message'
  | 'invite'
  | 'agent_approval'
  | 'later_reminder';

export type NativeNotificationDecision = 'show' | 'suppress';

export type NativeNotificationCandidate = {
  candidateId: string;
  roomId: string;
  eventId?: string;
  kind: NativeNotificationDecisionKind;
  title: string;
  body: string;
  route?: string;
  suppressIfFocusedRoom: boolean;
  isEncrypted: boolean;
};

export type NativeNotificationDecisionReadback = {
  decision: NativeNotificationDecision;
  reason?: string;
  candidate?: NativeNotificationCandidate;
  /** SDK push tweak echoes for a shown message; presentation hints only. */
  highlight?: boolean;
  sound?: boolean;
};

/**
 * Identity and presentation only. Any mode, highlight, sender, or encryption
 * verdict belongs to Core; the wire rejects such keys.
 */
export type NativeNotificationDecideInput = {
  roomId: string;
  eventId?: string;
  kind: NativeNotificationDecisionKind;
  title: string;
  body: string;
  route?: string;
  suppressIfFocusedRoom?: boolean;
};

export type NativeNotificationInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeNotificationDecisionReadback>>;

function isOptionalBoolean(value: unknown): value is boolean | undefined {
  return value === undefined || typeof value === 'boolean';
}

function acceptsNativeNotificationDecisionReadback(
  value: NativeNotificationDecisionReadback
): boolean {
  if (value.decision !== 'show' && value.decision !== 'suppress') return false;
  if (!isOptionalBoolean(value.highlight) || !isOptionalBoolean(value.sound)) return false;
  if (value.decision === 'show') {
    const candidate = value.candidate;
    if (!candidate || typeof candidate.candidateId !== 'string' || !candidate.candidateId) {
      return false;
    }
    if (typeof candidate.roomId !== 'string' || !candidate.roomId) return false;
  }
  if (value.decision === 'suppress' && value.candidate !== undefined) return false;
  return true;
}

const defaultInvoke: NativeNotificationInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeNotificationDecisionReadback>(command, args);

/** Core suppress/show decision for one observed event. There is no TS fallback. */
export async function decideNotificationWithNativeOwner(
  input: NativeNotificationDecideInput,
  invoke: NativeNotificationInvoke = defaultInvoke
): Promise<NativeNotificationDecisionReadback> {
  const result = await invoke('matrix_notification_decide', { request: input });
  if (!result.available || !result.value) {
    throw new Error('Native notification decisions are unavailable.');
  }
  if (!acceptsNativeNotificationDecisionReadback(result.value)) {
    throw new Error('Native notification decision did not match the closed vocabulary.');
  }
  return result.value;
}

/** Record the platform-observed focused room in Core (null clears focus). */
export async function setNotificationFocusWithNativeOwner(
  roomId: string | null,
  invoke: (
    command: string,
    args?: Record<string, unknown>
  ) => Promise<DesktopInvokeResult<unknown>> = invokeDesktopWithAvailability
): Promise<void> {
  const result = await invoke('matrix_notification_focus_set', { roomId });
  if (!result.available) {
    throw new Error('Native notification focus is unavailable.');
  }
}

/**
 * Closed delivery receipt the platform reports back to Core after handing a
 * shown candidate to the OS. Omit it when nothing was attempted (system
 * notifications off or no permission).
 */
export type NativeNotificationDeliveryOutcome = 'delivered' | 'failed';

/**
 * Acknowledge a shown candidate with its delivery receipt. Core releases the
 * pending candidate, records the receipt in an identifier-free ledger, and
 * retains dedup memory either way: a failed OS delivery is counted, never
 * retried, so the same event cannot notify twice.
 */
export async function dismissNotificationWithNativeOwner(
  candidateId: string,
  outcome?: NativeNotificationDeliveryOutcome,
  invoke: (
    command: string,
    args?: Record<string, unknown>
  ) => Promise<DesktopInvokeResult<boolean>> = invokeDesktopWithAvailability
): Promise<boolean> {
  const args: Record<string, unknown> = { candidateId };
  if (outcome !== undefined) args.outcome = outcome;
  const result = await invoke('matrix_notification_dismiss', args);
  if (!result.available || typeof result.value !== 'boolean') {
    throw new Error('Native notification dismiss is unavailable.');
  }
  return result.value;
}
