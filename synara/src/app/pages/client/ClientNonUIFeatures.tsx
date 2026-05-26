import { useAtomValue } from 'jotai';
import React, { ReactNode, useCallback, useEffect, useMemo, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { ClientEvent, MatrixEvent, Room, RoomEvent, RoomEventHandlerMap } from 'matrix-js-sdk';
import { roomToUnreadAtom, unreadEqual, unreadInfoToUnread } from '../../state/room/roomToUnread';
import LogoPNG from '../../../../public/res/png/synara.png';
import LogoUnreadPNG from '../../../../public/res/png/synara-unread.png';
import LogoHighlightPNG from '../../../../public/res/png/synara-highlight.png';
import NotificationSound from '../../../../public/sound/notification.ogg';
import InviteSound from '../../../../public/sound/invite.ogg';
import { notificationPermission, setFavicon } from '../../utils/dom';
import { useSetting } from '../../state/hooks/settings';
import { desktopPlatformSettingsAtom, settingsAtom } from '../../state/settings';
import { allInvitesAtom } from '../../state/room-list/inviteList';
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
import { NotificationType, UnreadInfo } from '../../../types/matrix/room';
import { getMxIdLocalPart, mxcUrlToHttp } from '../../utils/matrix';
import { useSelectedRoom } from '../../hooks/router/useSelectedRoom';
import { useInboxNotificationsSelected } from '../../hooks/router/useInbox';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useAccountData } from '../../hooks/useAccountData';
import { AccountDataEvent, SynaraLaterContent } from '../../../types/matrix/accountData';
import { getSortedLaterItems, updateLaterContent } from '../../utils/later';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { PerformanceDebugOverlay } from '../../components/performance/PerformanceDebugOverlay';
import {
  getPlatformNotificationSummary,
  setPlatformBadgeCount,
  setPlatformShortcuts,
  setPlatformTrayState,
  showPlatformNotification,
  supportsPlatformGlobalShortcuts,
  supportsPlatformSystemNotifications,
  supportsPlatformTrayState,
} from '../../platform';
import { detectAgentApprovalPrompt } from '../../utils/agentApprovals';

const RECENT_AGENT_APPROVAL_MS = 10 * 60 * 1000;

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

