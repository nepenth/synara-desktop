import { useAtomValue } from 'jotai';
import React, { ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import LogoPNG from '../../../../public/res/png/synara.png';
import LogoUnreadPNG from '../../../../public/res/png/synara-unread.png';
import LogoHighlightPNG from '../../../../public/res/png/synara-highlight.png';
import NotificationSound from '../../../../public/sound/notification.ogg';
import InviteSound from '../../../../public/sound/invite.ogg';
import { notificationPermission, setFavicon } from '../../utils/dom';
import { useSetting } from '../../state/hooks/settings';
import { desktopPlatformSettingsAtom, settingsAtom } from '../../state/settings';
import { allInvitesAtom, useNativeInviteSyncing } from '../../state/room-list/inviteList';
import { usePreviousValue } from '../../hooks/usePreviousValue';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { getInboxInvitesPath } from '../pathUtils';
import { getMemberDisplayName, getThreadRootEventId } from '../../utils/room';
import { getMxIdLocalPart } from '../../utils/matrix';
import { useSelectedRoom } from '../../hooks/router/useSelectedRoom';
import { useInboxNotificationsSelected } from '../../hooks/router/useInbox';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { getSortedLaterItems } from '../../utils/later';
import { laterContentAtom } from '../../state/laterList';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { PerformanceDebugOverlay } from '../../components/performance/PerformanceDebugOverlay';
import {
  getPlatformNotificationSummary,
  registerPlatformAgentActionListener,
  registerPlatformNotificationActionListener,
  setPlatformBadgeCount,
  setPlatformShortcuts,
  setPlatformTrayState,
  showPlatformNotification,
  subscribePlatformTrayDndToggle,
  supportsPlatformGlobalShortcuts,
  supportsPlatformSystemNotifications,
  supportsPlatformTrayState,
} from '../../platform';
import {
  AGENT_APPROVAL_NATIVE_ACTION_TTL_MS,
  AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS,
  AGENT_APPROVAL_NOTIFICATION_KIND,
  buildAgentApprovalNativeActionDedupeKey,
  createAgentApprovalNativeActionDedupeStore,
  detectAgentApprovalPrompt,
  planAgentApprovalNativeNotificationAction,
} from '../../utils/agentApprovals';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { buildDesktopNotificationRoomRoute } from '../../utils/desktop';
import { notifiedEventIdsCache } from '../../notifications/notificationCaches';
import { DesktopUpdaterProvider } from '../../features/desktop-updater/DesktopUpdaterProvider';
import { decideAgentApprovalWithNativeOwner } from '../../features/room/nativeReactionOwner';
import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  setNotificationFocusWithNativeOwner,
  type NativeNotificationDeliveryOutcome,
} from '../../features/room/nativeNotificationDecision';
import { markLaterRemindedWithNativeOwner } from '../../features/room/nativeLaterOwner';
import {
  subscribeNativeNotificationObservations,
  type NativeNotificationObservation,
} from '../../features/room/nativeNotificationObservation';

const RECENT_AGENT_APPROVAL_MS = AGENT_APPROVAL_NATIVE_ACTION_TTL_MS;
// Local submit-memory bound. Core `(room, event)` dedup is authoritative;
// this set only guards against a duplicated observation of the same event.
const NOTIFICATION_SUBMITTED_CACHE_MAX = 500;

const getDurableApprovalStorage = (): Storage | null => {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
};

function SystemEmojiFeature() {
  const [twitterEmoji] = useSetting(settingsAtom, 'twitterEmoji');

  if (twitterEmoji) {
    document.documentElement.style.setProperty('--font-emoji', 'Twemoji');
  } else {
    document.documentElement.style.setProperty('--font-emoji', 'Twemoji_DISABLED');
  }

  return null;
}

function PageZoomFeature() {
  const [pageZoom] = useSetting(settingsAtom, 'pageZoom');

  if (pageZoom === 100) {
    document.documentElement.style.removeProperty('font-size');
  } else {
    document.documentElement.style.setProperty('font-size', `calc(1em * ${pageZoom / 100})`);
  }

  return null;
}

