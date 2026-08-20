import { IconName, IconSrc } from 'folds';

import { AccountDataEvent, MarkedUnreadContent } from '../../types/matrix/accountData';
import type { EventId, RoomId, UserId } from '../features/matrix-dto/ids';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';
import type { RoomJoinRulePresentation } from '../features/matrix-dto/roomJoinRule';
import {
  IRoomCreateContent,
  MessageEvent,
  NativeEventContentEvent,
  NotificationType,
  RoomToParents,
  RoomType,
  StateEvent,
  UnreadInfo,
} from '../../types/matrix/room';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import {
  getNativeRoomStateProjection,
  getNativeSpecialUsers,
} from '../features/matrix-dto/nativeRoomStateProjection';

/**
 * SDK-neutral structural projections used by this utility boundary.
 *
 * These are narrow, read-only interfaces satisfied by live SDK runtime objects
 * and by the test doubles / native DTO shapes. They deliberately do not
 * re-export any SDK type so this file stays free of the SDK import, while
 * callers that still hold live SDK objects keep typechecking.
 */

type EventContentReading = { [key: string]: any };

type RelationReading = {
  rel_type?: string;
  event_id?: string;
  key?: string;
};

/** Narrow structural projection of a room event. */
export type MatrixEventReading = {
  getContent<T extends EventContentReading = EventContentReading>(): T;
  getPrevContent(): EventContentReading;
  getSender(): string | undefined;
  getType(): string;
  getStateKey(): string | undefined;
  getTs(): number;
  getId(): string | undefined;
  getRoomId(): string | undefined;
  isRedacted(): boolean;
  isSending(): boolean;
  getRelation(): RelationReading | null;
  getAssociatedId?(): string | undefined;
  status?: string | null;
  threadRootId?: string;
  isEncrypted?(): boolean;
  getEffectiveEvent?(): unknown;
  event: { sender?: string; [key: string]: unknown };
};

/** Narrow structural projection of a room member. */
export type MemberReading = {
  userId: UserId;
  rawDisplayName: string;
  /** js-sdk RoomMember.name (display name or userId); reading may omit it. */
  name?: string;
  getMxcAvatarUrl(): string | undefined;
  events: { member?: MatrixEventReading };
};

type RoomStateReading = {
  getStateEvents(eventType: string): MatrixEventReading[];
  getStateEvents(eventType: string, stateKey: string): MatrixEventReading | null;
};

/** Narrow structural projection of a room. */
export type RoomReading = {
  roomId: RoomId;
  name: string;
  currentState: RoomStateReading;
  getLiveTimeline(): {
    getState(direction: string): RoomStateReading | undefined;
    getEvents(): MatrixEventReading[];
  };
  getMember(userId: UserId): MemberReading | null;
  getMembers(): MemberReading[];
  getMxcAvatarUrl(): string | null;
  getAvatarFallbackMember(): MemberReading | undefined;
  getUnreadNotificationCount(type?: string): number;
  getEventReadUpTo(userId: UserId): string | null;
  getLastActiveTimestamp?(): number | undefined;
  getBumpStamp?(): number | undefined;
  lastMessagePreview?: string;
  getThreads?(): { events: MatrixEventReading[] }[];
  accountData: { get(eventType: string): MatrixEventReading | undefined };
  getMyMembership(): string;
  getJoinRule(): string;
  getJoinedMemberCount(): number;
  getCanonicalAlias(): string | null;
  getType(): string | undefined;
  getVersion(): string;
  isCallRoom(): boolean;
  isSpaceRoom(): boolean;
  isFavorite?: boolean;
  getTimelineForEvent?(
    eventId: string
  ): { getTimelineSet(): EventTimelineSetReading; getEvents(): MatrixEventReading[] } | null;
  hasMembershipState(userId: UserId, membership: string): boolean;
};

type PushRuleActionReading = string | { [key: string]: any };

type PushRuleReading = {
  actions: PushRuleActionReading[];
  conditions?: { kind?: string }[];
  rule_id: string;
};

type PowerLevelsReading = {
  users_default?: number;
  users?: Record<string, number>;
};

/** Narrow structural projection of a matrix client. */
export type MatrixClientReading = {
  getAccountData(eventType: string): MatrixEventReading | undefined;
  getRoomPushRule(scope: string, roomId: string): PushRuleReading | undefined;
  getUserId(): string | null;
  getRooms(): RoomReading[];
  getRoom(roomId: string): RoomReading | null;
  mxcUrlToHttp(
    mxcUrl: string,
    width?: number,
    height?: number,
    resizeMethod?: string,
    allowDirectLinks?: boolean,
    allowRedirects?: boolean,
    useAuthentication?: boolean
  ): string | null;
};

type RelationsReading = {
  getRelations(): MatrixEventReading[];
};

