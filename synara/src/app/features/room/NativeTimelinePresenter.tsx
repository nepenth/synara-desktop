import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Box, Button, Scroll, Text, config } from 'folds';
import { sanitizeCustomHtml } from '../../utils/sanitize';
import { setNativeComposerReplyDraft } from './nativeComposerDraft';
import { createLaterItemFromIds, upsertLaterWithNativeOwner } from './nativeLaterOwner';
import { toggleReactionWithNativeOwner } from './nativeReactionOwner';
import {
  editTextWithNativeTimelineAction,
  forwardMediaWithNativeTimelineAction,
  forwardTextWithNativeTimelineAction,
  pinWithNativeTimelineAction,
  pollVoteWithNativeTimelineAction,
  redactWithNativeTimelineAction,
  reportWithNativeTimelineAction,
  unpinWithNativeTimelineAction,
} from './nativeTimelineAction';
import {
  callDeclineWithNativeTimelineOwner,
  isNativeTimelineForwardMedia,
} from './nativeTimelineActions';
import {
  nativeTimelineMediaSrc,
  type NativeTimelineRowCapabilities,
  type NativeTimelineViewRow,
  useNativeTimelineView,
} from './nativeTimelineView';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';

type NativeTimelinePresenterProps = {
  roomId: string;
  eventId?: string;
};

type NativeTimelineViewport = {
  atBottom: boolean;
  anchor?: {
    itemId: string;
    eventId?: string;
    offsetPx: number;
  };
};

const NATIVE_TIMELINE_VIEWPORT_LIMIT = 100;
const nativeTimelineViewports = new Map<string, NativeTimelineViewport>();

const setNativeTimelineViewport = (roomId: string, viewport: NativeTimelineViewport) => {
  nativeTimelineViewports.delete(roomId);
  nativeTimelineViewports.set(roomId, viewport);
  if (nativeTimelineViewports.size > NATIVE_TIMELINE_VIEWPORT_LIMIT) {
    const oldestRoomId = nativeTimelineViewports.keys().next().value;
    if (oldestRoomId) nativeTimelineViewports.delete(oldestRoomId);
  }
};

const rowKey = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'sticker') return row.event.itemId;
  return row.itemId;
};

const rowEventId = (row: NativeTimelineViewRow): string | undefined => {
  if (row.kind === 'sticker') return row.event.eventId;
  if ('eventId' in row) return row.eventId;
  return undefined;
};

const findAnchorIndex = (
  rows: NativeTimelineViewRow[],
  anchor: Pick<NonNullable<NativeTimelineViewport['anchor']>, 'itemId' | 'eventId'>
): number =>
  rows.findIndex(
    (row) => rowKey(row) === anchor.itemId || (anchor.eventId && rowEventId(row) === anchor.eventId)
  );

const rowCapabilities = (row: NativeTimelineViewRow): NativeTimelineRowCapabilities | undefined => {
  if (row.kind === 'sticker') return row.event.capabilities;
  if ('capabilities' in row) return row.capabilities;
  return undefined;
};

type NativeTimelineRowProps = {
  row: NativeTimelineViewRow;
  roomId: string;
  onActionError: (message: string) => void;
};

const runNativeRowAction = (
  action: () => Promise<unknown>,
  onActionError: (message: string) => void,
  failureLabel: string
) => {
  void action().catch((error) => {
    onActionError(error instanceof Error ? error.message : failureLabel);
  });
};

