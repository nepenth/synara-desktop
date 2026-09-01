/* eslint-disable react/destructuring-assignment */
import React, { forwardRef, MouseEventHandler, useCallback, useMemo, useRef } from 'react';
import {
  Avatar,
  Box,
  Chip,
  color,
  config,
  Header,
  Icon,
  IconButton,
  Icons,
  Menu,
  Scroll,
  Spinner,
  Text,
  toRem,
} from 'folds';
import { Opts as LinkifyOpts } from 'linkifyjs';
import { HTMLReactParserOptions } from 'html-react-parser';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRoomPinnedEvents } from '../../../hooks/useRoomPinnedEvents';
import * as css from './RoomPinMenu.css';
import { SequenceCard } from '../../../components/sequence-card';
import { useRoomEvent } from '../../../hooks/useRoomEvent';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';
import {
  AvatarBase,
  DefaultPlaceholder,
  ImageContent,
  MessageNotDecryptedContent,
  MessageUnsupportedContent,
  ModernLayout,
  MSticker,
  RedactedContent,
  Reply,
  Time,
  Username,
  UsernameBold,
} from '../../../components/message';
import { UserAvatar } from '../../../components/user-avatar';
import { getMxIdLocalPart } from '../../../utils/matrix';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { getEditedEvent, getMemberAvatarMxc, getMemberDisplayName } from '../../../utils/room';
import type { EventTimelineSetReading, MatrixEventReading } from '../../../utils/room';
import type { EventedRoomReading } from '../../../utils/roomEvents';
import { GetContentCallback, MessageEvent, StateEvent } from '../../../../types/matrix/room';
import { unpinWithNativeTimelineAction } from '../nativeTimelineAction';
import { useMentionClickHandler } from '../../../hooks/useMentionClickHandler';
import {
  factoryRenderLinkifyWithMention,
  getReactCustomHtmlParser,
  LINKIFY_OPTS,
  makeMentionCustomProps,
  renderMatrixMention,
} from '../../../plugins/react-custom-html-parser';
import { RenderMatrixEvent, useMatrixEventRenderer } from '../../../hooks/useMatrixEventRenderer';
import { RenderMessageContent } from '../../../components/RenderMessageContent';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import * as customHtmlCss from '../../../styles/CustomHtml.css';
import { NativeEventContent, type NativeEventSource } from '../message';
import { Image } from '../../../components/media';
import { ImageViewer } from '../../../components/image-viewer';
import { useRoomNavigate } from '../../../hooks/useRoomNavigate';
import { VirtualTile } from '../../../components/virtualizer';
import { usePowerLevelsContext } from '../../../hooks/usePowerLevels';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { ContainerColor } from '../../../styles/ContainerColor.css';
import { usePowerLevelTags } from '../../../hooks/usePowerLevelTags';
import { useTheme } from '../../../hooks/useTheme';
import { PowerIcon } from '../../../components/power';
import colorMXID from '../../../../util/colorMXID';
import { useIsDirectRoom } from '../../../hooks/useRoom';
import { useRoomCreators } from '../../../hooks/useRoomCreators';
import { useRoomPermissions } from '../../../hooks/useRoomPermissions';
import {
  GetMemberPowerTag,
  getPowerTagIconSrc,
  useAccessiblePowerTagColors,
  useGetMemberPowerTag,
} from '../../../hooks/useMemberPowerTag';
import { useRoomCreatorsTag } from '../../../hooks/useRoomCreatorsTag';
import { resolveMatrixThumbnailUrl } from '../../../matrix/media';
import { IImageContent } from '../../../../types/matrix/common';

/** A js-sdk-backed MatrixEvent as read by the pin menu (renderer receives live SDK events). */
type PinEventReading = MatrixEventReading & {
  getUnsigned(): {
    redacted_because?: { content: { reason?: string; [key: string]: unknown } };
    [key: string]: unknown;
  };
  replacingEvent(): MatrixEventReading | null;
};

/** Room projection with the pin-menu timeline accessors synara already relies on. */
type PinRoomReading = EventedRoomReading & {
  getTimelineForEvent(
    eventId: string
  ): { getTimelineSet(): EventTimelineSetReading; getEvents(): MatrixEventReading[] } | null;
};

