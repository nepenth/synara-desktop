import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { Box, Button, Text } from 'folds';
import { useNativeTimelineView, type NativeTimelineThreadSummary } from './nativeTimelineView';
import { inlineThreadReplies, type InlineThreadReplyCache } from './nativeInlineThreadModel';
import {
  INLINE_THREAD_INITIAL_REPLIES,
  INLINE_THREAD_REVEAL_STEP,
  visibleInlineReplies,
  type ThreadDisplayMode,
} from '../../utils/threadDisplay';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import { NativePlainMessageBody } from './nativeLinkUnfurlCard';
import { NativeTimelineNotice } from './NativeTimelineNotice';
import { nativeTimelineLoadErrorMessage } from './nativeTimelineErrorCopy';
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
  /** Last replies of each expanded root, kept while its row is unmounted. */
  replyCache?: Map<string, InlineThreadReplyCache>;
};

const NativeThreadDisplayContext = createContext<NativeThreadDisplay | undefined>(undefined);

export const NativeThreadDisplayProvider = NativeThreadDisplayContext.Provider;

export const useNativeThreadDisplay = (): NativeThreadDisplay | undefined =>
  useContext(NativeThreadDisplayContext);

/** Expanded-root state for inline threads, owned by the room presenter. */
export const useInlineThreadExpansion = (roomId: string) => {
  const [state, setState] = useState<{
    roomId: string;
    roots: ReadonlySet<string>;
    replyCache: Map<string, InlineThreadReplyCache>;
  }>(() => ({ roomId, roots: new Set(), replyCache: new Map() }));
  // A room switch starts over: no expanded roots, no cached replies.
  if (state.roomId !== roomId) setState({ roomId, roots: new Set(), replyCache: new Map() });
  return useMemo(
    () => ({
      replyCache: state.replyCache,
      isExpanded: (rootEventId: string) => state.roomId === roomId && state.roots.has(rootEventId),
      toggleExpanded: (rootEventId: string) => {
        // Collapsing forgets the replies; expanding again starts fresh.
        state.replyCache.delete(rootEventId);
        setState((current) => {
          if (current.roomId !== roomId) return current;
          const next = new Set(current.roots);
          if (next.has(rootEventId)) next.delete(rootEventId);
          else next.add(rootEventId);
          return { ...current, roots: next };
        });
      },
    }),
    [roomId, state]
  );
};

const EARLIER_REPLIES_FAILED = 'Synara could not load earlier replies. Try again.';

/**
 * Replies of one thread, open while mounted: the stream closes when the row
 * scrolls away or the thread collapses, so inline threads never hold more
 * than the visible expanded ones. The last replies stay in `replyCache`, so a
 * row that scrolls back shows them at once while its stream reopens.
 */
function InlineThreadReplies({
  roomId,
  rootEventId,
  replyCache,
  onReply,
}: {
  roomId: string;
  rootEventId: string;
  replyCache?: Map<string, InlineThreadReplyCache>;
  onReply?: () => void;
}) {
  const input = useMemo(
    () => ({ roomId, position: { kind: 'thread', rootEventId } as const }),
    [roomId, rootEventId]
  );
  const controller = useNativeTimelineView(input);
  const { state, paginate, reload } = controller;
  const [cached] = useState(() => replyCache?.get(rootEventId));
  const [visible, setVisible] = useState(cached?.visible ?? INLINE_THREAD_INITIAL_REPLIES);
  const [loadingEarlier, setLoadingEarlier] = useState(false);
  const [earlierError, setEarlierError] = useState(false);

  const snapshot = state.status === 'ready' ? state.snapshot : undefined;
  const rows = snapshot?.rows ?? cached?.rows;
  const moreOnServer = snapshot
    ? snapshot.pagination.backward === 'available'
    : (cached?.moreOnServer ?? false);

  useEffect(() => {
    if (!replyCache || !snapshot) return;
    replyCache.set(rootEventId, {
      rows: snapshot.rows,
      moreOnServer: snapshot.pagination.backward === 'available',
      visible,
    });
  }, [replyCache, rootEventId, snapshot, visible]);

  const loadEarlier = useCallback(() => {
    setLoadingEarlier(true);
    setEarlierError(false);
    paginate('backwards')
      .then(() => setVisible((count) => count + INLINE_THREAD_REVEAL_STEP))
      .catch(() => setEarlierError(true))
      .finally(() => setLoadingEarlier(false));
  }, [paginate]);

  const loadError =
    state.status === 'error' ? (
      <NativeTimelineNotice
        tone="error"
        message={nativeTimelineLoadErrorMessage(state.error)}
        action={{ label: 'Retry', onClick: reload }}
      />
    ) : null;

  if (!rows) {
    return (
      loadError ?? (
        <Text size="T200" priority="300" className={htmlCss.InlineThreadStatus}>
          Loading replies…
        </Text>
      )
    );
  }

  const replies = inlineThreadReplies(rows, rootEventId);
  const { shown, hidden } = visibleInlineReplies(replies, visible);

  return (
    <Box direction="Column" alignItems="Start" gap="200" className={htmlCss.InlineThreadReplies}>
      {loadError}
      {(hidden > 0 || moreOnServer) && (
        <Button
          size="300"
          variant="Secondary"
          fill="None"
          radii="300"
          disabled={loadingEarlier || (hidden === 0 && !snapshot)}
          onClick={() => {
            if (hidden > 0) setVisible((count) => count + INLINE_THREAD_REVEAL_STEP);
            else loadEarlier();
          }}
        >
          <Text size="B300">
            {loadingEarlier ? 'Loading earlier replies…' : 'Show earlier replies'}
          </Text>
        </Button>
      )}
      {earlierError && (
        <NativeTimelineNotice
          tone="error"
          message={EARLIER_REPLIES_FAILED}
          action={{ label: 'Retry', onClick: loadEarlier, disabled: loadingEarlier }}
          onDismiss={() => setEarlierError(false)}
        />
      )}
      {shown.map((reply) => (
        <Box key={reply.key} direction="Column" gap="100" className={htmlCss.InlineThreadReply}>
          <Text size="T300" className={htmlCss.SenderName}>
            {reply.senderName}
          </Text>
          {reply.formattedBody ? (
            <NativeFormattedBody html={reply.formattedBody} fallbackBody={reply.body} />
          ) : (
            <NativePlainMessageBody
              body={reply.body}
              style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}
            />
          )}
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
          replyCache={display.replyCache}
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
