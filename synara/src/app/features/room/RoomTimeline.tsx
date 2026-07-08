/* eslint-disable react/destructuring-assignment */
import React, {
  Dispatch,
  MouseEventHandler,
  RefObject,
  SetStateAction,
  Suspense,
  useCallback,
  useEffect,
  lazy,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
} from 'react';
import {
  Direction,
  EventTimeline,
  EventTimelineSet,
  EventTimelineSetHandlerMap,
  IContent,
  MatrixClient,
  MatrixEvent,
  Room,
  RoomEvent,
  RoomEventHandlerMap,
} from 'matrix-js-sdk';
import { EventType } from 'matrix-js-sdk/lib/@types/event';
import { HTMLReactParserOptions } from 'html-react-parser';
import classNames from 'classnames';
import { ReactEditor } from 'slate-react';
import { Editor } from 'slate';
import { ErrorBoundary } from 'react-error-boundary';
import { SessionMembershipData } from 'matrix-js-sdk/lib/matrixrtc/CallMembership';
import to from 'await-to-js';
import { useAtomValue, useSetAtom } from 'jotai';
import { useVirtualizer } from '@tanstack/react-virtual';
import {
  Badge,
  Box,
  Chip,
  ContainerColor,
  Icon,
  Icons,
  Line,
  Scroll,
  Text,
  as,
  color,
  config,
  toRem,
} from 'folds';
import { isKeyHotkey } from 'is-hotkey';
import { Opts as LinkifyOpts } from 'linkifyjs';
import { useTranslation } from 'react-i18next';
import { eventWithShortcode, factoryEventSentBy, getMxIdLocalPart } from '../../utils/matrix';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useAlive } from '../../hooks/useAlive';
import { editableActiveElement, scrollToBottom } from '../../utils/dom';
import {
  DefaultPlaceholder,
  CompactPlaceholder,
  Reply,
  MessageBase,
  Time,
  MessageNotDecryptedContent,
  RedactedContent,
  MSticker,
  ImageContent,
  EventContent,
} from '../../components/message';
import {
  factoryRenderLinkifyWithMention,
  getReactCustomHtmlParser,
  LINKIFY_OPTS,
  makeMentionCustomProps,
  renderMatrixMention,
} from '../../plugins/react-custom-html-parser';
import {
  canEditEvent,
  decryptAllTimelineEvent,
  getEditedEvent,
  getEventReactions,
  getLatestEditableEvt,
  getMemberDisplayName,
  getReactionContent,
  isMembershipChanged,
  reactionOrEditEvent,
  roomHaveUnread,
} from '../../utils/room';
import { useSetting } from '../../state/hooks/settings';
import { MessageLayout, settingsAtom } from '../../state/settings';
import { useMatrixEventRenderer } from '../../hooks/useMatrixEventRenderer';
import { Reactions, Message, Event, EncryptedContent, MessageForwardItem } from './message';
import { useMemberEventParser } from '../../hooks/useMemberEventParser';
import * as customHtmlCss from '../../styles/CustomHtml.css';
import { RoomIntro } from '../../components/room-intro';
import {
  getIntersectionObserverEntry,
  useIntersectionObserver,
} from '../../hooks/useIntersectionObserver';
import { markAsRead, markAsUnread, markEventAsUnread } from '../../utils/notifications';
import { useDebounce } from '../../hooks/useDebounce';
import { getResizeObserverEntry, useResizeObserver } from '../../hooks/useResizeObserver';
import * as css from './RoomTimeline.css';
import { timeDayMonthYear, today, yesterday } from '../../utils/time';
import { createMentionElement, isEmptyEditor, moveCursor } from '../../components/editor';
import { roomIdToReplyDraftAtomFamily } from '../../state/room/roomInputDrafts';
import { usePowerLevelsContext } from '../../hooks/usePowerLevels';
import { GetContentCallback, MessageEvent, StateEvent } from '../../../types/matrix/room';
import { useKeyDown } from '../../hooks/useKeyDown';
import { useDocumentFocusChange } from '../../hooks/useDocumentFocusChange';
import { RenderMessageContent } from '../../components/RenderMessageContent';
import { Image } from '../../components/media';
import { ImageViewer } from '../../components/image-viewer';
import { roomToParentsAtom } from '../../state/room/roomToParents';
import { MessageUnsupportedContent } from '../../components/message/content/FallbackContent';
import { useRoomUnread } from '../../state/hooks/unread';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { useMentionClickHandler } from '../../hooks/useMentionClickHandler';
import { useSpoilerClickHandler } from '../../hooks/useSpoilerClickHandler';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useIgnoredUsers } from '../../hooks/useIgnoredUsers';
import { useImagePackRooms } from '../../hooks/useImagePackRooms';
import { useIsDirectRoom } from '../../hooks/useRoom';
import { useOpenUserRoomProfile } from '../../state/hooks/userRoomProfile';
import { useSpaceOptionally } from '../../hooks/useSpace';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { useAccessiblePowerTagColors, useGetMemberPowerTag } from '../../hooks/useMemberPowerTag';
import { useTheme } from '../../hooks/useTheme';
import { useRoomCreatorsTag } from '../../hooks/useRoomCreatorsTag';
import { usePowerLevelTags } from '../../hooks/usePowerLevelTags';
import { createLaterItem, setLaterItem } from '../../utils/later';
import { useAccountData } from '../../hooks/useAccountData';
import { AccountDataEvent, SynaraUnreadAnchorContent } from '../../../types/matrix/accountData';
import { parsePollStartContent } from '../../utils/polls';
import { addRoomNoteItemAccountData, createMessageRoomNoteItem } from '../../utils/roomNotes';
import { isPerformanceDebugEnabled, perfLog } from '../../utils/performance';
import {
  buildTimelineRowsWithState,
  estimateTimelineRowSize,
  getRestoredVirtualScrollTop,
  getTimelineRowKey,
  getVirtualAnchorOffset,
  isVirtualRangeAtEnd,
  shouldPaginateVirtualRange,
  TIMELINE_VIRTUAL_OVERSCAN,
  TimelineRowsBuildState,
  TimelineVirtualAnchor,
  TimelineVirtualRow,
} from '../../utils/timelineVirtualization';
import {
  clearTimelinePaginationError,
  setTimelinePaginationError,
  shouldShowTimelinePaginationLoader,
  type TimelinePaginationDirection,
  type TimelinePaginationErrors,
} from '../../utils/timelinePagination';
import {
  getLoadedLiveTailEventId,
  shouldRestoreRoomTimelineViewport,
} from '../../utils/timelineLifecycle';
import {
  getEventIdAbsoluteIndex,
  getEventTimeline,
  getFirstLinkedTimeline,
  getLinkedTimelines,
  getLiveTimeline,
  getTimelinesEventsCount,
  timelineToEventsCount,
} from '../../utils/timelineLinks';
import {
  getEmptyTimeline,
  getInitialTimeline,
  getRoomUnreadInfo,
  getTimelineEndWindow,
  hasUnreadForInitialScroll,
  timelineHasEvents,
  canRestoreViewportFromInitialTimeline,
  type TimelineWindow,
} from '../../utils/timelineOpening';

const PollContent = lazy(() =>
  import('../../components/message/content/PollContent').then((module) => ({
    default: module.PollContent,
  }))
);

const TimelineFloat = as<'div', css.TimelineFloatVariants>(
  ({ position, className, ...props }, ref) => (
    <Box
      className={classNames(css.TimelineFloat({ position }), className)}
      justifyContent="Center"
      alignItems="Center"
      gap="200"
      {...props}
      ref={ref}
    />
  )
);

const TimelineDivider = as<'div', { variant?: ContainerColor | 'Inherit' }>(
  ({ variant, children, ...props }, ref) => (
    <Box gap="100" justifyContent="Center" alignItems="Center" {...props} ref={ref}>
      <Line style={{ flexGrow: 1 }} variant={variant} size="300" />
      {children}
      <Line style={{ flexGrow: 1 }} variant={variant} size="300" />
    </Box>
  )
);

type RoomTimelineProps = {
  room: Room;
  eventId?: string;
  roomInputRef: RefObject<HTMLElement | null>;
  editor: Editor;
};

const PAGINATION_LIMIT = 80;
const LIVE_END_PIN_MIN_MS = 700;
const LIVE_END_PIN_MAX_MS = 5000;
const LIVE_END_PIN_STABLE_FRAMES = 10;
const LIVE_END_BOTTOM_TOLERANCE = 24;
const COMPOSER_RESIZE_BOTTOM_TOLERANCE = 160;
const VIRTUAL_ANCHOR_RESTORE_SCROLL_TOLERANCE = 4;

type ScrollToOptions = {
  offset?: number;
  align?: 'start' | 'center' | 'end';
  behavior?: 'auto' | 'instant' | 'smooth';
  stopInView?: boolean;
};

type ScrollToElement = (element: HTMLElement, opts?: ScrollToOptions) => boolean;
type ScrollToItem = (index: number, opts?: ScrollToOptions) => boolean;

const getScrollBottomGap = (scrollEl: HTMLElement): number =>
  scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.offsetHeight;

const isScrollNearBottom = (
  scrollEl: HTMLElement,
  tolerance = LIVE_END_BOTTOM_TOLERANCE
): boolean => getScrollBottomGap(scrollEl) <= tolerance;

const isNearVirtualRangeEnd = (
  range: { endIndex: number } | undefined,
  rowsLength: number,
  tolerance = TIMELINE_VIRTUAL_OVERSCAN + 1
): boolean => {
  if (rowsLength === 0 || typeof range?.endIndex !== 'number') return false;
  const tailIndex = Math.max(0, rowsLength - tolerance);
  return range.endIndex >= tailIndex;
};

const isElementBottomInScrollView = (
  scrollEl: HTMLElement,
  element: HTMLElement,
  tolerance = LIVE_END_BOTTOM_TOLERANCE
): boolean => {
  const scrollRect = scrollEl.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();
  return (
    elementRect.bottom >= scrollRect.top && elementRect.bottom <= scrollRect.bottom + tolerance
  );
};

type RoomTimelineViewport = {
  atBottom: boolean;
  anchor?: TimelineVirtualAnchor;
  liveTailEventId?: string;
  updatedAtMs: number;
};

const ROOM_TIMELINE_VIEWPORT_LIMIT = 100;
const roomTimelineViewports = new Map<string, RoomTimelineViewport>();

const setRoomTimelineViewport = (
  roomId: string,
  viewport: Omit<RoomTimelineViewport, 'updatedAtMs'>
) => {
  roomTimelineViewports.delete(roomId);
  roomTimelineViewports.set(roomId, { ...viewport, updatedAtMs: Date.now() });
  if (roomTimelineViewports.size > ROOM_TIMELINE_VIEWPORT_LIMIT) {
    const oldestRoomId = roomTimelineViewports.keys().next().value;
    if (oldestRoomId) roomTimelineViewports.delete(oldestRoomId);
  }
};

type TimelineEventRow = TimelineVirtualRow & {
  kind: 'event';
  eventId: string;
  eventIndex: number;
  mEvent: MatrixEvent;
  eventTimeline: EventTimeline;
  timelineSet: EventTimelineSet;
  collapse: boolean;
  newDivider?: 'server' | 'client';
  dayDividerTs?: number;
};

type TimelineLoaderRow = TimelineVirtualRow & {
  kind: 'loader';
  direction: 'backward' | 'forward';
  observe: boolean;
  placeholderIndex: number;
};

type TimelineDividerRow = TimelineVirtualRow & {
  kind: 'divider';
  divider: 'server-unread' | 'client-unread' | 'day';
  ts?: number;
};

type TimelineIntroRow = TimelineVirtualRow & {
  kind: 'intro';
};

type TimelineBottomRow = TimelineVirtualRow & {
  kind: 'bottom';
};

type TimelineRow =
  | TimelineEventRow
  | TimelineLoaderRow
  | TimelineDividerRow
  | TimelineIntroRow
  | TimelineBottomRow;