const NativeTimelineRowActions = ({
  roomId,
  eventId,
  body,
  rowKind,
  messageType,
  hasMedia,
  capabilities,
  onActionError,
}: {
  roomId: string;
  eventId?: string;
  body?: string;
  rowKind?: NativeTimelineViewRow['kind'];
  messageType?: string;
  hasMedia?: boolean;
  capabilities?: NativeTimelineRowCapabilities;
  onActionError: (message: string) => void;
}) => {
  const [editing, setEditing] = useState(false);
  const [editBody, setEditBody] = useState(body ?? '');
  const [forwarding, setForwarding] = useState(false);
  const [forwardTargetRoomId, setForwardTargetRoomId] = useState('');
  const [forwardAsQuote, setForwardAsQuote] = useState(false);

  useEffect(() => {
    if (!editing) setEditBody(body ?? '');
  }, [body, editing]);

  if (!eventId || !capabilities) return null;
  const buttons: React.ReactNode[] = [];
  if (capabilities.reply) {
    buttons.push(
      <Button
        key="reply"
        size="300"
        fill="Soft"
        onClick={() =>
          runNativeRowAction(
            async () => {
              const result = await setNativeComposerReplyDraft({ roomId, eventId });
              if (result === 'unavailable') {
                throw new Error('Native reply draft is unavailable.');
              }
            },
            onActionError,
            'Native reply draft failed.'
          )
        }
      >
        Reply
      </Button>
    );
  }
  if (capabilities.edit) {
    buttons.push(
      <Button
        key="edit"
        size="300"
        fill="Soft"
        onClick={() => {
          setForwarding(false);
          setEditing((open) => !open);
          setEditBody(body ?? '');
        }}
      >
        {editing ? 'Cancel edit' : 'Edit'}
      </Button>
    );
  }
  if (capabilities.forward) {
    buttons.push(
      <Button
        key="forward"
        size="300"
        fill="Soft"
        onClick={() => {
          setEditing(false);
          setForwarding((open) => !open);
        }}
      >
        {forwarding ? 'Cancel forward' : 'Forward'}
      </Button>
    );
  }
  if (capabilities.redact) {
    buttons.push(
      <Button
        key="redact"
        size="300"
        fill="Soft"
        onClick={() =>
          runNativeRowAction(
            () => redactWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native redact failed.'
          )
        }
      >
        Redact
      </Button>
    );
  }
  if (capabilities.report) {
    buttons.push(
      <Button
        key="report"
        size="300"
        fill="Soft"
        onClick={() =>
          runNativeRowAction(
            () => reportWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native report failed.'
          )
        }
      >
        Report
      </Button>
    );
  }
  if (capabilities.pin) {
    buttons.push(
      <Button
        key="pin"
        size="300"
        fill="Soft"
        onClick={() =>
          runNativeRowAction(
            () => pinWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native pin failed.'
          )
        }
      >
        Pin
      </Button>
    );
    buttons.push(
      <Button
        key="unpin"
        size="300"
        fill="Soft"
        onClick={() =>
          runNativeRowAction(
            () => unpinWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native unpin failed.'
          )
        }
      >
        Unpin
      </Button>
    );
  }
  // Later is a room-event affordance for any remote timeline item with an id.
  buttons.push(
    <Button
      key="later"
      size="300"
      fill="Soft"
      onClick={() =>
        runNativeRowAction(
          () => upsertLaterWithNativeOwner(createLaterItemFromIds(roomId, eventId, 'saved')),
          onActionError,
          'Native later save failed.'
        )
      }
    >
      Save for later
    </Button>
  );
  if (buttons.length === 0) return null;

  const submitEdit = () => {
    const nextBody = editBody.trim();
    if (!nextBody) {
      onActionError('Edited body cannot be empty.');
      return;
    }
    runNativeRowAction(
      async () => {
        await editTextWithNativeTimelineAction({ roomId, eventId, body: nextBody });
        setEditing(false);
      },
      onActionError,
      'Native edit failed.'
    );
  };

  const submitForward = () => {
    const targetRoomId = forwardTargetRoomId.trim();
    if (!targetRoomId) {
      onActionError('Target room id is required to forward.');
      return;
    }
    const useMedia =
      !forwardAsQuote &&
      isNativeTimelineForwardMedia({
        kind: rowKind,
        messageType,
        hasMedia,
      });
    runNativeRowAction(
      async () => {
        if (useMedia) {
          await forwardMediaWithNativeTimelineAction({
            sourceRoomId: roomId,
            eventId,
            targetRoomId,
          });
        } else {
          await forwardTextWithNativeTimelineAction({
            sourceRoomId: roomId,
            eventId,
            targetRoomId,
            asQuote: forwardAsQuote,
          });
        }
        setForwarding(false);
        setForwardTargetRoomId('');
        setForwardAsQuote(false);
      },
      onActionError,
      'Native forward failed.'
    );
  };

  return (
    <Box direction="Column" gap="100">
      <Box gap="100" wrap="Wrap">
        {buttons}
      </Box>
      {editing && (
        <Box direction="Column" gap="100">
          <textarea
            value={editBody}
            onChange={(event) => setEditBody(event.target.value)}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Edit message body"
          />
          <Box gap="100">
            <Button size="300" onClick={submitEdit}>
              Save edit
            </Button>
          </Box>
        </Box>
      )}
      {forwarding && (
        <Box direction="Column" gap="100">
          <input
            value={forwardTargetRoomId}
            onChange={(event) => setForwardTargetRoomId(event.target.value)}
            placeholder="!target:example.org"
            style={{ width: '100%' }}
            aria-label="Forward target room id"
          />
          <label style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <input
              type="checkbox"
              checked={forwardAsQuote}
              onChange={(event) => setForwardAsQuote(event.target.checked)}
            />
            <Text size="T200">Forward as quote</Text>
          </label>
          <Box gap="100">
            <Button size="300" onClick={submitForward}>
              Send forward
            </Button>
          </Box>
        </Box>
      )}
    </Box>
  );
};

