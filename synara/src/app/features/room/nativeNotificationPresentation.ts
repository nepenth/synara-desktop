import type { PlatformNotificationPayload } from '../../platform/notifications';
import {
  AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS,
  AGENT_APPROVAL_NOTIFICATION_KIND,
} from '../../utils/agentApprovals';
import { buildDesktopNotificationRoomRoute } from '../../utils/desktop';
import type { NativeNotificationCandidate } from './nativeNotificationDecision';

/** Presentation follows the SDK-validated candidate, including late promotions.
 * This adapter neither classifies message text nor evaluates approval expiry.
 */
export function buildNativeObservedNotificationPresentation(
  candidate: NativeNotificationCandidate,
  source: { roomId: string; eventId: string }
): PlatformNotificationPayload {
  if (
    candidate.roomId !== source.roomId ||
    candidate.eventId !== source.eventId ||
    (candidate.kind !== 'message' && candidate.kind !== 'agent_approval')
  ) {
    throw new Error('Notification candidate does not match the observed event.');
  }
  const approval = candidate.kind === 'agent_approval';
  return {
    title: candidate.title,
    body: candidate.body,
    route: candidate.route ?? buildDesktopNotificationRoomRoute(source.roomId, source.eventId),
    dismissKeys: approval
      ? [`room:${source.roomId}`, `event:${source.eventId}`]
      : [`room:${source.roomId}`],
    ...(approval
      ? {
          actions: AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS,
          actionContext: {
            kind: AGENT_APPROVAL_NOTIFICATION_KIND,
            roomId: source.roomId,
            eventId: source.eventId,
          },
        }
      : {}),
  };
}

/** Every shown candidate is acknowledged, including stale-generation and
 * failed-delivery paths. A missing generation permits no platform delivery.
 * Acknowledgement is best effort: Core may already have retired its owner.
 */
export async function deliverNativeObservedNotificationCandidate(options: {
  candidate: NativeNotificationCandidate;
  observedGeneration: number;
  presentOrdinaryMessages?: boolean;
  currentGeneration: () => number | undefined;
  deliver: (candidate: NativeNotificationCandidate) => Promise<'delivered' | 'failed' | undefined>;
  acknowledge: (candidateId: string, outcome?: 'delivered' | 'failed') => Promise<unknown>;
}): Promise<void> {
  let outcome: 'delivered' | 'failed' | undefined;
  try {
    if (options.currentGeneration() !== options.observedGeneration) return;
    if (options.candidate.kind === 'message' && options.presentOrdinaryMessages === false) return;
    outcome = await options.deliver(options.candidate);
  } catch {
    outcome = 'failed';
  } finally {
    try {
      await options.acknowledge(options.candidate.candidateId, outcome);
    } catch {
      // Logout/account replacement may already have detached and cleared the
      // old owner. Never turn a failed acknowledgement into a delivery retry.
    }
  }
}
