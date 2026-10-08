import { useState } from 'react';
import { getRecentEmojis } from '../plugins/recent-emoji';
import { IEmoji } from '../plugins/emoji';

/** Recently used emoji (Element's `io.element.recent_emoji` account data). */
export const useRecentEmoji = (limit?: number): IEmoji[] => {
  const [recentEmoji] = useState(() => getRecentEmojis(limit));
  return recentEmoji;
};