/** Narrow structural projection of an event timeline set's relations. */
export type EventTimelineSetReading = {
  relations: {
    getChildEventsForEvent(
      eventId: string,
      relationType: string,
      eventType: string
    ): RelationsReading | undefined;
  };
};

/**
 * Prefer the room's indexed current state when available, falling back to the
 * live timeline's forward state — mirrors timelineLifecycle.getRoomCurrentState
 * without the SDK type import.
 */
const getRoomCurrentState = (room: RoomReading): RoomStateReading | undefined =>
  room.currentState ?? room.getLiveTimeline().getState('f');

export const getStateEvent = (
  room: RoomReading,
  eventType: StateEvent,
  stateKey = ''
): MatrixEventReading | undefined =>
  getRoomCurrentState(room)?.getStateEvents(eventType, stateKey) ?? undefined;

export const getStateEvents = (room: RoomReading, eventType: StateEvent): MatrixEventReading[] =>
  getRoomCurrentState(room)?.getStateEvents(eventType) ?? [];

export const getAccountData = (
  mx: MatrixClientReading,
  eventType: AccountDataEvent
): MatrixEventReading | undefined => mx.getAccountData(eventType);

export const isSpace = (room: RoomReading | null): boolean => {
  if (!room) return false;
  // Native room-list summaries carry this classification without exposing a
  // fabricated m.room.create event. Live SDK rooms also provide this method.
  if (room.isSpaceRoom?.() === true) return true;
  const event = getStateEvent(room, StateEvent.RoomCreate);
  if (!event) return false;
  return event.getContent().type === RoomType.Space;
};

export const isRoom = (room: RoomReading | null): boolean => {
  if (!room) return false;
  if (room.isSpaceRoom?.() === true) return false;
  const event = getStateEvent(room, StateEvent.RoomCreate);
  if (!event) return true;
  return event.getContent().type !== RoomType.Space;
};

export const isUnsupportedRoom = (room: RoomReading | null): boolean => {
  if (!room) return false;
  const event = getStateEvent(room, StateEvent.RoomCreate);
  if (!event) return true; // Consider room unsupported if m.room.create event doesn't exist
  return event.getContent().type !== undefined && event.getContent().type !== RoomType.Space;
};

export const getAllParents = (roomToParents: RoomToParents, roomId: string): Set<string> => {
  const allParents = new Set<string>();

  const addAllParentIds = (rId: string) => {
    if (allParents.has(rId)) return;
    allParents.add(rId);

    const parents = roomToParents.get(rId);
    parents?.forEach((id) => addAllParentIds(id));
  };
  addAllParentIds(roomId);
  allParents.delete(roomId);
  return allParents;
};

export const mapParentWithChildren = (
  roomToParents: RoomToParents,
  roomId: string,
  children: string[]
) => {
  const allParents = getAllParents(roomToParents, roomId);
  children.forEach((childId) => {
    if (allParents.has(childId)) {
      // Space cycle detected.
      return;
    }
    const parents = roomToParents.get(childId) ?? new Set<string>();
    parents.add(roomId);
    roomToParents.set(childId, parents);
  });
};

export const getOrphanParents = (roomToParents: RoomToParents, roomId: string): string[] => {
  const parents = getAllParents(roomToParents, roomId);
  const orphanParents = Array.from(parents).filter(
    (parentRoomId) => !roomToParents.has(parentRoomId)
  );

  return orphanParents;
};

const isMutedRule = (rule: PushRuleReading) =>
  // Check for empty actions (new spec) or dont_notify (deprecated)
  (rule.actions.length === 0 || rule.actions[0] === 'dont_notify') &&
  rule.conditions?.[0]?.kind === 'event_match';

const findMutedRule = (overrideRules: PushRuleReading[], roomId: string) =>
  overrideRules.find((rule) => rule.rule_id === roomId && isMutedRule(rule));

export const getNotificationType = (mx: MatrixClientReading, roomId: string): NotificationType => {
  let roomPushRule: PushRuleReading | undefined;
  try {
    roomPushRule = mx.getRoomPushRule('global', roomId);
  } catch {
    roomPushRule = undefined;
  }

  if (!roomPushRule) {
    const overrideRules = mx.getAccountData(AccountDataEvent.PushRules)?.getContent<{
      global?: { override?: PushRuleReading[] };
    }>()?.global?.override;
    if (!overrideRules) return NotificationType.Default;

    return findMutedRule(overrideRules, roomId) ? NotificationType.Mute : NotificationType.Default;
  }

  if (roomPushRule.actions[0] === 'notify') return NotificationType.AllMessages;
  return NotificationType.MentionsAndKeywords;
};