function FaviconUpdater() {
  const roomToUnread = useAtomValue(roomToUnreadAtom);

  useEffect(() => {
    let notification = false;
    let highlight = false;
    roomToUnread.forEach((unread) => {
      if (unread.total > 0) {
        notification = true;
      }
      if (unread.highlight > 0) {
        highlight = true;
      }
    });

    if (notification) {
      setFavicon(highlight ? LogoHighlightPNG : LogoUnreadPNG);
    } else {
      setFavicon(LogoPNG);
    }
  }, [roomToUnread]);

  return null;
}

function TrayDoNotDisturbSync() {
  const [, setShowNotifications] = useSetting(settingsAtom, 'showNotifications');

  useEffect(() => {
    if (!supportsPlatformTrayState()) return undefined;

    return subscribePlatformTrayDndToggle(() => {
      setShowNotifications((current) => !current);
    });
  }, [setShowNotifications]);

  return null;
}

function PlatformBadgeAndTrayUpdater() {
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const invites = useAtomValue(allInvitesAtom);
  const laterContent = useAtomValue(laterContentAtom);
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');

  useEffect(() => {
    const activeLaterCount = getSortedLaterItems(laterContent).filter(
      (item) => !item.completedAt
    ).length;
    const summary = getPlatformNotificationSummary({
      unreadCounts: roomToUnread.values(),
      laterActiveCount: activeLaterCount,
      inviteCount: invites.length,
    });

    setPlatformBadgeCount(summary.appBadgeCount);
    if (supportsPlatformTrayState()) {
      setPlatformTrayState({
        unreadCount: summary.unreadCount,
        highlightCount: summary.highlightCount,
        laterCount: summary.laterActiveCount,
        notificationInboxCount: summary.inboxBadgeCount,
        doNotDisturb: !showNotifications,
      }).catch(() => undefined);
    }
  }, [invites.length, laterContent, roomToUnread, showNotifications]);

  return null;
}

function InviteNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const invites = useAtomValue(allInvitesAtom);
  const perviousInviteLen = usePreviousValue(invites.length, 0);
  const nativeInviteSyncing = useNativeInviteSyncing();

  const navigate = useNavigate();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [notificationSound] = useSetting(settingsAtom, 'isNotificationSounds');

  const notify = useCallback(
    (count: number) => {
      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title: 'Invitation',
          body: `You have ${count} new invitation request.`,
        }).catch(() => undefined);
        return;
      }

      const noti = new window.Notification('Invitation', {
        icon: LogoPNG,
        badge: LogoPNG,
        body: `You have ${count} new invitation request.`,
        silent: true,
      });

      noti.onclick = () => {
        if (!window.closed) navigate(getInboxInvitesPath());
        noti.close();
      };
    },
    [navigate]
  );

  const playSound = useCallback(() => {
    const audioElement = audioRef.current;
    audioElement?.play();
  }, []);

  useEffect(() => {
    if (invites.length > perviousInviteLen && nativeInviteSyncing) {
      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        notify(invites.length - perviousInviteLen);
      }

      if (notificationSound) {
        playSound();
      }
    }
  }, [
    invites,
    perviousInviteLen,
    nativeInviteSyncing,
    showNotifications,
    notificationSound,
    notify,
    playSound,
  ]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={InviteSound} type="audio/ogg" />
    </audio>
  );
}

function MessageNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const notifRef = useRef<Notification | undefined>(undefined);
  // Submitted `(roomId, eventId)` pairs Core durably recorded (shown,
  // duplicate, or own events). Core dedup is authoritative; this bounded set
  // only guards against a duplicated observation of the same event.
  // Transient suppressions are deliberately not remembered.
  const submittedRef = useRef<Set<string>>(new Set());

  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [notificationSound] = useSetting(settingsAtom, 'isNotificationSounds');

  const { navigateRoom } = useRoomNavigate();
  const notificationSelected = useInboxNotificationsSelected();
  const selectedRoomId = useSelectedRoom();

  // Report the platform focus observation into Core. A selected room only
  // suppresses while its window is focused; background windows still notify.
  useEffect(() => {
    const reportFocus = () => {
      const focused = document.hasFocus() ? selectedRoomId ?? null : null;
      setNotificationFocusWithNativeOwner(focused).catch(() => undefined);
    };
    reportFocus();
    window.addEventListener('focus', reportFocus);
    window.addEventListener('blur', reportFocus);
    return () => {
      window.removeEventListener('focus', reportFocus);
      window.removeEventListener('blur', reportFocus);
    };
  }, [selectedRoomId]);

  const rememberSubmitted = useCallback((roomId: string, eventId: string) => {
    const submitted = submittedRef.current;
    submitted.add(`${roomId}:${eventId}`);
    if (submitted.size > NOTIFICATION_SUBMITTED_CACHE_MAX) {
      const oldest = submitted.values().next().value;
      if (oldest !== undefined) submitted.delete(oldest);
    }
  }, []);

  const notify = useCallback(
    async ({
      title,
      body,
      roomAvatar,
      roomId,
      eventId,
      route,
    }: {
      title: string;
      body: string;
      roomAvatar?: string;
      roomId: string;
      eventId: string;
      route?: string;
    }): Promise<NativeNotificationDeliveryOutcome> => {
      // The OS answer is the delivery receipt Core records with the
      // acknowledgement. Errors are reported as `failed`, never swallowed
      // into a silent success.
      if (supportsPlatformSystemNotifications()) {
        try {
          const shown = await showPlatformNotification({
            title,
            body,
            route: route ?? buildDesktopNotificationRoomRoute(roomId, eventId),
          });
          return shown ? 'delivered' : 'failed';
        } catch {
          return 'failed';
        }
      }

      try {
        const noti = new window.Notification(title, {
          icon: roomAvatar,
          badge: roomAvatar,
          body,
          silent: true,
        });

        noti.onclick = () => {
          if (!window.closed) navigateRoom(roomId, eventId);
          noti.close();
          notifRef.current = undefined;
        };

        notifRef.current?.close();
        notifRef.current = noti;
        return 'delivered';
      } catch {
        return 'failed';
      }
    },
    [navigateRoom]
  );

  const playSound = useCallback(() => {
    const audioElement = audioRef.current;
    audioElement?.play();
  }, []);

  const decideAndNotify = useCallback(
    async (observation: NativeNotificationObservation) => {
      const { roomId, eventId, sender } = observation;
      // Agent approvals travel the Core approval-decision path, never the
      // generic message route.
      if (observation.body !== undefined && detectAgentApprovalPrompt({ body: observation.body })) {
        return;
      }
      const room = mx.getRoom(roomId);
      if (!room || room.isSpaceRoom()) return;

      const cacheKey = `${roomId}:${eventId}`;
      if (submittedRef.current.has(cacheKey)) return;

      let readback;
      try {
        // Identity and presentation only. Core loads this exact event from
        // the SDK, compares its sender with the session, and reads the
        // SDK-evaluated push actions (room mode, mentions, keywords, mute,
        // suppressed edits). No mode, highlight, or body leaves the renderer.
        readback = await decideNotificationWithNativeOwner({
          roomId,
          eventId,
          kind: 'message',
          // Privacy-filtered product strings only: room name and a fixed
          // summary. Never message content, ciphertext, or identifiers.
          title: room.name ?? 'Unknown',
          body: `New inbox notification from ${
            getMemberDisplayName(room, sender) ?? getMxIdLocalPart(sender) ?? sender
          }`,
          route: buildDesktopNotificationRoomRoute(roomId, eventId),
          suppressIfFocusedRoom: true,
        });
      } catch {
        // Core unavailable (no session, event not yet loaded, push context
        // still settling): fail silent without remembering. Core observed
        // this event once; there is no renderer retry and no TS policy
        // fallback.
        return;
      }
      // Remember only outcomes Core durably recorded: shown candidates and
      // already-seen or own events. Core observes each event once, so this
      // set is a guard against a duplicated observation, not a resubmit
      // schedule; Core's own dedup remains the authority.
      if (
        readback.decision === 'show' ||
        readback.reason === 'duplicate-event' ||
        readback.reason === 'own-event'
      ) {
        rememberSubmitted(roomId, eventId);
      }
      if (readback.decision !== 'show' || !readback.candidate) return;
      const shownCandidateId = readback.candidate.candidateId;

      // Delivery receipt: undefined when nothing was attempted, otherwise the
      // OS answer. The candidate stays pending in Core until the OS has
      // answered, so the acknowledgement carries a real outcome.
      let outcome: NativeNotificationDeliveryOutcome | undefined;
      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        const avatarMxc =
          room.getAvatarFallbackMember()?.getMxcAvatarUrl() ?? room.getMxcAvatarUrl();
        outcome = await notify({
          title: readback.candidate.title,
          body: readback.candidate.body,
          roomAvatar: avatarMxc
            ? resolveMatrixThumbnailUrl(mx, avatarMxc, 96, { useAuthentication })
            : undefined,
          roomId,
          eventId,
          route: readback.candidate.route,
        });
      }

      // Sound follows the SDK push tweak Core echoed (one-to-one rooms,
      // mentions, keywords, and any account rule that sets a sound), gated by
      // the local preference. A notification the OS refused stays silent so
      // sound never claims a delivery that did not happen.
      if (notificationSound && readback.sound === true && outcome !== 'failed') {
        playSound();
      }

      // Ack with the receipt to release the pending candidate. Core retains
      // bounded recent-event dedup independently of the pending queue and
      // does not retry a failed delivery.
      void dismissNotificationWithNativeOwner(shownCandidateId, outcome).catch(() => undefined);
    },
    [
      mx,
      notificationSound,
      showNotifications,
      playSound,
      notify,
      rememberSubmitted,
      useAuthentication,
    ]
  );

  // Observation pump: Core pushes one observation per live message-like
  // event the SDK sync delivered (`matrix-notification-observed`). The
  // renderer no longer scans timelines, listens for `Room.timeline`, or gates
  // on a sync state; every observation still goes through Core decide.
  useEffect(() => {
    const dispose = subscribeNativeNotificationObservations(
      () => mx.getSyncStateData()?.sessionGeneration,
      (observation) => {
        // The notification inbox triage view suppresses message toasts while
        // the user is working through notifications.
        if (notificationSelected) return;
        void decideAndNotify(observation);
      }
    );
    return dispose;
  }, [mx, notificationSelected, decideAndNotify]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={NotificationSound} type="audio/ogg" />
    </audio>
  );
}

function AgentApprovalNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const mx = useMatrixClient();
  const accountScope = mx.getUserId();
  const nativeActionDedupe = useMemo(
    () =>
      accountScope
        ? createAgentApprovalNativeActionDedupeStore(getDurableApprovalStorage(), accountScope)
        : undefined,
    [accountScope]
  );
  const { navigateRoom } = useRoomNavigate();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');

  const notify = useCallback(
    ({
      roomId,
      eventId,
      approvalEventId,
      roomName,
      title,
      body,
    }: {
      roomId: string;
      eventId: string;
      approvalEventId: string;
      roomName: string;
      title: string;
      body: string;
      commandPreview?: string;
    }) => {
      // Keep dangerous command text out of OS-level notification surfaces.
      // The exact prompt remains available through the Review route.
      const notificationBody = body;

      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title,
          body: `${roomName}: ${notificationBody}`,
          route: buildDesktopNotificationRoomRoute(roomId, eventId),
          // Approve-always is not offered on native OS notifications.
          actions: AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS,
          actionContext: {
            kind: AGENT_APPROVAL_NOTIFICATION_KIND,
            roomId,
            eventId: approvalEventId,
          },
        }).catch(() => undefined);
        return;
      }

      const noti = new window.Notification(title, {
        icon: LogoHighlightPNG,
        badge: LogoHighlightPNG,
        body: `${roomName}: ${notificationBody}`,
        silent: true,
      });

      noti.onclick = () => {
        if (!window.closed) navigateRoom(roomId, eventId);
        noti.close();
      };
    },
    [navigateRoom]
  );

  const handleNativeNotificationAction = useCallback(
    async (payload: {
      actionId: string;
      context?: { kind: string; roomId?: string; eventId?: string };
    }) => {
      const { actionId, context } = payload;
      const earlyPlan = planAgentApprovalNativeNotificationAction({
        actionId,
        context,
        nowMs: Date.now(),
        // Require full event validation before send; early plan without eventResolved rejects.
      });

      // Fast-reject malformed payloads without I/O.
      if (earlyPlan.type === 'reject' && earlyPlan.reason !== 'event-not-validated') {
        if (import.meta.env?.DEV) {
          // eslint-disable-next-line no-console
          console.debug(
            '[synara:agent-approval] native action rejected',
            earlyPlan.reason,
            payload
          );
        }
        return;
      }

      const roomId = context?.roomId?.trim();
      const eventId = context?.eventId?.trim();
      if (!roomId || !eventId) return;

      // Approve-always: never send ♾️ from a native notification; open the room instead.
      if (earlyPlan.type === 'open-room') {
        if (import.meta.env?.DEV) {
          // eslint-disable-next-line no-console
          console.debug('[synara:agent-approval] approve-always routed to room', earlyPlan.reason);
        }
        navigateRoom(earlyPlan.roomId, earlyPlan.eventId);
        return;
      }

      const dedupe = nativeActionDedupe;
      if (!dedupe) {
        navigateRoom(roomId, eventId);
        return;
      }
      const provisionalDedupeKey = buildAgentApprovalNativeActionDedupeKey(roomId, eventId);
      if (dedupe.has(provisionalDedupeKey)) {
        return;
      }

      dedupe.add(provisionalDedupeKey);
      try {
        await decideAgentApprovalWithNativeOwner({
          roomId,
          eventId,
          actionId,
        });
      } catch (error) {
        dedupe.remove(provisionalDedupeKey);
        // Expired, signed-out, stale, or otherwise rejected decisions fail
        // closed, then open the exact event so the action never appears to
        // have succeeded silently.
        navigateRoom(roomId, eventId);
        if (import.meta.env?.DEV) {
          // eslint-disable-next-line no-console
          console.debug('[synara:agent-approval] native decision failed closed', error);
        }
      }
    },
    [nativeActionDedupe, navigateRoom]
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void | Promise<void>) | undefined;

    registerPlatformNotificationActionListener((payload) => {
      void handleNativeNotificationAction(payload);
    }).then((nextUnlisten) => {
      if (disposed) {
        void nextUnlisten?.();
        return;
      }
      unlisten = nextUnlisten;
    });

    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [handleNativeNotificationAction]);

  const playSound = useCallback(() => {
    audioRef.current?.play();
  }, []);

  const notifyApprovalEvent = useCallback(
    (observation: NativeNotificationObservation) => {
      const { eventId, sender, originServerTs, body } = observation;
      if (body === undefined) return;
      const room = mx.getRoom(observation.roomId);
      if (!room || room.isSpaceRoom()) return;
      if (sender === mx.getUserId()) return;
      if (Date.now() - originServerTs > RECENT_AGENT_APPROVAL_MS) return;

      if (notifiedEventIdsCache.has(eventId)) return;

      const prompt = detectAgentApprovalPrompt({ body });
      if (!prompt) return;

      notifiedEventIdsCache.add(eventId);
      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        notify({
          roomId: room.roomId,
          // Review/default-click must focus the exact approval prompt. The
          // room router can still expose its thread context after anchoring.
          eventId,
          approvalEventId: eventId,
          roomName: room.name ?? 'Unknown',
          title: prompt.title,
          body: prompt.body,
          commandPreview: prompt.commandPreview,
        });
      }

      if (showNotifications) {
        playSound();
      }
    },
    [mx, notify, playSound, showNotifications]
  );

  // Approval prompts ride the same Core observation stream as messages; the
  // renderer detects the prompt in the observed body instead of scanning.
  useEffect(() => {
    const dispose = subscribeNativeNotificationObservations(
      () => mx.getSyncStateData()?.sessionGeneration,
      notifyApprovalEvent
    );
    return dispose;
  }, [mx, notifyApprovalEvent]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={NotificationSound} type="audio/ogg" />
    </audio>
  );
}

