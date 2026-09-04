import { useAtomValue } from 'jotai';
import React, { ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import type { MatrixEventReading } from '../../utils/room';
import type { EventedRoomReading } from '../../utils/roomEvents';

type NonUIRoomReading = EventedRoomReading & {
  findEventById(eventId: string): MatrixEventReading | undefined;
  isDirect?: boolean;
  isEncrypted?: boolean;
  notificationMode?: 'all' | 'mentions' | 'mute' | 'default';
};
type LocalMx = ReturnType<typeof useMatrixClient>;
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
import { getMemberDisplayName, getThreadRootEventId, isNotificationEvent } from '../../utils/room';
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
import { getLoadedLiveTimelineEvents } from '../../utils/timelineLifecycle';
import { DesktopUpdaterProvider } from '../../features/desktop-updater/DesktopUpdaterProvider';
import { decideAgentApprovalWithNativeOwner } from '../../features/room/nativeReactionOwner';
import {
  decideNotificationWithNativeOwner,
  dismissNotificationWithNativeOwner,
  eventIsHighlightObservation,
  resolveObservedNotificationRoomMode,
  roomOverrideMapFromSnapshots,
  setNotificationFocusWithNativeOwner,
} from '../../features/room/nativeNotificationDecision';
import {
  nativeRoomNotificationsSnapshot,
  subscribeNativeRoomNotifications,
} from '../../features/settings/notifications/nativeRoomNotification';
import {
  nativePushRulesSnapshot,
  subscribeNativePushRules,
  type NativePushRulesSnapshot,
} from '../../features/settings/notifications/nativePushRules';
import {
  getOwnProfileNative,
  OWN_PROFILE_CHANGED_EVENT,
} from '../../features/settings/account/nativeProfile';
import { markLaterRemindedWithNativeOwner } from '../../features/room/nativeLaterOwner';

const RECENT_AGENT_APPROVAL_MS = AGENT_APPROVAL_NATIVE_ACTION_TTL_MS;
// Only freshly observed timeline events are submitted to the Core decision
// stream. Older loaded history never notifies; Core dedup is the authority,
// this bound only keeps startup scans cheap.
const RECENT_MESSAGE_NOTIFICATION_MS = 5 * 60 * 1000;
// Local submit-memory bound. Core `(room, event)` dedup is authoritative;
// this set only avoids resubmitting the same event on every sync tick.
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
  // only keeps sync scans from resubmitting recorded events on every tick.
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

  const snapshotRef = useRef<{
    roomModes: Map<string, 'all' | 'mentions' | 'mute'>;
    pushRules: NativePushRulesSnapshot | null;
    ownDisplayName: string | null;
  }>({
    roomModes: new Map(),
    pushRules: null,
    ownDisplayName: null,
  });

  const loadNotificationSnapshots = useCallback(async () => {
    try {
      const [rooms, rules, profile] = await Promise.all([
        nativeRoomNotificationsSnapshot(),
        nativePushRulesSnapshot(),
        getOwnProfileNative().catch(() => 'legacy' as const),
      ]);
      snapshotRef.current = {
        roomModes: roomOverrideMapFromSnapshots(rooms),
        pushRules: rules,
        ownDisplayName:
          profile !== 'legacy' && typeof profile.displayName === 'string' && profile.displayName
            ? profile.displayName
            : snapshotRef.current.ownDisplayName,
      };
    } catch {
      // Keep the last good cache. decideAndNotify fail-closes without defaults.
    }
  }, []);

  useEffect(() => {
    void loadNotificationSnapshots();
    const unsubRooms = subscribeNativeRoomNotifications(() => {
      void loadNotificationSnapshots();
    });
    const unsubRules = subscribeNativePushRules(() => {
      void loadNotificationSnapshots();
    });
    const onProfile = () => {
      void loadNotificationSnapshots();
    };
    window.addEventListener(OWN_PROFILE_CHANGED_EVENT, onProfile);
    const interval = window.setInterval(() => {
      void loadNotificationSnapshots();
    }, 30_000);
    return () => {
      unsubRooms();
      unsubRules();
      window.removeEventListener(OWN_PROFILE_CHANGED_EVENT, onProfile);
      window.clearInterval(interval);
    };
  }, [loadNotificationSnapshots]);

  const rememberSubmitted = useCallback((roomId: string, eventId: string) => {
    const submitted = submittedRef.current;
    submitted.add(`${roomId}:${eventId}`);
    if (submitted.size > NOTIFICATION_SUBMITTED_CACHE_MAX) {
      const oldest = submitted.values().next().value;
      if (oldest !== undefined) submitted.delete(oldest);
    }
  }, []);

  const notify = useCallback(
    ({
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
    }) => {
      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title,
          body,
          route: route ?? buildDesktopNotificationRoomRoute(roomId, eventId),
        }).catch(() => undefined);
        return;
      }

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
    },
    [navigateRoom]
  );

  const playSound = useCallback(() => {
    const audioElement = audioRef.current;
    audioElement?.play();
  }, []);

  const decideAndNotify = useCallback(
    async (mEvent: MatrixEventReading, room: NonUIRoomReading) => {
      const sender = mEvent.getSender();
      const eventId = mEvent.getId();
      if (!sender || !eventId) return;
      // Agent approvals travel the Core approval-decision path, never the
      // generic message route.
      if (detectAgentApprovalPrompt(mEvent.getContent<Record<string, unknown>>())) return;

      const cacheKey = `${room.roomId}:${eventId}`;
      if (submittedRef.current.has(cacheKey)) return;

      const userId = mx.getUserId();
      const openEventId = getThreadRootEventId(room.findEventById(eventId)) ?? eventId;
      if (!snapshotRef.current.pushRules) {
        await loadNotificationSnapshots();
      }
      const { roomModes, pushRules, ownDisplayName } = snapshotRef.current;
      const ciphertext = mEvent.getType() === 'm.room.encrypted';
      const content = mEvent.getContent<Record<string, unknown>>();
      const plaintextBody = !ciphertext && typeof content.body === 'string' ? content.body : null;
      const roomMode = resolveObservedNotificationRoomMode({
        userDefined: roomModes.get(room.roomId),
        listMode: room.notificationMode,
        isEncrypted: room.isEncrypted === true || ciphertext,
        isDirect: room.isDirect === true,
        defaults: pushRules,
      });
      let readback;
      try {
        readback = await decideNotificationWithNativeOwner({
          roomId: room.roomId,
          eventId,
          kind: 'message',
          // Privacy-filtered product strings only: room name and a fixed
          // summary. Never message content, ciphertext, or identifiers.
          title: room.name ?? 'Unknown',
          body: `New inbox notification from ${
            getMemberDisplayName(room, sender) ?? getMxIdLocalPart(sender) ?? sender
          }`,
          route: buildDesktopNotificationRoomRoute(room.roomId, openEventId),
          suppressIfFocusedRoom: true,
          isEncrypted: ciphertext,
          roomMode,
          highlight: eventIsHighlightObservation({
            content,
            userId,
            isEncrypted: ciphertext,
            body: plaintextBody,
            keywords: pushRules?.keywords,
            displayName:
              (userId ? getMemberDisplayName(room, userId) : undefined) ?? ownDisplayName,
            localpart: userId ? getMxIdLocalPart(userId) : null,
            flags: pushRules?.mentions,
          }),
          isOwnEvent: userId != null && sender === userId,
        });
      } catch {
        // Core unavailable (no session): fail silent without remembering, so
        // the next sync scan retries through the same Core owner. There is no
        // TS policy fallback.
        return;
      }
      // Remember only outcomes Core durably recorded: shown candidates and
      // already-seen or own events. Transient suppressions (focus, mute,
      // mentions-only) stay resubmittable so a cleared focus or changed mode
      // can still notify while the event is recent; Core re-decides each time.
      if (
        readback.decision === 'show' ||
        readback.reason === 'duplicate-event' ||
        readback.reason === 'own-event'
      ) {
        rememberSubmitted(room.roomId, eventId);
      }
      if (readback.decision !== 'show' || !readback.candidate) return;
      const shownCandidateId = readback.candidate.candidateId;

      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        const avatarMxc =
          room.getAvatarFallbackMember()?.getMxcAvatarUrl() ?? room.getMxcAvatarUrl();
        notify({
          title: readback.candidate.title,
          body: readback.candidate.body,
          roomAvatar: avatarMxc
            ? resolveMatrixThumbnailUrl(mx, avatarMxc, 96, { useAuthentication })
            : undefined,
          roomId: room.roomId,
          eventId: openEventId,
          route: readback.candidate.route,
        });
      }

      // Sound follows the Core decision: suppressed events stay silent.
      if (notificationSound) {
        playSound();
      }

      // Ack delivery to release the pending candidate. Core retains bounded
      // recent-event dedup independently of the pending queue.
      void dismissNotificationWithNativeOwner(shownCandidateId).catch(() => undefined);
    },
    [
      mx,
      notificationSound,
      showNotifications,
      playSound,
      notify,
      rememberSubmitted,
      useAuthentication,
      loadNotificationSnapshots,
    ]
  );

  useEffect(() => {
    const handleTimelineEvent = (
      mEvent: MatrixEventReading,
      room: NonUIRoomReading | undefined,
      toStartOfTimeline: boolean,
      removed: boolean,
      data: { liveEvent?: boolean; [key: string]: unknown }
    ) => {
      if (mx.getSyncState() !== 'SYNCING') return;
      // The notification inbox triage view suppresses message toasts while
      // the user is working through notifications.
      if (notificationSelected) return;
      if (!room || !data.liveEvent || room.isSpaceRoom() || !isNotificationEvent(mEvent)) {
        return;
      }
      void decideAndNotify(mEvent, room);
    };
    mx.on(
      'Room.timeline' as unknown as Parameters<LocalMx['on']>[0],
      handleTimelineEvent as unknown as Parameters<LocalMx['on']>[1]
    );
    return () => {
      mx.removeListener(
        'Room.timeline' as unknown as Parameters<LocalMx['removeListener']>[0],
        handleTimelineEvent as unknown as Parameters<LocalMx['removeListener']>[1]
      );
    };
  }, [mx, notificationSelected, decideAndNotify]);

  // The native facade emits `sync`, not `Room.timeline`, so live decisions
  // also flow from sync-driven scans of freshly loaded events. This is an
  // observation pump, not policy: every event still goes through Core decide.
  useEffect(() => {
    const scanRecentMessageEvents = () => {
      if (mx.getSyncState() !== 'SYNCING') return;
      if (notificationSelected) return;
      const now = Date.now();
      mx.getRooms().forEach((room) => {
        const candidate = room as unknown as NonUIRoomReading;
        if (candidate.isSpaceRoom()) return;
        const events = getLoadedLiveTimelineEvents(candidate);
        let submitted = 0;
        for (let index = events.length - 1; index >= 0 && submitted < 20; index -= 1) {
          const event = events[index] as unknown as MatrixEventReading | undefined;
          if (!event) continue;
          if (now - event.getTs() > RECENT_MESSAGE_NOTIFICATION_MS) break;
          if (!isNotificationEvent(event)) continue;
          submitted += 1;
          void decideAndNotify(event, candidate);
        }
      });
    };

    scanRecentMessageEvents();
    const interval = window.setInterval(scanRecentMessageEvents, 30_000);
    mx.on('sync' as unknown as Parameters<LocalMx['on']>[0], scanRecentMessageEvents);
    return () => {
      window.clearInterval(interval);
      mx.removeListener(
        'sync' as unknown as Parameters<LocalMx['removeListener']>[0],
        scanRecentMessageEvents
      );
    };
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
    (mEvent: MatrixEventReading, room: NonUIRoomReading) => {
      if (room.isSpaceRoom()) return;
      if (mEvent.getSender() === mx.getUserId()) return;
      if (Date.now() - mEvent.getTs() > RECENT_AGENT_APPROVAL_MS) return;

      const eventId = mEvent.getId();
      if (!eventId || notifiedEventIdsCache.has(eventId)) return;

      const prompt = detectAgentApprovalPrompt(mEvent.getContent<Record<string, unknown>>());
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

  useEffect(() => {
    const handleTimelineEvent = (
      mEvent: MatrixEventReading,
      room: NonUIRoomReading,
      toStartOfTimeline: boolean,
      removed: boolean
    ) => {
      if (!room || toStartOfTimeline || removed) return;
      notifyApprovalEvent(mEvent, room);
    };

    mx.on(
      'Room.timeline' as unknown as Parameters<LocalMx['on']>[0],
      handleTimelineEvent as unknown as Parameters<LocalMx['on']>[1]
    );
    return () => {
      mx.removeListener(
        'Room.timeline' as unknown as Parameters<LocalMx['removeListener']>[0],
        handleTimelineEvent as unknown as Parameters<LocalMx['removeListener']>[1]
      );
    };
  }, [mx, notifyApprovalEvent]);

  useEffect(() => {
    const scanRecentApprovalEvents = () => {
      mx.getRooms().forEach((room) => {
        if (room.isSpaceRoom()) return;
        const events = getLoadedLiveTimelineEvents(room);
        for (let index = events.length - 1; index >= 0; index -= 1) {
          const event = events[index];
          if (!event) continue;
          if (Date.now() - event.getTs() > RECENT_AGENT_APPROVAL_MS) break;
          notifyApprovalEvent(event as MatrixEventReading, room);
        }
      });
    };

    scanRecentApprovalEvents();
    const interval = window.setInterval(scanRecentApprovalEvents, 30_000);
    mx.on('sync' as unknown as Parameters<LocalMx['on']>[0], scanRecentApprovalEvents);
    return () => {
      window.clearInterval(interval);
      mx.removeListener(
        'sync' as unknown as Parameters<LocalMx['removeListener']>[0],
        scanRecentApprovalEvents
      );
    };
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