const NativeTimelineRow = ({ row, roomId, onActionError }: NativeTimelineRowProps) => {
  const capabilities = rowCapabilities(row);
  const eventId = rowEventId(row);
  const runReaction = (key: string) => {
    if (!eventId || !capabilities?.react) return;
    void toggleReactionWithNativeOwner({ roomId, eventId, key }).catch((error) => {
      onActionError(error instanceof Error ? error.message : 'Native reaction failed.');
    });
  };
  const runDecline = () => {
    if (!eventId || !capabilities?.declineCall) return;
    void callDeclineWithNativeTimelineOwner(
      { roomId, eventId },
      isSynaraDesktop(),
      invokeDesktopWithAvailability
    )
      .then((result) => {
        if (result === 'unavailable') onActionError('Native call decline is unavailable.');
      })
      .catch((error) => {
        onActionError(error instanceof Error ? error.message : 'Native call decline failed.');
      });
  };
  const runPollVote = (answerId: string) => {
    if (!eventId || !capabilities?.vote) return;
    runNativeRowAction(
      () =>
        pollVoteWithNativeTimelineAction({
          roomId,
          eventId,
          answerIds: [answerId],
        }),
      onActionError,
      'Native poll vote failed.'
    );
  };

  switch (row.kind) {
    case 'message': {
      const mediaSrc = row.media ? nativeTimelineMediaSrc(row.media) : undefined;
      const isEmote = row.messageType === 'emote';
      return (
        <Box
          direction="Column"
          gap="100"
          style={{ padding: `${config.space.S200} ${config.space.S400}` }}
        >
          <Text size="L400">{row.senderName}</Text>
          {row.reply && (
            <Box
              direction="Column"
              gap="100"
              style={{
                opacity: 0.8,
                borderLeft: '2px solid currentColor',
                paddingLeft: config.space.S200,
              }}
            >
              <Text size="T200">{row.reply.senderName}</Text>
              <Text size="T200" style={{ whiteSpace: 'pre-wrap' }}>
                {row.reply.body}
              </Text>
            </Box>
          )}
          {row.formattedBody ? (
            <div
              // Defense in depth: re-sanitize Matrix HTML before the unselected shell renders it.
              // eslint-disable-next-line react/no-danger
              dangerouslySetInnerHTML={{ __html: sanitizeCustomHtml(row.formattedBody) }}
              style={{
                whiteSpace: 'pre-wrap',
                fontStyle: isEmote ? 'italic' : undefined,
                fontSize: 'inherit',
              }}
            />
          ) : (
            <Text
              size="T400"
              style={{ whiteSpace: 'pre-wrap', fontStyle: isEmote ? 'italic' : undefined }}
            >
              {isEmote ? `* ${row.body}` : row.body}
            </Text>
          )}
          {row.edited ? (
            <Text size="T200" style={{ opacity: 0.7 }}>
              Edited
            </Text>
          ) : null}
          {row.thread && (
            <Text size="T200" style={{ opacity: 0.8 }}>
              Thread · {row.thread.replyCount} {row.thread.replyCount === 1 ? 'reply' : 'replies'}
              {row.thread.latestEventId ? ` · latest ${row.thread.latestEventId}` : ''}
            </Text>
          )}
          {mediaSrc && row.messageType === 'image' && (
            <img src={mediaSrc} alt={row.body} style={{ maxWidth: '100%', maxHeight: 480 }} />
          )}
          {mediaSrc && row.messageType === 'audio' && (
            // Matrix media metadata does not provide a captions track.
            // eslint-disable-next-line jsx-a11y/media-has-caption
            <audio src={mediaSrc} controls />
          )}
          {mediaSrc && row.messageType === 'video' && (
            // Matrix media metadata does not provide a captions track.
            // eslint-disable-next-line jsx-a11y/media-has-caption
            <video src={mediaSrc} controls style={{ maxWidth: '100%', maxHeight: 480 }} />
          )}
          {mediaSrc && row.messageType === 'file' && (
            <a href={mediaSrc} download>
              {row.body}
            </a>
          )}
          {(row.reactions?.length || capabilities?.react) && (
            <Box gap="100" wrap="Wrap">
              {row.reactions?.map((reaction) => (
                <Button
                  key={reaction.key}
                  size="300"
                  variant={reaction.own ? 'Primary' : 'Secondary'}
                  fill="Soft"
                  disabled={!capabilities?.react}
                  onClick={() => runReaction(reaction.key)}
                >
                  {reaction.key} {reaction.count}
                </Button>
              ))}
              {capabilities?.react && (
                <Button size="300" fill="Soft" onClick={() => runReaction('👍')}>
                  React
                </Button>
              )}
            </Box>
          )}
          <NativeTimelineRowActions
            roomId={roomId}
            eventId={eventId}
            body={row.body}
            rowKind={row.kind}
            messageType={row.messageType}
            hasMedia={Boolean(row.media)}
            capabilities={capabilities}
            onActionError={onActionError}
          />
        </Box>
      );
    }
    case 'membership':
    case 'state':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">{row.summary}</Text>
        </Box>
      );
    case 'poll':
      return (
        <Box
          direction="Column"
          gap="100"
          style={{ padding: `${config.space.S200} ${config.space.S400}` }}
        >
          <Text size="L400">{row.question}</Text>
          <Text size="T300">{row.closed ? 'Poll closed' : 'Poll open'}</Text>
          {(row.answers ?? []).map((answer) => (
            <Button
              key={answer.id}
              size="300"
              variant={answer.own ? 'Primary' : 'Secondary'}
              fill="Soft"
              disabled={row.closed || !capabilities?.vote}
              onClick={() => runPollVote(answer.id)}
            >
              {answer.text} ({answer.voteCount})
            </Button>
          ))}
          {capabilities?.react && (
            <Button size="300" fill="Soft" onClick={() => runReaction('👍')}>
              React
            </Button>
          )}
          <NativeTimelineRowActions
            roomId={roomId}
            eventId={eventId}
            rowKind={row.kind}
            capabilities={capabilities}
            onActionError={onActionError}
          />
        </Box>
      );
    case 'call':
      return (
        <Box
          direction="Column"
          gap="100"
          style={{ padding: `${config.space.S200} ${config.space.S400}` }}
        >
          <Text size="T300">{row.callKind}</Text>
          {capabilities?.declineCall && (
            <Button size="300" fill="Soft" onClick={runDecline}>
              Decline call
            </Button>
          )}
        </Box>
      );
    case 'date_separator':
      return (
        <Box style={{ padding: `${config.space.S300} ${config.space.S400}` }}>
          <Text size="T300">{new Date(row.timestampMs ?? 0).toLocaleDateString()}</Text>
        </Box>
      );
    case 'read_marker':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">Read up to here</Text>
        </Box>
      );
    case 'unread_marker':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">New messages</Text>
        </Box>
      );
    case 'timeline_start':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">Beginning of timeline</Text>
        </Box>
      );
    case 'redacted':
    case 'encrypted_unavailable':
    case 'other':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">{row.summary ?? 'Unsupported timeline event'}</Text>
        </Box>
      );
    case 'sticker': {
      const mediaSrc = nativeTimelineMediaSrc(row.media);
      return (
        <Box
          direction="Column"
          gap="100"
          style={{ padding: `${config.space.S200} ${config.space.S400}` }}
        >
          {mediaSrc ? (
            <img src={mediaSrc} alt="Sticker" style={{ maxWidth: 256, maxHeight: 256 }} />
          ) : (
            <Text size="T300">Sticker media is unavailable.</Text>
          )}
          {capabilities?.react && (
            <Button size="300" fill="Soft" onClick={() => runReaction('👍')}>
              React
            </Button>
          )}
          <NativeTimelineRowActions
            roomId={roomId}
            eventId={eventId}
            rowKind={row.kind}
            hasMedia={Boolean(row.media)}
            capabilities={capabilities}
            onActionError={onActionError}
          />
        </Box>
      );
    }
    case 'pagination':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">{row.state === 'loading' ? 'Loading messages…' : 'More messages'}</Text>
        </Box>
      );
    default:
      return null;
  }
};