const useEventTimelineLoader = (
  mx: MatrixClient,
  room: Room,
  onLoad: (eventId: string, linkedTimelines: EventTimeline[], evtAbsIndex: number) => void,
  onError: (err: Error | null) => void
) => {
  const loadEventTimeline = useCallback(
    async (eventId: string) => {
      const [err, replyEvtTimeline] = await to(
        mx.getEventTimeline(room.getUnfilteredTimelineSet(), eventId)
      );
      if (!replyEvtTimeline) {
        onError(err ?? null);
        return;
      }
      const linkedTimelines = getLinkedTimelines(replyEvtTimeline);
      const absIndex = getEventIdAbsoluteIndex(linkedTimelines, replyEvtTimeline, eventId);

      if (absIndex === undefined) {
        onError(err ?? null);
        return;
      }

      onLoad(eventId, linkedTimelines, absIndex);
    },
    [mx, room, onLoad, onError]
  );

  return loadEventTimeline;
};

const useTimelinePagination = (
  mx: MatrixClient,
  timeline: TimelineWindow,
  setTimeline: Dispatch<SetStateAction<TimelineWindow>>,
  limit: number,
  onPaginationError: (direction: TimelinePaginationDirection, err: unknown | null) => void
) => {
  const timelineRef = useRef(timeline);
  timelineRef.current = timeline;
  const alive = useAlive();

  const handleTimelinePagination = useMemo(() => {
    let fetching = false;

    const recalibratePagination = (
      linkedTimelines: EventTimeline[],
      timelinesEventsCount: number[],
      backwards: boolean
    ) => {
      const topTimeline = linkedTimelines[0];
      const timelineMatch = (mt: EventTimeline) => (t: EventTimeline) => t === mt;

      const newLTimelines = getLinkedTimelines(topTimeline);
      const topTmIndex = newLTimelines.findIndex(timelineMatch(topTimeline));
      const topAddedTm = topTmIndex === -1 ? [] : newLTimelines.slice(0, topTmIndex);

      const topTmAddedEvt =
        timelineToEventsCount(newLTimelines[topTmIndex]) - timelinesEventsCount[0];
      const offsetRange = getTimelinesEventsCount(topAddedTm) + (backwards ? topTmAddedEvt : 0);

      setTimeline((currentTimeline) => ({
        linkedTimelines: newLTimelines,
        range:
          offsetRange > 0
            ? {
                start: currentTimeline.range.start + offsetRange,
                end: currentTimeline.range.end + offsetRange,
              }
            : { ...currentTimeline.range },
      }));
    };

    return async (backwards: boolean) => {
      if (fetching) return;
      const { linkedTimelines: lTimelines } = timelineRef.current;
      const timelinesEventsCount = lTimelines.map(timelineToEventsCount);

      const timelineToPaginate = backwards ? lTimelines[0] : lTimelines[lTimelines.length - 1];
      if (!timelineToPaginate) return;

      const paginationToken = timelineToPaginate.getPaginationToken(
        backwards ? Direction.Backward : Direction.Forward
      );
      if (
        !paginationToken &&
        getTimelinesEventsCount(lTimelines) !==
          getTimelinesEventsCount(getLinkedTimelines(timelineToPaginate))
      ) {
        recalibratePagination(lTimelines, timelinesEventsCount, backwards);
        return;
      }

      fetching = true;
      const [err] = await to(
        mx.paginateEventTimeline(timelineToPaginate, {
          backwards,
          limit,
        })
      );
      fetching = false;
      if (err) {
        onPaginationError(backwards ? 'backward' : 'forward', err);
        return;
      }
      onPaginationError(backwards ? 'backward' : 'forward', null);
      const fetchedTimeline =
        timelineToPaginate.getNeighbouringTimeline(
          backwards ? Direction.Backward : Direction.Forward
        ) ?? timelineToPaginate;
      // Decrypt all event ahead of render cycle
      const roomId = fetchedTimeline.getRoomId();
      const room = roomId ? mx.getRoom(roomId) : null;

      if (room?.hasEncryptionStateEvent()) {
        await to(decryptAllTimelineEvent(mx, fetchedTimeline));
      }
      if (alive()) {
        recalibratePagination(lTimelines, timelinesEventsCount, backwards);
      }
    };
  }, [mx, alive, onPaginationError, setTimeline, limit]);
  return handleTimelinePagination;
};

const useLiveEventArrive = (room: Room, onArrive: (mEvent: MatrixEvent) => void) => {
  useEffect(() => {
    const handleTimelineEvent: EventTimelineSetHandlerMap[RoomEvent.Timeline] = (
      mEvent,
      eventRoom,
      toStartOfTimeline,
      removed,
      data
    ) => {
      if (eventRoom?.roomId !== room.roomId || !data.liveEvent) return;
      onArrive(mEvent);
    };
    const handleRedaction: RoomEventHandlerMap[RoomEvent.Redaction] = (mEvent, eventRoom) => {
      if (eventRoom?.roomId !== room.roomId) return;
      onArrive(mEvent);
    };

    room.on(RoomEvent.Timeline, handleTimelineEvent);
    room.on(RoomEvent.Redaction, handleRedaction);
    return () => {
      room.removeListener(RoomEvent.Timeline, handleTimelineEvent);
      room.removeListener(RoomEvent.Redaction, handleRedaction);
    };
  }, [room, onArrive]);
};

const useLiveTimelineRefresh = (room: Room, onRefresh: () => void) => {
  useEffect(() => {
    const handleTimelineRefresh: RoomEventHandlerMap[RoomEvent.TimelineRefresh] = (r) => {
      if (r.roomId !== room.roomId) return;
      onRefresh();
    };

    room.on(RoomEvent.TimelineRefresh, handleTimelineRefresh);
    return () => {
      room.removeListener(RoomEvent.TimelineRefresh, handleTimelineRefresh);
    };
  }, [room, onRefresh]);
};

const useLiveTimelineReset = (room: Room, onReset: () => void) => {
  useEffect(() => {
    const handleTimelineReset: EventTimelineSetHandlerMap[RoomEvent.TimelineReset] = (
      eventRoom
    ) => {
      if (eventRoom?.roomId !== room.roomId) return;
      onReset();
    };

    room.on(RoomEvent.TimelineReset, handleTimelineReset);
    return () => {
      room.removeListener(RoomEvent.TimelineReset, handleTimelineReset);
    };
  }, [room, onReset]);
};

const toScrollBehavior = (behavior?: ScrollToOptions['behavior']): 'auto' | 'smooth' | undefined =>
  behavior === 'instant' ? 'auto' : behavior;

