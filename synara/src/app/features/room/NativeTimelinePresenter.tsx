import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Box, Button, Scroll, Text, config } from 'folds';
import {
  type NativeTimelineViewRow,
  useNativeTimelineView,
} from './nativeTimelineView';

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
  rows.findIndex((row) => rowKey(row) === anchor.itemId || (anchor.eventId && rowEventId(row) === anchor.eventId));

const NativeTimelineRow = ({ row }: { row: NativeTimelineViewRow }) => {
  switch (row.kind) {
    case 'message':
      return (
        <Box direction="Column" gap="100" style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="L400">{row.senderName}</Text>
          <Text size="T400" style={{ whiteSpace: 'pre-wrap' }}>
            {row.body}
          </Text>
        </Box>
      );
    case 'membership':
    case 'state':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">{row.summary}</Text>
        </Box>
      );
    case 'poll':
      return (
        <Box direction="Column" gap="100" style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="L400">{row.question}</Text>
          <Text size="T300">{row.closed ? 'Poll closed' : 'Poll open'}</Text>
        </Box>
      );
    case 'call':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">{row.callKind}</Text>
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
    case 'sticker':
      return (
        <Box style={{ padding: `${config.space.S200} ${config.space.S400}` }}>
          <Text size="T300">Sticker media is unavailable until the native media resolver is ready.</Text>
        </Box>
      );
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
  const [actionError, setActionError] = useState<string>();
  const scrollRef = useRef<HTMLDivElement>(null);
  const rows = controller.state.status === 'ready' ? controller.state.snapshot.rows : [];
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: rows.length,
    getScrollElement: useCallback(() => scrollRef.current, []),
    getItemKey: useCallback((index) => rowKey(rows[index]), [rows]),
    estimateSize: useCallback(() => 64, []),
    overscan: 8,
  });

  const initialPlacementRef = useRef<string>();
  const saveViewport = useCallback(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || rows.length === 0) return;
    const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 8;
    if (atBottom) {
      setNativeTimelineViewport(roomId, { atBottom: true });
      return;
    }
    const visible = virtualizer
      .getVirtualItems()
      .find((item) => item.end > scrollEl.scrollTop);
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
    if (controller.state.status !== 'ready') return undefined;
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const onScroll = () => saveViewport();
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    saveViewport();
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      saveViewport();
    };
  }, [controller.state.status, controller.state.status === 'ready' ? controller.state.snapshot.revision : undefined, saveViewport]);

  useLayoutEffect(() => {
    if (controller.state.status !== 'ready' || rows.length === 0) return undefined;
    const { snapshot, selectedPosition } = controller.state;
    const selectedAnchor =
      selectedPosition.kind === 'focused'
        ? { eventId: selectedPosition.target_event_id, itemId: selectedPosition.target_event_id }
        : selectedPosition.kind === 'unread'
          ? { eventId: selectedPosition.anchor_event_id, itemId: selectedPosition.anchor_event_id }
          : selectedPosition.kind === 'restored' && selectedPosition.anchor_event_id
            ? { eventId: selectedPosition.anchor_event_id, itemId: selectedPosition.anchor_event_id }
            : undefined;
    const placementKey = `${roomId}:${snapshot.sessionGeneration}:${selectedPosition.kind}:${selectedAnchor?.eventId ?? ''}`;
    const initialPlacement = initialPlacementRef.current !== placementKey;
    const savedViewport = nativeTimelineViewports.get(roomId);
    const anchor = initialPlacement && selectedAnchor ? selectedAnchor : savedViewport?.anchor;
    const anchorIndex = anchor ? findAnchorIndex(rows, anchor) : -1;
    if (selectedPosition.kind === 'live_bottom' || (initialPlacement && savedViewport?.atBottom)) {
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
    } else if (anchorIndex >= 0) {
      virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'auto' });
      const offsetPx = initialPlacement && selectedAnchor ? 0 : savedViewport?.anchor?.offsetPx ?? 0;
      const animationFrame = window.requestAnimationFrame(() => {
        if (scrollRef.current && offsetPx !== 0) scrollRef.current.scrollTop += offsetPx;
      });
      initialPlacementRef.current = placementKey;
      return () => window.cancelAnimationFrame(animationFrame);
    }
    initialPlacementRef.current = placementKey;
    return undefined;
  }, [
    controller.state,
    roomId,
    rows,
    virtualizer,
  ]);

  if (controller.state.status === 'unavailable') return null;
  if (controller.state.status === 'loading') {
    return <Text size="T300">Opening native timeline…</Text>;
  }
  if (controller.state.status === 'error') {
    return <Text size="T300">{controller.state.error.message}</Text>;
  }

  const { snapshot } = controller.state;
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
          <Button size="300" onClick={() => runAction(() => controller.setReadState('mark_unread'))}>
            Mark unread
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
                <NativeTimelineRow row={row} />
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