/**
 * SDK-neutral, virtualized presentation of the native timeline DTO. This
 * component is intentionally unselected while V-TIMELINE's retained action,
 * media, and viewport paths are incomplete; it is not a legacy fallback.
 */
export function NativeTimelinePresenter({ roomId, eventId }: NativeTimelinePresenterProps) {
  const openingViewport = useMemo(
    () => (eventId ? undefined : nativeTimelineViewports.get(roomId)),
    [eventId, roomId]
  );
  const input = useMemo(
    () => ({
      roomId,
      position: eventId
        ? ({ kind: 'focused', eventId } as const)
        : ({
            kind: 'normal',
            restoredAnchorEventId: openingViewport?.atBottom
              ? undefined
              : openingViewport?.anchor?.eventId,
          } as const),
    }),
    [eventId, openingViewport, roomId]
  );
  const controller = useNativeTimelineView(input);
  const timelineState = controller.state;
  const readyState = timelineState.status === 'ready' ? timelineState : undefined;
  const [actionError, setActionError] = useState<string>();
  const scrollRef = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => readyState?.snapshot.rows ?? [], [readyState?.snapshot.rows]);
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: rows.length,
    getScrollElement: useCallback(() => scrollRef.current, []),
    getItemKey: useCallback(
      (index: number) => {
        const row = rows[index];
        return row ? rowKey(row) : index;
      },
      [rows]
    ),
    estimateSize: useCallback(() => 64, []),
    overscan: 8,
  });

  const initialPlacementRef = useRef<string | undefined>(undefined);
  const saveViewport = useCallback(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || rows.length === 0) return;
    const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 8;
    if (atBottom) {
      setNativeTimelineViewport(roomId, { atBottom: true });
      return;
    }
    const visible = virtualizer.getVirtualItems().find((item) => item.end > scrollEl.scrollTop);
    const row = visible ? rows[visible.index] : undefined;
    if (!visible || !row) return;
    setNativeTimelineViewport(roomId, {
      atBottom: false,
      anchor: {
        itemId: rowKey(row),
        eventId: rowEventId(row),
        offsetPx: scrollEl.scrollTop - visible.start,
      },
    });
  }, [roomId, rows, virtualizer]);

  useEffect(() => {
    if (!readyState) return undefined;
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const onScroll = () => saveViewport();
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    saveViewport();
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      saveViewport();
    };
  }, [readyState, saveViewport]);

  useLayoutEffect(() => {
    if (!readyState || rows.length === 0) return undefined;
    const { snapshot, selectedPosition } = readyState;
    const selectedAnchor =
      selectedPosition.kind === 'focused'
        ? { eventId: selectedPosition.target_event_id, itemId: selectedPosition.target_event_id }
        : selectedPosition.kind === 'unread'
        ? { eventId: selectedPosition.anchor_event_id, itemId: selectedPosition.anchor_event_id }
        : selectedPosition.kind === 'restored' && selectedPosition.anchor_event_id
        ? {
            eventId: selectedPosition.anchor_event_id,
            itemId: selectedPosition.anchor_event_id,
          }
        : undefined;
    const placementKey = `${roomId}:${snapshot.sessionGeneration}:${selectedPosition.kind}:${
      selectedAnchor?.eventId ?? ''
    }`;
    const initialPlacement = initialPlacementRef.current !== placementKey;
    const savedViewport = nativeTimelineViewports.get(roomId);
    const anchor = initialPlacement && selectedAnchor ? selectedAnchor : savedViewport?.anchor;
    const anchorIndex = anchor ? findAnchorIndex(rows, anchor) : -1;
    if (selectedPosition.kind === 'live_bottom' || (initialPlacement && savedViewport?.atBottom)) {
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
    } else if (anchorIndex >= 0) {
      virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'auto' });
      const offsetPx =
        initialPlacement && selectedAnchor ? 0 : savedViewport?.anchor?.offsetPx ?? 0;
      const animationFrame = window.requestAnimationFrame(() => {
        if (scrollRef.current && offsetPx !== 0) scrollRef.current.scrollTop += offsetPx;
      });
      initialPlacementRef.current = placementKey;
      return () => window.cancelAnimationFrame(animationFrame);
    }
    initialPlacementRef.current = placementKey;
    return undefined;
  }, [readyState, roomId, rows, virtualizer]);

  if (timelineState.status === 'unavailable') return null;
  if (timelineState.status === 'loading') {
    return <Text size="T300">Opening native timeline…</Text>;
  }
  if (timelineState.status === 'error') {
    return <Text size="T300">{timelineState.error.message}</Text>;
  }

  if (!readyState) return null;
  const { snapshot } = readyState;
  const runAction = (action: () => Promise<void>) => {
    setActionError(undefined);
    void action().catch((error) => {
      setActionError(error instanceof Error ? error.message : 'Native timeline action failed.');
    });
  };

  return (
    <Box grow="Yes" direction="Column" style={{ minHeight: 0 }}>
      <Box gap="200" style={{ padding: config.space.S200 }}>
        {snapshot.capabilities.markRead && (
          <Button size="300" onClick={() => runAction(() => controller.setReadState('mark_read'))}>
            Mark read
          </Button>
        )}
        {snapshot.capabilities.markUnread && (
          <Button
            size="300"
            onClick={() => runAction(() => controller.setReadState('mark_unread'))}
          >
            Mark unread
          </Button>
        )}
        {snapshot.position.kind !== 'live_bottom' && (
          <Button size="300" onClick={() => runAction(() => controller.jumpLatest())}>
            Jump to latest
          </Button>
        )}
      </Box>
      {actionError && <Text size="T300">{actionError}</Text>}
      {snapshot.capabilities.paginateBackward && snapshot.pagination.backward !== 'exhausted' && (
        <Button size="300" onClick={() => runAction(() => controller.paginate('backwards'))}>
          Load older messages
        </Button>
      )}
      <Scroll ref={scrollRef} visibility="Hover">
        <div style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}>
          {virtualizer.getVirtualItems().map((virtualItem) => {
            const row = rows[virtualItem.index];
            if (!row) return null;
            return (
              <div
                key={virtualItem.key}
                ref={virtualizer.measureElement}
                data-index={virtualItem.index}
                data-native-timeline-row-kind={row.kind}
                data-native-timeline-event-id={rowEventId(row)}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  transform: `translateY(${virtualItem.start}px)`,
                  width: '100%',
                }}
              >
                <NativeTimelineRow row={row} roomId={roomId} onActionError={setActionError} />
              </div>
            );
          })}
        </div>
      </Scroll>
      {snapshot.capabilities.paginateForward && snapshot.pagination.forward !== 'exhausted' && (
        <Button size="300" onClick={() => runAction(() => controller.paginate('forwards'))}>
          Load newer messages
        </Button>
      )}
    </Box>
  );
}