function LaterReminderNotifications() {
  const mx = useMatrixClient();
  const { navigateRoom } = useRoomNavigate();
  const laterContent = useAtomValue(laterContentAtom);
  const reminders = useMemo(
    () => getSortedLaterItems(laterContent).filter((item) => item.kind === 'reminder'),
    [laterContent]
  );
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const notifiedRef = useRef<Set<string>>(new Set());

  const notify = useCallback(
    (body: string, roomId: string, eventId: string) => {
      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title: 'Reminder',
          body,
          route: buildDesktopNotificationRoomRoute(roomId, eventId),
        }).catch(() => undefined);
        return;
      }

      const noti = new window.Notification('Reminder', {
        icon: LogoPNG,
        badge: LogoPNG,
        body,
        silent: true,
      });

      noti.onclick = () => {
        if (!window.closed) navigateRoom(roomId, eventId);
        noti.close();
      };
    },
    [navigateRoom]
  );

  useEffect(() => {
    const checkDueReminders = () => {
      const now = Date.now();
      const dueReminders = reminders.filter((item) => {
        const notifyKey = `${item.id}:${item.dueTs ?? ''}`;
        return (
          item.dueTs && item.dueTs <= now && !item.remindedAt && !notifiedRef.current.has(notifyKey)
        );
      });
      if (dueReminders.length === 0) return;

      dueReminders.forEach((dueReminder) => {
        notifiedRef.current.add(`${dueReminder.id}:${dueReminder.dueTs ?? ''}`);
        if (
          showNotifications &&
          (supportsPlatformSystemNotifications() || notificationPermission('granted'))
        ) {
          const room = mx.getRoom(dueReminder.roomId);
          const event = room?.findEventById(dueReminder.eventId);
          const openEventId = getThreadRootEventId(event) ?? dueReminder.eventId;
          notify('A saved reminder is due.', dueReminder.roomId, openEventId);
        }
      });
      dueReminders.forEach((dueReminder) => {
        void markLaterRemindedWithNativeOwner(dueReminder.id, now).catch(() => undefined);
      });
    };

    checkDueReminders();
    const interval = window.setInterval(checkDueReminders, 60_000);
    return () => window.clearInterval(interval);
  }, [mx, laterContent, reminders, showNotifications, notify]);

  return null;
}

