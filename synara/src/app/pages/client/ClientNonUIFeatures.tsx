import { useAtomValue } from 'jotai';
import { isKeyHotkey } from 'is-hotkey';
import React, { ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import type { MatrixEventReading } from '../../utils/room';
import type { EventedRoomReading } from '../../utils/roomEvents';

type NonUIRoomReading = EventedRoomReading & {
  findEventById(eventId: string): MatrixEventReading | undefined;
};
type LocalMx = ReturnType<typeof useMatrixClient>;
import { roomToUnreadAtom, unreadEqual, unreadInfoToUnread } from '../../state/room/roomToUnread';
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
import {
  getMemberDisplayName,
  getNotificationType,
  getThreadRootEventId,
  getUnreadInfo,
  isNotificationEvent,
} from '../../utils/room';
import { NotificationType } from '../../../types/matrix/room';
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
  isDesktopPlatform,
  readPlatformClipboardText,
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
import {
  notifiedEventIdsCache,
  unreadNotificationCache,
} from '../../notifications/notificationCaches';
import { getLoadedLiveTimelineEvents } from '../../utils/timelineLifecycle';
import { DesktopUpdaterProvider } from '../../features/desktop-updater/DesktopUpdaterProvider';
import { decideAgentApprovalWithNativeOwner } from '../../features/room/nativeReactionOwner';
import { markLaterRemindedWithNativeOwner } from '../../features/room/nativeLaterOwner';

const RECENT_AGENT_APPROVAL_MS = AGENT_APPROVAL_NATIVE_ACTION_TTL_MS;

const getSessionStorage = (): Storage | null => {
  try {
    return typeof sessionStorage === 'undefined' ? null : sessionStorage;
  } catch {
    return null;
  }
};

const NATIVE_PASTE_EVENT = 'synara://native-paste';
const TEXT_INPUT_TYPES = new Set(['', 'email', 'password', 'search', 'tel', 'text', 'url']);

type NativeTextPasteTarget = HTMLInputElement | HTMLTextAreaElement | HTMLElement;

const inputEvent = (text: string): Event => {
  if (typeof InputEvent === 'undefined') {
    return new Event('input', { bubbles: true });
  }

  return new InputEvent('input', {
    bubbles: true,
    inputType: 'insertFromPaste',
    data: text,
  });
};

const nativeTextPasteTarget = (): NativeTextPasteTarget | undefined => {
  const activeElement = document.activeElement;
  if (activeElement instanceof HTMLTextAreaElement) {
    if (activeElement.disabled || activeElement.readOnly) return undefined;
    return activeElement;
  }
  if (activeElement instanceof HTMLInputElement) {
    if (activeElement.disabled || activeElement.readOnly) return undefined;
    if (!TEXT_INPUT_TYPES.has(activeElement.type)) return undefined;
    return activeElement;
  }
  if (activeElement instanceof HTMLElement && activeElement.isContentEditable) {
    if (activeElement.getAttribute('data-editable-name') === 'RoomInput') return undefined;
    return activeElement;
  }
  return undefined;
};

const pasteTextIntoTarget = (target: NativeTextPasteTarget, text: string): boolean => {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    const start = target.selectionStart ?? target.value.length;
    const end = target.selectionEnd ?? start;
    target.setRangeText(text, start, end, 'end');
    target.dispatchEvent(inputEvent(text));
    return true;
  }

  target.focus();
  return document.execCommand('insertText', false, text);
};

