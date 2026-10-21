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
import {
  allInvitesAtom,
  useNativeInvites,
  useNativeInviteSyncing,
} from '../../state/room-list/inviteList';
import { useNativeRoomListSnapshot } from '../../state/room-list/roomList';
import { getInboxInvitesPath } from '../pathUtils';
import { getMemberDisplayName, getThreadRootEventId } from '../../utils/room';
import { getMxIdLocalPart } from '../../utils/matrix';
import { useSelectedRoom } from '../../hooks/router/useSelectedRoom';
import { useInboxNotificationsSelected } from '../../hooks/router/useInbox';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { getSortedLaterItems } from '../../utils/later';
import { getTrayNotificationListCount } from '../../notifications/badgeSummary';
import { laterContentAtom } from '../../state/laterList';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { PerformanceDebugOverlay } from '../../components/performance/PerformanceDebugOverlay';
import {
  useApprovalInbox,
  useApprovalInboxSummary,
} from '../../features/approvals/ApprovalInboxProvider';
import {
  getPlatformNotificationSummary,
  registerPlatformAgentActionListener,
  registerPlatformNotificationActionListener,
  setPlatformBadgeCount,
  setPlatformShortcuts,
  setPlatformTrayState,
  showPlatformNotification,
  playPlatformNotificationSound,
  subscribePlatformTrayDndToggle,
  supportsPlatformGlobalShortcuts,
  supportsPlatformSystemNotifications,
  supportsPlatformTrayState,
} from '../../platform';
import {
  buildAgentApprovalNativeActionDedupeKey,
  buildAgentApprovalNotificationText,
  createAgentApprovalNativeActionDedupeStore,
  executeAgentApprovalNativeActionOnce,
  planAgentApprovalNativeNotificationAction,
} from '../../utils/agentApprovals';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { dismissDesktopNotifications } from '../../utils/desktop';
import { DesktopAppShortcuts } from '../../features/desktop-keyboard/DesktopAppShortcuts';
import {
  createApprovalAlertWithdrawal,
  createInviteArrivalTracker,
  createRoomReadTracker,
  inviteNotificationText,
  isDocumentActive,
  shouldPlayNotificationSound,
  shouldPresentOrdinaryMessage,
} from '../../notifications/notificationDelivery';
import { useLatestRef } from '../../hooks/useLatestRef';
import { notifiedEventIdsCache } from '../../notifications/notificationCaches';
import { DesktopUpdaterProvider } from '../../features/desktop-updater/DesktopUpdaterProvider';
import { decideAgentApprovalWithNativeOwner } from '../../features/room/nativeReactionOwner';
import { subscribeApprovalDecisions } from '../../features/approvals/approvalDecisionEvents';
import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  setNotificationFocusWithNativeOwner,
  type NativeNotificationDeliveryOutcome,
  type NativeNotificationCandidate,
} from '../../features/room/nativeNotificationDecision';
import {
  buildNativeObservedNotificationPresentation,
  createObservedBrowserNotificationRegistry,
  isNativeNotificationActionForSession,
  deliverNativeObservedNotificationCandidate,
} from '../../features/room/nativeNotificationPresentation';
import { markLaterRemindedWithNativeOwner } from '../../features/room/nativeLaterOwner';
import {
  subscribeNativeNotificationObservations,
  type NativeNotificationObservation,
} from '../../features/room/nativeNotificationObservation';
import { getMyUserId } from '../../state/nativeIdentity';

import { getNativeRoom, nativeSession } from '../../native/nativeSession';
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
  const { presentation } = useNativeRoomListSnapshot();
  const { pendingCount: agentApprovalCount } = useApprovalInboxSummary();
  const invites = useAtomValue(allInvitesAtom);
  const laterContent = useAtomValue(laterContentAtom);
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');

  useEffect(() => {
    const activeLaterCount = getSortedLaterItems(laterContent).filter(
      (item) => !item.completedAt
    ).length;
    const summary = getPlatformNotificationSummary({
      highlightTotal: presentation.highlightTotal,
      unreadTotal: presentation.unreadTotal,
      laterActiveCount: activeLaterCount,
      inviteCount: invites.length,
      agentApprovalCount,
    });

    setPlatformBadgeCount(summary.appBadgeCount);
    if (supportsPlatformTrayState()) {
      setPlatformTrayState({
        unreadCount: summary.unreadCount,
        highlightCount: summary.highlightCount,
        laterCount: summary.laterActiveCount,
        notificationInboxCount: getTrayNotificationListCount(summary),
        doNotDisturb: !showNotifications,
      }).catch(() => undefined);
    }
  }, [invites.length, laterContent, presentation, showNotifications, agentApprovalCount]);

  return null;
}