function PlatformAgentActionListener() {
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void | Promise<void>) | undefined;

    void registerPlatformAgentActionListener().then((cleanup) => {
      if (disposed) {
        void cleanup?.();
        return;
      }
      unlisten = cleanup;
    });

    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, []);

  return null;
}

function DesktopShortcutSync() {
  const [showShortcut] = useSetting(desktopPlatformSettingsAtom, 'desktopShortcutShow');
  const [laterShortcut] = useSetting(desktopPlatformSettingsAtom, 'desktopShortcutLater');
  const [notificationsShortcut] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopShortcutNotifications'
  );

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    if (supportsPlatformGlobalShortcuts()) {
      let cancelled = false;
      const sync = async () => {
        const result = await setPlatformShortcuts({
          show: showShortcut,
          later: laterShortcut,
          notifications: notificationsShortcut,
        });
        if (!cancelled && result.success) {
          // Sync completed.
        }
      };
      sync();
      cleanup = () => {
        cancelled = true;
      };
    }
    return cleanup;
  }, [showShortcut, laterShortcut, notificationsShortcut]);

  return null;
}

type ClientNonUIFeaturesProps = {
  children: ReactNode;
};

export function ClientNonUIFeatures({ children }: ClientNonUIFeaturesProps) {
  return (
    <DesktopUpdaterProvider>
      <SystemEmojiFeature />
      <PageZoomFeature />
      <FaviconUpdater />
      <TrayDoNotDisturbSync />
      <PlatformBadgeAndTrayUpdater />
      <DesktopShortcutSync />
      <PlatformAgentActionListener />
      <InviteNotifications />
      <AgentApprovalNotifications />
      <MessageNotifications />
      <LaterReminderNotifications />
      <PerformanceDebugOverlay />
      {children}
    </DesktopUpdaterProvider>
  );
}