type PinnedMessageProps = {
  room: PinRoomReading;
  eventId: string;
  renderContent: RenderMatrixEvent<[PinEventReading, string, GetContentCallback]>;
  onOpen: (roomId: string, eventId: string) => void;
  canPinEvent: boolean;
  getMemberPowerTag: GetMemberPowerTag;
  accessibleTagColors: Map<string, string>;
  legacyUsernameColor: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
};
function PinnedMessage({
  room,
  eventId,
  renderContent,
  onOpen,
  canPinEvent,
  getMemberPowerTag,
  accessibleTagColors,
  legacyUsernameColor,
  hour24Clock,
  dateFormatString,
}: PinnedMessageProps) {
  const pinnedEvent = useRoomEvent(room as unknown as Parameters<typeof useRoomEvent>[0], eventId);
  const useAuthentication = useMediaAuthentication();
  const mx = useMatrixClient();

  const [unpinState, unpin] = useAsyncCallback(
    useCallback(
      () => unpinWithNativeTimelineAction({ roomId: room.roomId, eventId }),
      [room.roomId, eventId]
    )
  );

  const handleOpenClick: MouseEventHandler = (evt) => {
    evt.stopPropagation();
    const evtId = evt.currentTarget.getAttribute('data-event-id');
    if (!evtId) return;
    onOpen(room.roomId, evtId);
  };

  const handleUnpinClick: MouseEventHandler = (evt) => {
    evt.stopPropagation();
    unpin();
  };

  const renderOptions = () => (
    <Box shrink="No" gap="200" alignItems="Center">
      <Chip data-event-id={eventId} onClick={handleOpenClick} variant="Secondary" radii="Pill">
        <Text size="T200">Open</Text>
      </Chip>
      {canPinEvent && (
        <IconButton
          data-event-id={eventId}
          variant="Secondary"
          size="300"
          radii="Pill"
          onClick={unpinState.status === AsyncStatus.Loading ? undefined : handleUnpinClick}
          aria-disabled={unpinState.status === AsyncStatus.Loading}
        >
          {unpinState.status === AsyncStatus.Loading ? (
            <Spinner size="100" />
          ) : (
            <Icon src={Icons.Cross} size="100" />
          )}
        </IconButton>
      )}
    </Box>
  );

  if (pinnedEvent === undefined) return <DefaultPlaceholder variant="Secondary" />;
  if (pinnedEvent === null)
    return (
      <Box gap="300" justifyContent="SpaceBetween" alignItems="Center">
        <Box>
          <Text style={{ color: color.Critical.Main }}>Failed to load message!</Text>
        </Box>
        {renderOptions()}
      </Box>
    );

  const sender = pinnedEvent.getSender()!;
  const displayName = getMemberDisplayName(room, sender) ?? getMxIdLocalPart(sender) ?? sender;
  const senderAvatarMxc = getMemberAvatarMxc(room, sender);
  const getContent = (() => pinnedEvent.getContent()) as GetContentCallback;

  const memberPowerTag = getMemberPowerTag(sender);
  const tagColor = memberPowerTag?.color
    ? accessibleTagColors?.get(memberPowerTag.color)
    : undefined;
  const tagIconSrc = memberPowerTag?.icon
    ? getPowerTagIconSrc(mx, useAuthentication, memberPowerTag.icon)
    : undefined;

  const usernameColor = legacyUsernameColor ? colorMXID(sender) : tagColor;

  return (
    <ModernLayout
      before={
        <AvatarBase>
          <Avatar size="300">
            <UserAvatar
              userId={sender}
              src={
                senderAvatarMxc
                  ? resolveMatrixThumbnailUrl(mx, senderAvatarMxc, 48, { useAuthentication })
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
            ts={pinnedEvent.getTs()}
            hour24Clock={hour24Clock}
            dateFormatString={dateFormatString}
          />
        </Box>
        {renderOptions()}
      </Box>
      {pinnedEvent.replyEventId && (
        <Reply
          room={room as unknown as Parameters<typeof useRoomEvent>[0]}
          replyEventId={pinnedEvent.replyEventId}
          threadRootId={pinnedEvent.threadRootId}
          onClick={handleOpenClick}
          getMemberPowerTag={getMemberPowerTag}
          accessibleTagColors={accessibleTagColors}
          legacyUsernameColor={legacyUsernameColor}
        />
      )}
      {renderContent(pinnedEvent.getType(), false, pinnedEvent, displayName, getContent)}
    </ModernLayout>
  );
}

type RoomPinMenuProps = {
  room: PinRoomReading;
  requestClose: () => void;
  mode?: 'menu' | 'drawer';
};
export const RoomPinMenu = forwardRef<HTMLDivElement, RoomPinMenuProps>(
  ({ room, requestClose, mode = 'menu' }, ref) => {
    const mx = useMatrixClient();
    const userId = mx.getUserId()!;
    const powerLevels = usePowerLevelsContext();
    const creators = useRoomCreators(room);

    const permissions = useRoomPermissions(creators, powerLevels);
    const canPinEvent = permissions.stateEvent(StateEvent.RoomPinnedEvents, userId);
    const canSendReaction = permissions.event(MessageEvent.Reaction, userId);

    const creatorsTag = useRoomCreatorsTag();
    const powerLevelTags = usePowerLevelTags(room, powerLevels);
    const getMemberPowerTag = useGetMemberPowerTag(room, creators, powerLevels);

    const theme = useTheme();
    const accessibleTagColors = useAccessiblePowerTagColors(
      theme.kind,
      creatorsTag,
      powerLevelTags
    );

    const pinnedEvents = useRoomPinnedEvents(room);
    const sortedPinnedEvent = useMemo(() => Array.from(pinnedEvents).reverse(), [pinnedEvents]);
    const useAuthentication = useMediaAuthentication();
    const [mediaAutoLoad] = useSetting(settingsAtom, 'mediaAutoLoad');

    const direct = useIsDirectRoom();
    const [legacyUsernameColor] = useSetting(settingsAtom, 'legacyUsernameColor');

    const [hour24Clock] = useSetting(settingsAtom, 'hour24Clock');
    const [dateFormatString] = useSetting(settingsAtom, 'dateFormatString');

    const { navigateRoom } = useRoomNavigate();
    const scrollRef = useRef<HTMLDivElement>(null);

    const virtualizer = useVirtualizer({
      count: sortedPinnedEvent.length,
      getScrollElement: () => scrollRef.current,
      estimateSize: () => 75,
      overscan: 4,
    });

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
          useAuthentication,
          handleMentionClick: mentionClickHandler,
        }),
      [mx, room, linkifyOpts, mentionClickHandler, useAuthentication]
    );

    const renderMatrixEvent = useMatrixEventRenderer<[PinEventReading, string, GetContentCallback]>(
      {
        [MessageEvent.RoomMessage]: (event, displayName, getContent) => {
          if (event.isRedacted()) {
            return (
              <RedactedContent reason={event.getUnsigned().redacted_because?.content.reason} />
            );
          }

          return (
            <RenderMessageContent
              displayName={displayName}
              msgType={event.getContent().msgtype ?? ''}
              ts={event.getTs()}
              getContent={getContent}
              edited={!!event.replacingEvent()}
              mediaAutoLoad={mediaAutoLoad}
              htmlReactParserOptions={htmlReactParserOptions}
              linkifyOpts={linkifyOpts}
              outlineAttachment
              agentApprovalTarget={{
                roomId: room.roomId,
                eventId: event.getId()!,
                canSendReaction,
              }}
            />
          );
        },
        [MessageEvent.RoomMessageEncrypted]: (event, displayName) => {
          const eventId = event.getId()!;
          const evtTimeline = room.getTimelineForEvent(eventId);

          const mEvent = evtTimeline?.getEvents().find((e) => e.getId() === eventId);

          if (!mEvent || !evtTimeline) {
            return (
              <Box grow="Yes" direction="Column">
                <Text size="T400" priority="300">
                  <code className={customHtmlCss.Code}>{event.getType()}</code>
                  {' event'}
                </Text>
              </Box>
            );
          }

          return (
            <NativeEventContent
              roomId={room.roomId}
              mEvent={mEvent as unknown as NativeEventSource}
            >
              {(resolvedEvent) => {
                if (resolvedEvent.redacted) return <RedactedContent />;
                if (resolvedEvent.type === MessageEvent.Sticker)
                  return (
                    <MSticker
                      content={resolvedEvent.content as unknown as IImageContent}
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
                if (resolvedEvent.type === MessageEvent.RoomMessage) {
                  const editedEvent = getEditedEvent(
                    eventId,
                    resolvedEvent,
                    evtTimeline.getTimelineSet()
                  );
                  const getContent = (() =>
                    editedEvent?.getContent<Record<string, unknown>>()['m.new_content'] ??
                    resolvedEvent.content) as GetContentCallback;

                  const msgType =
                    typeof resolvedEvent.content.msgtype === 'string'
                      ? resolvedEvent.content.msgtype
                      : '';

                  return (
                    <RenderMessageContent
                      displayName={displayName}
                      msgType={msgType}
                      ts={resolvedEvent.originServerTs}
                      edited={!!editedEvent}
                      getContent={getContent}
                      mediaAutoLoad={mediaAutoLoad}
                      htmlReactParserOptions={htmlReactParserOptions}
                      linkifyOpts={linkifyOpts}
                      agentApprovalTarget={{
                        roomId: room.roomId,
                        eventId,
                        canSendReaction,
                      }}
                    />
                  );
                }
                if (resolvedEvent.type === MessageEvent.RoomMessageEncrypted)
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
            </NativeEventContent>
          );
        },
        [MessageEvent.Sticker]: (event, displayName, getContent) => {
          if (event.isRedacted()) {
            return (
              <RedactedContent reason={event.getUnsigned().redacted_because?.content.reason} />
            );
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
      },
      undefined,
      (event) => {
        if (event.isRedacted()) {
          return <RedactedContent reason={event.getUnsigned().redacted_because?.content.reason} />;
        }
        return (
          <Box grow="Yes" direction="Column">
            <Text size="T400" priority="300">
              <code className={customHtmlCss.Code}>{event.getType()}</code>
              {' event'}
            </Text>
          </Box>
        );
      }
    );

    const handleOpen = (roomId: string, eventId: string) => {
      navigateRoom(roomId, eventId);
      requestClose();
    };

    return (
      <Menu ref={ref} className={mode === 'drawer' ? css.PinDrawer : css.PinMenu}>
        <Box grow="Yes" direction="Column">
          <Header className={css.PinMenuHeader} size="500">
            <Box grow="Yes">
              <Text size="H5">Pinned Messages</Text>
            </Box>
            <Box shrink="No">
              <IconButton size="300" onClick={requestClose} radii="300">
                <Icon src={Icons.Cross} size="400" />
              </IconButton>
            </Box>
          </Header>
          <Box grow="Yes">
            <Scroll ref={scrollRef} size="300" hideTrack visibility="Hover">
              <Box className={css.PinMenuContent} direction="Column" gap="100">
                {sortedPinnedEvent.length > 0 ? (
                  <div
                    style={{
                      position: 'relative',
                      height: virtualizer.getTotalSize(),
                    }}
                  >
                    {virtualizer.getVirtualItems().map((vItem) => {
                      const eventId = sortedPinnedEvent[vItem.index];
                      if (!eventId) return null;

                      return (
                        <VirtualTile
                          virtualItem={vItem}
                          style={{ paddingBottom: config.space.S200 }}
                          ref={virtualizer.measureElement}
                          key={vItem.index}
                        >
                          <SequenceCard
                            style={{ padding: config.space.S400, borderRadius: config.radii.R300 }}
                            variant="SurfaceVariant"
                            direction="Column"
                          >
                            <PinnedMessage
                              room={room}
                              eventId={eventId}
                              renderContent={renderMatrixEvent}
                              onOpen={handleOpen}
                              canPinEvent={canPinEvent}
                              getMemberPowerTag={getMemberPowerTag}
                              accessibleTagColors={accessibleTagColors}
                              legacyUsernameColor={legacyUsernameColor || direct}
                              hour24Clock={hour24Clock}
                              dateFormatString={dateFormatString}
                            />
                          </SequenceCard>
                        </VirtualTile>
                      );
                    })}
                  </div>
                ) : (
                  <Box
                    className={ContainerColor({ variant: 'SurfaceVariant' })}
                    style={{
                      marginBottom: config.space.S200,
                      padding: `${config.space.S700} ${config.space.S400} ${toRem(60)}`,
                      borderRadius: config.radii.R300,
                    }}
                    grow="Yes"
                    direction="Column"
                    gap="400"
                    justifyContent="Center"
                    alignItems="Center"
                  >
                    <Icon src={Icons.Pin} size="600" />
                    <Box
                      style={{ maxWidth: toRem(300) }}
                      direction="Column"
                      gap="200"
                      alignItems="Center"
                    >
                      <Text size="H4" align="Center">
                        No Pinned Messages
                      </Text>
                      <Text size="T400" align="Center">
                        Users with sufficient power level can pin a messages from its context menu.
                      </Text>
                    </Box>
                  </Box>
                )}
              </Box>
            </Scroll>
          </Box>
        </Box>
      </Menu>
    );
  }
);
