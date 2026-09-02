import { useMemo } from 'react';
import type { MatrixClientReading, MemberReading, RoomReading } from '../utils/room';
import { useTranslation } from 'react-i18next';
import {
  getDMRoomFor,
  guessDmRoomUserId,
  isRoomAlias,
  isRoomId,
  isServerName,
  isUserId,
  rateLimitedActions,
} from '../utils/matrix';
import { addRoomIdToMDirect, removeRoomIdFromMDirect } from '../features/room/nativeMDirect';
import { useRoomNavigate } from './useRoomNavigate';
import { Membership, StateEvent } from '../../types/matrix/room';
import { getStateEvent } from '../utils/room';
import { splitWithSpace } from '../utils/common';
import { sendPollWithNativeDesktopOwner } from '../features/room/nativePoll';
import { parsePollCommand, type ParsedPoll } from '../utils/polls';
import { getRoomCurrentState } from '../utils/timelineLifecycle';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import { createRoomWithNativeOwner } from '../components/nativeRoomCreateOwner';
import { joinRoomWithNativeOwner } from '../components/nativeRoomJoinOwner';
import { leaveRoomWithNativeOwner } from '../components/nativeRoomLeaveOwner';
import {
  banUserWithNativeOwner,
  inviteUserWithNativeOwner,
  kickUserWithNativeOwner,
  unbanUserWithNativeOwner,
} from '../components/nativeRoomModerationOwner';

type ServerMemberReading = MemberReading & { membership: string };

type CommandsClientReading = MatrixClientReading & {
  getSafeUserId(): string;
  getIgnoredUsers(): string[];
  setIgnoredUsers(ids: string[]): Promise<unknown>;
  sendStateEvent(
    roomId: string,
    eventType: string,
    content: unknown,
    stateKey?: string
  ): Promise<unknown>;
  timestampToEvent(
    roomId: string,
    timestamp: number,
    direction: 'f' | 'b'
  ): Promise<{ event_id: string }>;
  http: {
    authedRequest<T>(method: string, path: string, opts: { limit: number }): Promise<T>;
  };
  createMessagesRequest(
    roomId: string,
    fromToken: string,
    limit: number,
    direction: 'f' | 'b',
    filter?: unknown
  ): Promise<{
    end?: string;
    chunk: {
      type: string;
      sender: string;
      unsigned?: { redacted_because?: unknown };
      event_id: string;
    }[];
  }>;
  redactEvent(
    roomId: string,
    eventId: string,
    txnId?: string,
    opts?: { reason?: string }
  ): Promise<unknown>;
};

type ContextResponseReading = { start?: string; end?: string };

type RoomServerAclEventContent = {
  allow?: string[];
  deny?: string[];
  allow_ip_literals?: boolean;
};

export const SHRUG = '¯\\_(ツ)_/¯';
export const TABLEFLIP = '(╯°□°)╯︵ ┻━┻';
export const UNFLIP = '┬─┬ノ( º_ºノ)';

const FLAG_PAT = '(?:^|\\s)-(\\w+)\\b';
const FLAG_REG = new RegExp(FLAG_PAT);
const FLAG_REG_G = new RegExp(FLAG_PAT, 'g');

export const splitPayloadContentAndFlags = (payload: string): [string, string | undefined] => {
  const flagMatch = payload.match(FLAG_REG);

  if (!flagMatch) {
    return [payload, undefined];
  }
  const content = payload.slice(0, flagMatch.index);
  const flags = payload.slice(flagMatch.index);

  return [content, flags];
};

export const parseFlags = (flags: string | undefined): Record<string, string | undefined> => {
  const result: Record<string, string> = {};
  if (!flags) return result;

  const matches: { key: string; index: number; match: string }[] = [];

  for (let match = FLAG_REG_G.exec(flags); match !== null; match = FLAG_REG_G.exec(flags)) {
    matches.push({ key: match[1], index: match.index, match: match[0] });
  }

  for (let i = 0; i < matches.length; i += 1) {
    const { key, match } = matches[i];
    const start = matches[i].index + match.length;
    const end = i + 1 < matches.length ? matches[i + 1].index : flags.length;
    const value = flags.slice(start, end).trim();
    result[key] = value;
  }

  return result;
};