function DesktopNativeTextPasteFeature() {
  useEffect(() => {
    if (!isDesktopPlatform()) return undefined;

    const pasteIntoFocusedTextField = async (evt?: Event) => {
      const target = nativeTextPasteTarget();
      if (!target) return false;

      evt?.preventDefault();
      const text = await readPlatformClipboardText();
      if (!text) return false;

      return pasteTextIntoTarget(target, text);
    };
    const handleNativePasteEvent = (evt: Event) => {
      void pasteIntoFocusedTextField(evt);
    };
    const handleNativePasteKey = (evt: KeyboardEvent) => {
      if (!isKeyHotkey('mod+v', evt)) return;
      void pasteIntoFocusedTextField(evt);
    };

    window.addEventListener(NATIVE_PASTE_EVENT, handleNativePasteEvent);
    window.addEventListener('keydown', handleNativePasteKey, true);
    return () => {
      window.removeEventListener(NATIVE_PASTE_EVENT, handleNativePasteEvent);
      window.removeEventListener('keydown', handleNativePasteKey, true);
    };
  }, []);

  return null;
}

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

  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const [showNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [notificationSound] = useSetting(settingsAtom, 'isNotificationSounds');

  const { navigateRoom } = useRoomNavigate();
  const notificationSelected = useInboxNotificationsSelected();
  const selectedRoomId = useSelectedRoom();

  const notify = useCallback(
    ({
      roomName,
      roomAvatar,
      username,
      roomId,
      eventId,
    }: {
      roomName: string;
      roomAvatar?: string;
      username: string;
      roomId: string;
      eventId: string;
    }) => {
      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title: roomName,
          body: `New inbox notification from ${username}`,
          route: buildDesktopNotificationRoomRoute(roomId, eventId),
        }).catch(() => undefined);
        return;
      }

      const noti = new window.Notification(roomName, {
        icon: roomAvatar,
        badge: roomAvatar,
        body: `New inbox notification from ${username}`,
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

  useEffect(() => {
    const handleTimelineEvent = (
      mEvent: MatrixEventReading,
      room: NonUIRoomReading | undefined,
      toStartOfTimeline: boolean,
      removed: boolean,
      data: { liveEvent?: boolean; [key: string]: unknown }
    ) => {
      if (mx.getSyncState() !== 'SYNCING') return;
      if (document.hasFocus() && (selectedRoomId === room?.roomId || notificationSelected)) return;
      if (
        !room ||
        !data.liveEvent ||
        room.isSpaceRoom() ||
        !isNotificationEvent(mEvent) ||
        getNotificationType(mx, room.roomId) === NotificationType.Mute
      ) {
        return;
      }

      const sender = mEvent.getSender();
      const eventId = mEvent.getId();
      if (!sender || !eventId || mEvent.getSender() === mx.getUserId()) return;
      if (detectAgentApprovalPrompt(mEvent.getContent<Record<string, unknown>>())) return;
      const openEventId = getThreadRootEventId(room.findEventById(eventId)) ?? eventId;
      const unreadInfo = getUnreadInfo(room);
      const cachedUnreadInfo = unreadNotificationCache.get(room.roomId);
      unreadNotificationCache.set(room.roomId, unreadInfo);

      if (unreadInfo.total === 0) return;
      if (
        cachedUnreadInfo &&
        unreadEqual(unreadInfoToUnread(cachedUnreadInfo), unreadInfoToUnread(unreadInfo))
      ) {
        return;
      }

      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        const avatarMxc =
          room.getAvatarFallbackMember()?.getMxcAvatarUrl() ?? room.getMxcAvatarUrl();
        notify({
          roomName: room.name ?? 'Unknown',
          roomAvatar: avatarMxc
            ? resolveMatrixThumbnailUrl(mx, avatarMxc, 96, { useAuthentication })
            : undefined,
          username: getMemberDisplayName(room, sender) ?? getMxIdLocalPart(sender) ?? sender,
          roomId: room.roomId,
          eventId: openEventId,
        });
      }

      if (notificationSound) {
        playSound();
      }
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
  }, [
    mx,
    notificationSound,
    notificationSelected,
    showNotifications,
    playSound,
    notify,
    selectedRoomId,
    useAuthentication,
  ]);

  return (
    // eslint-disable-next-line jsx-a11y/media-has-caption
    <audio ref={audioRef} style={{ display: 'none' }}>
      <source src={NotificationSound} type="audio/ogg" />
    </audio>
  );
}

function AgentApprovalNotifications() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const nativeActionDedupeRef = useRef(
    createAgentApprovalNativeActionDedupeStore(getSessionStorage())
  );

  const mx = useMatrixClient();
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

      const dedupe = nativeActionDedupeRef.current;
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
    [navigateRoom]
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
      const openEventId = getThreadRootEventId(room.findEventById(eventId)) ?? eventId;
      if (
        showNotifications &&
        (supportsPlatformSystemNotifications() || notificationPermission('granted'))
      ) {
        notify({
          roomId: room.roomId,
          eventId: openEventId,
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
      <DesktopNativeTextPasteFeature />
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