/** Withdraws a room's message banners once it is read here or elsewhere, or left. */
function ReadRoomNotificationCleanup() {
  const { rooms } = useNativeRoomListSnapshot();
  const readTracker = useMemo(createRoomReadTracker, []);

  useEffect(() => {
    const read = readTracker(rooms);
    if (read.length === 0) return;
    dismissDesktopNotifications(read.map((roomId) => `room:${roomId}`)).catch(() => undefined);
  }, [readTracker, rooms]);

  return null;
}

function InviteNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const inviteSnapshot = useNativeInvites();
  const nativeInviteSyncing = useNativeInviteSyncing();
  const arrivals = useMemo(createInviteArrivalTracker, []);

  const navigate = useNavigate();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [notificationSound] = useSetting(settingsAtom, 'isNotificationSounds');

  const notify = useCallback(
    async (count: number): Promise<'delivered' | 'failed'> => {
      const { title, body } = inviteNotificationText(count);
      if (supportsPlatformSystemNotifications()) {
        try {
          const shown = await showPlatformNotification({
            title,
            body,
            route: getInboxInvitesPath(),
          });
          return shown ? 'delivered' : 'failed';
        } catch {
          return 'failed';
        }
      }

      try {
        const noti = new window.Notification(title, {
          icon: LogoPNG,
          badge: LogoPNG,
          body,
          silent: true,
        });

        noti.onclick = () => {
          if (!window.closed) navigate(getInboxInvitesPath());
          noti.close();
        };
        return 'delivered';
      } catch {
        return 'failed';
      }
    },
    [navigate]
  );

  const playSound = useCallback(() => {
    void playPlatformNotificationSound('invite').then((playedNative) => {
      if (!playedNative) {
        void audioRef.current?.play();
      }
    });
  }, []);

  // Settings changes alone never re-announce invites the tracker has seen.
  const deliveryRef = useLatestRef({ showNotifications, notificationSound, notify, playSound });

  useEffect(() => {
    const arrived = arrivals(
      {
        sessionGeneration: inviteSnapshot.sessionGeneration,
        roomIds: inviteSnapshot.invites.map((invite) => invite.roomId),
      },
      nativeInviteSyncing
    );
    if (arrived === 0) return;
    const delivery = deliveryRef.current;
    if (
      !delivery.showNotifications ||
      !(supportsPlatformSystemNotifications() || notificationPermission('granted'))
    ) {
      return;
    }
    void delivery.notify(arrived).then((outcome) => {
      if (shouldPlayNotificationSound({ soundsEnabled: delivery.notificationSound, outcome }))
        delivery.playSound();
    });
  }, [arrivals, inviteSnapshot, nativeInviteSyncing, deliveryRef]);

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

  const browserNotifications = useMemo(createObservedBrowserNotificationRegistry, []);
  useEffect(() => () => browserNotifications.clear(), [browserNotifications]);
  const useAuthentication = useMediaAuthentication();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [notificationSound] = useSetting(settingsAtom, 'isNotificationSounds');

  const { navigateRoom, roomPath } = useRoomNavigate();
  const notificationSelected = useInboxNotificationsSelected();
  const notificationSelectedRef = useLatestRef(notificationSelected);
  const selectedRoomId = useSelectedRoom();

  // Report the platform focus observation into Core. A selected room only
  // suppresses while its window is focused; background windows still notify.
  useEffect(() => {
    const reportFocus = () => {
      const focused = document.hasFocus() ? (selectedRoomId ?? null) : null;
      setNotificationFocusWithNativeOwner(focused).catch(() => undefined);
      if (focused) {
        void dismissDesktopNotifications([`room:${focused}`]);
      }
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
      candidate,
      roomAvatar,
      roomId,
      eventId,
      sessionGeneration,
    }: {
      candidate: NativeNotificationCandidate;
      roomAvatar?: string;
      roomId: string;
      eventId: string;
      sessionGeneration: number;
    }): Promise<NativeNotificationDeliveryOutcome> => {
      let presentation;
      try {
        presentation = buildNativeObservedNotificationPresentation(candidate, {
          roomId,
          eventId,
          sessionGeneration,
        });
      } catch {
        return 'failed';
      }
      const approval = candidate.kind === 'agent_approval';
      // The OS answer is the delivery receipt Core records with the
      // acknowledgement. Errors are reported as `failed`, never swallowed
      // into a silent success.
      if (supportsPlatformSystemNotifications()) {
        try {
          const shown = await showPlatformNotification(presentation);
          return shown ? 'delivered' : 'failed';
        } catch {
          return 'failed';
        }
      }

      try {
        const noti = new window.Notification(presentation.title, {
          icon: approval ? LogoHighlightPNG : roomAvatar,
          badge: approval ? LogoHighlightPNG : roomAvatar,
          body: presentation.body,
          silent: true,
        });

        browserNotifications.track(candidate.candidateId, noti);
        noti.onclose = () => browserNotifications.forget(candidate.candidateId);
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
    [navigateRoom, browserNotifications]
  );

  const playSound = useCallback(() => {
    void playPlatformNotificationSound('message').then((playedNative) => {
      if (!playedNative) {
        void audioRef.current?.play();
      }
    });
  }, []);

  const decideAndNotify = useCallback(
    async (observation: NativeNotificationObservation) => {
      const { roomId, eventId, sender } = observation;
      // Observations already classified as approvals use the approval pump.
      // Core may still promote an opaque message during this route's lookup.
      if (observation.agentApproval !== undefined) {
        return;
      }
      const room = getNativeRoom(roomId);

      const cacheKey = `${roomId}:${eventId}`;
      if (submittedRef.current.has(cacheKey)) return;
      // Core emits an unresolved encrypted observation only after its bounded
      // decrypt budget. Submit it once: Core can resolve newer plaintext or
      // return a nonsticky not-ready error without consuming dedup.

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
          title: room?.name ?? 'Unknown',
          body: `New inbox notification from ${
            (room ? getMemberDisplayName(room, sender) : undefined) ??
            getMxIdLocalPart(sender) ??
            sender
          }`,
          route: roomPath(roomId, { eventId }),
          suppressIfFocusedRoom: true,
        });
      } catch {
        // Core unavailable (no session, event not yet loaded, push context
        // still settling): fail silent without remembering. Core observed
        // this event once; there is no renderer retry and no TS policy
        // fallback.
        return;
      }
      if (readback.decision !== 'show' || !readback.candidate) {
        return;
      }
      const shownCandidate = readback.candidate;
      await deliverNativeObservedNotificationCandidate({
        candidate: shownCandidate,
        observedGeneration: observation.sessionGeneration,
        // Decided at delivery time: the window may have been hidden since.
        presentOrdinaryMessages: shouldPresentOrdinaryMessage({
          notificationListSelected: notificationSelectedRef.current,
          windowActive: isDocumentActive(document),
          isSpaceRoom: room?.isSpaceRoom() ?? false,
        }),
        currentGeneration: () => nativeSession().getSyncStateData()?.sessionGeneration,
        acknowledge: dismissNotificationWithNativeOwner,
        cancelDelivery: async () => {
          browserNotifications.cancel(shownCandidate.candidateId);
          await dismissDesktopNotifications([`candidate:${shownCandidate.candidateId}`]);
        },
        deliver: async (candidate) => {
          let outcome: NativeNotificationDeliveryOutcome | undefined;
          if (
            showNotifications &&
            (supportsPlatformSystemNotifications() || notificationPermission('granted'))
          ) {
            const avatarMxc =
              room?.getAvatarFallbackMember()?.getMxcAvatarUrl() ?? room?.getMxcAvatarUrl();
            outcome = await notify({
              candidate,
              roomAvatar: avatarMxc
                ? resolveMatrixThumbnailUrl(avatarMxc, 96, { useAuthentication })
                : undefined,
              roomId,
              eventId,
              sessionGeneration: observation.sessionGeneration,
            });
          }
          return outcome;
        },
        commit: (outcome) => {
          rememberSubmitted(roomId, eventId);
          if (shownCandidate.kind === 'agent_approval') {
            if (outcome === 'delivered' && !supportsPlatformSystemNotifications()) playSound();
          } else if (
            shouldPlayNotificationSound({
              soundsEnabled: notificationSound,
              requested: readback.sound === true,
              outcome,
            })
          ) {
            playSound();
          }
        },
      });
    },
    [
      browserNotifications,
      notificationSound,
      notificationSelectedRef,
      showNotifications,
      playSound,
      notify,
      rememberSubmitted,
      roomPath,
      useAuthentication,
    ]
  );
  const decideAndNotifyRef = useLatestRef(decideAndNotify);

  // Observation pump: Core pushes one observation per live message-like
  // event the SDK sync delivered (`matrix-notification-observed`). The
  // renderer no longer scans timelines, listens for `Room.timeline`, or gates
  // on a sync state; every observation still goes through Core decide.
  // Registered once: settings and route changes reach the handler through a
  // ref, so no observation is missed while a listener is being replaced.
  useEffect(() => {
    const dispose = subscribeNativeNotificationObservations(
      () => nativeSession().getSyncStateData()?.sessionGeneration,
      (observation) => {
        void decideAndNotifyRef.current(observation);
      }
    );
    return dispose;
  }, [decideAndNotifyRef]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={NotificationSound} type="audio/ogg" />
    </audio>
  );
}

function AgentApprovalNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const browserNotifications = useMemo(createObservedBrowserNotificationRegistry, []);
  useEffect(() => () => browserNotifications.clear(), [browserNotifications]);
  const accountScope = getMyUserId();
  const nativeActionState = useMemo(
    () => ({
      inFlight: new Set<string>(),
      completed: accountScope
        ? createAgentApprovalNativeActionDedupeStore(getDurableApprovalStorage(), accountScope)
        : undefined,
    }),
    [accountScope]
  );
  const nativeActionsInFlight = nativeActionState.inFlight;
  const nativeActionDedupe = nativeActionState.completed;
  const { navigateRoom, roomPath } = useRoomNavigate();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [showApprovalDetails] = useSetting(settingsAtom, 'showApprovalNotificationDetails');

  const notify = useCallback(
    async ({
      candidate,
      roomId,
      eventId,
      sessionGeneration,
    }: {
      candidate: NativeNotificationCandidate;
      roomId: string;
      eventId: string;
      sessionGeneration: number;
    }): Promise<NativeNotificationDeliveryOutcome> => {
      const presentation = buildNativeObservedNotificationPresentation(candidate, {
        roomId,
        eventId,
        sessionGeneration,
      });
      if (supportsPlatformSystemNotifications()) {
        const shown = await showPlatformNotification(presentation);
        return shown ? 'delivered' : 'failed';
      }
      const noti = new window.Notification(presentation.title, {
        icon: LogoHighlightPNG,
        badge: LogoHighlightPNG,
        body: presentation.body,
        silent: true,
      });
      browserNotifications.track(candidate.candidateId, noti);
      noti.onclose = () => browserNotifications.forget(candidate.candidateId);
      noti.onclick = () => {
        if (!window.closed) navigateRoom(roomId, eventId);
        noti.close();
      };
      return 'delivered';
    },
    [navigateRoom, browserNotifications]
  );

  const handleNativeNotificationAction = useCallback(
    async (payload: {
      sessionGeneration?: number;
      actionId: string;
      context?: { kind: string; roomId?: string; eventId?: string };
    }) => {
      const { actionId, context, sessionGeneration } = payload;
      if (
        sessionGeneration === undefined ||
        !isNativeNotificationActionForSession(
          sessionGeneration,
          nativeSession().getSyncStateData()?.sessionGeneration
        )
      )
        return;
      const earlyPlan = planAgentApprovalNativeNotificationAction({
        actionId,
        context,
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
      try {
        await executeAgentApprovalNativeActionOnce({
          key: provisionalDedupeKey,
          completed: dedupe,
          inFlight: nativeActionsInFlight,
          execute: () =>
            decideAgentApprovalWithNativeOwner({
              roomId,
              eventId,
              actionId,
              notificationSessionGeneration: sessionGeneration,
            }),
        });
      } catch (error) {
        // Expired, signed-out, stale, or otherwise rejected decisions fail
        // closed, then open the exact event so the action never appears to
        // have succeeded silently.
        if (
          isNativeNotificationActionForSession(
            sessionGeneration,
            nativeSession().getSyncStateData()?.sessionGeneration
          )
        )
          navigateRoom(roomId, eventId);
        if (import.meta.env?.DEV) {
          // eslint-disable-next-line no-console
          console.debug('[synara:agent-approval] native decision failed closed', error);
        }
      }
    },
    [nativeActionDedupe, nativeActionsInFlight, navigateRoom]
  );
  const handleNativeNotificationActionRef = useLatestRef(handleNativeNotificationAction);

  // One long-lived action listener; an Approve/Deny click that lands while a
  // replacement listener is still registering would otherwise be lost.
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void | Promise<void>) | undefined;

    registerPlatformNotificationActionListener((payload) => {
      void handleNativeNotificationActionRef.current(payload);
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
  }, [handleNativeNotificationActionRef]);

  useEffect(() => {
    return subscribeApprovalDecisions((notice) => {
      void dismissDesktopNotifications([`event:${notice.eventId}`]);
    });
  }, []);

  // A delivered approval alert is withdrawn when it expires or Core reports
  // the prompt decided (on another device, too), so stale buttons never stay.
  const approvalAlerts = useMemo(
    () =>
      createApprovalAlertWithdrawal({
        withdraw: (eventId) => {
          dismissDesktopNotifications([`event:${eventId}`]).catch(() => undefined);
        },
      }),
    []
  );
  useEffect(() => () => approvalAlerts.clear(), [approvalAlerts]);
  const { items: approvalItems } = useApprovalInbox();
  useEffect(() => {
    approvalItems.forEach((item) => {
      if (item.status !== 'pending') approvalAlerts.settle(item.eventId);
    });
  }, [approvalAlerts, approvalItems]);

  const playSound = useCallback(() => {
    void playPlatformNotificationSound('message').then((playedNative) => {
      if (!playedNative) {
        void audioRef.current?.play();
      }
    });
  }, []);

  const notifyApprovalEvent = useCallback(
    async (observation: NativeNotificationObservation) => {
      const { eventId, agentApproval } = observation;
      if (!agentApproval || agentApproval.expired) return;
      const room = getNativeRoom(observation.roomId);

      if (notifiedEventIdsCache.has(eventId)) return;

      let readback;
      try {
        readback = await decideNotificationWithNativeOwner({
          roomId: observation.roomId,
          eventId,
          kind: 'agent_approval',
          ...buildAgentApprovalNotificationText({
            roomName: room?.name ?? 'Unknown',
            summary: agentApproval.summary,
            showDetails: showApprovalDetails,
          }),
          route: roomPath(observation.roomId, { eventId }),
          suppressIfFocusedRoom: false,
        });
      } catch {
        return;
      }
      if (readback.decision !== 'show' || !readback.candidate) return;
      const shownCandidate = readback.candidate;
      await deliverNativeObservedNotificationCandidate({
        candidate: shownCandidate,
        observedGeneration: observation.sessionGeneration,
        currentGeneration: () => nativeSession().getSyncStateData()?.sessionGeneration,
        acknowledge: dismissNotificationWithNativeOwner,
        cancelDelivery: async () => {
          browserNotifications.cancel(shownCandidate.candidateId);
          await dismissDesktopNotifications([`candidate:${shownCandidate.candidateId}`]);
        },
        deliver: async (candidate) => {
          let outcome: NativeNotificationDeliveryOutcome | undefined;
          if (
            showNotifications &&
            (supportsPlatformSystemNotifications() || notificationPermission('granted'))
          ) {
            outcome = await notify({
              roomId: observation.roomId,
              eventId,
              candidate,
              sessionGeneration: observation.sessionGeneration,
            });
          }
          return outcome;
        },
        commit: (outcome) => {
          notifiedEventIdsCache.add(eventId);
          if (outcome === 'delivered') approvalAlerts.track(eventId, agentApproval.expiresAt);
          // Native approval delivery already owns its time-sensitive sound.
          if (outcome === 'delivered' && !supportsPlatformSystemNotifications()) playSound();
        },
      });
    },
    [
      approvalAlerts,
      browserNotifications,
      notify,
      playSound,
      roomPath,
      showApprovalDetails,
      showNotifications,
    ]
  );
  const notifyApprovalEventRef = useLatestRef(notifyApprovalEvent);

  // Approval prompts ride the same Core observation stream as messages; the
  // renderer consumes Core classification and Core revalidates before delivery.
  useEffect(() => {
    const dispose = subscribeNativeNotificationObservations(
      () => nativeSession().getSyncStateData()?.sessionGeneration,
      (observation) => notifyApprovalEventRef.current(observation)
    );
    return dispose;
  }, [notifyApprovalEventRef]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={NotificationSound} type="audio/ogg" />
    </audio>
  );
}

function LaterReminderNotifications() {
  const { navigateRoom, roomPath } = useRoomNavigate();
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
          route: roomPath(roomId, { eventId }),
          dismissKeys: [`room:${roomId}`],
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
    [navigateRoom, roomPath]
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
          const room = getNativeRoom(dueReminder.roomId);
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
  }, [laterContent, reminders, showNotifications, notify]);

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
      <ReadRoomNotificationCleanup />
      <DesktopShortcutSync />
      <DesktopAppShortcuts />
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