export const parseUsers = (payload: string): string[] => {
  const users: string[] = [];

  splitWithSpace(payload).forEach((item) => {
    if (isUserId(item)) {
      users.push(item);
    }
  });

  return users;
};

export const parseServers = (payload: string): string[] => {
  const servers: string[] = [];

  splitWithSpace(payload).forEach((item) => {
    if (isServerName(item)) {
      servers.push(item);
    }
  });

  return servers;
};

const getServerMembers = (room: RoomReading, server: string): ServerMemberReading[] => {
  const members = room.getMembers() as ServerMemberReading[];
  return members.filter((member) => member.userId.endsWith(`:${server}`));
};

export const parseTimestampFlag = (input: string): number | undefined => {
  const match = input.match(/^(\d+(?:\.\d+)?)([dhms])$/); // supports floats like 1.5d

  if (!match) {
    return undefined;
  }

  const value = parseFloat(match[1]); // supports decimal values
  const unit = match[2];

  const now = Date.now(); // in milliseconds
  let delta = 0;

  switch (unit) {
    case 'd':
      delta = value * 24 * 60 * 60 * 1000;
      break;
    case 'h':
      delta = value * 60 * 60 * 1000;
      break;
    case 'm':
      delta = value * 60 * 1000;
      break;
    case 's':
      delta = value * 1000;
      break;
    default:
      return undefined;
  }

  const timestamp = now - delta;
  return timestamp;
};

export type CommandExe = (payload: string) => Promise<void>;

export enum Command {
  Me = 'me',
  Notice = 'notice',
  Shrug = 'shrug',
  StartDm = 'startdm',
  Join = 'join',
  Leave = 'leave',
  Invite = 'invite',
  DisInvite = 'disinvite',
  Kick = 'kick',
  Ban = 'ban',
  UnBan = 'unban',
  Ignore = 'ignore',
  UnIgnore = 'unignore',
  MyRoomNick = 'myroomnick',
  MyRoomAvatar = 'myroomavatar',
  ConvertToDm = 'converttodm',
  ConvertToRoom = 'converttoroom',
  TableFlip = 'tableflip',
  UnFlip = 'unflip',
  Delete = 'delete',
  Acl = 'acl',
  Poll = 'poll',
}

export type CommandContent = {
  name: string;
  description: string;
  exe: CommandExe;
};

export type CommandRecord = Record<Command, CommandContent>;

export type PollCommandExecutor = (poll: ParsedPoll) => Promise<void>;