const NOTIFICATION_EVENT_TYPES = [
  'm.room.create',
  'm.room.message',
  'm.room.encrypted',
  'm.room.member',
  'm.sticker',
];
export const isNotificationEvent = (mEvent: MatrixEventReading) => {
  const eType = mEvent.getType();
  if (!NOTIFICATION_EVENT_TYPES.includes(eType)) {
    return false;
  }
  if (eType === 'm.room.member') return false;

  if (mEvent.isRedacted()) return false;
  if (mEvent.getRelation()?.rel_type === 'm.replace') return false;

  return true;
};

export const isRoomMarkedUnread = (room: RoomReading): boolean =>
  room.accountData.get(AccountDataEvent.MarkedUnread)?.getContent<MarkedUnreadContent>().unread ===
  true;

export const getThreadRootEventId = (
  event: MatrixEventReading | undefined
): EventId | undefined => {
  const relation = event?.getRelation();
  return relation?.rel_type === 'm.thread' && typeof relation.event_id === 'string'
    ? relation.event_id
    : undefined;
};

export const getUnreadInfo = (room: RoomReading): UnreadInfo => {
  const total = room.getUnreadNotificationCount('total');
  const highlight = room.getUnreadNotificationCount('highlight');
  return {
    roomId: room.roomId,
    highlight,
    total: highlight > total ? highlight : total,
  };
};

export const getRoomIconSrc = (
  icons: Record<IconName, IconSrc>,
  roomType?: string,
  joinRule?: RoomJoinRulePresentation | null
): IconSrc => {
  if (roomType === RoomType.Space) {
    if (joinRule === 'public') return icons.SpaceGlobe;
    if (joinRule === 'invite' || joinRule === 'knock' || joinRule === 'private') {
      return icons.SpaceLock;
    }
    return icons.Space;
  }

  if (roomType === RoomType.Call) {
    if (joinRule === 'public') return icons.VolumeHighGlobe;
    if (joinRule === 'invite' || joinRule === 'knock' || joinRule === 'private') {
      return icons.VolumeHighLock;
    }
    return icons.VolumeHigh;
  }

  if (joinRule === 'public') return icons.HashGlobe;
  if (joinRule === 'invite' || joinRule === 'knock' || joinRule === 'private') {
    return icons.HashLock;
  }
  return icons.Hash;
};

export const getRoomAvatarUrl = (
  mx: MatrixClientReading,
  room: RoomReading,
  size: 32 | 96 = 32,
  useAuthentication = false
): string | undefined => {
  const mxcUrl = room.getMxcAvatarUrl();
  return mxcUrl
    ? mx.mxcUrlToHttp(mxcUrl, size, size, 'crop', undefined, false, useAuthentication) ?? undefined
    : undefined;
};

export const getDirectRoomAvatarUrl = (
  mx: MatrixClientReading,
  room: RoomReading,
  size: 32 | 96 = 32,
  useAuthentication = false
): string | undefined => {
  const mxcUrl = room.getAvatarFallbackMember()?.getMxcAvatarUrl();

  if (!mxcUrl) {
    return getRoomAvatarUrl(mx, room, size, useAuthentication);
  }

  return (
    mx.mxcUrlToHttp(mxcUrl, size, size, 'crop', undefined, false, useAuthentication) ?? undefined
  );
};

export const trimReplyFromBody = (body: string): string => {
  const match = body.match(/^> <.+?> .+\n(>.*\n)*?\n/m);
  if (!match) return body;
  return body.slice(match[0].length);
};

export const trimReplyFromFormattedBody = (formattedBody: string): string => {
  const suffix = '</mx-reply>';
  const i = formattedBody.lastIndexOf(suffix);
  if (i < 0) {
    return formattedBody;
  }
  return formattedBody.slice(i + suffix.length);
};

export const getMemberDisplayName = (room: RoomReading, userId: UserId): string | undefined => {
  const member = room.getMember(userId);
  const name = member?.rawDisplayName;
  if (name === userId) return undefined;
  return name;
};

type SdkRoomMemberReading = {
  getMxcAvatarUrl(): string | undefined;
  rawDisplayName: string;
  userId: UserId;
};

export const getMemberSearchStr = (
  member: SdkRoomMemberReading | NativeRoomMember,
  query: string,
  mxIdToName: (mxId: string) => string
): string[] => {
  const displayName = !('getMxcAvatarUrl' in member)
    ? member.displayName ?? member.userId
    : member.rawDisplayName;
  return [
    displayName === member.userId ? mxIdToName(member.userId) : displayName,
    query.startsWith('@') || query.indexOf(':') > -1 ? member.userId : mxIdToName(member.userId),
  ];
};

export const getMemberAvatarMxc = (room: RoomReading, userId: UserId): string | undefined => {
  const member = room.getMember(userId);
  return member?.getMxcAvatarUrl();
};

