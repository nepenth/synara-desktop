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
