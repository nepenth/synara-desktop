/* eslint-disable react/destructuring-assignment */
import React, { MouseEventHandler, useMemo } from 'react';
import type { useRoomEvent } from '../../hooks/useRoomEvent';
import type { RoomReading } from '../../utils/room';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { HTMLReactParserOptions } from 'html-react-parser';
import { Avatar, Box, Chip, Header, Icon, Icons, Text, config } from 'folds';
import * as depthCss from '../../styles/Depth.css';
import { Opts as LinkifyOpts } from 'linkifyjs';
import {
  factoryRenderLinkifyWithMention,
  getReactCustomHtmlParser,
  LINKIFY_OPTS,
  makeHighlightRegex,
  makeMentionCustomProps,
  renderMatrixMention,
} from '../../plugins/react-custom-html-parser';
import { getMxIdLocalPart } from '../../utils/matrix';
import { useMatrixEventRenderer } from '../../hooks/useMatrixEventRenderer';
import { GetContentCallback, MessageEvent, StateEvent } from '../../../types/matrix/room';
import {
  AvatarBase,
  ImageContent,
  MSticker,
  ModernLayout,
  RedactedContent,
  Reply,
  Time,
  Username,
  UsernameBold,
} from '../../components/message';
import { RenderMessageContent } from '../../components/RenderMessageContent';
import { Image } from '../../components/media';
import { ImageViewer } from '../../components/image-viewer';
import * as customHtmlCss from '../../styles/CustomHtml.css';
import { RoomAvatar, RoomIcon } from '../../components/room-avatar';
import { getMemberAvatarMxc, getMemberDisplayName, getRoomAvatarUrl } from '../../utils/room';
import { ResultItem } from './useMessageSearch';
import { SequenceCard } from '../../components/sequence-card';
import { UserAvatar } from '../../components/user-avatar';
import { useMentionClickHandler } from '../../hooks/useMentionClickHandler';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { usePowerLevels } from '../../hooks/usePowerLevels';
import { usePowerLevelTags } from '../../hooks/usePowerLevelTags';
import { useTheme } from '../../hooks/useTheme';
import { normalizeRoomJoinRulePresentation } from '../matrix-dto/roomJoinRule';
import { PowerIcon } from '../../components/power';
import colorMXID from '../../../util/colorMXID';
import {
  getPowerTagIconSrc,
  useAccessiblePowerTagColors,
  useGetMemberPowerTag,
} from '../../hooks/useMemberPowerTag';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useRoomCreatorsTag } from '../../hooks/useRoomCreatorsTag';

type SearchResultEvent = {
  type?: string;
  event_id: string;
  room_id: string;
  sender?: string;
  origin_server_ts?: number;
  unsigned?: { redacted_because?: { content: { reason?: string } } };
  content: {
    msgtype?: string;
    body?: string;
    'm.new_content'?: Record<string, unknown>;
    'm.relates_to'?: {
      rel_type?: string;
      event_id?: string;
      'm.in_reply_to'?: { event_id?: string };
    };
    [key: string]: unknown;
  };
};

type SearchResultRoomReading = {
  roomId: string;
  name?: string;
  getType(): string | undefined;
  getJoinRule(): string;
};