export const isMembershipChanged = (mEvent: MatrixEventReading): boolean =>
  mEvent.getContent().membership !== mEvent.getPrevContent().membership ||
  mEvent.getContent().reason !== mEvent.getPrevContent().reason;

export const getEventEdits = (
  timelineSet: EventTimelineSetReading,
  eventId: EventId,
  eventType: string
): RelationsReading | undefined =>
  timelineSet.relations.getChildEventsForEvent(eventId, 'm.replace', eventType);

const getLatestEdit = (
  targetEvent: MatrixEventReading | NativeEventContentEvent,
  editEvents: MatrixEventReading[]
): MatrixEventReading | undefined => {
  const eventByTargetSender = (rEvent: MatrixEventReading) =>
    rEvent.getSender() ===
    ('getSender' in targetEvent ? targetEvent.getSender() : targetEvent.sender);
  return editEvents.sort((m1, m2) => m2.getTs() - m1.getTs()).find(eventByTargetSender);
};

export const getEditedEvent = (
  mEventId: EventId,
  mEvent: MatrixEventReading | NativeEventContentEvent,
  timelineSet: EventTimelineSetReading
): MatrixEventReading | undefined => {
  const eventType = 'getType' in mEvent ? mEvent.getType() : mEvent.type;
  const edits = getEventEdits(timelineSet, mEventId, eventType);
  return edits && getLatestEdit(mEvent, edits.getRelations());
};

export const canEditEvent = (mx: MatrixClientReading, mEvent: MatrixEventReading) => {
  const content = mEvent.getContent();
  const relationType = content['m.relates_to']?.rel_type;
  return (
    mEvent.getSender() === mx.getUserId() &&
    (!relationType || relationType === 'm.thread') &&
    mEvent.getType() === MessageEvent.RoomMessage &&
    (content.msgtype === 'm.text' ||
      content.msgtype === 'm.emote' ||
      content.msgtype === 'm.notice')
  );
};

export const reactionOrEditEvent = (mEvent: MatrixEventReading) =>
  mEvent.getRelation()?.rel_type === 'm.annotation' ||
  mEvent.getRelation()?.rel_type === 'm.replace';

export const getMentionContent = (userIds: string[], room: boolean) => {
  const mMentions: { user_ids?: string[]; room?: boolean } = {};
  if (userIds.length > 0) {
    mMentions.user_ids = userIds;
  }
  if (room) {
    mMentions.room = true;
  }

  return mMentions;
};

export const getAllVersionsRoomCreator = (room: RoomReading): Set<string> => {
  if (isNativeMatrixSession()) {
    return new Set(getNativeRoomStateProjection(room.roomId)?.creators ?? []);
  }

  const creators = new Set<string>();

  const createEvent = getStateEvent(room, StateEvent.RoomCreate);
  const createContent = createEvent?.getContent<IRoomCreateContent>();
  const creator = createEvent?.getSender();
  if (typeof creator === 'string') creators.add(creator);

  if (createContent && Array.isArray(createContent.additional_creators)) {
    createContent.additional_creators.forEach((c) => {
      if (typeof c === 'string') creators.add(c);
    });
  }

  return creators;
};

export const guessPerfectParent = (
  mx: MatrixClientReading,
  roomId: string,
  parents: string[]
): string | undefined => {
  if (parents.length === 1) {
    return parents[0];
  }

  const getSpecialUsers = (rId: string): string[] => {
    const r = mx.getRoom(rId);
    if (!r) return [];

    if (isNativeMatrixSession()) {
      return getNativeSpecialUsers(getNativeRoomStateProjection(r.roomId));
    }

    const specialUsers: Set<string> = new Set();

    getAllVersionsRoomCreator(r).forEach((c) => specialUsers.add(c));

    const powerLevels = getStateEvent(
      r,
      StateEvent.RoomPowerLevels
    )?.getContent<PowerLevelsReading>();

    const { users_default: usersDefault, users } = powerLevels ?? {};
    const defaultPower = typeof usersDefault === 'number' ? usersDefault : 0;

    if (typeof users === 'object')
      Object.keys(users).forEach((userId) => {
        if (users[userId] > defaultPower) {
          specialUsers.add(userId);
        }
      });

    return Array.from(specialUsers);
  };

  let perfectParent: string | undefined;
  let score = 0;

  const roomSpecialUsers = getSpecialUsers(roomId);
  parents.forEach((parentId) => {
    const parentSpecialUsers = getSpecialUsers(parentId);
    const matchedUsersCount = parentSpecialUsers.filter((userId) =>
      roomSpecialUsers.includes(userId)
    ).length;

    if (matchedUsersCount > score) {
      score = matchedUsersCount;
      perfectParent = parentId;
    }
  });

  return perfectParent;
};
