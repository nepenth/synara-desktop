import React, { useCallback, useEffect, useState } from 'react';
import { Box, Header, Icon, IconButton, Icons, Line, Scroll, Spinner, Text, config } from 'folds';
import { ContainerColor } from '../../styles/ContainerColor.css';
import * as depthCss from '../../styles/Depth.css';
import { nativeThreadList, type NativeThreadSummary } from './nativeThreadList';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { useRoom } from '../../hooks/useRoom';

type RoomThreadListPanelProps = {
  requestClose: () => void;
};

export function RoomThreadListPanel({ requestClose }: RoomThreadListPanelProps) {
  const room = useRoom();
  const { navigateThread } = useRoomNavigate();
  const [threads, setThreads] = useState<NativeThreadSummary[]>([]);
  const [endReached, setEndReached] = useState(true);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  const applySnapshot = useCallback(async (action: 'open' | 'paginate') => {
    setError(undefined);
    setLoading(true);
    const snapshot = await nativeThreadList(room.roomId, action);
    setLoading(false);
    if (snapshot === 'unavailable') {
      setError('Thread list is unavailable.');
      return;
    }
    setThreads(snapshot.threads);
    setEndReached(snapshot.endReached);
    setTruncated(snapshot.truncated);
  }, [room.roomId]);

  useEffect(() => {
    void applySnapshot('open');
    return () => {
      void nativeThreadList(room.roomId, 'close');
    };
  }, [applySnapshot, room.roomId]);

  return (
    <Box className={ContainerColor({ variant: 'Surface' })} direction="Column" grow="Yes">
      <Header
        size="600"
        data-tauri-drag-region
        style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
      >
        <Box grow="Yes" alignItems="Center" gap="200">
          <Icon src={Icons.Thread} size="300" />
          <Text size="H4">Threads</Text>
        </Box>
        <IconButton
          className={depthCss.quietInteractiveSurface}
          size="300"
          onClick={requestClose}
          radii="300"
          aria-label="Close threads"
        >
          <Icon src={Icons.Cross} size="400" />
        </IconButton>
      </Header>
      <Line variant="Surface" size="300" />
      <Scroll size="300" hideTrack visibility="Hover">
        <Box direction="Column" style={{ padding: config.space.S400 }} gap="200">
          {loading && threads.length === 0 && <Spinner size="400" />}
          {error && (
            <Text size="T300" priority="400">
              {error}
            </Text>
          )}
          {!loading && !error && threads.length === 0 && (
            <Text size="T300" priority="400">
              No threads in this room yet.
            </Text>
          )}
          {threads.map((thread) => (
            <Box
              key={thread.rootEventId}
              as="button"
              type="button"
              direction="Column"
              gap="100"
              style={{
                textAlign: 'left',
                padding: config.space.S200,
                border: 0,
                background: 'transparent',
                cursor: 'pointer',
              }}
              onClick={() => {
                navigateThread(room.roomId, thread.rootEventId);
                requestClose();
              }}
            >
              <Text size="T300" truncate>
                {thread.latestEventId ?? thread.rootEventId}
              </Text>
              <Text size="T200" priority="400">
                {thread.replyCount === 1 ? '1 reply' : `${thread.replyCount} replies`}
                {thread.participated ? ' · participated' : ''}
              </Text>
            </Box>
          ))}
          {truncated && (
            <Text size="T200" priority="400">
              Showing the 256 most recently active threads.
            </Text>
          )}
          {!endReached && (
            <IconButton
              className={depthCss.quietInteractiveSurface}
              onClick={() => void applySnapshot('paginate')}
              disabled={loading}
              aria-label="Load more threads"
            >
              <Text size="T300">Load more</Text>
            </IconButton>
          )}
        </Box>
      </Scroll>
    </Box>
  );
}