type SearchResultGroupProps = {
  room: SearchResultRoomReading;
  highlights: string[];
  items: ResultItem[];
  mediaAutoLoad?: boolean;
  onOpen: (roomId: string, eventId: string) => void;
  legacyUsernameColor?: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
};
export function SearchResultGroup({
  room,
  highlights,
  items,
  mediaAutoLoad,
  onOpen,
  legacyUsernameColor,
  hour24Clock,
  dateFormatString,
}: SearchResultGroupProps) {
  const mx = useMatrixClient();
  const eventedRoom = room as unknown as EventedRoomReading;
  const roomReading = eventedRoom as unknown as RoomReading;
  const useAuthentication = useMediaAuthentication();
  const highlightRegex = useMemo(() => makeHighlightRegex(highlights), [highlights]);

  const powerLevels = usePowerLevels(eventedRoom);
  const creators = useRoomCreators(eventedRoom);

  const creatorsTag = useRoomCreatorsTag();
  const powerLevelTags = usePowerLevelTags(eventedRoom, powerLevels);
  const getMemberPowerTag = useGetMemberPowerTag(eventedRoom, creators, powerLevels);

  const theme = useTheme();
  const accessibleTagColors = useAccessiblePowerTagColors(theme.kind, creatorsTag, powerLevelTags);

  const mentionClickHandler = useMentionClickHandler(room.roomId);

  const linkifyOpts = useMemo<LinkifyOpts>(
    () => ({
      ...LINKIFY_OPTS,
      render: factoryRenderLinkifyWithMention((href) =>
        renderMatrixMention(mx, room.roomId, href, makeMentionCustomProps(mentionClickHandler))
      ),
    }),
    [mx, room, mentionClickHandler]
  );
  const htmlReactParserOptions = useMemo<HTMLReactParserOptions>(
    () =>
      getReactCustomHtmlParser(mx, room.roomId, {
        linkifyOpts,
        highlightRegex,
        useAuthentication,
        handleMentionClick: mentionClickHandler,
      }),
    [mx, room, linkifyOpts, highlightRegex, mentionClickHandler, useAuthentication]
  );

  const renderMatrixEvent = useMatrixEventRenderer<[SearchResultEvent, string, GetContentCallback]>(
    {
      [MessageEvent.RoomMessage]: (event, displayName, getContent) => {
        if (event.unsigned?.redacted_because) {
          return <RedactedContent reason={event.unsigned?.redacted_because.content.reason} />;
        }

        return (
          <RenderMessageContent
            displayName={displayName}
            msgType={event.content.msgtype ?? ''}
            ts={event.origin_server_ts ?? 0}
            getContent={getContent}
            mediaAutoLoad={mediaAutoLoad}
            htmlReactParserOptions={htmlReactParserOptions}
            linkifyOpts={linkifyOpts}
            highlightRegex={highlightRegex}
            outlineAttachment
            agentApprovalTarget={{
              roomId: event.room_id,
              eventId: event.event_id,
            }}
          />
        );
      },
      [MessageEvent.Reaction]: (event, displayName, getContent) => {
        if (event.unsigned?.redacted_because) {
          return <RedactedContent reason={event.unsigned?.redacted_because.content.reason} />;
        }
        return (
          <MSticker
            content={getContent()}
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
      },
      [StateEvent.RoomTombstone]: (event) => {
        const { content } = event;
        return (
          <Box grow="Yes" direction="Column">
            <Text size="T400" priority="300">
              Room Tombstone. {content.body}
            </Text>
          </Box>
        );
      },
    },
    undefined,
    (event) => {
      if (event.unsigned?.redacted_because) {
        return <RedactedContent reason={event.unsigned?.redacted_because.content.reason} />;
      }
      return (
        <Box grow="Yes" direction="Column">
          <Text size="T400" priority="300">
            <code className={customHtmlCss.Code}>{event.type}</code>
            {' event'}
          </Text>
        </Box>
      );
    }
  );

  const handleOpenClick: MouseEventHandler = (evt) => {
    const eventId = evt.currentTarget.getAttribute('data-event-id');
    if (!eventId) return;
    onOpen(room.roomId, eventId);
  };

  return (
    <Box direction="Column" gap="200">
      <Header size="300">
        <Box gap="200" grow="Yes">
          <Avatar size="200" radii="300">
            <RoomAvatar
              roomId={room.roomId}
              src={getRoomAvatarUrl(mx, roomReading, 96, useAuthentication)}
              alt={room.name}
              renderFallback={() => (
                <RoomIcon
                  size="50"
                  roomType={room.getType()}
                  joinRule={normalizeRoomJoinRulePresentation(room.getJoinRule())}
                  filled
                />
              )}
            />
          </Avatar>
          <Text size="H4" truncate>
            {room.name}
          </Text>
        </Box>
      </Header>
      <Box direction="Column" gap="100">
        {items.map((item) => {
          const { event } = item;

          const displayName =
            getMemberDisplayName(roomReading, event.sender) ??
            getMxIdLocalPart(event.sender) ??
            event.sender;
          const senderAvatarMxc = getMemberAvatarMxc(roomReading, event.sender);

          const relation = event.content['m.relates_to'];
          const mainEventId =
            relation?.rel_type === 'm.replace' ? relation.event_id : event.event_id;

          const getContent = (() =>
            event.content['m.new_content'] ?? event.content) as GetContentCallback;

          const replyEventId = relation?.['m.in_reply_to']?.event_id;
          const threadRootId = relation?.rel_type === 'm.thread' ? relation.event_id : undefined;

          const memberPowerTag = getMemberPowerTag(event.sender);
          const tagColor = memberPowerTag?.color
            ? accessibleTagColors?.get(memberPowerTag.color)
            : undefined;
          const tagIconSrc = memberPowerTag?.icon
            ? getPowerTagIconSrc(mx, useAuthentication, memberPowerTag.icon)
            : undefined;

          const usernameColor = legacyUsernameColor ? colorMXID(event.sender) : tagColor;

          return (
            <SequenceCard
              key={event.event_id}
              style={{ padding: config.space.S400 }}
              variant="SurfaceVariant"
              direction="Column"
            >
              <ModernLayout
                before={
                  <AvatarBase>
                    <Avatar size="300">
                      <UserAvatar
                        userId={event.sender}
                        src={
                          senderAvatarMxc
                            ? resolveMatrixThumbnailUrl(mx, senderAvatarMxc, 48, {
                                useAuthentication,
                              })
                            : undefined
                        }
                        alt={displayName}
                        renderFallback={() => <Icon size="200" src={Icons.User} filled />}
                      />
                    </Avatar>
                  </AvatarBase>
                }
              >
                <Box gap="300" justifyContent="SpaceBetween" alignItems="Center" grow="Yes">
                  <Box gap="200" alignItems="Baseline">
                    <Box alignItems="Center" gap="200">
                      <Username style={{ color: usernameColor }}>
                        <Text as="span" truncate>
                          <UsernameBold>{displayName}</UsernameBold>
                        </Text>
                      </Username>
                      {tagIconSrc && <PowerIcon size="100" iconSrc={tagIconSrc} />}
                    </Box>
                    <Time
                      ts={event.origin_server_ts ?? 0}
                      hour24Clock={hour24Clock}
                      dateFormatString={dateFormatString}
                    />
                  </Box>
                  <Box shrink="No" gap="200" alignItems="Center">
                    <Chip
                      className={depthCss.quietInteractiveSurface}
                      data-event-id={mainEventId}
                      onClick={handleOpenClick}
                      variant="Secondary"
                      radii="400"
                    >
                      <Text size="T200">Open</Text>
                    </Chip>
                  </Box>
                </Box>
                {replyEventId && (
                  <Reply
                    room={room as unknown as Parameters<typeof useRoomEvent>[0]}
                    replyEventId={replyEventId}
                    threadRootId={threadRootId}
                    onClick={handleOpenClick}
                    getMemberPowerTag={getMemberPowerTag}
                    accessibleTagColors={accessibleTagColors}
                    legacyUsernameColor={legacyUsernameColor}
                  />
                )}
                {renderMatrixEvent(
                  event.type,
                  false,
                  event as unknown as SearchResultEvent,
                  displayName,
                  getContent
                )}
              </ModernLayout>
            </SequenceCard>
          );
        })}
      </Box>
    </Box>
  );
}
