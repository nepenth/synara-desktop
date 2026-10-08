import React, { createContext, useContext, useMemo, useState } from 'react';
import { Box, Button, Text } from 'folds';
import { useNativeTimelineView, type NativeTimelineThreadSummary } from './nativeTimelineView';
import { inlineThreadReplies } from './nativeInlineThreadModel';
import {
  INLINE_THREAD_INITIAL_REPLIES,
  INLINE_THREAD_REVEAL_STEP,
  visibleInlineReplies,
  type ThreadDisplayMode,
} from '../../utils/threadDisplay';
import * as htmlCss from './nativeTimelineHtml.css';

/**
 * Thread presentation for the room timeline. The presenter provides it; row
 * surfaces read it so the inline mode needs no extra prop threading.
 */
export type NativeThreadDisplay = {
  mode: ThreadDisplayMode;
  roomId: string;
  /** Expanded roots survive virtualization unmounting their rows. */
  isExpanded: (rootEventId: string) => boolean;
  toggleExpanded: (rootEventId: string) => void;
  /** Open the thread in the side pane (inline "Reply in thread"). */
  openInPane?: (rootEventId: string, latestEventId?: string) => void;
};

const NativeThreadDisplayContext = createContext<NativeThreadDisplay | undefined>(undefined);

export const NativeThreadDisplayProvider = NativeThreadDisplayContext.Provider;

export const useNativeThreadDisplay = (): NativeThreadDisplay | undefined =>
  useContext(NativeThreadDisplayContext);

/** Expanded-root state for inline threads, owned by the room presenter. */
export const useInlineThreadExpansion = (roomId: string) => {
  const [state, setState] = useState<{ roomId: string; roots: ReadonlySet<string> }>({
    roomId,
    roots: new Set(),
  });
  return useMemo(
    () => ({
      isExpanded: (rootEventId: string) => state.roomId === roomId && state.roots.has(rootEventId),
      toggleExpanded: (rootEventId: string) =>
        setState((current) => {
          const next = new Set(current.roomId === roomId ? current.roots : []);
          if (next.has(rootEventId)) next.delete(rootEventId);
          else next.add(rootEventId);
          return { roomId, roots: next };
        }),
    }),
    [roomId, state]
  );
};

/**
 * Replies of one thread, open while mounted: the stream closes when the row
 * scrolls away or the thread collapses, so inline threads never hold more
 * than the visible expanded ones.
 */
function InlineThreadReplies({
  roomId,
  rootEventId,
  onReply,
}: {
  roomId: string;
  rootEventId: string;
  onReply?: () => void;
}) {
  const input = useMemo(
    () => ({ roomId, position: { kind: 'thread', rootEventId } as const }),
    [roomId, rootEventId]
  );
  const controller = useNativeTimelineView(input);
  const [visible, setVisible] = useState(INLINE_THREAD_INITIAL_REPLIES);
  const { state, paginate } = controller;

  if (state.status !== 'ready') {
    return (
      <Text size="T200" priority="300" className={htmlCss.InlineThreadStatus}>
        {state.status === 'error' ? 'Could not load replies.' : 'Loading replies…'}
      </Text>
    );
  }

  const replies = inlineThreadReplies(state.snapshot.rows, rootEventId);
  const { shown, hidden } = visibleInlineReplies(replies, visible);
  const moreOnServer = state.snapshot.pagination.backward === 'available';

  return (
    <Box direction="Column" alignItems="Start" gap="200" className={htmlCss.InlineThreadReplies}>
      {(hidden > 0 || moreOnServer) && (
        <Button
          size="300"
          variant="Secondary"
          fill="None"
          radii="300"
          onClick={() => {
            if (hidden > 0) setVisible((count) => count + INLINE_THREAD_REVEAL_STEP);
            else void paginate('backwards');
          }}
        >
          <Text size="B300">Show earlier replies</Text>
        </Button>
      )}
      {shown.map((reply) => (
        <Box key={reply.key} direction="Column" gap="100" className={htmlCss.InlineThreadReply}>
          <Text size="T300" className={htmlCss.SenderName}>
            {reply.senderName}
          </Text>
          <Text size="T300" style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>
            {reply.body}
          </Text>
        </Box>
      ))}
      {onReply && (
        <Box>
          <Button size="300" variant="Secondary" fill="Soft" radii="300" onClick={onReply}>
            <Text size="B300">Reply in thread</Text>
          </Button>
        </Box>
      )}
    </Box>
  );
}

/** Collapsed summary plus, when expanded, the thread's replies under the root. */
export function NativeInlineThread({
  display,
  thread,
}: {
  display: NativeThreadDisplay;
  thread: NativeTimelineThreadSummary;
}) {
  const { rootEventId, replyCount } = thread;
  const expanded = display.isExpanded(rootEventId);
  const label = `${replyCount} ${replyCount === 1 ? 'reply' : 'replies'}`;
  return (
    <Box direction="Column" gap="200" data-inline-thread={rootEventId}>
      <Box>
        <Button
          size="300"
          fill="Soft"
          radii="300"
          aria-expanded={expanded}
          onClick={() => display.toggleExpanded(rootEventId)}
        >
          <Text size="B300">{expanded ? `Hide ${label}` : `Thread · ${label}`}</Text>
        </Button>
      </Box>
      {expanded && (
        <InlineThreadReplies
          roomId={display.roomId}
          rootEventId={rootEventId}
          onReply={
            display.openInPane
              ? () => display.openInPane?.(rootEventId, thread.latestEventId)
              : undefined
          }
        />
      )}
    </Box>
  );
}