export function RoomTimeline({ room, eventId, roomInputRef, editor }: RoomTimelineProps) {
  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
  const [messageLayout] = useSetting(settingsAtom, 'messageLayout');
  const [messageSpacing] = useSetting(settingsAtom, 'messageSpacing');
  const [legacyUsernameColor] = useSetting(settingsAtom, 'legacyUsernameColor');
  const direct = useIsDirectRoom();
  const [hideMembershipEvents] = useSetting(settingsAtom, 'hideMembershipEvents');
  const [hideNickAvatarEvents] = useSetting(settingsAtom, 'hideNickAvatarEvents');
  const [mediaAutoLoad] = useSetting(settingsAtom, 'mediaAutoLoad');
  const [showHiddenEvents] = useSetting(settingsAtom, 'showHiddenEvents');
  const [showDeveloperTools] = useSetting(settingsAtom, 'developerTools');

  const [hour24Clock] = useSetting(settingsAtom, 'hour24Clock');
  const [dateFormatString] = useSetting(settingsAtom, 'dateFormatString');

  const ignoredUsersList = useIgnoredUsers();
  const ignoredUsersSet = useMemo(() => new Set(ignoredUsersList), [ignoredUsersList]);

  const setReplyDraft = useSetAtom(roomIdToReplyDraftAtomFamily(room.roomId));
  const powerLevels = usePowerLevelsContext();
  const creators = useRoomCreators(room);

  const creatorsTag = useRoomCreatorsTag();
  const powerLevelTags = usePowerLevelTags(room, powerLevels);
  const getMemberPowerTag = useGetMemberPowerTag(room, creators, powerLevels);

  const theme = useTheme();
  const accessiblePowerTagColors = useAccessiblePowerTagColors(
    theme.kind,
    creatorsTag,
    powerLevelTags
  );

  const permissions = useRoomPermissions(creators, powerLevels);

  const canRedact = permissions.action('redact', mx.getSafeUserId());
  const canDeleteOwn = permissions.event(MessageEvent.RoomRedaction, mx.getSafeUserId());
  const canSendReaction = permissions.event(MessageEvent.Reaction, mx.getSafeUserId());
  const canPinEvent = permissions.stateEvent(StateEvent.RoomPinnedEvents, mx.getSafeUserId());
  const [editId, setEditId] = useState<string>();

  const roomToParents = useAtomValue(roomToParentsAtom);
  const unread = useRoomUnread(room.roomId, roomToUnreadAtom);
  const unreadAnchorContent = useAccountData(AccountDataEvent.SynaraUnreadAnchor)?.getContent() as
    | SynaraUnreadAnchorContent
    | undefined;
  const unreadAnchorEventId = unreadAnchorContent?.anchors?.[room.roomId]?.eventId;
  const { navigateRoom } = useRoomNavigate();
  const mentionClickHandler = useMentionClickHandler(room.roomId);
  const spoilerClickHandler = useSpoilerClickHandler();
  const openUserRoomProfile = useOpenUserRoomProfile();
  const space = useSpaceOptionally();

  const imagePackRooms: Room[] = useImagePackRooms(room.roomId, roomToParents);
  const roomOpenedAtMsRef = useRef(Date.now());

  const savedViewportRef = useRef(!eventId ? roomTimelineViewports.get(room.roomId) : undefined);
  const savedViewport = savedViewportRef.current;
  const initialTimelineWindow = useMemo(
    () => (eventId ? getEmptyTimeline() : getInitialTimeline(room, PAGINATION_LIMIT)),
    [eventId, room]
  );
  const hasInitialUnread =
    hasUnreadForInitialScroll(unread, unreadAnchorEventId) || roomHaveUnread(mx, room);
  const currentLiveTailEventId = getLoadedLiveTailEventId(room);
  const shouldRestoreSavedViewport =
    !eventId &&
    canRestoreViewportFromInitialTimeline(savedViewport, initialTimelineWindow) &&
    shouldRestoreRoomTimelineViewport(savedViewport, {
      hasUnread: hasInitialUnread,
      currentLiveTailEventId,
      nowMs: roomOpenedAtMsRef.current,
    });
  const suppressInitialUnreadScrollRef = useRef(shouldRestoreSavedViewport);

  // Only use the read-receipt marker for initial placement when the room is actually unread.
  // Otherwise a stale receipt can reopen a recently read room several messages above the live end.
  const shouldOpenAtUnread = !shouldRestoreSavedViewport && hasInitialUnread;
  const [unreadInfo, setUnreadInfo] = useState(() =>
    shouldOpenAtUnread ? getRoomUnreadInfo(room, unreadAnchorEventId, true) : undefined
  );
  const readUptoEventIdRef = useRef<string | undefined>(undefined);
  readUptoEventIdRef.current = unreadInfo?.readUptoEventId;

  const atBottomAnchorRef = useRef<HTMLElement>(null);
  const [atBottom, setAtBottom] = useState<boolean>(
    shouldRestoreSavedViewport ? savedViewport?.atBottom ?? true : true
  );
  const atBottomRef = useRef(atBottom);
  atBottomRef.current = atBottom;
  const setAtBottomState = useCallback((nextAtBottom: boolean) => {
    atBottomRef.current = nextAtBottom;
    setAtBottom(nextAtBottom);
  }, []);

  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollToBottomRef = useRef({
    count: 0,
    smooth: true,
  });
  const initialScrollPlacedRef = useRef(false);
  const liveEndPinRef = useRef(
    !eventId &&
      !shouldOpenAtUnread &&
      (!shouldRestoreSavedViewport || !savedViewport || savedViewport.atBottom)
  );
  const liveEndPinStateRef = useRef({
    lastScrollHeight: 0,
    lastTotalSize: 0,
    stableFrames: 0,
    startedAt: 0,
  });
  const liveTimelineResetPendingRef = useRef(false);
  const latestTimelineRequestRef = useRef(0);
  const startLiveEndPin = useCallback(() => {
    liveEndPinRef.current = true;
    liveEndPinStateRef.current = {
      lastScrollHeight: 0,
      lastTotalSize: 0,
      stableFrames: 0,
      startedAt: 0,
    };
  }, []);

  const [focusItem, setFocusItem] = useState<
    | {
        index: number;
        scrollTo: boolean;
        highlight: boolean;
      }
    | undefined
  >();
  const alive = useAlive();

  const linkifyOpts = useMemo<LinkifyOpts>(
    () => ({
      ...LINKIFY_OPTS,
      render: factoryRenderLinkifyWithMention((href) =>
        renderMatrixMention(mx, room.roomId, href, makeMentionCustomProps(mentionClickHandler))
      ),
    }),
    [mx, room, mentionClickHandler]
  );

  useEffect(() => {
    if (suppressInitialUnreadScrollRef.current) {
      suppressInitialUnreadScrollRef.current = false;
      setUnreadInfo(undefined);
      return;
    }
    if (hasInitialUnread) {
      setUnreadInfo((currentUnreadInfo) => {
        if (currentUnreadInfo?.scrollTo) return currentUnreadInfo;
        return getRoomUnreadInfo(room, unreadAnchorEventId, false);
      });
      return;
    }
    setUnreadInfo(undefined);
  }, [room, unreadAnchorEventId, unread, hasInitialUnread]);
  const htmlReactParserOptions = useMemo<HTMLReactParserOptions>(
    () =>
      getReactCustomHtmlParser(mx, room.roomId, {
        linkifyOpts,
        useAuthentication,
        handleSpoilerClick: spoilerClickHandler,
        handleMentionClick: mentionClickHandler,
      }),
    [mx, room, linkifyOpts, spoilerClickHandler, mentionClickHandler, useAuthentication]
  );
  const parseMemberEvent = useMemberEventParser();

  const [timeline, setTimeline] = useState<TimelineWindow>(() => initialTimelineWindow);
  const [paginationErrors, setPaginationErrors] = useState<TimelinePaginationErrors>({});
  const [, startTimelineTransition] = useTransition();
  const eventsLength = getTimelinesEventsCount(timeline.linkedTimelines);
  const liveTimelineLinked =
    timeline.linkedTimelines[timeline.linkedTimelines.length - 1] === getLiveTimeline(room);
  const canPaginateBack =
    typeof timeline.linkedTimelines[0]?.getPaginationToken(Direction.Backward) === 'string';
  const canPaginateForward =
    typeof timeline.linkedTimelines[timeline.linkedTimelines.length - 1]?.getPaginationToken(
      Direction.Forward
    ) === 'string';
  const loadedAtStart = !canPaginateBack;
  const loadedAtEnd = liveTimelineLinked && !canPaginateForward;
  const atLiveEndRef = useRef(loadedAtEnd);
  atLiveEndRef.current = loadedAtEnd;

  const handlePaginationError = useCallback(
    (direction: TimelinePaginationDirection, err: unknown | null) => {
      setPaginationErrors((current) =>
        err
          ? setTimelinePaginationError(current, direction, err)
          : clearTimelinePaginationError(current, direction)
      );
    },
    []
  );

  const handleTimelinePagination = useTimelinePagination(
    mx,
    timeline,
    setTimeline,
    PAGINATION_LIMIT,
    handlePaginationError
  );

  const getScrollElement = useCallback(() => scrollRef.current, []);
  const timelineRowsBuildStateRef = useRef<
    TimelineRowsBuildState<TimelineRow, MatrixEvent> | undefined
  >(undefined);
  const currentReadUptoEventId = readUptoEventIdRef.current;
  const cancelLiveEndPin = useCallback(() => {
    liveEndPinRef.current = false;
    liveEndPinStateRef.current.stableFrames = 0;
  }, []);

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;

    const handleScroll = () => {
      if (liveEndPinRef.current) return;
      const nextAtBottom = isActuallyAtLiveBottomRef.current();
      if (atBottomRef.current !== nextAtBottom) {
        setAtBottomState(nextAtBottom);
      }
    };

    scrollEl.addEventListener('scroll', handleScroll, { passive: true });
    return () => scrollEl.removeEventListener('scroll', handleScroll);
  }, [setAtBottomState]);

  useEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;

    const cancelForUserScroll = () => {
      if (liveEndPinRef.current) cancelLiveEndPin();
    };
    const cancelForScrollKey = (evt: KeyboardEvent) => {
      if (!liveEndPinRef.current) return;
      if (
        evt.key === 'ArrowUp' ||
        evt.key === 'ArrowDown' ||
        evt.key === 'PageUp' ||
        evt.key === 'PageDown' ||
        evt.key === 'Home' ||
        evt.key === 'End' ||
        evt.key === ' '
      ) {
        cancelLiveEndPin();
      }
    };

    scrollEl.addEventListener('wheel', cancelForUserScroll, { passive: true });
    scrollEl.addEventListener('touchstart', cancelForUserScroll, { passive: true });
    scrollEl.addEventListener('pointerdown', cancelForUserScroll, { passive: true });
    window.addEventListener('keydown', cancelForScrollKey);
    return () => {
      scrollEl.removeEventListener('wheel', cancelForUserScroll);
      scrollEl.removeEventListener('touchstart', cancelForUserScroll);
      scrollEl.removeEventListener('pointerdown', cancelForUserScroll);
      window.removeEventListener('keydown', cancelForScrollKey);
    };
  }, [cancelLiveEndPin]);

  useEffect(() => {
    timelineRowsBuildStateRef.current = undefined;
  }, [room.roomId, eventId]);

  useEffect(() => {
    setPaginationErrors({});
  }, [room.roomId, eventId]);

  const timelineRows = useMemo(() => {
    const { rows, state } = buildTimelineRowsWithState(
      timeline.linkedTimelines,
      {
        showIntro: loadedAtStart && eventsLength > 0,
        showBackLoader: shouldShowTimelinePaginationLoader(
          canPaginateBack,
          paginationErrors,
          'backward'
        ),
        showFrontLoader: shouldShowTimelinePaginationLoader(
          !loadedAtEnd,
          paginationErrors,
          'forward'
        ),
        compact: messageLayout === MessageLayout.Compact,
        ignoredUsersSet,
        showHiddenEvents,
        readUptoEventId: currentReadUptoEventId,
        unreadAnchorEventId,
        currentUserId: mx.getUserId(),
      },
      {
        getTimelinesEventsCount,
        isReactionOrEditEvent: reactionOrEditEvent,
        createEventRow: ({ mEvent, eventId: rowEventId, eventIndex, eventTimeline, collapse }) => ({
          kind: 'event',
          key: rowEventId,
          eventId: rowEventId,
          eventIndex,
          mEvent,
          eventTimeline,
          timelineSet: eventTimeline.getTimelineSet(),
          collapse,
        }),
      },
      timelineRowsBuildStateRef.current
    );
    timelineRowsBuildStateRef.current = state;
    return rows;
  }, [
    timeline.linkedTimelines,
    loadedAtStart,
    eventsLength,
    canPaginateBack,
    loadedAtEnd,
    paginationErrors,
    messageLayout,
    ignoredUsersSet,
    showHiddenEvents,
    currentReadUptoEventId,
    unreadAnchorEventId,
    mx,
  ]);
  const eventIndexToRowIndex = useMemo(() => {
    const eventMap = new Map<number, number>();
    timelineRows.forEach((row, rowIndex) => {
      if (row.kind === 'event') eventMap.set(row.eventIndex, rowIndex);
    });
    return eventMap;
  }, [timelineRows]);
  const eventIdToRowIndex = useMemo(() => {
    const eventMap = new Map<string, number>();
    timelineRows.forEach((row, rowIndex) => {
      if (row.kind === 'event') eventMap.set(row.eventId, rowIndex);
    });
    return eventMap;
  }, [timelineRows]);
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: timelineRows.length,
    getScrollElement,
    getItemKey: useCallback(
      (index: number) => getTimelineRowKey(timelineRows[index]),
      [timelineRows]
    ),
    estimateSize: useCallback(
      (index) =>
        estimateTimelineRowSize(
          timelineRows[index],
          messageLayout === MessageLayout.Compact,
          messageLayout === MessageLayout.Compact ? 42 : 96
        ),
      [timelineRows, messageLayout]
    ),
    overscan: TIMELINE_VIRTUAL_OVERSCAN,
    scrollingDelay: 120,
  });
  const virtualItems = virtualizer.getVirtualItems();
  const savedViewportRestoreAnchor =
    !eventId && shouldRestoreSavedViewport && savedViewport && !savedViewport.atBottom
      ? savedViewport.anchor
      : undefined;
  const savedViewportRestoreKey = savedViewportRestoreAnchor
    ? `${room.roomId}:${savedViewportRestoreAnchor.eventId}:${savedViewportRestoreAnchor.offsetTop}`
    : undefined;
  const pendingVirtualAnchorRef = useRef<TimelineVirtualAnchor | undefined>(
    savedViewportRestoreAnchor
  );
  const pendingVirtualAnchorScrollTopRef = useRef<number | undefined>(undefined);
  const savedViewportRestoreKeyRef = useRef(savedViewportRestoreKey);
  const restoringSavedViewportRef = useRef(Boolean(savedViewportRestoreAnchor));

  if (savedViewportRestoreKeyRef.current !== savedViewportRestoreKey) {
    savedViewportRestoreKeyRef.current = savedViewportRestoreKey;
    pendingVirtualAnchorRef.current = savedViewportRestoreAnchor;
    restoringSavedViewportRef.current = Boolean(savedViewportRestoreAnchor);
  }

  const getTimelineEventElement = useCallback(
    (targetEventId: string): HTMLElement | undefined =>
      (scrollRef.current?.querySelector(
        `[data-message-id="${CSS.escape(targetEventId)}"]`
      ) as HTMLElement) ?? undefined,
    []
  );

  const getCurrentVirtualAnchor = useCallback((): TimelineVirtualAnchor | undefined => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;

    const scrollRect = scrollEl.getBoundingClientRect();
    const anchorCandidates = virtualizer
      .getVirtualItems()
      .map((virtualItem) => {
        const row = timelineRows[virtualItem.index];
        if (row.kind !== 'event') return false;
        const element = getTimelineEventElement(row.eventId);
        if (!element) return false;
        return {
          eventId: row.eventId,
          rect: element.getBoundingClientRect(),
        };
      })
      .filter((candidate): candidate is { eventId: string; rect: DOMRect } => candidate !== false);

    const anchorCandidate =
      anchorCandidates.find(
        (candidate) =>
          candidate.rect.top >= scrollRect.top && candidate.rect.top < scrollRect.bottom
      ) ??
      anchorCandidates.find(
        (candidate) =>
          candidate.rect.bottom > scrollRect.top && candidate.rect.top < scrollRect.bottom
      );

    if (!anchorCandidate) return undefined;
    return {
      eventId: anchorCandidate.eventId,
      offsetTop: getVirtualAnchorOffset(scrollRect.top, anchorCandidate.rect.top),
    };
  }, [getTimelineEventElement, timelineRows, virtualizer]);

  const getCurrentVirtualAnchorRef = useRef(getCurrentVirtualAnchor);
  getCurrentVirtualAnchorRef.current = getCurrentVirtualAnchor;
  const lastKnownVirtualAnchorRef = useRef<TimelineVirtualAnchor | undefined>(undefined);

  const captureVirtualAnchor = useCallback(() => {
    if (restoringSavedViewportRef.current) return;
    const anchor = getCurrentVirtualAnchor();
    if (anchor) lastKnownVirtualAnchorRef.current = anchor;
    pendingVirtualAnchorRef.current = anchor;
    pendingVirtualAnchorScrollTopRef.current = scrollRef.current?.scrollTop;
  }, [getCurrentVirtualAnchor]);

  const getPersistableVirtualAnchor = useCallback((): TimelineVirtualAnchor | undefined => {
    const anchor = getCurrentVirtualAnchorRef.current();
    if (anchor) {
      lastKnownVirtualAnchorRef.current = anchor;
      return anchor;
    }
    return lastKnownVirtualAnchorRef.current;
  }, []);

  useLayoutEffect(() => {
    if (eventId) return;
    const anchor = getCurrentVirtualAnchorRef.current();
    if (anchor) lastKnownVirtualAnchorRef.current = anchor;
  }, [eventId, timelineRows.length, virtualItems, virtualizer.range]);

  const latestLiveEventRow = useMemo(() => {
    if (!liveTimelineLinked || eventsLength === 0) return undefined;
    for (let index = timelineRows.length - 1; index >= 0; index -= 1) {
      const row = timelineRows[index];
      if (row.kind === 'event' && row.eventIndex === eventsLength - 1) return row;
    }
    return undefined;
  }, [eventsLength, liveTimelineLinked, timelineRows]);

  const isLatestLiveEventBottomVisible = useCallback((): boolean => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || !latestLiveEventRow) return false;
    const latestElement = getTimelineEventElement(latestLiveEventRow.eventId);
    if (!latestElement) return false;
    return isElementBottomInScrollView(scrollEl, latestElement);
  }, [getTimelineEventElement, latestLiveEventRow]);

  const isActuallyAtLiveBottom = useCallback((): boolean => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return false;
    if (isLatestLiveEventBottomVisible()) return true;
    if (!isScrollNearBottom(scrollEl)) return false;
    if (loadedAtEnd) {
      return isVirtualRangeAtEnd(virtualizer.range ?? undefined, timelineRows.length);
    }
    return isNearVirtualRangeEnd(virtualizer.range ?? undefined, timelineRows.length);
  }, [isLatestLiveEventBottomVisible, loadedAtEnd, timelineRows.length, virtualizer.range]);

  const isActuallyAtLiveBottomRef = useRef(isActuallyAtLiveBottom);
  isActuallyAtLiveBottomRef.current = isActuallyAtLiveBottom;

  const getPersistableAtLiveBottom = useCallback((): boolean => {
    if (isActuallyAtLiveBottomRef.current()) return true;
    return atLiveEndRef.current && (atBottomRef.current || liveEndPinRef.current);
  }, []);
  const persistLiveBottomViewport = useCallback(() => {
    setRoomTimelineViewport(room.roomId, {
      atBottom: true,
      liveTailEventId: getLoadedLiveTailEventId(room),
    });
  }, [room]);

  useEffect(
    () => () => {
      if (eventId) return;
      const restoreAnchor = restoringSavedViewportRef.current
        ? pendingVirtualAnchorRef.current ?? savedViewport?.anchor
        : undefined;
      const atLiveBottom = restoreAnchor ? false : getPersistableAtLiveBottom();
      const anchor = atLiveBottom ? undefined : restoreAnchor ?? getPersistableVirtualAnchor();
      setRoomTimelineViewport(room.roomId, {
        atBottom: atLiveBottom,
        anchor,
        liveTailEventId: atLiveBottom ? getLoadedLiveTailEventId(room) : undefined,
      });
    },
    [
      eventId,
      getPersistableAtLiveBottom,
      getPersistableVirtualAnchor,
      room,
      room.roomId,
      savedViewport,
    ]
  );

  const paginateVirtualTimeline = useCallback(
    (backwards: boolean) => {
      if (backwards) captureVirtualAnchor();
      handleTimelinePagination(backwards);
    },
    [captureVirtualAnchor, handleTimelinePagination]
  );

  useLayoutEffect(() => {
    const anchor = pendingVirtualAnchorRef.current;
    if (!anchor) return undefined;
    const rowIndex = eventIdToRowIndex.get(anchor.eventId);
    if (typeof rowIndex !== 'number') {
      return undefined;
    }

    const scrollEl = scrollRef.current;
    const capturedScrollTop = pendingVirtualAnchorScrollTopRef.current;
    if (
      !restoringSavedViewportRef.current &&
      scrollEl &&
      typeof capturedScrollTop === 'number' &&
      Math.abs(scrollEl.scrollTop - capturedScrollTop) > VIRTUAL_ANCHOR_RESTORE_SCROLL_TOLERANCE
    ) {
      pendingVirtualAnchorRef.current = undefined;
      pendingVirtualAnchorScrollTopRef.current = undefined;
      return undefined;
    }

    virtualizer.scrollToIndex(rowIndex, { align: 'start', behavior: 'auto' });
    const raf = window.requestAnimationFrame(() => {
      const restoreScrollEl = scrollRef.current;
      const anchorElement = getTimelineEventElement(anchor.eventId);
      if (!restoreScrollEl || !anchorElement) {
        return;
      }
      const nextScrollTop = getRestoredVirtualScrollTop(
        restoreScrollEl.scrollTop,
        anchor,
        restoreScrollEl.getBoundingClientRect().top,
        anchorElement.getBoundingClientRect().top
      );
      restoreScrollEl.scrollTo({ top: nextScrollTop, behavior: 'instant' });
      pendingVirtualAnchorRef.current = undefined;
      pendingVirtualAnchorScrollTopRef.current = undefined;
      restoringSavedViewportRef.current = false;
      initialScrollPlacedRef.current = true;
    });
    return () => window.cancelAnimationFrame(raf);
  }, [
    eventIdToRowIndex,
    getTimelineEventElement,
    timelineRows,
    virtualItems,
    virtualizer,
    virtualizer.range,
  ]);

  useEffect(() => {
    if (restoringSavedViewportRef.current) return;
    const range = virtualizer.range ?? undefined;
    const pagination = shouldPaginateVirtualRange(range, timelineRows, eventsLength);
    if (pagination.backward && canPaginateBack) {
      paginateVirtualTimeline(true);
      return;
    }
    if (pagination.forward && (!loadedAtEnd || canPaginateForward)) {
      paginateVirtualTimeline(false);
    }
  }, [
    virtualItems,
    virtualizer.range,
    timelineRows,
    eventsLength,
    canPaginateBack,
    canPaginateForward,
    loadedAtEnd,
    paginateVirtualTimeline,
  ]);

  const isRowInView = useCallback((element: HTMLElement): boolean => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return false;
    const scrollRect = scrollEl.getBoundingClientRect();
    const rowRect = element.getBoundingClientRect();
    return rowRect.top >= scrollRect.top && rowRect.bottom <= scrollRect.bottom;
  }, []);

  const scrollToElement = useCallback<ScrollToElement>(
    (element, opts) => {
      const scrollEl = scrollRef.current;
      if (!scrollEl) return false;
      const rowElement =
        (element.closest('[data-timeline-row-index]') as HTMLElement | null) ?? element;
      if (opts?.stopInView && isRowInView(rowElement)) return false;

      const scrollRect = scrollEl.getBoundingClientRect();
      const rowRect = rowElement.getBoundingClientRect();
      let scrollTo = scrollEl.scrollTop + rowRect.top - scrollRect.top;
      if (opts?.align === 'center') {
        scrollTo -= Math.round(scrollEl.clientHeight / 2) - Math.round(rowElement.clientHeight / 2);
      } else if (opts?.align === 'end') {
        scrollTo -= scrollEl.clientHeight - rowElement.clientHeight;
      }
      scrollEl.scrollTo({
        top: scrollTo - (opts?.offset ?? 0),
        behavior: toScrollBehavior(opts?.behavior),
      });
      return true;
    },
    [isRowInView]
  );

  const scrollToItem = useCallback<ScrollToItem>(
    (eventIndex, opts) => {
      const rowIndex = eventIndexToRowIndex.get(eventIndex);
      if (typeof rowIndex !== 'number') return false;
      const row = timelineRows[rowIndex];
      if (row.kind === 'event') {
        const element = getTimelineEventElement(row.eventId);
        if (element && opts?.stopInView && isRowInView(element)) return false;
      }

      virtualizer.scrollToIndex(rowIndex, {
        align: opts?.align ?? 'start',
        behavior: toScrollBehavior(opts?.behavior),
      });
      if (opts?.offset) {
        const { offset } = opts;
        window.requestAnimationFrame(() => {
          const scrollEl = scrollRef.current;
          if (!scrollEl) return;
          scrollEl.scrollBy({ top: -offset, behavior: 'instant' });
        });
      }
      return true;
    },
    [eventIndexToRowIndex, getTimelineEventElement, isRowInView, timelineRows, virtualizer]
  );

  const loadEventTimeline = useEventTimelineLoader(
    mx,
    room,
    useCallback(
      (evtId, lTimelines, evtAbsIndex) => {
        if (!alive()) return;
        const evLength = getTimelinesEventsCount(lTimelines);

        setFocusItem({
          index: evtAbsIndex,
          scrollTo: true,
          highlight: evtId !== readUptoEventIdRef.current,
        });
        setTimeline({
          linkedTimelines: lTimelines,
          range: {
            start: Math.max(evtAbsIndex - PAGINATION_LIMIT, 0),
            end: Math.min(evtAbsIndex + PAGINATION_LIMIT, evLength),
          },
        });
      },
      [alive]
    ),
    useCallback(() => {
      if (!alive()) return;
      setTimeline(getInitialTimeline(room, PAGINATION_LIMIT));
      scrollToBottomRef.current.count += 1;
      scrollToBottomRef.current.smooth = false;
    }, [alive, room])
  );

  useLiveEventArrive(
    room,
    useCallback(
      (mEvt: MatrixEvent) => {
        const shouldFollowLiveEnd =
          liveEndPinRef.current || isActuallyAtLiveBottom() || atBottomRef.current;
        const shouldReattachLiveTimeline =
          liveTimelineResetPendingRef.current ||
          (!liveTimelineLinked && shouldFollowLiveEnd) ||
          timelineRows.length === 0;

        if (shouldReattachLiveTimeline) {
          const nextTimeline = getInitialTimeline(room, PAGINATION_LIMIT);
          const shouldReplaceVisibleTimeline = shouldFollowLiveEnd || timelineRows.length === 0;
          if (shouldReplaceVisibleTimeline && timelineHasEvents(nextTimeline)) {
            liveTimelineResetPendingRef.current = false;
            setTimeline(nextTimeline);
          }

          if (shouldFollowLiveEnd) {
            startLiveEndPin();
            setAtBottomState(true);
            scrollToBottomRef.current.count += 1;
            scrollToBottomRef.current.smooth = true;
            if (document.hasFocus() && (!unreadInfo || mEvt.getSender() === mx.getUserId())) {
              requestAnimationFrame(() =>
                markAsRead(mx, mEvt.getRoomId()!, hideActivity, 'loaded-live-tail')
              );
            }
            return;
          }
        }

        // if user is at bottom of timeline
        // keep paginating timeline and conditionally mark as read
        // otherwise we update timeline without paginating
        // so timeline can be updated with evt like: edits, reactions etc
        if (shouldFollowLiveEnd) {
          if (document.hasFocus() && (!unreadInfo || mEvt.getSender() === mx.getUserId())) {
            // Check if the document is in focus (user is actively viewing the app),
            // and either there are no unread messages or the latest message is from the current user.
            // If either condition is met, trigger the markAsRead function to send a read receipt.
            requestAnimationFrame(() =>
              markAsRead(mx, mEvt.getRoomId()!, hideActivity, 'loaded-live-tail')
            );
          }

          if (!document.hasFocus() && !unreadInfo) {
            setUnreadInfo(getRoomUnreadInfo(room));
          }

          scrollToBottomRef.current.count += 1;
          scrollToBottomRef.current.smooth = true;

          setTimeline((ct) => ({ ...ct }));
          return;
        }
        captureVirtualAnchor();
        startTimelineTransition(() => {
          setTimeline((ct) => ({ ...ct }));
        });
        if (!unreadInfo) {
          setUnreadInfo(getRoomUnreadInfo(room));
        }
      },
      [
        captureVirtualAnchor,
        isActuallyAtLiveBottom,
        liveTimelineLinked,
        mx,
        room,
        unreadInfo,
        hideActivity,
        setAtBottomState,
        startLiveEndPin,
        startTimelineTransition,
        timelineRows.length,
      ]
    )
  );

  const handleOpenEvent = useCallback(
    async (
      evtId: string,
      highlight = true,
      onScroll: ((scrolled: boolean) => void) | undefined = undefined
    ) => {
      const evtTimeline = getEventTimeline(room, evtId);
      const absoluteIndex =
        evtTimeline && getEventIdAbsoluteIndex(timeline.linkedTimelines, evtTimeline, evtId);

      if (typeof absoluteIndex === 'number') {
        const scrolled = scrollToItem(absoluteIndex, {
          behavior: 'smooth',
          align: 'center',
          stopInView: true,
        });
        if (onScroll) onScroll(scrolled);
        setFocusItem({
          index: absoluteIndex,
          scrollTo: false,
          highlight,
        });
      } else {
        setTimeline(getEmptyTimeline());
        loadEventTimeline(evtId);
      }
    },
    [room, timeline, scrollToItem, loadEventTimeline]
  );

  useLiveTimelineRefresh(
    room,
    useCallback(() => {
      const nextTimeline = getInitialTimeline(room, PAGINATION_LIMIT);
      if (!timelineHasEvents(nextTimeline)) {
        liveTimelineResetPendingRef.current = true;
        perfLog('room-timeline.defer-empty-refresh', { roomId: room.roomId });
        return;
      }

      if (liveTimelineLinked || atBottomRef.current || liveEndPinRef.current) {
        if (!isActuallyAtLiveBottom() && !atBottomRef.current && !liveEndPinRef.current) {
          captureVirtualAnchor();
        }
        setTimeline(nextTimeline);
      }
    }, [captureVirtualAnchor, isActuallyAtLiveBottom, room, liveTimelineLinked])
  );

  useLiveTimelineReset(
    room,
    useCallback(() => {
      liveTimelineResetPendingRef.current = true;
      perfLog('room-timeline.live-reset', {
        roomId: room.roomId,
        atBottom: atBottomRef.current,
        liveEndPinned: liveEndPinRef.current,
      });
      if (atBottomRef.current || liveEndPinRef.current) {
        startLiveEndPin();
      } else {
        captureVirtualAnchor();
      }
    }, [captureVirtualAnchor, room.roomId, startLiveEndPin])
  );

  // Stay at bottom when room editor resize
  useResizeObserver(
    useMemo(() => {
      let mounted = false;
      return (entries) => {
        if (!mounted) {
          // skip initial mounting call
          mounted = true;
          return;
        }
        if (!roomInputRef.current) return;
        const editorBaseEntry = getResizeObserverEntry(roomInputRef.current, entries);
        const scrollElement = getScrollElement();
        if (!editorBaseEntry || !scrollElement) return;

        if (
          atBottomRef.current &&
          isScrollNearBottom(scrollElement, COMPOSER_RESIZE_BOTTOM_TOLERANCE)
        ) {
          scrollToBottom(scrollElement);
        }
      };
    }, [getScrollElement, roomInputRef]),
    useCallback(() => roomInputRef.current, [roomInputRef])
  );

  const tryAutoMarkAsRead = useCallback(() => {
    const readUptoEventId = readUptoEventIdRef.current;
    if (!readUptoEventId) {
      requestAnimationFrame(() => markAsRead(mx, room.roomId, hideActivity, 'loaded-live-tail'));
      return;
    }
    const evtTimeline = getEventTimeline(room, readUptoEventId);
    const latestTimeline = evtTimeline && getFirstLinkedTimeline(evtTimeline, Direction.Forward);
    if (latestTimeline === room.getLiveTimeline()) {
      requestAnimationFrame(() => markAsRead(mx, room.roomId, hideActivity, 'loaded-live-tail'));
    }
  }, [mx, room, hideActivity]);

  useLayoutEffect(() => {
    if (!liveEndPinRef.current || !loadedAtEnd || timelineRows.length === 0) {
      return undefined;
    }

    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;

    let animationFrame: number | undefined;
    const state = liveEndPinStateRef.current;
    if (state.startedAt === 0) {
      state.startedAt = performance.now();
      state.lastScrollHeight = scrollEl.scrollHeight;
      state.lastTotalSize = virtualizer.getTotalSize();
      state.stableFrames = 0;
    }

    const pinLiveEnd = () => {
      if (!liveEndPinRef.current) return;

      scrollToBottom(scrollEl, 'instant');

      const bottomGap = getScrollBottomGap(scrollEl);
      const totalSize = virtualizer.getTotalSize();
      const bottomRendered = isVirtualRangeAtEnd(
        virtualizer.range ?? undefined,
        timelineRows.length
      );
      const unchangedHeight =
        state.lastScrollHeight === scrollEl.scrollHeight && state.lastTotalSize === totalSize;
      const elapsed = performance.now() - state.startedAt;

      if (bottomRendered && bottomGap <= 2 && unchangedHeight) {
        state.stableFrames += 1;
      } else {
        state.stableFrames = 0;
        state.lastScrollHeight = scrollEl.scrollHeight;
        state.lastTotalSize = totalSize;
      }

      if (
        (elapsed >= LIVE_END_PIN_MIN_MS && state.stableFrames >= LIVE_END_PIN_STABLE_FRAMES) ||
        elapsed >= LIVE_END_PIN_MAX_MS
      ) {
        cancelLiveEndPin();
        setAtBottomState(true);
        return;
      }

      animationFrame = window.requestAnimationFrame(pinLiveEnd);
    };

    pinLiveEnd();

    return () => {
      if (animationFrame) window.cancelAnimationFrame(animationFrame);
    };
  }, [
    cancelLiveEndPin,
    loadedAtEnd,
    setAtBottomState,
    timelineRows.length,
    virtualItems,
    virtualizer,
    virtualizer.range,
  ]);

  useEffect(() => {
    if (liveEndPinRef.current) return;

    const virtualBottomVisible = isActuallyAtLiveBottom();

    if (virtualBottomVisible) {
      setAtBottomState(true);
      if (document.hasFocus()) {
        tryAutoMarkAsRead();
      }
      return;
    }

    if (virtualItems.length > 0) {
      setAtBottomState(false);
    }
  }, [
    setAtBottomState,
    isActuallyAtLiveBottom,
    timelineRows.length,
    tryAutoMarkAsRead,
    virtualItems,
  ]);

  const debounceSetAtBottom = useDebounce(
    useCallback(
      (entry: IntersectionObserverEntry) => {
        if (!entry.isIntersecting && !liveEndPinRef.current) setAtBottomState(false);
      },
      [setAtBottomState]
    ),
    { wait: 1000 }
  );
  useIntersectionObserver(
    useCallback(
      (entries) => {
        const target = atBottomAnchorRef.current;
        if (!target) return;
        const targetEntry = getIntersectionObserverEntry(target, entries);
        if (targetEntry) debounceSetAtBottom(targetEntry);
        if (targetEntry?.isIntersecting && atLiveEndRef.current) {
          setAtBottomState(true);
          if (document.hasFocus()) {
            tryAutoMarkAsRead();
          }
        }
      },
      [debounceSetAtBottom, setAtBottomState, tryAutoMarkAsRead]
    ),
    useCallback(
      () => ({
        root: getScrollElement(),
        rootMargin: '100px',
      }),
      [getScrollElement]
    ),
    useCallback(() => atBottomAnchorRef.current, [])
  );

  useDocumentFocusChange(
    useCallback(
      (inFocus) => {
        if (inFocus && atBottomRef.current) {
          tryAutoMarkAsRead();
        }
      },
      [tryAutoMarkAsRead]
    )
  );

  // Handle room-level shortcuts and up arrow edit
  useKeyDown(
    window,
    useCallback(
      (evt) => {
        if (isKeyHotkey('mod+shift+u', evt) && !editableActiveElement()) {
          evt.preventDefault();
          markAsUnread(mx, room.roomId);
          return;
        }

        if (
          isKeyHotkey('arrowup', evt) &&
          editableActiveElement() &&
          document.activeElement?.getAttribute('data-editable-name') === 'RoomInput' &&
          isEmptyEditor(editor)
        ) {
          const editableEvt = getLatestEditableEvt(room.getLiveTimeline(), (mEvt) =>
            canEditEvent(mx, mEvt)
          );
          const editableEvtId = editableEvt?.getId();
          if (!editableEvtId) return;
          setEditId(editableEvtId);
          evt.preventDefault();
        }
      },
      [mx, room, editor]
    )
  );

  useEffect(() => {
    if (eventId) {
      setTimeline(getEmptyTimeline());
      loadEventTimeline(eventId);
    }
  }, [eventId, loadEventTimeline]);

  // Scroll to bottom on initial timeline load
  useLayoutEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || eventId) return;
    if (shouldRestoreSavedViewport && savedViewport && !savedViewport.atBottom) {
      return;
    }
    if (!shouldRestoreSavedViewport || !savedViewport || savedViewport.atBottom) {
      if (initialScrollPlacedRef.current) return;
      initialScrollPlacedRef.current = true;
      scrollToBottom(scrollEl);
      return;
    }
  }, [eventId, savedViewport, shouldRestoreSavedViewport]);

  // if live timeline is linked and unreadInfo change
  // Scroll to last read message
  useLayoutEffect(() => {
    const { readUptoEventId, inLiveTimeline, scrollTo } = unreadInfo ?? {};
    if (readUptoEventId && inLiveTimeline && scrollTo) {
      const linkedTimelines = getLinkedTimelines(getLiveTimeline(room));
      const evtTimeline = getEventTimeline(room, readUptoEventId);
      const absoluteIndex =
        evtTimeline && getEventIdAbsoluteIndex(linkedTimelines, evtTimeline, readUptoEventId);
      if (typeof absoluteIndex === 'number') {
        scrollToItem(absoluteIndex, {
          behavior: 'instant',
          align: 'start',
          stopInView: true,
        });
      }
    }
  }, [room, unreadInfo, scrollToItem]);

  // scroll to focused message
  useLayoutEffect(() => {
    if (focusItem && focusItem.scrollTo) {
      scrollToItem(focusItem.index, {
        behavior: 'instant',
        align: 'center',
        stopInView: true,
      });
    }

    if (!focusItem) return undefined;

    const timeout = window.setTimeout(() => {
      if (!alive()) return;
      setFocusItem((currentItem) => {
        if (currentItem === focusItem) return undefined;
        return currentItem;
      });
    }, 2000);

    return () => window.clearTimeout(timeout);
  }, [alive, focusItem, scrollToItem]);

  // scroll to bottom of timeline
  const scrollToBottomCount = scrollToBottomRef.current.count;
  useLayoutEffect(() => {
    if (scrollToBottomCount > 0) {
      const scrollEl = scrollRef.current;
      if (scrollEl) {
        scrollToBottom(scrollEl, scrollToBottomRef.current.smooth ? 'smooth' : 'instant');
        setAtBottomState(true);
      }
    }
  }, [scrollToBottomCount, setAtBottomState]);

  // Remove unreadInfo on mark as read
  useEffect(() => {
    if (!unread) {
      setUnreadInfo(undefined);
    }
  }, [unread]);

  // scroll out of view msg editor in view.
  useEffect(() => {
    if (editId) {
      const editMsgElement =
        (scrollRef.current?.querySelector(`[data-message-id="${editId}"]`) as HTMLElement) ??
        undefined;
      if (editMsgElement) {
        scrollToElement(editMsgElement, {
          align: 'center',
          behavior: 'smooth',
          stopInView: true,
        });
      }
    }
  }, [scrollToElement, editId]);

  const handleJumpToLatest = async () => {
    if (eventId) {
      navigateRoom(room.roomId, undefined, { replace: true });
    }
    const requestId = latestTimelineRequestRef.current + 1;
    latestTimelineRequestRef.current = requestId;
    startLiveEndPin();
    pendingVirtualAnchorRef.current = undefined;
    pendingVirtualAnchorScrollTopRef.current = undefined;
    restoringSavedViewportRef.current = false;
    lastKnownVirtualAnchorRef.current = undefined;
    setAtBottomState(true);
    persistLiveBottomViewport();

    const scrollLatestToBottom = () => {
      scrollToBottomRef.current.count += 1;
      scrollToBottomRef.current.smooth = false;
      persistLiveBottomViewport();
    };

    try {
      const latestTimeline = await mx.getLatestTimeline(room.getUnfilteredTimelineSet());
      if (!alive() || latestTimelineRequestRef.current !== requestId) return;
      const nextTimeline = latestTimeline
        ? getTimelineEndWindow(getLinkedTimelines(latestTimeline), PAGINATION_LIMIT)
        : getInitialTimeline(room, PAGINATION_LIMIT);
      setTimeline(
        timelineHasEvents(nextTimeline) ? nextTimeline : getInitialTimeline(room, PAGINATION_LIMIT)
      );
      liveTimelineResetPendingRef.current = false;
      perfLog('room-timeline.jump-latest', {
        roomId: room.roomId,
        eventCount: getTimelinesEventsCount(nextTimeline.linkedTimelines),
        fromLiveTimeline: nextTimeline.linkedTimelines.includes(getLiveTimeline(room)),
      });
    } catch (err) {
      if (!alive() || latestTimelineRequestRef.current !== requestId) return;
      setTimeline(getInitialTimeline(room, PAGINATION_LIMIT));
      perfLog('room-timeline.jump-latest-failed', {
        roomId: room.roomId,
        error: err instanceof Error ? err.message : String(err),
      });
    } finally {
      if (alive() && latestTimelineRequestRef.current === requestId) {
        scrollLatestToBottom();
      }
    }
  };

  const handleJumpToUnread = () => {
    if (unreadInfo?.readUptoEventId) {
      setTimeline(getEmptyTimeline());
      loadEventTimeline(unreadInfo.readUptoEventId);
    }
  };

  const handleMarkAsRead = () => {
    markAsRead(mx, room.roomId, hideActivity);
  };

  const handleMarkEventAsUnread = useCallback(
    (targetEventId: string) => {
      markEventAsUnread(mx, room, targetEventId);
    },
    [mx, room]
  );

  const handleSaveLater = useCallback(
    (targetEventId: string) => {
      const targetEvent = room.findEventById(targetEventId);
      const item = targetEvent && createLaterItem(room, targetEvent, 'saved');
      if (item) setLaterItem(mx, item);
    },
    [mx, room]
  );

  const handleAddToNotes = useCallback(
    (targetEventId: string) => {
      const targetEvent = room.findEventById(targetEventId);
      const item = targetEvent && createMessageRoomNoteItem(room, targetEvent);
      if (item) addRoomNoteItemAccountData(mx, item);
    },
    [mx, room]
  );

  const handleRemindLater = useCallback(
    (targetEventId: string, dueTs: number) => {
      const targetEvent = room.findEventById(targetEventId);
      const item = targetEvent && createLaterItem(room, targetEvent, 'reminder', dueTs);
      if (item) setLaterItem(mx, item);
    },
    [mx, room]
  );

  const handleOpenReply: MouseEventHandler = useCallback(
    async (evt) => {
      const targetId = evt.currentTarget.getAttribute('data-event-id');
      if (!targetId) return;
      handleOpenEvent(targetId);
    },
    [handleOpenEvent]
  );

  const handleUserClick: MouseEventHandler<HTMLButtonElement> = useCallback(
    (evt) => {
      evt.preventDefault();
      evt.stopPropagation();
      const userId = evt.currentTarget.getAttribute('data-user-id');
      if (!userId) {
        console.warn('Button should have "data-user-id" attribute!');
        return;
      }
      openUserRoomProfile(
        room.roomId,
        space?.roomId,
        userId,
        evt.currentTarget.getBoundingClientRect()
      );
    },
    [room, space, openUserRoomProfile]
  );
  const handleUsernameClick: MouseEventHandler<HTMLButtonElement> = useCallback(
    (evt) => {
      evt.preventDefault();
      const userId = evt.currentTarget.getAttribute('data-user-id');
      if (!userId) {
        console.warn('Button should have "data-user-id" attribute!');
        return;
      }
      const name = getMemberDisplayName(room, userId) ?? getMxIdLocalPart(userId) ?? userId;
      editor.insertNode(
        createMentionElement(
          userId,
          name.startsWith('@') ? name : `@${name}`,
          userId === mx.getUserId()
        )
      );
      ReactEditor.focus(editor);
      moveCursor(editor);
    },
    [mx, room, editor]
  );

  const handleReplyClick: MouseEventHandler<HTMLButtonElement> = useCallback(
    (evt, startThread = false) => {
      const replyId = evt.currentTarget.getAttribute('data-event-id');
      if (!replyId) {
        console.warn('Button should have "data-event-id" attribute!');
        return;
      }
      const replyEvt = room.findEventById(replyId);
      if (!replyEvt) return;
      const editedReply = getEditedEvent(replyId, replyEvt, room.getUnfilteredTimelineSet());
      const content: IContent = editedReply?.getContent()['m.new_content'] ?? replyEvt.getContent();
      const { body, formatted_body: formattedBody } = content;
      const { 'm.relates_to': relation } = startThread
        ? { 'm.relates_to': { rel_type: 'm.thread', event_id: replyId } }
        : replyEvt.getWireContent();
      const senderId = replyEvt.getSender();
      if (senderId && typeof body === 'string') {
        setReplyDraft({
          userId: senderId,
          eventId: replyId,
          body,
          formattedBody,
          relation,
        });
        setTimeout(() => ReactEditor.focus(editor), 100);
      }
    },
    [room, setReplyDraft, editor]
  );

  const handleReactionToggle = useCallback(
    (targetEventId: string, key: string, shortcode?: string) => {
      const relations = getEventReactions(room.getUnfilteredTimelineSet(), targetEventId);
      const allReactions = relations?.getSortedAnnotationsByKey() ?? [];
      const [, reactionsSet] = allReactions.find(([k]) => k === key) ?? [];
      const reactions = reactionsSet ? Array.from(reactionsSet) : [];
      const myReaction = reactions.find(factoryEventSentBy(mx.getUserId()!));

      if (myReaction && !!myReaction?.isRelation()) {
        mx.redactEvent(room.roomId, myReaction.getId()!);
        return;
      }
      const rShortcode =
        shortcode ||
        (reactions.find(eventWithShortcode)?.getContent().shortcode as string | undefined);
      mx.sendEvent(
        room.roomId,
        MessageEvent.Reaction as any,
        getReactionContent(targetEventId, key, rShortcode)
      );
    },
    [mx, room]
  );
  const handleEdit = useCallback(
    (editEvtId?: string) => {
      if (editEvtId) {
        setEditId(editEvtId);
        return;
      }
      setEditId(undefined);
      ReactEditor.focus(editor);
    },
    [editor]
  );
  const { t } = useTranslation();
  const [forwardSelection, setForwardSelection] = useState<{ eventId: string; item: number }[]>([]);
  const forwardSelectionEventIds = useMemo(
    () => new Set(forwardSelection.map((selection) => selection.eventId)),
    [forwardSelection]
  );
  const selectedForwardEvents = useMemo(
    () =>
      forwardSelection
        .map((selection) => room.findEventById(selection.eventId))
        .filter((event): event is MatrixEvent => !!event),
    [room, forwardSelection]
  );

  useEffect(() => {
    setForwardSelection([]);
  }, [room.roomId]);

  const handleToggleForwardSelection = useCallback((targetEventId: string, item: number) => {
    setForwardSelection((currentSelection) => {
      const selectedIndex = currentSelection.findIndex(
        (selection) => selection.eventId === targetEventId
      );
      const sortedSelection = [...currentSelection].sort((a, b) => a.item - b.item);

      if (selectedIndex >= 0) {
        const firstItem = sortedSelection[0]?.item;
        const lastItem = sortedSelection[sortedSelection.length - 1]?.item;
        if (item !== firstItem && item !== lastItem) return [];
        return currentSelection.filter((selection) => selection.eventId !== targetEventId);
      }

      if (sortedSelection.length === 0) return [{ eventId: targetEventId, item }];

      const firstItem = sortedSelection[0].item;
      const lastItem = sortedSelection[sortedSelection.length - 1].item;
      if (item === firstItem - 1 || item === lastItem + 1) {
        return [...currentSelection, { eventId: targetEventId, item }].sort(
          (a, b) => a.item - b.item
        );
      }

      return [{ eventId: targetEventId, item }];
    });
  }, []);

  const renderMatrixEvent = useMatrixEventRenderer<
    [string, MatrixEvent, number, EventTimelineSet, boolean]
  >(
    {
      [MessageEvent.RoomMessage]: (mEventId, mEvent, item, timelineSet, collapse) => {
        const reactionRelations = getEventReactions(timelineSet, mEventId);
        const reactions = reactionRelations && reactionRelations.getSortedAnnotationsByKey();
        const hasReactions = reactions && reactions.length > 0;
        const { replyEventId, threadRootId } = mEvent;
        const highlighted = focusItem?.index === item && focusItem.highlight;

        const editedEvent = getEditedEvent(mEventId, mEvent, timelineSet);
        const getContent = (() =>
          editedEvent?.getContent()['m.new_content'] ?? mEvent.getContent()) as GetContentCallback;

        const senderId = mEvent.getSender() ?? '';
        const senderDisplayName =
          getMemberDisplayName(room, senderId) ?? getMxIdLocalPart(senderId) ?? senderId;

        return (
          <Message
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            timelineItem={item}
            messageSpacing={messageSpacing}
            messageLayout={messageLayout}
            collapse={collapse}
            highlight={highlighted}
            edit={editId === mEventId}
            canDelete={canRedact || (canDeleteOwn && mEvent.getSender() === mx.getUserId())}
            canSendReaction={canSendReaction}
            canRedactReactions={canRedact}
            canPinEvent={canPinEvent}
            imagePackRooms={imagePackRooms}
            relations={hasReactions ? reactionRelations : undefined}
            onUserClick={handleUserClick}
            onUsernameClick={handleUsernameClick}
            onReplyClick={handleReplyClick}
            onReactionToggle={handleReactionToggle}
            onEditId={handleEdit}
            onMarkUnread={handleMarkEventAsUnread}
            onSaveLater={handleSaveLater}
            onAddToNotes={handleAddToNotes}
            onRemind={handleRemindLater}
            onToggleForwardSelection={handleToggleForwardSelection}
            forwardSelectionMode={forwardSelection.length > 0}
            selectedForForward={forwardSelectionEventIds.has(mEventId)}
            reply={
              replyEventId && (
                <Reply
                  room={room}
                  timelineSet={timelineSet}
                  replyEventId={replyEventId}
                  threadRootId={threadRootId}
                  onClick={handleOpenReply}
                  getMemberPowerTag={getMemberPowerTag}
                  accessibleTagColors={accessiblePowerTagColors}
                  legacyUsernameColor={legacyUsernameColor || direct}
                />
              )
            }
            reactions={
              reactionRelations && (
                <Reactions
                  style={{ marginTop: config.space.S200 }}
                  room={room}
                  relations={reactionRelations}
                  mEventId={mEventId}
                  canSendReaction={canSendReaction}
                  canRedact={canRedact}
                  onReactionToggle={handleReactionToggle}
                />
              )
            }
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
            memberPowerTag={getMemberPowerTag(senderId)}
            accessibleTagColors={accessiblePowerTagColors}
            legacyUsernameColor={legacyUsernameColor || direct}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          >
            {mEvent.isRedacted() ? (
              <RedactedContent reason={mEvent.getUnsigned().redacted_because?.content.reason} />
            ) : (
              <RenderMessageContent
                displayName={senderDisplayName}
                msgType={mEvent.getContent().msgtype ?? ''}
                ts={mEvent.getTs()}
                edited={!!editedEvent}
                getContent={getContent}
                mediaAutoLoad={mediaAutoLoad}
                htmlReactParserOptions={htmlReactParserOptions}
                linkifyOpts={linkifyOpts}
                outlineAttachment={messageLayout === MessageLayout.Bubble}
                agentApprovalTarget={{
                  roomId: room.roomId,
                  eventId: mEventId,
                  canSendReaction,
                }}
              />
            )}
          </Message>
        );
      },
      [MessageEvent.RoomMessageEncrypted]: (mEventId, mEvent, item, timelineSet, collapse) => {
        const reactionRelations = getEventReactions(timelineSet, mEventId);
        const reactions = reactionRelations && reactionRelations.getSortedAnnotationsByKey();
        const hasReactions = reactions && reactions.length > 0;
        const { replyEventId, threadRootId } = mEvent;
        const highlighted = focusItem?.index === item && focusItem.highlight;

        return (
          <Message
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            timelineItem={item}
            messageSpacing={messageSpacing}
            messageLayout={messageLayout}
            collapse={collapse}
            highlight={highlighted}
            edit={editId === mEventId}
            canDelete={canRedact || (canDeleteOwn && mEvent.getSender() === mx.getUserId())}
            canSendReaction={canSendReaction}
            canRedactReactions={canRedact}
            canPinEvent={canPinEvent}
            imagePackRooms={imagePackRooms}
            relations={hasReactions ? reactionRelations : undefined}
            onUserClick={handleUserClick}
            onUsernameClick={handleUsernameClick}
            onReplyClick={handleReplyClick}
            onReactionToggle={handleReactionToggle}
            onEditId={handleEdit}
            onMarkUnread={handleMarkEventAsUnread}
            onSaveLater={handleSaveLater}
            onAddToNotes={handleAddToNotes}
            onRemind={handleRemindLater}
            onToggleForwardSelection={handleToggleForwardSelection}
            forwardSelectionMode={forwardSelection.length > 0}
            selectedForForward={forwardSelectionEventIds.has(mEventId)}
            reply={
              replyEventId && (
                <Reply
                  room={room}
                  timelineSet={timelineSet}
                  replyEventId={replyEventId}
                  threadRootId={threadRootId}
                  onClick={handleOpenReply}
                  getMemberPowerTag={getMemberPowerTag}
                  accessibleTagColors={accessiblePowerTagColors}
                  legacyUsernameColor={legacyUsernameColor || direct}
                />
              )
            }
            reactions={
              reactionRelations && (
                <Reactions
                  style={{ marginTop: config.space.S200 }}
                  room={room}
                  relations={reactionRelations}
                  mEventId={mEventId}
                  canSendReaction={canSendReaction}
                  canRedact={canRedact}
                  onReactionToggle={handleReactionToggle}
                />
              )
            }
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
            memberPowerTag={getMemberPowerTag(mEvent.getSender() ?? '')}
            accessibleTagColors={accessiblePowerTagColors}
            legacyUsernameColor={legacyUsernameColor || direct}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          >
            <EncryptedContent mEvent={mEvent}>
              {() => {
                if (mEvent.isRedacted()) return <RedactedContent />;
                if (mEvent.getType() === MessageEvent.Sticker)
                  return (
                    <MSticker
                      content={mEvent.getContent()}
                      renderImageContent={(props) => (
                        <ImageContent
                          {...props}
                          autoPlay={mediaAutoLoad}
                          renderImage={(p) => <Image {...p} loading="lazy" />}
                          renderViewer={(p) => <ImageViewer {...p} />}
                        />
                      )}
                    />
                  );
                if (mEvent.getType() === MessageEvent.RoomMessage) {
                  const editedEvent = getEditedEvent(mEventId, mEvent, timelineSet);
                  const getContent = (() =>
                    editedEvent?.getContent()['m.new_content'] ??
                    mEvent.getContent()) as GetContentCallback;

                  const senderId = mEvent.getSender() ?? '';
                  const senderDisplayName =
                    getMemberDisplayName(room, senderId) ?? getMxIdLocalPart(senderId) ?? senderId;
                  return (
                    <RenderMessageContent
                      displayName={senderDisplayName}
                      msgType={mEvent.getContent().msgtype ?? ''}
                      ts={mEvent.getTs()}
                      edited={!!editedEvent}
                      getContent={getContent}
                      mediaAutoLoad={mediaAutoLoad}
                      htmlReactParserOptions={htmlReactParserOptions}
                      linkifyOpts={linkifyOpts}
                      outlineAttachment={messageLayout === MessageLayout.Bubble}
                      agentApprovalTarget={{
                        roomId: room.roomId,
                        eventId: mEventId,
                        canSendReaction,
                      }}
                    />
                  );
                }
                if (mEvent.getType() === MessageEvent.RoomMessageEncrypted)
                  return (
                    <Text>
                      <MessageNotDecryptedContent />
                    </Text>
                  );
                return (
                  <Text>
                    <MessageUnsupportedContent />
                  </Text>
                );
              }}
            </EncryptedContent>
          </Message>
        );
      },
      [MessageEvent.Sticker]: (mEventId, mEvent, item, timelineSet, collapse) => {
        const reactionRelations = getEventReactions(timelineSet, mEventId);
        const reactions = reactionRelations && reactionRelations.getSortedAnnotationsByKey();
        const hasReactions = reactions && reactions.length > 0;
        const highlighted = focusItem?.index === item && focusItem.highlight;

        return (
          <Message
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            timelineItem={item}
            messageSpacing={messageSpacing}
            messageLayout={messageLayout}
            collapse={collapse}
            highlight={highlighted}
            canDelete={canRedact || (canDeleteOwn && mEvent.getSender() === mx.getUserId())}
            canSendReaction={canSendReaction}
            canRedactReactions={canRedact}
            canPinEvent={canPinEvent}
            imagePackRooms={imagePackRooms}
            relations={hasReactions ? reactionRelations : undefined}
            onUserClick={handleUserClick}
            onUsernameClick={handleUsernameClick}
            onReplyClick={handleReplyClick}
            onReactionToggle={handleReactionToggle}
            onMarkUnread={handleMarkEventAsUnread}
            onSaveLater={handleSaveLater}
            onAddToNotes={handleAddToNotes}
            onRemind={handleRemindLater}
            onToggleForwardSelection={handleToggleForwardSelection}
            forwardSelectionMode={forwardSelection.length > 0}
            selectedForForward={forwardSelectionEventIds.has(mEventId)}
            reactions={
              reactionRelations && (
                <Reactions
                  style={{ marginTop: config.space.S200 }}
                  room={room}
                  relations={reactionRelations}
                  mEventId={mEventId}
                  canSendReaction={canSendReaction}
                  canRedact={canRedact}
                  onReactionToggle={handleReactionToggle}
                />
              )
            }
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
            memberPowerTag={getMemberPowerTag(mEvent.getSender() ?? '')}
            accessibleTagColors={accessiblePowerTagColors}
            legacyUsernameColor={legacyUsernameColor || direct}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          >
            {mEvent.isRedacted() ? (
              <RedactedContent reason={mEvent.getUnsigned().redacted_because?.content.reason} />
            ) : (
              <MSticker
                content={mEvent.getContent()}
                renderImageContent={(props) => (
                  <ImageContent
                    {...props}
                    autoPlay={mediaAutoLoad}
                    renderImage={(p) => <Image {...p} loading="lazy" />}
                    renderViewer={(p) => <ImageViewer {...p} />}
                  />
                )}
              />
            )}
          </Message>
        );
      },
      [EventType.PollStart]: (mEventId, mEvent, item) => {
        const highlighted = focusItem?.index === item && focusItem.highlight;
        const poll = parsePollStartContent(mEvent.getContent<Record<string, unknown>>());
        if (!poll) return null;

        const senderId = mEvent.getSender() ?? '';
        const senderDisplayName =
          getMemberDisplayName(room, senderId) ?? getMxIdLocalPart(senderId) ?? senderId;

        return (
          <Message
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            timelineItem={item}
            messageSpacing={messageSpacing}
            messageLayout={messageLayout}
            collapse={false}
            highlight={highlighted}
            canDelete={canRedact || (canDeleteOwn && mEvent.getSender() === mx.getUserId())}
            canSendReaction={canSendReaction}
            canRedactReactions={canRedact}
            canPinEvent={canPinEvent}
            imagePackRooms={imagePackRooms}
            onUserClick={handleUserClick}
            onUsernameClick={handleUsernameClick}
            onReplyClick={handleReplyClick}
            onReactionToggle={handleReactionToggle}
            onMarkUnread={handleMarkEventAsUnread}
            onSaveLater={handleSaveLater}
            onAddToNotes={handleAddToNotes}
            onRemind={handleRemindLater}
            onToggleForwardSelection={handleToggleForwardSelection}
            forwardSelectionMode={forwardSelection.length > 0}
            selectedForForward={forwardSelectionEventIds.has(mEventId)}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
            memberPowerTag={getMemberPowerTag(senderId)}
            accessibleTagColors={accessiblePowerTagColors}
            legacyUsernameColor={legacyUsernameColor || direct}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          >
            <Box direction="Column" gap="200">
              <Text size="T200" priority="300">
                {senderDisplayName}
              </Text>
              <ErrorBoundary fallback={<MessageUnsupportedContent />}>
                <Suspense fallback={null}>
                  <PollContent roomId={room.roomId} eventId={mEventId} poll={poll} />
                </Suspense>
              </ErrorBoundary>
            </Box>
          </Message>
        );
      },
      [StateEvent.RoomMember]: (mEventId, mEvent, item) => {
        const membershipChanged = isMembershipChanged(mEvent);
        if (membershipChanged && hideMembershipEvents) return null;
        if (!membershipChanged && hideNickAvatarEvents) return null;

        const highlighted = focusItem?.index === item && focusItem.highlight;
        const parsed = parseMemberEvent(mEvent);

        const timeJSX = (
          <Time
            ts={mEvent.getTs()}
            compact={messageLayout === MessageLayout.Compact}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        );

        return (
          <Event
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            highlight={highlighted}
            messageSpacing={messageSpacing}
            canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
          >
            <EventContent
              messageLayout={messageLayout}
              time={timeJSX}
              iconSrc={parsed.icon}
              content={
                <Box grow="Yes" direction="Column">
                  <Text size="T300" priority="300">
                    {parsed.body}
                  </Text>
                </Box>
              }
            />
          </Event>
        );
      },
      [StateEvent.RoomName]: (mEventId, mEvent, item) => {
        const highlighted = focusItem?.index === item && focusItem.highlight;
        const senderId = mEvent.getSender() ?? '';
        const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

        const timeJSX = (
          <Time
            ts={mEvent.getTs()}
            compact={messageLayout === MessageLayout.Compact}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        );

        return (
          <Event
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            highlight={highlighted}
            messageSpacing={messageSpacing}
            canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
          >
            <EventContent
              messageLayout={messageLayout}
              time={timeJSX}
              iconSrc={Icons.Hash}
              content={
                <Box grow="Yes" direction="Column">
                  <Text size="T300" priority="300">
                    <b>{senderName}</b>
                    {t('Organisms.RoomCommon.changed_room_name')}
                  </Text>
                </Box>
              }
            />
          </Event>
        );
      },
      [StateEvent.RoomTopic]: (mEventId, mEvent, item) => {
        const highlighted = focusItem?.index === item && focusItem.highlight;
        const senderId = mEvent.getSender() ?? '';
        const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

        const timeJSX = (
          <Time
            ts={mEvent.getTs()}
            compact={messageLayout === MessageLayout.Compact}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        );

        return (
          <Event
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            highlight={highlighted}
            messageSpacing={messageSpacing}
            canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
          >
            <EventContent
              messageLayout={messageLayout}
              time={timeJSX}
              iconSrc={Icons.Hash}
              content={
                <Box grow="Yes" direction="Column">
                  <Text size="T300" priority="300">
                    <b>{senderName}</b>
                    {' changed room topic'}
                  </Text>
                </Box>
              }
            />
          </Event>
        );
      },
      [StateEvent.RoomAvatar]: (mEventId, mEvent, item) => {
        const highlighted = focusItem?.index === item && focusItem.highlight;
        const senderId = mEvent.getSender() ?? '';
        const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

        const timeJSX = (
          <Time
            ts={mEvent.getTs()}
            compact={messageLayout === MessageLayout.Compact}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        );

        return (
          <Event
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            highlight={highlighted}
            messageSpacing={messageSpacing}
            canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
          >
            <EventContent
              messageLayout={messageLayout}
              time={timeJSX}
              iconSrc={Icons.Hash}
              content={
                <Box grow="Yes" direction="Column">
                  <Text size="T300" priority="300">
                    <b>{senderName}</b>
                    {' changed room avatar'}
                  </Text>
                </Box>
              }
            />
          </Event>
        );
      },
      [StateEvent.GroupCallMemberPrefix]: (mEventId, mEvent, item) => {
        const highlighted = focusItem?.index === item && focusItem.highlight;
        const senderId = mEvent.getSender() ?? '';
        const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

        const content = mEvent.getContent<SessionMembershipData>();
        const prevContent = mEvent.getPrevContent();

        const callJoined = content.application;
        if (callJoined && 'application' in prevContent) {
          return null;
        }

        const timeJSX = (
          <Time
            ts={mEvent.getTs()}
            compact={messageLayout === MessageLayout.Compact}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        );

        return (
          <Event
            key={mEvent.getId()}
            data-message-item={item}
            data-message-id={mEventId}
            room={room}
            mEvent={mEvent}
            highlight={highlighted}
            messageSpacing={messageSpacing}
            canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
            hideReadReceipts={hideActivity}
            showDeveloperTools={showDeveloperTools}
          >
            <EventContent
              messageLayout={messageLayout}
              time={timeJSX}
              iconSrc={callJoined ? Icons.Phone : Icons.PhoneDown}
              content={
                <Box grow="Yes" direction="Column">
                  <Text size="T300" priority="300">
                    <b>{senderName}</b>
                    {callJoined ? ' joined the call' : ' ended the call'}
                  </Text>
                </Box>
              }
            />
          </Event>
        );
      },
    },
    (mEventId, mEvent, item) => {
      if (!showHiddenEvents) return null;
      const highlighted = focusItem?.index === item && focusItem.highlight;
      const senderId = mEvent.getSender() ?? '';
      const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

      const timeJSX = (
        <Time
          ts={mEvent.getTs()}
          compact={messageLayout === MessageLayout.Compact}
          hour24Clock={hour24Clock}
          dateFormatString={dateFormatString}
        />
      );

      return (
        <Event
          key={mEvent.getId()}
          data-message-item={item}
          data-message-id={mEventId}
          room={room}
          mEvent={mEvent}
          highlight={highlighted}
          messageSpacing={messageSpacing}
          canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
          hideReadReceipts={hideActivity}
          showDeveloperTools={showDeveloperTools}
        >
          <EventContent
            messageLayout={messageLayout}
            time={timeJSX}
            iconSrc={Icons.Code}
            content={
              <Box grow="Yes" direction="Column">
                <Text size="T300" priority="300">
                  <b>{senderName}</b>
                  {' sent '}
                  <code className={customHtmlCss.Code}>{mEvent.getType()}</code>
                  {' state event'}
                </Text>
              </Box>
            }
          />
        </Event>
      );
    },
    (mEventId, mEvent, item) => {
      if (!showHiddenEvents) return null;
      if (Object.keys(mEvent.getContent()).length === 0) return null;
      if (mEvent.getRelation()) return null;
      if (mEvent.isRedaction()) return null;

      const highlighted = focusItem?.index === item && focusItem.highlight;
      const senderId = mEvent.getSender() ?? '';
      const senderName = getMemberDisplayName(room, senderId) || getMxIdLocalPart(senderId);

      const timeJSX = (
        <Time
          ts={mEvent.getTs()}
          compact={messageLayout === MessageLayout.Compact}
          hour24Clock={hour24Clock}
          dateFormatString={dateFormatString}
        />
      );

      return (
        <Event
          key={mEvent.getId()}
          data-message-item={item}
          data-message-id={mEventId}
          room={room}
          mEvent={mEvent}
          highlight={highlighted}
          messageSpacing={messageSpacing}
          canDelete={canRedact || mEvent.getSender() === mx.getUserId()}
          hideReadReceipts={hideActivity}
          showDeveloperTools={showDeveloperTools}
        >
          <EventContent
            messageLayout={messageLayout}
            time={timeJSX}
            iconSrc={Icons.Code}
            content={
              <Box grow="Yes" direction="Column">
                <Text size="T300" priority="300">
                  <b>{senderName}</b>
                  {' sent '}
                  <code className={customHtmlCss.Code}>{mEvent.getType()}</code>
                  {' event'}
                </Text>
              </Box>
            }
          />
        </Event>
      );
    }
  );

  const renderDividerRow = (row: TimelineDividerRow) => {
    if (row.divider === 'day') {
      const ts = row.ts ?? Date.now();
      return (
        <MessageBase space={messageSpacing}>
          <TimelineDivider variant="Surface">
            <Badge as="span" size="500" variant="Secondary" fill="None" radii="300">
              <Text size="L400">
                {(() => {
                  if (today(ts)) return 'Today';
                  if (yesterday(ts)) return 'Yesterday';
                  return timeDayMonthYear(ts);
                })()}
              </Text>
            </Badge>
          </TimelineDivider>
        </MessageBase>
      );
    }

    const clientUnread = row.divider === 'client-unread';
    return (
      <MessageBase space={messageSpacing}>
        <TimelineDivider
          style={{ color: clientUnread ? color.Warning.Main : color.Success.Main }}
          variant="Inherit"
        >
          <Badge
            as="span"
            size="500"
            variant={clientUnread ? 'Warning' : 'Success'}
            fill="Solid"
            radii="300"
          >
            {clientUnread && <Icon size="50" src={Icons.Bookmark} />}
            <Text size="L400">{clientUnread ? 'Marked Unread Here' : 'New Messages'}</Text>
          </Badge>
        </TimelineDivider>
      </MessageBase>
    );
  };

  const renderLoaderRow = (row: TimelineLoaderRow) => (
    <MessageBase>
      {messageLayout === MessageLayout.Compact ? (
        <CompactPlaceholder key={row.key} />
      ) : (
        <DefaultPlaceholder key={row.key} />
      )}
    </MessageBase>
  );

  const renderEventRow = (row: TimelineEventRow) => {
    const perfStart = isPerformanceDebugEnabled() ? performance.now() : 0;
    const { mEvent, eventId: mEventId, eventIndex: item, timelineSet, collapse } = row;

    const eventJSX = renderMatrixEvent(
      mEvent.getType(),
      typeof mEvent.getStateKey() === 'string',
      mEventId,
      mEvent,
      item,
      timelineSet,
      collapse
    );

    if (perfStart) {
      const duration = performance.now() - perfStart;
      if (duration > 8) {
        perfLog('room-timeline.slow-event-render', {
          roomId: room.roomId,
          eventId: mEventId,
          eventType: mEvent.getType(),
          msgtype: mEvent.getContent().msgtype,
          item,
          durationMs: Math.round(duration * 100) / 100,
        });
      }
    }

    return eventJSX;
  };

  const renderTimelineRow = (row: TimelineRow) => {
    if (row.kind === 'intro') {
      return (
        <div
          style={{
            padding: `${config.space.S700} ${config.space.S400} ${config.space.S600} ${
              messageLayout === MessageLayout.Compact ? config.space.S400 : toRem(64)
            }`,
          }}
        >
          <RoomIntro room={room} />
        </div>
      );
    }

    if (row.kind === 'loader') {
      return renderLoaderRow(row);
    }

    if (row.kind === 'divider') {
      return renderDividerRow(row);
    }

    if (row.kind === 'bottom') {
      return <span ref={atBottomAnchorRef} style={{ display: 'block', height: 1 }} />;
    }

    return renderEventRow(row);
  };

  return (
    <Box grow="Yes" style={{ position: 'relative' }}>
      {selectedForwardEvents.length > 0 && (
        <TimelineFloat
          position="Top"
          role="region"
          aria-label={t('modernization.forward.selection_region_aria_label')}
        >
          <Chip
            variant="Primary"
            radii="Pill"
            outlined
            before={<Icon size="50" src={Icons.Check} />}
          >
            <Text size="L400">
              {t('modernization.forward.selected_count', { count: selectedForwardEvents.length })}
            </Text>
          </Chip>
          <MessageForwardItem
            room={room}
            mEvents={selectedForwardEvents}
            label={t('modernization.forward.forward_selected')}
            onClose={() => setForwardSelection([])}
          />
          <Chip
            variant="SurfaceVariant"
            radii="Pill"
            outlined
            before={<Icon size="50" src={Icons.Cross} />}
            onClick={() => setForwardSelection([])}
          >
            <Text size="L400">{t('modernization.forward.clear_selection')}</Text>
          </Chip>
        </TimelineFloat>
      )}
      {paginationErrors.backward && (
        <TimelineFloat position="Top">
          <Chip
            variant="Critical"
            radii="Pill"
            outlined
            onClick={() => {
              setPaginationErrors((current) => clearTimelinePaginationError(current, 'backward'));
              void handleTimelinePagination(true);
            }}
          >
            <Text size="L400">{`Could not load older messages. ${paginationErrors.backward}`}</Text>
          </Chip>
        </TimelineFloat>
      )}
      {paginationErrors.forward && (
        <TimelineFloat position="Bottom">
          <Chip
            variant="Critical"
            radii="Pill"
            outlined
            onClick={() => {
              setPaginationErrors((current) => clearTimelinePaginationError(current, 'forward'));
              void handleTimelinePagination(false);
            }}
          >
            <Text size="L400">{`Could not load newer messages. ${paginationErrors.forward}`}</Text>
          </Chip>
        </TimelineFloat>
      )}
      {unreadInfo?.readUptoEventId && !unreadInfo?.inLiveTimeline && (
        <TimelineFloat position="Top">
          <Chip
            variant="Primary"
            radii="Pill"
            outlined
            before={<Icon size="50" src={Icons.MessageUnread} />}
            onClick={handleJumpToUnread}
          >
            <Text size="L400">Jump to Unread</Text>
          </Chip>

          <Chip
            variant="SurfaceVariant"
            radii="Pill"
            outlined
            before={<Icon size="50" src={Icons.CheckTwice} />}
            onClick={handleMarkAsRead}
          >
            <Text size="L400">Mark as Read</Text>
          </Chip>
        </TimelineFloat>
      )}
      <Scroll ref={scrollRef} visibility="Hover">
        <Box
          direction="Column"
          justifyContent="End"
          style={{
            minHeight: '100%',
            padding: `${config.space.S600} 0`,
            overflowAnchor: 'none',
          }}
        >
          <div
            style={{
              height: virtualizer.getTotalSize(),
              minHeight: timelineRows.length === 0 ? '100%' : undefined,
              position: 'relative',
              width: '100%',
              flexShrink: 0,
            }}
          >
            {virtualItems.map((virtualItem) => {
              const row = timelineRows[virtualItem.index];
              if (!row) return null;
              return (
                <div
                  key={virtualItem.key}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  data-timeline-row-index={virtualItem.index}
                  data-timeline-row-kind={row.kind}
                  data-timeline-event-id={row.kind === 'event' ? row.eventId : undefined}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    transform: `translateY(${virtualItem.start}px)`,
                    contain: 'layout style',
                    overflowAnchor: 'none',
                  }}
                >
                  {renderTimelineRow(row)}
                </div>
              );
            })}
          </div>
        </Box>
      </Scroll>
      {!atBottom && (
        <TimelineFloat position="Bottom">
          <Chip
            variant="SurfaceVariant"
            radii="Pill"
            outlined
            before={<Icon size="50" src={Icons.ArrowBottom} />}
            onClick={handleJumpToLatest}
          >
            <Text size="L400">Jump to Latest</Text>
          </Chip>
        </TimelineFloat>
      )}
    </Box>
  );
}
