import React, { ReactNode } from 'react';
import { NativePermalinkProvider } from './nativeTimelineFormattedBody';
import { useNativePermalinkHandler } from './useNativePermalinkHandler';

/**
 * In-app permalinks for message text shown outside a room view (inbox,
 * message search): user mentions open the profile from `roomId`, room and
 * event links navigate, as they do in the timeline.
 */
export function NativeRoomPermalinks({
  roomId,
  children,
}: {
  roomId: string;
  children: ReactNode;
}) {
  const handler = useNativePermalinkHandler(roomId);
  return <NativePermalinkProvider value={handler}>{children}</NativePermalinkProvider>;
}