export const useCommands = (
  mx: MatrixClientReading,
  room: RoomReading,
  executePoll?: PollCommandExecutor
): CommandRecord => {
  const c = useMemo(() => mx as unknown as CommandsClientReading, [mx]);
  const { navigateRoom } = useRoomNavigate();
  const { t } = useTranslation();

  const commands: CommandRecord = useMemo(
    () => ({
      [Command.Me]: {
        name: Command.Me,
        description: 'Send action message',
        exe: async () => undefined,
      },
      [Command.Notice]: {
        name: Command.Notice,
        description: 'Send notice message',
        exe: async () => undefined,
      },
      [Command.Shrug]: {
        name: Command.Shrug,
        description: 'Send ¯\\_(ツ)_/¯ as message',
        exe: async () => undefined,
      },
      [Command.TableFlip]: {
        name: Command.TableFlip,
        description: `Send ${TABLEFLIP} as message`,
        exe: async () => undefined,
      },
      [Command.UnFlip]: {
        name: Command.UnFlip,
        description: `Send ${UNFLIP} as message`,
        exe: async () => undefined,
      },
      [Command.StartDm]: {
        name: Command.StartDm,
        description: 'Start direct message with user. Example: /startdm userId1',
        exe: async (payload) => {
          const rawIds = splitWithSpace(payload);
          const userIds = rawIds.filter((id) => isUserId(id) && id !== c.getSafeUserId());
          if (userIds.length === 0) return;
          if (userIds.length === 1) {
            const dmRoomId = getDMRoomFor(mx, userIds[0])?.roomId;
            if (dmRoomId) {
              navigateRoom(dmRoomId);
              return;
            }
          }
          const roomId = await createRoomWithNativeOwner(
            {
              isDirect: true,
              invite: userIds,
              visibility: 'private',
              preset: 'trusted_private_chat',
              encryption: true,
            },
            isSynaraDesktop(),
            (command, args) => invokeDesktopWithAvailability(command, args)
          );
          await addRoomIdToMDirect(roomId, userIds[0]);
          navigateRoom(roomId);
        },
      },
      [Command.Join]: {
        name: Command.Join,
        description: 'Join room with address. Example: /join address1 address2',
        exe: async (payload) => {
          const rawIds = splitWithSpace(payload);
          const roomIdOrAliases = rawIds.filter(
            (idOrAlias) => isRoomId(idOrAlias) || isRoomAlias(idOrAlias)
          );
          await Promise.all(
            roomIdOrAliases.map((idOrAlias) =>
              joinRoomWithNativeOwner(
                idOrAlias,
                undefined,
                isSynaraDesktop(),
                invokeDesktopWithAvailability
              )
            )
          );
        },
      },
      [Command.Leave]: {
        name: Command.Leave,
        description: 'Leave current room.',
        exe: async (payload) => {
          const roomIds =
            payload.trim() === ''
              ? [room.roomId]
              : splitWithSpace(payload).filter((id) => isRoomId(id));
          await Promise.all(
            roomIds.map((id) =>
              leaveRoomWithNativeOwner(id, isSynaraDesktop(), invokeDesktopWithAvailability)
            )
          );
        },
      },
      [Command.Invite]: {
        name: Command.Invite,
        description: 'Invite user to room. Example: /invite userId1 userId2 [-r reason]',
        exe: async (payload) => {
          const [content, flags] = splitPayloadContentAndFlags(payload);
          const users = parseUsers(content);
          const flagToContent = parseFlags(flags);
          const reason = flagToContent.r;
          await rateLimitedActions(users, (id) =>
            inviteUserWithNativeOwner(
              room.roomId,
              id,
              reason,
              isSynaraDesktop(),
              invokeDesktopWithAvailability
            )
          );
        },
      },
      [Command.DisInvite]: {
        name: Command.DisInvite,
        description: 'Disinvite user to room. Example: /disinvite userId1 userId2 [-r reason]',
        exe: async (payload) => {
          const [content, flags] = splitPayloadContentAndFlags(payload);
          const users = parseUsers(content);
          const flagToContent = parseFlags(flags);
          const reason = flagToContent.r;
          await rateLimitedActions(users, (id) =>
            kickUserWithNativeOwner(
              room.roomId,
              id,
              reason,
              isSynaraDesktop(),
              invokeDesktopWithAvailability
            )
          );
        },
      },
      [Command.Kick]: {
        name: Command.Kick,
        description: 'Kick user from room. Example: /kick userId1 userId2 servername [-r reason]',
        exe: async (payload) => {
          const [content, flags] = splitPayloadContentAndFlags(payload);
          const users = parseUsers(content);
          const servers = parseServers(content);
          const flagToContent = parseFlags(flags);
          const reason = flagToContent.r;

          const serverMembers = servers?.flatMap((server) => getServerMembers(room, server));
          const serverUsers = serverMembers
            ?.filter((m) => m.membership !== Membership.Ban)
            .map((m) => m.userId);

          if (Array.isArray(serverUsers)) {
            serverUsers.forEach((user) => {
              if (!users.includes(user)) users.push(user);
            });
          }

          await rateLimitedActions(users, (id) =>
            kickUserWithNativeOwner(
              room.roomId,
              id,
              reason,
              isSynaraDesktop(),
              invokeDesktopWithAvailability
            )
          );
        },
      },
      [Command.Ban]: {
        name: Command.Ban,
        description: 'Ban user from room. Example: /ban userId1 userId2 servername [-r reason]',
        exe: async (payload) => {
          const [content, flags] = splitPayloadContentAndFlags(payload);
          const users = parseUsers(content);
          const servers = parseServers(content);
          const flagToContent = parseFlags(flags);
          const reason = flagToContent.r;

          const serverMembers = servers?.flatMap((server) => getServerMembers(room, server));
          const serverUsers = serverMembers?.map((m) => m.userId);

          if (Array.isArray(serverUsers)) {
            serverUsers.forEach((user) => {
              if (!users.includes(user)) users.push(user);
            });
          }

          await rateLimitedActions(users, (id) =>
            banUserWithNativeOwner(
              room.roomId,
              id,
              reason,
              isSynaraDesktop(),
              invokeDesktopWithAvailability
            )
          );
        },
      },
      [Command.UnBan]: {
        name: Command.UnBan,
        description: 'Unban user from room. Example: /unban userId1 userId2',
        exe: async (payload) => {
          const rawIds = splitWithSpace(payload);
          const users = rawIds.filter((id) => isUserId(id));
          await rateLimitedActions(users, (id) =>
            unbanUserWithNativeOwner(
              room.roomId,
              id,
              isSynaraDesktop(),
              invokeDesktopWithAvailability
            )
          );
        },
      },
      [Command.Ignore]: {
        name: Command.Ignore,
        description: 'Ignore user. Example: /ignore userId1 userId2',
        exe: async (payload) => {
          const rawIds = splitWithSpace(payload);
          const userIds = rawIds.filter((id) => isUserId(id));
          if (userIds.length > 0) {
            let ignoredUsers = c.getIgnoredUsers().concat(userIds);
            ignoredUsers = [...new Set(ignoredUsers)];
            await c.setIgnoredUsers(ignoredUsers);
          }
        },
      },
      [Command.UnIgnore]: {
        name: Command.UnIgnore,
        description: 'Unignore user. Example: /unignore userId1 userId2',
        exe: async (payload) => {
          const rawIds = splitWithSpace(payload);
          const userIds = rawIds.filter((id) => isUserId(id));
          if (userIds.length > 0) {
            const ignoredUsers = c.getIgnoredUsers();
            await c.setIgnoredUsers(ignoredUsers.filter((id) => !userIds.includes(id)));
          }
        },
      },
      [Command.MyRoomNick]: {
        name: Command.MyRoomNick,
        description: 'Change nick in current room.',
        exe: async (payload) => {
          const nick = payload.trim();
          if (nick === '') return;
          const mEvent = getRoomCurrentState(
            room as unknown as Parameters<typeof getRoomCurrentState>[0]
          )?.getStateEvents(StateEvent.RoomMember, c.getSafeUserId());
          const content = mEvent?.getContent();
          if (!content) return;
          await c.sendStateEvent(
            room.roomId,
            StateEvent.RoomMember as any,
            {
              ...content,
              displayname: nick,
            },
            c.getSafeUserId()
          );
        },
      },
      [Command.MyRoomAvatar]: {
        name: Command.MyRoomAvatar,
        description: 'Change profile picture in current room. Example /myroomavatar mxc://xyzabc',
        exe: async (payload) => {
          if (payload.match(/^mxc:\/\/\S+$/)) {
            const mEvent = getRoomCurrentState(
              room as unknown as Parameters<typeof getRoomCurrentState>[0]
            )?.getStateEvents(StateEvent.RoomMember, c.getSafeUserId());
            const content = mEvent?.getContent();
            if (!content) return;
            await c.sendStateEvent(
              room.roomId,
              StateEvent.RoomMember as any,
              {
                ...content,
                avatar_url: payload,
              },
              c.getSafeUserId()
            );
          }
        },
      },
      [Command.ConvertToDm]: {
        name: Command.ConvertToDm,
        description: 'Convert room to direct message',
        exe: async () => {
          const dmUserId = guessDmRoomUserId(room, c.getSafeUserId());
          await addRoomIdToMDirect(room.roomId, dmUserId);
        },
      },
      [Command.ConvertToRoom]: {
        name: Command.ConvertToRoom,
        description: 'Convert direct message to room',
        exe: async () => {
          await removeRoomIdFromMDirect(room.roomId);
        },
      },
      [Command.Delete]: {
        name: Command.Delete,
        description:
          'Delete messages from users. Example: /delete userId1 servername -past 1d|2h|5m|30s [-t m.room.message] [-r spam]',
        exe: async (payload) => {
          const [content, flags] = splitPayloadContentAndFlags(payload);
          const users = parseUsers(content);
          const servers = parseServers(content);

          const flagToContent = parseFlags(flags);
          const reason = flagToContent.r;
          const pastContent = flagToContent.past ?? '';
          const msgTypeContent = flagToContent.t;
          const messageTypes: string[] = msgTypeContent ? splitWithSpace(msgTypeContent) : [];

          const ts = parseTimestampFlag(pastContent);
          if (!ts) return;

          const serverMembers = servers?.flatMap((server) => getServerMembers(room, server));
          const serverUsers = serverMembers?.map((m) => m.userId);

          if (Array.isArray(serverUsers)) {
            serverUsers.forEach((user) => {
              if (!users.includes(user)) users.push(user);
            });
          }

          const result = await c.timestampToEvent(room.roomId, ts, 'f');
          const startEventId = result.event_id;

          const path = `/rooms/${encodeURIComponent(room.roomId)}/context/${encodeURIComponent(
            startEventId
          )}`;
          const eventContext = await c.http.authedRequest<ContextResponseReading>('GET', path, {
            limit: 0,
          });

          let token: string | undefined = eventContext.start;
          while (token) {
            // eslint-disable-next-line no-await-in-loop
            const response = await c.createMessagesRequest(room.roomId, token, 20, 'f', undefined);
            const { end, chunk } = response;
            // remove until the latest event;
            token = end;

            const eventsToDelete = chunk.filter(
              (roomEvent) =>
                (messageTypes.length > 0 ? messageTypes.includes(roomEvent.type) : true) &&
                users.includes(roomEvent.sender) &&
                roomEvent.unsigned?.redacted_because === undefined
            );

            const eventIds = eventsToDelete.map((roomEvent) => roomEvent.event_id);

            // eslint-disable-next-line no-await-in-loop
            await rateLimitedActions(eventIds, (eventId) =>
              c.redactEvent(room.roomId, eventId, undefined, { reason })
            );
          }
        },
      },
      [Command.Acl]: {
        name: Command.Acl,
        description:
          'Manage server access control list. Example /acl [-a servername1] [-d servername2] [-ra servername1] [-rd servername2]',
        exe: async (payload) => {
          const [, flags] = splitPayloadContentAndFlags(payload);

          const flagToContent = parseFlags(flags);
          const allowFlag = flagToContent.a;
          const denyFlag = flagToContent.d;
          const removeAllowFlag = flagToContent.ra;
          const removeDenyFlag = flagToContent.rd;

          const allowList = allowFlag ? splitWithSpace(allowFlag) : [];
          const denyList = denyFlag ? splitWithSpace(denyFlag) : [];
          const removeAllowList = removeAllowFlag ? splitWithSpace(removeAllowFlag) : [];
          const removeDenyList = removeDenyFlag ? splitWithSpace(removeDenyFlag) : [];

          const serverAcl = getStateEvent(
            room,
            StateEvent.RoomServerAcl
          )?.getContent<RoomServerAclEventContent>();

          const aclContent: RoomServerAclEventContent = {
            allow: serverAcl?.allow ? [...serverAcl.allow] : [],
            allow_ip_literals: serverAcl?.allow_ip_literals,
            deny: serverAcl?.deny ? [...serverAcl.deny] : [],
          };

          allowList.forEach((servername) => {
            if (!Array.isArray(aclContent.allow) || aclContent.allow.includes(servername)) return;
            aclContent.allow.push(servername);
          });
          denyList.forEach((servername) => {
            if (!Array.isArray(aclContent.deny) || aclContent.deny.includes(servername)) return;
            aclContent.deny.push(servername);
          });

          aclContent.allow = aclContent.allow?.filter(
            (servername) => !removeAllowList.includes(servername)
          );
          aclContent.deny = aclContent.deny?.filter(
            (servername) => !removeDenyList.includes(servername)
          );

          aclContent.allow?.sort();
          aclContent.deny?.sort();

          await c.sendStateEvent(room.roomId, StateEvent.RoomServerAcl as any, aclContent);
        },
      },
      [Command.Poll]: {
        name: Command.Poll,
        description: t(
          'modernization.poll.command_description',
          'Create a poll. Example: /poll Deploy now? | Yes | No | max=2'
        ),
        exe: async (payload) => {
          const poll = parsePollCommand(payload);
          if (!poll) return;
          if (executePoll) {
            await executePoll(poll);
            return;
          }
          const owner = await sendPollWithNativeDesktopOwner({
            roomId: room.roomId,
            question: poll.question,
            answers: poll.answers.map((answer) => answer.text),
            maxSelections: poll.maxSelections,
          });
          if (owner === 'legacy') {
            throw new Error('Native Matrix session is required to send polls on desktop.');
          }
        },
      },
    }),
    [c, mx, room, navigateRoom, t, executePoll]
  );

  return commands;
};