function PlatformBadgeAndTrayUpdater() {
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const invites = useAtomValue(allInvitesAtom);
  const laterEvent = useAccountData(AccountDataEvent.SynaraLater);
  const laterContent = laterEvent?.getContent() as SynaraLaterContent | undefined;
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
  const mx = useMatrixClient();

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
    if (invites.length > perviousInviteLen && mx.getSyncState() === 'SYNCING') {
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
  }, [mx, invites, perviousInviteLen, showNotifications, notificationSound, notify, playSound]);

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
  const unreadCacheRef = useRef<Map<string, UnreadInfo>>(new Map());
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
    const handleTimelineEvent: RoomEventHandlerMap[RoomEvent.Timeline] = (
      mEvent,
      room,
      toStartOfTimeline,
      removed,
      data
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
      const cachedUnreadInfo = unreadCacheRef.current.get(room.roomId);
      unreadCacheRef.current.set(room.roomId, unreadInfo);

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
            ? mxcUrlToHttp(mx, avatarMxc, useAuthentication, 96, 96, 'crop') ?? undefined
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
    mx.on(RoomEvent.Timeline, handleTimelineEvent);
    return () => {
      mx.removeListener(RoomEvent.Timeline, handleTimelineEvent);
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
  const notifiedEventIdsRef = useRef<Set<string>>(new Set());
  const mx = useMatrixClient();
  const { navigateRoom } = useRoomNavigate();

  const notify = useCallback(
    ({
      roomId,
      eventId,
      roomName,
      title,
      body,
      commandPreview,
    }: {
      roomId: string;
      eventId: string;
      roomName: string;
      title: string;
      body: string;
      commandPreview?: string;
    }) => {
      const notificationBody = commandPreview ? `${body}\n${commandPreview}` : body;

      if (supportsPlatformSystemNotifications()) {
        showPlatformNotification({
          title,
          body: `${roomName}: ${notificationBody}`,
          route: `/home/${encodeURIComponent(roomId)}/${encodeURIComponent(eventId)}/`,
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

  const playSound = useCallback(() => {
    audioRef.current?.play();
  }, []);

  const notifyApprovalEvent = useCallback(
    (mEvent: MatrixEvent, room: Room) => {
      if (room.isSpaceRoom()) return;
      if (mEvent.getSender() === mx.getUserId()) return;
      if (Date.now() - mEvent.getTs() > RECENT_AGENT_APPROVAL_MS) return;

      const eventId = mEvent.getId();
      if (!eventId || notifiedEventIdsRef.current.has(eventId)) return;

      const prompt = detectAgentApprovalPrompt(mEvent.getContent<Record<string, unknown>>());
      if (!prompt) return;

      notifiedEventIdsRef.current.add(eventId);
      const openEventId = getThreadRootEventId(room.findEventById(eventId)) ?? eventId;
      if (supportsPlatformSystemNotifications() || notificationPermission('granted')) {
        notify({
          roomId: room.roomId,
          eventId: openEventId,
          roomName: room.name ?? 'Unknown',
          title: prompt.title,
          body: prompt.body,
          commandPreview: prompt.commandPreview,
        });
      }

      playSound();
    },
    [mx, notify, playSound]
  );

  useEffect(() => {
    const handleTimelineEvent: RoomEventHandlerMap[RoomEvent.Timeline] = (
      mEvent,
      room,
      toStartOfTimeline,
      removed
    ) => {
      if (!room || toStartOfTimeline || removed) return;
      notifyApprovalEvent(mEvent, room);
    };

    mx.on(RoomEvent.Timeline, handleTimelineEvent);
    return () => {
      mx.removeListener(RoomEvent.Timeline, handleTimelineEvent);
    };
  }, [mx, notifyApprovalEvent]);

  useEffect(() => {
    const scanRecentApprovalEvents = () => {
      mx.getRooms().forEach((room) => {
        if (room.isSpaceRoom()) return;
        const events = room.getLiveTimeline().getEvents() as MatrixEvent[];
        for (let index = events.length - 1; index >= 0; index -= 1) {
          const event = events[index];
          if (!event) continue;
          if (Date.now() - event.getTs() > RECENT_AGENT_APPROVAL_MS) break;
          notifyApprovalEvent(event, room);
        }
      });
    };

    scanRecentApprovalEvents();
    const interval = window.setInterval(scanRecentApprovalEvents, 30_000);
    mx.on(ClientEvent.Sync, scanRecentApprovalEvents);
    return () => {
      window.clearInterval(interval);
      mx.removeListener(ClientEvent.Sync, scanRecentApprovalEvents);
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
  const laterEvent = useAccountData(AccountDataEvent.SynaraLater);
  const laterContent = laterEvent?.getContent() as SynaraLaterContent | undefined;
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
      updateLaterContent(mx, (current) => {
        const items = { ...(current.items ?? {}) };
        dueReminders.forEach((dueReminder) => {
          const item = items[dueReminder.id];
          if (item?.kind === 'reminder' && item.dueTs === dueReminder.dueTs && !item.remindedAt) {
            items[dueReminder.id] = { ...item, remindedAt: now };
          }
        });
        return { ...current, items };
      });
    };

    checkDueReminders();
    const interval = window.setInterval(checkDueReminders, 60_000);
    return () => window.clearInterval(interval);
  }, [mx, laterContent, reminders, showNotifications, notify]);

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
    <>
      <SystemEmojiFeature />
      <PageZoomFeature />
      <FaviconUpdater />
      <PlatformBadgeAndTrayUpdater />
      <DesktopShortcutSync />
      <InviteNotifications />
      <AgentApprovalNotifications />
      <MessageNotifications />
      <LaterReminderNotifications />
      <PerformanceDebugOverlay />
      {children}
    </>
  );
}
